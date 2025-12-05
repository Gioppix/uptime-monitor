mod check;
mod fetch;

use crate::{
    collab::{NodePosition, RingRange, get_bucket_for_check},
    database::Database,
    eager_env,
    regions::Region,
    server::TaskUpdateType,
    worker::{
        check::{execute::execute_check, save::ResultSaveManager},
        fetch::{ServiceCheck, fetch_health_checks, fetch_specific_health_checks},
    },
};
use anyhow::Result;
use log::{error, info, trace, warn};
use std::{
    cmp::Ordering,
    collections::{BTreeSet, BinaryHeap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{
        Mutex, Semaphore,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        watch::{self, Receiver},
    },
    time,
};
use uuid::Uuid;

pub use fetch::Method;

const SCHEDULING_TOLERANCE_MILLIS: u64 = 100;

pub struct Task {
    last_execution_start: Option<Instant>,
    details: ServiceCheck,
}

impl Task {
    /// Returns the next scheduled execution time for this task.
    ///
    /// If the task has never been executed (`last_execution_start` is `None`),
    /// returns `now` for immediate execution. Otherwise, calculates the next
    /// execution as `last_execution_start + check_frequency_seconds`, but never
    /// schedules in the past (returns at least `now`).
    fn get_next_execution(&self, now: Instant) -> Instant {
        match self.last_execution_start {
            None => now,
            Some(last_start) => {
                let scheduled =
                    last_start + Duration::from_secs(self.details.check_frequency_seconds as u64);

                if scheduled < now - Duration::from_millis(SCHEDULING_TOLERANCE_MILLIS) {
                    now
                } else {
                    scheduled
                }
            }
        }
    }

    /// Returns the theoretical next execution time for this task.
    ///
    /// This is calculated as `last_execution_start + check_frequency_seconds`,
    /// or `None` if the task has never been executed.
    fn get_theoretical_time(&self) -> Option<Instant> {
        self.last_execution_start
            .map(|t| t + Duration::from_secs(self.details.check_frequency_seconds as u64))
    }
}

#[derive(Clone)]
pub struct WorkerMetadata {
    region: Region,
    bucket_version: i16,
    bucket_count: NodePosition,
}

pub struct Worker {
    database: Arc<Database>,
    metadata: WorkerMetadata,
    range_updates: Receiver<Option<RingRange>>,
    next_executions: Arc<Mutex<BinaryHeap<Task>>>,
    semaphore: Arc<Semaphore>,
    http_client: reqwest::Client,
    save_manager: ResultSaveManager,
    task_updates: UnboundedReceiver<TaskUpdateType>,
}

impl Worker {
    pub async fn new(
        database: Arc<Database>,
        region: Region,
        bucket_version: i16,
        bucket_count: NodePosition,
        range_updates: Receiver<Option<RingRange>>,
        task_updates: UnboundedReceiver<TaskUpdateType>,
    ) -> Result<Self> {
        let instance = Self {
            range_updates,
            metadata: WorkerMetadata {
                region,
                bucket_version,
                bucket_count,
            },
            next_executions: Default::default(),
            semaphore: Arc::new(Semaphore::new(*eager_env::MAX_CONCURRENT_HEALTH_CHECKS)),
            http_client: reqwest::Client::new(),
            save_manager: ResultSaveManager::new(database.clone(), region).await?,
            database,
            task_updates,
        };

        Ok(instance)
    }

    pub fn start(self) -> impl Future<Output = ()> {
        // Clone before moving `self`
        let sync_task_next_executions = self.next_executions.clone();
        let work_task_next_executions = self.next_executions.clone();
        let semaphore = self.semaphore.clone();
        let http_client = self.http_client.clone();
        let save_manager = Arc::new(self.save_manager);
        let mut task_updates = self.task_updates;

        let (queue_update_tx, queue_update_rx) = watch::channel(());

        // Thread that listens to changes
        let metadata_ru = self.metadata.clone();
        let queue_update_tx_ru = queue_update_tx.clone();
        let database_ru = self.database.clone();
        let mut range_updates_ru = self.range_updates.clone();
        let sync_task = tokio::spawn(async move {
            while range_updates_ru.changed().await.is_ok() {
                let range = *range_updates_ru.borrow();

                // Await here so that if the range updates in the meantime values are discarded,
                // except the last one that will be read on the next iteration
                let result = Self::handle_new_range(
                    &metadata_ru,
                    &sync_task_next_executions,
                    &database_ru,
                    range,
                )
                .await;

                if let Err(error) = result {
                    error!("error handling new range: {error}")
                }

                queue_update_tx_ru.send_replace(());
            }
        });

        // Thread that listens to task_updates and fetches/updates tasks
        let metadata_tu = self.metadata.clone();
        let database_tu = self.database.clone();
        let next_executions_tu = self.next_executions.clone();
        let range_updates_tu = self.range_updates.clone();
        let update_task = tokio::spawn(async move {
            while let Some(mut check_ids) = task_updates.recv().await {
                check_ids = Self::filter_check_ids_by_range(check_ids, *range_updates_tu.borrow());

                if check_ids.is_empty() {
                    continue;
                }

                trace!(
                    "Task update received for check_ids (filtered): {:?}",
                    check_ids
                );

                // Fetch the actual ServiceCheck objects from database
                let updated_checks = match fetch_specific_health_checks(
                    &database_tu,
                    metadata_tu.region,
                    &check_ids,
                )
                .await
                {
                    Ok(n) => n,
                    Err(e) => {
                        error!("Failed to fetch updated health checks: {}", e);
                        continue;
                    }
                };

                let mut executions = next_executions_tu.lock().await;
                Worker::update_tasks(&mut executions, &check_ids, updated_checks);
                drop(executions);

                let _ = queue_update_tx.send(());
            }
        });

        let (task_tx, mut task_rx) = mpsc::unbounded_channel();

        let work_task = tokio::spawn(Self::work_task_body(
            work_task_next_executions,
            queue_update_rx,
            task_tx,
        ));

        let save_manager_clone = save_manager.clone();
        let listen_task = tokio::spawn(async move {
            while let Some(task) = task_rx.recv().await {
                let semaphore_clone = semaphore.clone();
                let client_clone = http_client.clone();
                let save_manager_clone = save_manager_clone.clone();

                tokio::spawn(async move {
                    let guard = semaphore_clone.acquire().await.expect("semaphore closed");
                    let result = execute_check(&client_clone, &task, *eager_env::DEV_MODE).await;
                    drop(guard);

                    let result = result.and_then(|r| save_manager_clone.save(r));

                    if let Err(e) = result {
                        error!("error executing check: {e}");
                    }
                });
            }
        });

        info!("Worker started");

        // Return a future. It will not be executed until polled
        async move {
            work_task.abort();
            sync_task.abort();
            listen_task.abort();
            update_task.abort();

            // TODO fix to wait at least the MAXIMUM_TIMEOUT
            tokio::time::sleep(Duration::from_secs(5)).await;

            // This should succeed as other instances are dropped after the abortion
            match Arc::into_inner(save_manager) {
                Some(save_manager) => {
                    save_manager.close().await;
                }
                None => {
                    warn!("someone is still using save_manager");
                }
            }

            info!("Worker stopped");
        }
    }

    /// Spawns the main work loop that executes scheduled health check tasks.
    /// Waits for tasks to become ready based on their scheduled time, executes them,
    /// and reschedules them for their next execution. Responds to queue updates by
    /// re-evaluating the schedule immediately.
    ///
    /// # Parameters
    /// * `next_executions` - Shared priority queue of scheduled tasks
    /// * `queue_update_rx` - Receiver that signals when the task queue has been updated
    /// * `task_tx` - Channel sender for dispatching tasks ready for execution
    async fn work_task_body(
        next_executions: Arc<Mutex<BinaryHeap<Task>>>,
        mut queue_update_rx: Receiver<()>,
        task_tx: UnboundedSender<ServiceCheck>,
    ) {
        loop {
            let (tasks, next_task_time) =
                Self::get_tasks_to_execute_and_reschedule(next_executions.clone(), Instant::now())
                    .await;

            for task in tasks {
                trace!(
                    "Sent health check task for execution: {:?} {}",
                    task.check_name, task.check_frequency_seconds
                );
                let res = task_tx.send(task);
                if let Err(e) = res {
                    error!("error sending task to execution: {e}");
                }
            }

            let wait_duration = match next_task_time {
                Some(next_execution) => next_execution.saturating_duration_since(Instant::now()),
                None => {
                    // In dev mode, "disable" checking by default to spot bugs
                    let default_duration = if *eager_env::DEV_MODE { 100000 } else { 1 };
                    Duration::from_secs(default_duration)
                }
            };

            tokio::select! {
                _ = time::sleep(wait_duration) => {
                    // Time to execute the task
                }
                _ = queue_update_rx.changed() => {
                    // Queue was updated, re-evaluate
                }
            }
        }
    }

    /// Retrieves all tasks that are due for execution (scheduled at or before `now`),
    /// executes them, and reschedules them for their next run based on their frequency.
    ///
    /// Returns a tuple of (tasks to execute, next scheduled execution time).
    ///
    /// `now` is used for consistency in tests,
    async fn get_tasks_to_execute_and_reschedule(
        next_executions: Arc<Mutex<BinaryHeap<Task>>>,
        now: Instant,
    ) -> (Vec<ServiceCheck>, Option<Instant>) {
        let mut executions = next_executions.lock().await;

        let mut tasks_to_execute = Vec::new();
        while let Some(task) = executions.peek() {
            if task.get_next_execution(now) <= now {
                tasks_to_execute.push(executions.pop().expect("peeked"));
            } else {
                break;
            }
        }

        let tasks: Vec<ServiceCheck> = tasks_to_execute
            .into_iter()
            .map(|mut task| {
                // Prevent drift by using the "next execution" (that's in the past since it is not yet updated).
                // By definition it's not more than SCHEDULING_TOLERANCE_MILLIS in the past
                task.last_execution_start = Some(task.get_next_execution(now));

                let details = task.details.clone();
                executions.push(task);

                details
            })
            .collect();

        let next_execution_time = executions.peek().map(|task| task.get_next_execution(now));

        (tasks, next_execution_time)
    }

    async fn handle_new_range(
        metadata: &WorkerMetadata,
        next_executions: &Arc<Mutex<BinaryHeap<Task>>>,
        session: &Database,
        range: Option<RingRange>,
    ) -> Result<()> {
        match range {
            Some(range) => {
                let new_items = fetch_health_checks(
                    session,
                    metadata.region,
                    metadata.bucket_version,
                    range,
                    metadata.bucket_count,
                )
                .await?;

                let mut executions = next_executions.lock().await;
                Self::merge_new_checks(new_items, &mut executions);
            }
            None => {
                let mut executions = next_executions.lock().await;
                executions.clear()
            }
        }

        Ok(())
    }

    fn merge_new_checks(new_items: Vec<ServiceCheck>, heap: &mut BinaryHeap<Task>) {
        let new_item_set: HashSet<_> = new_items.iter().map(|item| item.check_id).collect();

        // Remove tasks that are not present in new_items
        let existing_tasks: Vec<Task> = heap.drain().collect();
        for task in existing_tasks {
            if new_item_set.contains(&task.details.check_id) {
                heap.push(task);
            }
        }

        // Track which items are already scheduled
        // TODO: update other fields
        let scheduled_items: HashSet<_> = heap.iter().map(|task| task.details.check_id).collect();

        // Schedule immediate executions for new items
        for item in new_items {
            if !scheduled_items.contains(&item.check_id) {
                heap.push(Task {
                    last_execution_start: None,
                    details: item,
                });
            }
        }
    }

    /// Updates the task heap by removing deleted tasks and updating/inserting modified tasks.
    ///
    /// # Parameters
    /// * `heap` - The binary heap of tasks to update
    /// * `update_list` - Set of task IDs that were fetched/updated
    /// * `fetched_tasks` - Vector of updated ServiceCheck objects to insert/update
    fn update_tasks(
        heap: &mut BinaryHeap<Task>,
        update_list: &BTreeSet<Uuid>,
        fetched_tasks: Vec<ServiceCheck>,
    ) {
        // Preserve execution times for tasks that are being updated
        let mut preserved_execution_times = std::collections::HashMap::new();

        let existing_tasks: Vec<Task> = heap.drain().collect();
        for task in existing_tasks {
            if update_list.contains(&task.details.check_id) {
                // Task is being updated, preserve its execution time
                preserved_execution_times.insert(task.details.check_id, task.last_execution_start);
            } else {
                // Task is not in update list, keep it as-is
                heap.push(task);
            }
        }

        // Insert/update tasks with preserved execution times where available
        for check in fetched_tasks {
            let last_execution_start = preserved_execution_times
                .get(&check.check_id)
                .copied()
                .flatten();
            heap.push(Task {
                last_execution_start,
                details: check,
            });
        }
    }

    /// Filters check IDs based on the current range assignment.
    /// Returns only check IDs that belong to buckets within the assigned range.
    /// If no range is assigned (None), returns an empty set.
    fn filter_check_ids_by_range(
        check_ids: BTreeSet<Uuid>,
        range: Option<RingRange>,
    ) -> BTreeSet<Uuid> {
        match range {
            Some(range) => check_ids
                .into_iter()
                .filter(|id| range.contains(get_bucket_for_check(*id).1 as u32))
                .collect(),
            None => Default::default(),
        }
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.get_theoretical_time() == other.get_theoretical_time()
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        // None (never executed) have the highest priority
        match (self.get_theoretical_time(), other.get_theoretical_time()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(s), Some(o)) => o.cmp(&s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_work_task_body() {
        let heap = Arc::new(Mutex::new(BinaryHeap::new()));
        let (queue_tx, queue_rx) = watch::channel(());
        let (task_tx, mut task_rx) = mpsc::unbounded_channel();

        let now = Instant::now();

        let check1 = ServiceCheck::example();
        let check1_id = check1.check_id;
        let last_execution_check_1 = now - Duration::from_secs(60);

        let check2 = ServiceCheck::example();
        let last_execution_check_2 = now + Duration::from_secs(100);

        {
            let mut h = heap.lock().await;
            h.push(Task {
                last_execution_start: Some(last_execution_check_1),
                details: check1,
            });
            h.push(Task {
                last_execution_start: Some(last_execution_check_2),
                details: check2,
            });
        }

        let heap_clone = heap.clone();
        let work_handle = tokio::spawn(Worker::work_task_body(heap_clone, queue_rx, task_tx));

        // Give work_task_body time to execute
        time::sleep(Duration::from_millis(50)).await;

        // Check that the past task was sent
        let received = task_rx.try_recv();
        assert_eq!(received.unwrap().check_id, check1_id);

        // Verify the task was rescheduled
        {
            let h = heap.lock().await;
            assert_eq!(h.len(), 2);
        }

        // Add a new task that should have been executed 1 second ago
        let check_immediate = ServiceCheck::example();
        let check_immediate_id = check_immediate.check_id;

        {
            let mut h = heap.lock().await;
            h.push(Task {
                last_execution_start: None,
                details: check_immediate,
            });
        }

        // Send update notification to trigger re-evaluation
        queue_tx.send_replace(());

        // Give work_task_body time to process the new task
        time::sleep(Duration::from_millis(50)).await;

        // Verify that the immediate task was executed
        let received_immediate = task_rx.try_recv();
        assert_eq!(received_immediate.unwrap().check_id, check_immediate_id);

        work_handle.abort();
    }

    #[tokio::test]
    async fn check_new_range() -> Result<()> {
        let (session, _keyspace) = create_test_database(Some(FIXTURES)).await?;
        let session = Arc::new(session);

        let (_tx, rx) = watch::channel(None);
        let (_tx, task_update_rx) = mpsc::unbounded_channel();
        let worker = Worker::new(session.clone(), Region::Hel1, 1, 10, rx, task_update_rx).await?;

        let check1_id = uuid!("00000000-0000-0000-0000-000000000001");
        let check2_id = uuid!("00000000-0000-0000-0000-000000000002");
        let check3_id = uuid!("99999999-9999-9999-9999-999999999999");

        let now = Instant::now();
        let scheduled_time_1 = now + Duration::from_secs(100);
        let scheduled_time_2 = now + Duration::from_secs(200);
        let scheduled_time_3 = now + Duration::from_secs(300);

        // Insert 3 tasks: 2 that will be in fixtures, 1 that won't
        {
            let mut heap = worker.next_executions.lock().await;
            let mut check1 = ServiceCheck::example();
            check1.check_id = check1_id;
            heap.push(Task {
                last_execution_start: Some(scheduled_time_1),
                details: check1,
            });

            let mut check2 = ServiceCheck::example();
            check2.check_id = check2_id;
            heap.push(Task {
                last_execution_start: Some(scheduled_time_2),
                details: check2,
            });

            let mut check3 = ServiceCheck::example();
            check3.check_id = check3_id;
            heap.push(Task {
                last_execution_start: Some(scheduled_time_3),
                details: check3,
            });
        }

        // Test with Some range
        let range = RingRange { start: 0, end: 3 };
        Worker::handle_new_range(
            &worker.metadata,
            &worker.next_executions,
            &session,
            Some(range),
        )
        .await?;

        {
            let heap = worker.next_executions.lock().await;
            assert_eq!(heap.len(), 3);

            let tasks: Vec<&Task> = heap.iter().collect();
            let task1 = tasks.iter().find(|t| t.details.check_id == check1_id);
            let task2 = tasks.iter().find(|t| t.details.check_id == check2_id);

            assert!(task1.is_some(), "Task 1 should be preserved");
            assert!(task2.is_some(), "Task 2 should be preserved");
            assert_eq!(
                task1.unwrap().last_execution_start,
                Some(scheduled_time_1),
                "Task 1 execution time should be preserved"
            );
            assert_eq!(
                task2.unwrap().last_execution_start,
                Some(scheduled_time_2),
                "Task 2 execution time should be preserved"
            );
        }

        // Test with None range (should clear)
        Worker::handle_new_range(&worker.metadata, &worker.next_executions, &session, None).await?;

        {
            let heap = worker.next_executions.lock().await;
            assert_eq!(heap.len(), 0);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_get_tasks_to_execute_and_reschedule_simple() {
        let heap = Arc::new(Mutex::new(BinaryHeap::new()));

        let now = Instant::now();

        let mut check1 = ServiceCheck::example();
        check1.check_id = uuid!("00000000-0000-0000-0000-000000000001");
        let check1_id = check1.check_id;
        let last_exec_check_1 = now - Duration::from_secs(60);

        let mut check2 = ServiceCheck::example();
        check2.check_id = uuid!("00000000-0000-0000-0000-000000000002");
        let check2_id = check2.check_id;
        let last_exec_check_2 = now - Duration::from_secs(65);

        let mut check3 = ServiceCheck::example();
        check3.check_id = uuid!("00000000-0000-0000-0000-000000000003");
        let last_exec_check_3 = now - Duration::from_secs(10);

        let mut check4 = ServiceCheck::example();
        check4.check_id = uuid!("00000000-0000-0000-0000-000000000004");
        let last_exec_check_4 = now - Duration::from_secs(20);

        {
            let mut h = heap.lock().await;
            h.push(Task {
                last_execution_start: Some(last_exec_check_1),
                details: check1,
            });
            h.push(Task {
                last_execution_start: Some(last_exec_check_2),
                details: check2,
            });
            h.push(Task {
                last_execution_start: Some(last_exec_check_3),
                details: check3,
            });
            h.push(Task {
                last_execution_start: Some(last_exec_check_4),
                details: check4,
            });
        }

        let (mut tasks, next_time) =
            Worker::get_tasks_to_execute_and_reschedule(heap.clone(), now).await;

        tasks.sort_by_key(|t| t.check_id);

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].check_id, check1_id);
        assert_eq!(tasks[1].check_id, check2_id);
        assert_eq!(next_time, Some(now + Duration::from_secs(40)));

        // Verify tasks were rescheduled back into the heap
        let h = heap.lock().await;
        assert_eq!(h.len(), 4);
    }

    #[tokio::test]
    async fn test_get_tasks_to_execute_and_reschedule_two_tasks() {
        let heap = Arc::new(Mutex::new(BinaryHeap::new()));

        let now = Instant::now();

        let mut check1 = ServiceCheck::example();
        check1.check_frequency_seconds = 100;
        let mut check2 = ServiceCheck::example();
        check2.check_frequency_seconds = 200;
        let mut check3 = ServiceCheck::example();
        check3.check_frequency_seconds = 300;

        {
            let mut h = heap.lock().await;
            h.push(Task {
                last_execution_start: Some(now - Duration::from_secs(101)),
                details: check1,
            });
            h.push(Task {
                last_execution_start: Some(now - Duration::from_secs(200)),
                details: check2,
            });
            h.push(Task {
                last_execution_start: Some(now - Duration::from_secs(100)),
                details: check3,
            });
        }

        let (tasks, next_time) =
            Worker::get_tasks_to_execute_and_reschedule(heap.clone(), now).await;

        assert_eq!(tasks.len(), 2);
        // The next execution is of one of the tasks just executed given its frequency
        assert_eq!(next_time, Some(now + Duration::from_secs(100)));
    }

    #[tokio::test]
    async fn test_task_ordering() {
        let now = Instant::now();

        let mut tasks = vec![
            Task {
                last_execution_start: None,
                details: ServiceCheck {
                    check_id: uuid!("00000000-0000-0000-0000-000000000001"),
                    ..ServiceCheck::example()
                },
            },
            Task {
                last_execution_start: Some(now - Duration::from_secs(59)),
                details: ServiceCheck {
                    check_id: uuid!("00000000-0000-0000-0000-000000000002"),
                    check_frequency_seconds: 60,
                    ..ServiceCheck::example()
                },
            },
            Task {
                last_execution_start: Some(now - Duration::from_secs(28)),
                details: ServiceCheck {
                    check_id: uuid!("00000000-0000-0000-0000-000000000003"),
                    check_frequency_seconds: 30,
                    ..ServiceCheck::example()
                },
            },
        ];

        tasks.sort();

        let ids: Vec<_> = tasks.iter().map(|t| t.details.check_id).collect();
        assert_eq!(
            ids,
            vec![
                uuid!("00000000-0000-0000-0000-000000000003"),
                uuid!("00000000-0000-0000-0000-000000000002"),
                uuid!("00000000-0000-0000-0000-000000000001"),
            ]
        );

        let mut heap = BinaryHeap::new();
        for task in tasks {
            heap.push(task);
        }

        let first = heap.pop().unwrap();
        assert_eq!(
            first.details.check_id,
            uuid!("00000000-0000-0000-0000-000000000001")
        );
    }

    fn create_check(
        num: u128,
        has_execution: bool,
        heap: &mut BinaryHeap<Task>,
    ) -> (uuid::Uuid, Option<Instant>) {
        let now = Instant::now();
        let check_id = uuid::Uuid::from_u128(num);
        let check_last_execution = if has_execution {
            Some(now - Duration::from_secs(10 * num as u64))
        } else {
            None
        };
        let mut check = ServiceCheck::example();
        check.check_id = check_id;
        heap.push(Task {
            last_execution_start: check_last_execution,
            details: check,
        });
        (check_id, check_last_execution)
    }

    #[tokio::test]
    async fn test_update_tasks() {
        let mut heap = BinaryHeap::new();

        let (check1_id, check1_last_execution) = create_check(1, true, &mut heap);
        let (check2_id, _check2_last_execution) = create_check(2, true, &mut heap);
        let (check3_id, check3_last_execution) = create_check(3, false, &mut heap);

        // Update check1 (preserve time), delete check2 (not in fetched), keep check3 (not in update list)
        let mut update_list = BTreeSet::new();
        update_list.insert(check1_id);
        update_list.insert(check2_id);

        let mut updated_check1 = ServiceCheck::example();
        updated_check1.check_id = check1_id;
        updated_check1.check_frequency_seconds = 999;

        Worker::update_tasks(&mut heap, &update_list, vec![updated_check1]);

        assert_eq!(heap.len(), 2);

        let tasks: Vec<Task> = heap.drain().collect();
        assert_eq!(tasks.len(), 2);
        let task1 = tasks
            .iter()
            .find(|t| t.details.check_id == check1_id)
            .unwrap();
        let task3 = tasks
            .iter()
            .find(|t| t.details.check_id == check3_id)
            .unwrap();

        assert_eq!(task1.last_execution_start, check1_last_execution);
        assert_eq!(task1.details.check_frequency_seconds, 999);
        assert_eq!(task3.last_execution_start, check3_last_execution);
    }

    /// Attention: this uses CURRENT_BUCKETS_COUNT env; it supposes it's greater than 3
    #[tokio::test]
    async fn test_filter_check_ids_by_range() {
        let check1_id = uuid!("00000000-0000-0000-0000-000000000001");
        let check2_id = uuid!("00000000-0000-0000-0000-000000000002");
        let check3_id = uuid!("00000000-0000-0000-0000-000000000003");

        let mut check_ids = BTreeSet::new();
        check_ids.insert(check1_id);
        check_ids.insert(check2_id);
        check_ids.insert(check3_id);

        let range1 = RingRange { start: 0, end: 2 };
        let filtered = Worker::filter_check_ids_by_range(check_ids.clone(), Some(range1));
        assert_eq!(filtered, BTreeSet::from([check1_id]));

        let range2 = RingRange { start: 0, end: 5 };
        let filtered = Worker::filter_check_ids_by_range(check_ids.clone(), Some(range2));
        assert_eq!(filtered, BTreeSet::from([check1_id, check2_id, check3_id]));

        let range3 = RingRange { start: 2, end: 4 };
        let filtered = Worker::filter_check_ids_by_range(check_ids.clone(), Some(range3));
        assert_eq!(filtered, BTreeSet::from([check2_id, check3_id]));

        // Test with None range
        let filtered = Worker::filter_check_ids_by_range(check_ids.clone(), None);
        assert!(filtered.is_empty());
    }
}
