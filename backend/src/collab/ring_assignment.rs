use rand::rng;
use rand_distr::num_traits::Pow;
use rand_distr::{Beta, Distribution, weighted::WeightedIndex};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heartbeat {
    pub node_id: Uuid,
    pub position: NodePosition,
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

pub type NodePosition = u128;

/// Represents a range on the ring [start, end).
///
/// The range is inclusive of `start` and exclusive of `end`.
/// When `end < start`, the range wraps around the ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingRange {
    pub start: NodePosition,
    pub end: NodePosition,
}

/// Chooses a position for a new node on the ring given the current state.
//
// Implementation
// It uses a weighted random selection strategy that prefers larger gaps between existing nodes,
// with weights proportional to gap_size^GAP_EXPONENT. Within the selected gap, it uses a
// Beta(BETA_FUNCTION_AB) distribution to avoid edge positions and naturally cluster toward
// the middle of the gap.
// Randomness is needed so that if nodes join together they don't overlap too much.
//
pub fn choose_new_node_position(state: &BTreeSet<Heartbeat>) -> NodePosition {
    /// A higher number means bigger gaps are preferred more
    const GAP_EXPONENT: f64 = 2.0;
    /// A higher number means the center of the chosen gap is preferred more
    const BETA_FUNCTION_AB: f64 = 3.0;

    if state.is_empty() {
        return NodePosition::MAX / 2;
    }

    let mut rng = rng();

    // (gaps(size, position), weights)
    let (gaps, weights): (Vec<_>, Vec<_>) = state
        .iter()
        .zip(state.iter().cycle().skip(1))
        .map(|(current, next)| {
            let gap = if next.position > current.position {
                next.position - current.position
            } else {
                NodePosition::MAX - current.position + next.position
            };
            let gap = gap as f64;

            ((gap, current.position), gap.pow(GAP_EXPONENT))
        })
        .unzip();

    // Guarantees:
    // - Each value is non-negative (u128)
    // - The sum is never 0
    // - The sum is at most NodePosition::MAX ^ p but converted to f64
    // - The list is not empty
    let dist = WeightedIndex::new(&weights).expect("Failed to create weighted distribution");

    // Select a gap randomly based on weights
    let selected_idx = dist.sample(&mut rng);
    let (gap_size, start_pos) = gaps[selected_idx];

    let beta =
        Beta::new(BETA_FUNCTION_AB, BETA_FUNCTION_AB).expect("Failed to create beta distribution");
    let offset_ratio = beta.sample(&mut rng);

    let offset = (gap_size * offset_ratio) as NodePosition;
    start_pos.wrapping_add(offset)
}

pub fn calculate_node_range(
    node_id: Uuid,
    replication_factor: usize,
    current_state: &BTreeSet<Heartbeat>,
) -> Option<RingRange> {
    let nodes: Vec<&Heartbeat> = current_state.iter().collect();

    // Find our position in the sorted list
    let our_idx = nodes.iter().position(|h| h.node_id == node_id)?;

    let our_position = nodes[our_idx].position;

    if nodes.len() == 1 {
        // We're the only node - we cover the entire ring
        return Some(RingRange {
            start: our_position,
            end: our_position,
        });
    }

    // Find the k-th successor (wrapping around)
    let end_idx = (our_idx + replication_factor) % nodes.len();
    let end_position = nodes[end_idx].position;

    Some(RingRange {
        start: our_position,
        end: end_position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::uuid;

    #[test]
    fn test_no_nodes_present() {
        let state = BTreeSet::new();

        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000001"), 1, &state),
            None
        );
    }

    #[test]
    fn test_single_node_covers_entire_ring() {
        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 100,
        });

        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000001"), 1, &state),
            Some(RingRange {
                start: 100,
                end: 100
            })
        );
    }

    #[test]
    fn test_poll_returns_none_when_node_not_present() {
        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 100,
        });

        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000002"), 1, &state),
            None
        );
    }

    #[test]
    fn test_wrapping_range() {
        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 100,
        });
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000002"),
            position: 200,
        });

        // node2 wraps around to node1
        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000002"), 1, &state),
            Some(RingRange {
                start: 200,
                end: 100
            })
        );
    }

    #[test]
    fn test_poll_replication_factor() {
        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 100,
        });
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000002"),
            position: 200,
        });
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000003"),
            position: 300,
        });

        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000001"), 1, &state),
            Some(RingRange {
                start: 100,
                end: 200
            })
        );

        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000001"), 2, &state),
            Some(RingRange {
                start: 100,
                end: 300
            })
        );

        // Since replication_factor > N it should gracefully degrade to the whole range
        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000001"), 3, &state),
            Some(RingRange {
                start: 100,
                end: 100
            })
        );

        // Since replication_factor > N it should gracefully degrade to the whole range
        assert_eq!(
            calculate_node_range(uuid!("00000000-0000-0000-0000-000000000001"), 30, &state),
            Some(RingRange {
                start: 100,
                end: 100
            })
        );
    }

    #[test]
    #[ignore]
    fn test_display_position_two_nodes() {
        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 0,
        });
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000002"),
            position: NodePosition::MAX / 4,
        });

        for (i, heartbeat) in state.iter().enumerate() {
            let percentage = (heartbeat.position as f64 / NodePosition::MAX as f64) * 100.0;
            println!("Node {} position: {}%", i + 1, percentage.floor());
        }
        println!("\n");

        for _ in 0..10 {
            let position = choose_new_node_position(&state);
            let percentage = (position as f64 / NodePosition::MAX as f64) * 100.0;
            println!("Chosen position: {}%", percentage.floor());
        }
    }
}
