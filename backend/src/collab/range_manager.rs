use crate::collab::assignment::RingRange;
use crate::collab::assignment::calculate_node_range;
use crate::collab::heartbeat::HeartbeatManagerTrait;
use anyhow::Result;
use log::{error, info};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::time;
use uuid::Uuid;

pub struct RangeManager {
    node_id: Uuid,
    replication_factor: u32,
}

impl RangeManager {
    pub fn new(node_id: Uuid, replication_factor: u32) -> Self {
        Self {
            node_id,
            replication_factor,
        }
    }

    async fn calculate_range<T: HeartbeatManagerTrait + Sync + Send>(
        &self,
        heartbeat: &T,
        tx: &mut Sender<Option<RingRange>>,
    ) -> Result<()> {
        let current_state = heartbeat.get_alive_workers().await?;
        let range = calculate_node_range(self.node_id, self.replication_factor, &current_state);
        let old_range = *tx.borrow();

        if old_range != range {
            info!(
                "Detected range change: old='{}', new='{}'",
                old_range
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                range
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "none".to_string())
            );
            tx.send(range)?;
        }

        Ok(())
    }

    pub async fn start<T: HeartbeatManagerTrait + Sync + Send + 'static>(
        self,
        interval: Duration,
        heartbeat: Arc<T>,
    ) -> (impl FnOnce(), Receiver<Option<RingRange>>) {
        let (mut tx, rx) = watch::channel(None);

        let task = tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                let result = self.calculate_range(heartbeat.as_ref(), &mut tx).await;

                if let Err(e) = result {
                    error!("error calculating range: {e}");
                }

                ticker.tick().await;
            }
        });

        let close_function = move || {
            task.abort();
            info!("RangeManager stopped");
        };

        info!("RangeManager started");

        (close_function, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collab::heartbeat::{Heartbeat, HeartbeatManagerTrait};
    use anyhow::Result;
    use std::collections::BTreeSet;
    use tokio::time::timeout;

    struct DummyHeartbeatManager {
        nodes: BTreeSet<Heartbeat>,
    }

    impl DummyHeartbeatManager {
        fn new(nodes: BTreeSet<Heartbeat>) -> Self {
            Self { nodes }
        }
    }

    impl HeartbeatManagerTrait for DummyHeartbeatManager {
        async fn get_alive_workers(&self) -> Result<BTreeSet<Heartbeat>> {
            Ok(self.nodes.clone())
        }
    }

    #[tokio::test]
    async fn test_range_manager_start() -> Result<()> {
        let node_id = Uuid::new_v4();
        let replication_factor = 2;

        // Create dummy heartbeat manager with some fixed nodes
        let mut nodes = BTreeSet::new();
        nodes.insert(Heartbeat {
            node_id,
            position: 0,
            socket_address: None,
        });
        nodes.insert(Heartbeat {
            node_id: Uuid::new_v4(),
            position: 1,
            socket_address: None,
        });
        nodes.insert(Heartbeat {
            node_id: Uuid::new_v4(),
            position: 2,
            socket_address: None,
        });

        let heartbeat = Arc::new(DummyHeartbeatManager::new(nodes));
        let range_manager = RangeManager::new(node_id, replication_factor);

        let (close_fn, mut rx) = range_manager
            .start(Duration::from_millis(100), heartbeat)
            .await;

        // Wait for a message on the channel
        rx.changed().await.expect("Channel should receive a value");

        {
            // Get the value from the changed event
            let range_value = rx
                .borrow_and_update()
                .expect("There should be a value present");
            assert_eq!(range_value, RingRange { start: 0, end: 2 });
        }

        // Verify that we get no other messages since nodes are not changing
        let result = timeout(Duration::from_millis(250), rx.changed()).await;
        assert!(
            result.is_err(),
            "Expected no new messages since nodes are not changing"
        );

        close_fn();

        // Verify that the channel is closed after calling close_fn
        let result = rx.changed().await;
        assert!(
            result.is_err(),
            "Expected channel to be closed after close_fn is called"
        );

        Ok(())
    }
}
