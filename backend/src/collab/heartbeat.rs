use crate::collab::assignment::NodePosition;
use crate::collab::network::get_first_network_address;
use crate::database::Database;
use crate::database::preparer::CachedPreparedStatement;
use crate::regions::Region;
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

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
                get_first_network_address().map(|a| a.to_string()),
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
           address
    FROM workers_heartbeats
    WHERE region = ?
      AND time_bucket_minutes = ?
      AND timestamp >= ?
    ",
);

async fn get_alive_workers(
    session: &Database,
    region: Region,
    within_duration: Duration,
) -> Result<BTreeSet<Heartbeat>> {
    let now = Utc::now();
    let cutoff = now - within_duration;
    let current_bucket = get_time_bucket_minutes(now);
    let cutoff_bucket = get_time_bucket_minutes(cutoff);

    let mut alive_workers = BTreeSet::new();

    // Query all buckets from cutoff_bucket to current_bucket (inclusive)
    for bucket in cutoff_bucket..=current_bucket {
        let rows = GET_ALIVE_WORKERS_QUERY
            .execute_unpaged(session, (region.to_identifier(), bucket, cutoff))
            .await?
            .into_rows_result()?;

        for row in rows.rows::<(Uuid, i32, DateTime<Utc>, Option<String>)>()? {
            let (process_id, position, _timestamp, socket_addr) = row?;

            if position < 0 {
                bail!("cannot have negative position");
            }

            alive_workers.insert(Heartbeat {
                node_id: process_id,
                position: position as u32,
                socket_address: socket_addr.and_then(|s| s.parse().ok()),
            });
        }
    }

    Ok(alive_workers)
}

pub trait HeartbeatManagerTrait {
    fn get_alive_workers(&self) -> impl Future<Output = Result<BTreeSet<Heartbeat>>> + Send;
}

pub struct HeartbeatManager {
    process_id: Uuid,
    region: Region,
    interval: Duration,
    session: Arc<Database>,
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl HeartbeatManager {
    pub async fn new(
        process_id: Uuid,
        replica_id: Option<String>,
        region: Region,
        interval: Duration,
        session: Arc<Database>,
        git_sha: Option<String>,
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

        insert_worker_metadata(
            &session,
            process_id,
            replica_id.as_deref(),
            git_sha.as_deref(),
        )
        .await?;

        Ok(Self {
            process_id,
            region,
            interval,
            session,
            task_handle: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn start(&self, position: NodePosition) {
        let mut handle = self.task_handle.lock().await;

        if handle.is_some() {
            return;
        }

        let process_id = self.process_id;
        let region = self.region;
        let interval = self.interval;
        let session = self.session.clone();

        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;

                let timestamp = Utc::now();
                let result =
                    insert_heartbeat(&session, region, process_id, position, timestamp).await;

                if let Err(e) = result {
                    error!("failed to send heartbeat: {e}");
                }
            }
        });

        *handle = Some(task);

        info!("HeartbeatManager started")
    }

    pub async fn stop(&self) {
        let mut handle = self.task_handle.lock().await;

        if let Some(task) = handle.take() {
            task.abort();
            info!("HeartbeatManager stopped")
        }
    }
}

impl HeartbeatManagerTrait for HeartbeatManager {
    async fn get_alive_workers(&self) -> Result<BTreeSet<Heartbeat>> {
        get_alive_workers(&self.session, self.region, self.interval * 2).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heartbeat {
    pub node_id: Uuid,
    pub position: NodePosition,
    pub socket_address: Option<SocketAddr>,
}

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

        let alive = get_alive_workers(&session, Region::Fsn1, Duration::from_secs(300)).await?;

        assert!(alive.iter().any(|h| h.node_id == process_1));
        assert!(alive.iter().any(|h| h.node_id == process_2));
        assert!(!alive.iter().any(|h| h.node_id == process_3));
        // process_4 is from a different region, so it should not be present
        assert!(!alive.iter().any(|h| h.node_id == process_4));

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
