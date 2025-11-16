//! Consistent hashing ring implementation using Cassandra for coordination.
//!
//! Each backend owns a range [self_position, next_k_position) where k is the replication factor.
//! When a backend dies, only k others expand their ranges to cover the gap.
//!
//! New backends join by selecting a position randomly, weighted to prefer splitting longer ranges
//! (avoiding collision when multiple backends join simultaneously).
//!
//! Nodes are ordered by (position, node_id) to handle the rare case of position collisions.
//! All state (heartbeats + ring positions) persists in Cassandra (eventually consistent).

mod bucket_assignment;
mod network;
mod ring_assignment;
