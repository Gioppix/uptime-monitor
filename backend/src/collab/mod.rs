//! Consistent hashing ring implementation using Cassandra for coordination.
//!
//! Each worker owns a range [self_position, next_k_position) where k is the replication factor.
//! When a worker dies, only k others expand their ranges to cover the gap.
//!
//! New workers join by selecting a position randomly, weighted to prefer splitting longer ranges
//! (this reduces collisions when multiple workers join simultaneously).
//!
//! Nodes are ordered by (position, node_id) to handle the rare case of position collisions.
//! All state (heartbeats + ring positions) persists in Cassandra (eventually consistent).

mod assignment;
pub mod heartbeat;
pub mod internode;
mod network;
pub mod range_manager;

use crate::collab::{
    assignment::choose_new_node_position,
    heartbeat::{HeartbeatManager, HeartbeatManagerTrait},
};
use anyhow::Result;

pub use assignment::{NodePosition, RingRange};

pub async fn decide_position(
    heartbeat: &HeartbeatManager,
    ring_size: NodePosition,
) -> Result<NodePosition> {
    let state = heartbeat.get_alive_workers().await?;

    let position = choose_new_node_position(&state, ring_size)?;

    Ok(position)
}
