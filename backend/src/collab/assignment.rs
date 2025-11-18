use crate::collab::heartbeat::Heartbeat;
use anyhow::Result;
use anyhow::bail;
use rand::rng;
use rand_distr::num_traits::Pow;
use rand_distr::{Beta, Distribution, weighted::WeightedIndex};
use std::collections::BTreeSet;
use std::fmt::Display;
use uuid::Uuid;

pub type NodePosition = u32;

/// Represents a range on the ring [start, end).
///
/// The range is inclusive of `start` and exclusive of `end`.
/// When `end < start`, the range wraps around the ring.
// TODO: also include ring size
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
pub fn choose_new_node_position(
    state: &BTreeSet<Heartbeat>,
    ring_size: NodePosition,
) -> Result<NodePosition> {
    /// A higher number means bigger gaps are preferred more
    const GAP_EXPONENT: f64 = 2.0;
    /// A higher number means the center of the chosen gap is preferred more
    const BETA_FUNCTION_AB: f64 = 3.0;

    if state.is_empty() {
        return Ok(0);
    }

    let mut rng = rng();

    for node in state {
        if node.position >= ring_size {
            bail!("invalid node position");
        }
    }

    // (gaps(size, position), weights)
    let (gaps, weights): (Vec<_>, Vec<_>) = state
        .iter()
        .zip(state.iter().cycle().skip(1))
        .map(|(current, next)| {
            let gap = if next.position > current.position {
                next.position - current.position
            } else {
                ring_size - current.position + next.position
            };
            let gap = gap as f64;

            ((gap, current.position), gap.pow(GAP_EXPONENT))
        })
        .unzip();

    // Guarantees:
    // - Each value is non-negative (it's a position)
    // - The sum is never 0 (it's the sum of powers of numbers whose sum is RING_SIZE)
    // - The sum is at most ring_size ^ p but converted to f64
    // - The list is not empty
    let dist = WeightedIndex::new(&weights)?;

    // Select a gap randomly based on weights
    let selected_idx = dist.sample(&mut rng);
    let (gap_size, start_pos) = gaps[selected_idx];

    let beta = Beta::new(BETA_FUNCTION_AB, BETA_FUNCTION_AB)?;
    let offset_ratio = beta.sample(&mut rng);

    let offset = (gap_size * offset_ratio) as NodePosition;

    let final_position = (start_pos + offset) % ring_size;

    Ok(final_position)
}

pub fn calculate_node_range(
    node_id: Uuid,
    replication_factor: u32,
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
    let end_idx = (our_idx + replication_factor as usize) % nodes.len();
    let end_position = nodes[end_idx].position;

    Some(RingRange {
        start: our_position,
        end: end_position,
    })
}

impl RingRange {
    pub fn iter(&self, ring_size: NodePosition) -> RingRangeIterator {
        RingRangeIterator {
            range: *self,
            ring_size,
            current: self.start,
            done: false,
        }
    }
}

impl Display for RingRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{})", self.start, self.end)
    }
}

pub struct RingRangeIterator {
    range: RingRange,
    ring_size: NodePosition,
    current: NodePosition,
    done: bool,
}

impl Iterator for RingRangeIterator {
    type Item = NodePosition;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let result = self.current;

        self.current = (self.current + 1) % self.ring_size;

        if self.current == self.range.end {
            self.done = true;
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::uuid;

    #[test]
    fn test_into_iter() {
        const RING_SIZE: NodePosition = 10;

        let range = RingRange { start: 0, end: 1 };
        let result: Vec<NodePosition> = range.iter(RING_SIZE).collect();
        assert_eq!(result, vec![0]);

        let range = RingRange { start: 0, end: 5 };
        let result: Vec<NodePosition> = range.iter(RING_SIZE).collect();
        assert_eq!(result, vec![0, 1, 2, 3, 4]);

        let range = RingRange { start: 9, end: 1 };
        let result: Vec<NodePosition> = range.iter(RING_SIZE).collect();
        assert_eq!(result, vec![9, 0]);

        let range = RingRange { start: 0, end: 0 };
        let result: Vec<NodePosition> = range.iter(RING_SIZE).collect();
        assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        // TODO: fix (currently loops forever, end >= size edge case)
        // let range = RingRange { start: 0, end: 3 };
        // let result: Vec<NodePosition> = range.iter(3).collect();
        // assert_eq!(result, vec![0, 1, 2]);
    }

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
        const TEST_RING_SIZE: NodePosition = 100;

        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 0,
        });
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000002"),
            position: TEST_RING_SIZE / 4,
        });

        for (i, heartbeat) in state.iter().enumerate() {
            let percentage = (heartbeat.position as f64 / TEST_RING_SIZE as f64) * 100.0;
            println!(
                "Node {} position: {} ({}%)",
                i + 1,
                heartbeat.position,
                percentage.floor()
            );
        }
        println!("\n");

        let mut results: Vec<_> = (0..10)
            .map(|_| choose_new_node_position(&state, TEST_RING_SIZE).unwrap())
            .collect();

        results.sort();

        for position in results {
            let percentage = (position as f64 / TEST_RING_SIZE as f64) * 100.0;
            println!("Chosen position: {} ({}%)", position, percentage.floor());
        }
    }

    #[test]
    fn test_two_nodes_1000_iterations() {
        const TEST_RING_SIZE: NodePosition = 10;

        let mut state = BTreeSet::new();
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000001"),
            position: 3,
        });
        state.insert(Heartbeat {
            node_id: uuid!("00000000-0000-0000-0000-000000000002"),
            position: 6,
        });

        for _ in 0..1000 {
            let position = choose_new_node_position(&state, TEST_RING_SIZE).unwrap();
            assert!(
                position < TEST_RING_SIZE,
                "Position {} should be less than {}",
                position,
                TEST_RING_SIZE
            );
        }
    }
}
