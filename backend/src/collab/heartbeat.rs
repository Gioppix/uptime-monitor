use crate::collab::assignment::NodePosition;
use crate::database::Database;
use crate::database::preparer::CachedPreparedStatement;
use crate::eager_env::{PORT, SELF_IP};
use crate::regions::Region;
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, watch};
use uuid::Uuid;

const HEARTBEAT_FRESHNESS_MULTIPLE: u32 = 2;

/// Returns the bucket's number (UTC minute)
fn get_time_bucket_minutes(timestamp: DateTime<Utc>) -> i64 {
    timestamp.timestamp() / 60
}

static INSERT_HEARTBEAT_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO workers_heartbeats (region,
                                    time_bucket_minutes,
                                    timestamp,
                                    position,
                                    process_id,
                                    address)
    VALUES (?, ?, ?, ?, ?, ?)
    ",
);

async fn insert_heartbeat(
    session: &Database,
    region: Region,
    process_id: Uuid,
    position: NodePosition,
    timestamp: DateTime<Utc>,
) -> Result<()> {
    let time_bucket = get_time_bucket_minutes(timestamp);

    INSERT_HEARTBEAT_QUERY
        .execute_unpaged(
            session,
            (
                region.to_identifier(),
                time_bucket,
                timestamp,
                position as i32,
                process_id,
                format!("{}:{}", *SELF_IP, *PORT),
            ),
        )
        .await?;

    Ok(())
}

static INSERT_WORKER_METADATA_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO workers_metadata (process_id,
                                  replica_id,
                                  git_sha)
    VALUES (?, ?, ?)
    ",
);

async fn insert_worker_metadata(
    session: &Database,
    process_id: Uuid,
    replica_id: Option<&str>,
    git_sha: Option<&str>,
) -> Result<()> {
    INSERT_WORKER_METADATA_QUERY
        .execute_unpaged(session, (process_id, replica_id, git_sha))
        .await?;

    Ok(())
}

static GET_ALIVE_WORKERS_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT process_id,
           position,
           timestamp,
           address,
           region
    FROM workers_heartbeats
    WHERE region IN ?
      AND time_bucket_minutes = ?
      AND timestamp >= ?
    ",
);

fn parse_heartbeat_row(
    process_id: Uuid,
    position: i32,
    socket_addr: Option<String>,
    region: String,
) -> Result<Heartbeat> {
    if position < 0 {
        bail!("cannot have negative position");
    }

    let region = Region::from_identifier(&region)?;

    Ok(Heartbeat {
        node_id: process_id,
        position: position as u32,
        socket_address: socket_addr.and_then(|s| s.parse().ok()),
        region,
    })
}

async fn fetch_alive_workers_within_interval(
    session: &Database,
    regions: &[Region],
    within_duration: Duration,
) -> Result<BTreeSet<Heartbeat>> {
    let now = Utc::now();
    let cutoff = now - within_duration;
    let current_bucket = get_time_bucket_minutes(now);
    let cutoff_bucket = get_time_bucket_minutes(cutoff);

    let mut alive_workers = BTreeSet::new();
    let mut latest_heartbeats = HashMap::new();

    // Query all buckets from cutoff_bucket to current_bucket (inclusive)
    for bucket in cutoff_bucket..=current_bucket {
        let rows = GET_ALIVE_WORKERS_QUERY
            .execute_unpaged(
                session,
                (
                    regions
                        .iter()
                        .map(|r| r.to_identifier())
                        .collect::<Vec<_>>(),
                    bucket,
                    cutoff,
                ),
            )
            .await?
            .into_rows_result()?;

        for row in rows.rows::<(Uuid, i32, DateTime<Utc>, Option<String>, String)>()? {
            let row_result = row.map_err(anyhow::Error::new).and_then(
                |(process_id, position, timestamp, socket_addr, region)| {
                    let heartbeat = parse_heartbeat_row(process_id, position, socket_addr, region)?;
                    Ok((process_id, timestamp, heartbeat))
                },
            );

            let (process_id, timestamp, heartbeat) = match row_result {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to parse heartbeat row: {}", e);
                    continue;
                }
            };

            // Keep only the most recent heartbeat per node_id
            match latest_heartbeats.entry(process_id) {
                Entry::Occupied(mut entry) => {
                    let (latest_timestamp, latest_heartbeat) = entry.get_mut();
                    if timestamp > *latest_timestamp {
                        *latest_timestamp = timestamp;
                        *latest_heartbeat = heartbeat;
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert((timestamp, heartbeat));
                }
            }
        }
    }

    // Extract only the heartbeats (not timestamps) into the result set
    for (_, (_, heartbeat)) in latest_heartbeats {
        alive_workers.insert(heartbeat);
    }

    Ok(alive_workers)
}

