use crate::collab::assignment::RingRange;
use crate::collab::assignment::calculate_node_range;
use crate::collab::heartbeat::Heartbeat;
use crate::regions::Region;
use anyhow::Result;
use log::{error, info};
use std::collections::BTreeSet;
use tokio::sync::watch;
use uuid::Uuid;

pub struct RangeManager {
    node_id: Uuid,
    replication_factor: u32,
    region: Region,
}

impl RangeManager {
    pub fn new(node_id: Uuid, replication_factor: u32, region: Region) -> Self {
        Self {
            node_id,
            replication_factor,
            region,
        }
    }

    fn calculate_range(
        &self,
        current_state: &BTreeSet<Heartbeat>,
        region: Region,
        tx: &mut watch::Sender<Option<RingRange>>,
    ) -> Result<()> {
        let range =
            calculate_node_range(self.node_id, self.replication_factor, current_state, region);
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

    pub async fn start(
        self,
        heartbeat_updates: watch::Receiver<BTreeSet<Heartbeat>>,
    ) -> (impl FnOnce(), watch::Receiver<Option<RingRange>>) {
        let (mut tx, rx) = watch::channel(None);

        let task = tokio::spawn(async move {
            let mut heartbeat_updates = heartbeat_updates;
            // The value used to initialize the channel is always already marked as "seed",
            // but we still want to process it to avoid having to wait the next heartbeat
            let mut first = true;

            while first || heartbeat_updates.changed().await.is_ok() {
                first = false;

                let current_state = heartbeat_updates.borrow_and_update();
                let result = self.calculate_range(&current_state, self.region, &mut tx);

                if let Err(e) = result {
                    error!("error calculating range: {e}");
                }
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
    use crate::{collab::heartbeat::Heartbeat, regions::Region};
    use anyhow::Result;
    use std::{collections::BTreeSet, time::Duration};
    use tokio::time::timeout;

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
            region: Region::Fsn1,
        });
        nodes.insert(Heartbeat {
            node_id: Uuid::new_v4(),
            position: 1,
            socket_address: None,
            region: Region::Fsn1,
        });
        nodes.insert(Heartbeat {
            node_id: Uuid::new_v4(),
            position: 2,
            socket_address: None,
            region: Region::Fsn1,
        });

        let range_manager = RangeManager::new(node_id, replication_factor, Region::Fsn1);

        let (_sender, alive_nodes_receiver) = watch::channel(nodes);

        let (close_fn, mut rx) = range_manager.start(alive_nodes_receiver).await;

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