pub struct HeartbeatManager {
    process_id: Uuid,
    region: Region,
    interval: Duration,
    session: Arc<Database>,
    /// Includes all regions.
    /// Comprised of `(last_fetched_at, alive_nodes)`.
    last_alive_nodes: Arc<Mutex<Option<(Instant, AliveNodes)>>>,
}

impl HeartbeatManager {
    pub async fn new(
        process_id: Uuid,
        region: Region,
        interval: Duration,
        session: Arc<Database>,
    ) -> Result<Self> {
        INSERT_HEARTBEAT_QUERY
            .optimistically_prepare(&session)
            .await?;
        INSERT_WORKER_METADATA_QUERY
            .optimistically_prepare(&session)
            .await?;
        GET_ALIVE_WORKERS_QUERY
            .optimistically_prepare(&session)
            .await?;

        insert_worker_metadata(&session, process_id, None, None).await?;

        Ok(Self {
            process_id,
            region,
            interval,
            session,
            last_alive_nodes: Default::default(),
        })
    }

    pub async fn start(
        &self,
        position: NodePosition,
    ) -> Result<(watch::Receiver<AliveNodes>, impl Future<Output = ()>)> {
        let process_id = self.process_id;
        let region = self.region;
        let interval = self.interval;

        let initial_alive_nodes = self.get_alive_workers_all_regions().await?;

        let (sender, alive_nodes_receiver) = watch::channel(initial_alive_nodes);

        let heartbeat_task_session = self.session.clone();
        let heartbeat_task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;

                let timestamp = Utc::now();
                let result = insert_heartbeat(
                    &heartbeat_task_session,
                    region,
                    process_id,
                    position,
                    timestamp,
                )
                .await;

                if let Err(e) = result {
                    error!("failed to send heartbeat: {e}");
                }
            }
        });

        let state_task_session = self.session.clone();

        let monitor_state_task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;

                let result = fetch_alive_workers_within_interval(
                    &state_task_session,
                    &[region],
                    interval * HEARTBEAT_FRESHNESS_MULTIPLE,
                )
                .await;

                match result {
                    Ok(alive_nodes) => {
                        // We use `send` and not other infallible methods to know whether all receivers were dropped (should not happen)
                        if let Err(e) = sender.send(alive_nodes) {
                            error!("failed to send alive nodes update: {e}");
                        }
                    }
                    Err(e) => {
                        error!("failed to get alive workers: {e}");
                    }
                }
            }
        });

        info!("HeartbeatManager started");

        let close_future = async move {
            info!("Starting stopping of heartbeat");

            heartbeat_task.abort();
            monitor_state_task.abort();

            info!("Stopped heartbeat");
        };

        Ok((alive_nodes_receiver, close_future))
    }

    pub async fn get_alive_workers_all_regions(&self) -> Result<AliveNodes> {
        // Hold the lock during the fetch so that only one thread fetches
        let mut lock = self.last_alive_nodes.lock().await;

        if let Some((last_fetched_at, cached)) = lock.as_ref() {
            // Check if the cached data is still fresh (within self.interval)
            if last_fetched_at.elapsed() < self.interval {
                return Ok(cached.clone());
            }
        }

        // Invalidate the data; this is only effective if the fetch fails
        *lock = None;

        let all_regions: Vec<Region> = Region::iter().collect();

        // Double the interval
        let alive_nodes = fetch_alive_workers_within_interval(
            &self.session,
            &all_regions,
            self.interval * HEARTBEAT_FRESHNESS_MULTIPLE,
        )
        .await?;

        *lock = Some((Instant::now(), alive_nodes.clone()));

        Ok(alive_nodes)
    }

    pub async fn get_alive_workers_same_region(&self) -> Result<AliveNodes> {
        fetch_alive_workers_within_interval(&self.session, &[self.region], self.interval * 2).await
    }

    #[cfg(test)]
    pub async fn register_nodes(&self, nodes: &[Heartbeat]) {
        let mut lock = self.last_alive_nodes.lock().await;
        let (_instant, set) = lock.get_or_insert_with(|| (Instant::now(), BTreeSet::new()));
        for node in nodes {
            set.insert(node.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heartbeat {
    pub node_id: Uuid,
    pub position: NodePosition,
    pub socket_address: Option<SocketAddr>,
    pub region: Region,
}

pub type AliveNodes = BTreeSet<Heartbeat>;

impl Ord for Heartbeat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.position
            .cmp(&other.position)
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for Heartbeat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Heartbeat {
    #[cfg(test)]
    pub fn example() -> Self {
        Self {
            node_id: Uuid::new_v4(),
            position: 0,
            socket_address: None,
            region: Region::Fsn1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;

    #[test]
    fn test_get_time_bucket() {
        let timestamp1 = DateTime::parse_from_rfc3339("2024-01-15T12:30:45Z")
            .unwrap()
            .with_timezone(&Utc);

        let timestamp2 = DateTime::parse_from_rfc3339("2024-01-15T12:30:01Z")
            .unwrap()
            .with_timezone(&Utc);

        let timestamp3 = DateTime::parse_from_rfc3339("2024-01-15T12:31:45Z")
            .unwrap()
            .with_timezone(&Utc);

        let bucket1 = get_time_bucket_minutes(timestamp1);
        let bucket2 = get_time_bucket_minutes(timestamp2);
        let bucket3 = get_time_bucket_minutes(timestamp3);

        assert_eq!(bucket1, bucket2);
        assert_ne!(bucket1, bucket3);
    }

    #[test]
    fn test_print_time_bucket() {
        let bucket = get_time_bucket_minutes(Utc::now());

        println!("get_time_bucket_minutes: {bucket}");
    }

    #[tokio::test]
    async fn test_insert_and_get_alive_workers() -> Result<()> {
        let (session, _) = create_test_database(None)
            .await
            .expect("Failed to create test database");

        let process_1 = Uuid::new_v4();
        let process_2 = Uuid::new_v4();
        let process_3 = Uuid::new_v4();
        let process_4 = Uuid::new_v4();
        let position = 42;

        insert_heartbeat(&session, Region::Fsn1, process_1, position, Utc::now()).await?;

        insert_heartbeat(
            &session,
            Region::Fsn1,
            process_2,
            position,
            Utc::now() - Duration::from_secs(240),
        )
        .await?;

        insert_heartbeat(
            &session,
            Region::Fsn1,
            process_3,
            position,
            Utc::now() - Duration::from_secs(310),
        )
        .await?;

        insert_heartbeat(&session, Region::Nbg1, process_4, position, Utc::now()).await?;

        let alive = fetch_alive_workers_within_interval(
            &session,
            &[Region::Fsn1],
            Duration::from_secs(300),
        )
        .await?;

        assert!(alive.iter().any(|h| h.node_id == process_1));
        assert!(alive.iter().any(|h| h.node_id == process_2));
        assert!(!alive.iter().any(|h| h.node_id == process_3));
        // process_4 is from a different region, so it should not be present
        assert!(!alive.iter().any(|h| h.node_id == process_4));

        // Test get_alive_workers_all_regions which should return workers from all regions
        let manager = HeartbeatManager::new(
            Uuid::new_v4(),
            Region::Fsn1,
            Duration::from_secs(300 / HEARTBEAT_FRESHNESS_MULTIPLE as u64),
            Arc::new(session),
        )
        .await?;

        let all_alive = manager.get_alive_workers_all_regions().await?;

        // process_1, process_2 should be present (same region)
        assert!(all_alive.iter().any(|h| h.node_id == process_1));
        assert!(all_alive.iter().any(|h| h.node_id == process_2));
        // process_3 is too old
        assert!(!all_alive.iter().any(|h| h.node_id == process_3));
        // process_4 should be present (different region)
        assert!(all_alive.iter().any(|h| h.node_id == process_4));

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_worker_metadata() -> Result<()> {
        let (session, _) = create_test_database(None)
            .await
            .expect("Failed to create test database");

        let process_id = Uuid::new_v4();
        let replica_id = "test-replica";
        let git_sha = "abc123";

        insert_worker_metadata(&session, process_id, Some(replica_id), Some(git_sha)).await?;

        // Query to verify the metadata was inserted
        let query = "
            SELECT process_id,
                   replica_id,
                   git_sha
            FROM workers_metadata
            WHERE process_id = ?
        ";

        let rows = session
            .query_unpaged(query, (process_id,))
            .await?
            .into_rows_result()?;

        let row = rows
            .rows::<(Uuid, Option<String>, Option<String>)>()?
            .next()
            .expect("Should have a row")?;

        let (pid, rid, sha) = row;
        assert_eq!(pid, process_id);
        assert_eq!(rid.as_deref(), Some(replica_id));
        assert_eq!(sha.as_deref(), Some(git_sha));

        Ok(())
    }
}
