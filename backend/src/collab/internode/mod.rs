pub mod messages;

use crate::{
    collab::{
        NodePosition,
        assignment::calculate_node_range,
        heartbeat::{Heartbeat, HeartbeatManager},
        internode::messages::InterNodeMessage,
    },
    eager_env::{BACKEND_INTERNAL_PASSWORD, REPLICATION_FACTOR},
};
use anyhow::Result;
use log::{error, warn};
use reqwest::Client;
use std::{collections::BTreeSet, net::SocketAddr};

pub struct MessageWithFilters {
    pub message: InterNodeMessage,
    pub filter_bucket: Option<NodePosition>,
}

pub type BroadcastBody = Vec<InterNodeMessage>;

/// Broadcasts messages to the given alive nodes.
/// Returns the number of hosts that received the messages successfully.
pub async fn broadcast(
    alive_nodes: &BTreeSet<Heartbeat>,
    messages: Vec<MessageWithFilters>,
    replication_factor: u32,
) -> usize {
    let client = Client::new();

    let tasks: Vec<_> = alive_nodes
        .iter()
        .filter_map(|node| match node.socket_address {
            Some(socket_addr) => Some((node, socket_addr)),
            None => {
                warn!("node {} has no socket address", node.node_id);
                None
            }
        })
        .map(|(node, socket_addr)| {
            let client = client.clone();
            let filtered_messages: Vec<_> = messages
                .iter()
                .filter(|m| {
                    let Some(filter_bucket) = m.filter_bucket else {
                        return true;
                    };

                    match calculate_node_range(
                        node.node_id,
                        replication_factor,
                        alive_nodes,
                        node.region,
                    ) {
                        Some(range) => range.contains(filter_bucket),
                        None => false,
                    }
                })
                .map(|m| m.message.clone())
                .collect();

            let url = format!("http://{}/internal", socket_addr);

            async move {
                if filtered_messages.is_empty() {
                    return false;
                }

                let message: BroadcastBody = filtered_messages;

                let result = client
                    .post(&url)
                    .json(&message)
                    .header(
                        "Authorization",
                        format!("Bearer {}", *BACKEND_INTERNAL_PASSWORD),
                    )
                    .send()
                    .await;

                match result {
                    Err(e) => {
                        error!("Failed to send broadcast to {}: {}", url, e);
                        false
                    }
                    Ok(response) => {
                        if !response.status().is_success() {
                            error!(
                                "Broadcast to {url} returned unsuccessful status: {}",
                                response.status()
                            );
                            false
                        } else {
                            true
                        }
                    }
                }
            }
        })
        .collect();

    let results = futures::future::join_all(tasks).await;
    results.into_iter().filter(|&success| success).count()
}

/// Broadcasts messages to all alive nodes.
/// Returns the socket addresses of the (allegedly) currently alive nodes and the number of
/// successful sends.
pub async fn standard_broadcast(
    heartbeat: &HeartbeatManager,
    messages: Vec<MessageWithFilters>,
) -> Result<(Vec<SocketAddr>, usize)> {
    let alive_nodes = heartbeat.get_alive_workers_all_regions().await?;
    let alive_ips = alive_nodes
        .iter()
        .filter_map(|node| node.socket_address)
        .collect();
    let success_count = broadcast(&alive_nodes, messages, *REPLICATION_FACTOR).await;
    Ok((alive_ips, success_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{regions::Region, server::start_server_test};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_standard_broadcast() {
        // Start two test servers
        let (port1, state1) = start_server_test(None).await;
        let (port2, state2) = start_server_test(None).await;

        // Register both servers as alive nodes in heartbeat manager
        let addr1: SocketAddr = format!("127.0.0.1:{}", port1).parse().unwrap();
        let addr2: SocketAddr = format!("127.0.0.1:{}", port2).parse().unwrap();

        state1
            .heartbeat_manager
            .register_nodes(&[
                Heartbeat {
                    node_id: state1.process_id,
                    position: 0,
                    socket_address: Some(addr1),
                    region: Region::Fsn1,
                },
                Heartbeat {
                    node_id: state2.process_id,
                    position: 1,
                    socket_address: Some(addr2),
                    region: Region::Fsn1,
                },
            ])
            .await;

        // Broadcast a message
        let messages = vec![MessageWithFilters {
            message: InterNodeMessage::ServiceCheckMutation {
                check_id: Uuid::new_v4(),
            },
            filter_bucket: None,
        }];

        let (ips, success_count) = standard_broadcast(&state1.heartbeat_manager, messages)
            .await
            .unwrap();

        assert_eq!(ips.len(), 2);
        assert_eq!(success_count, 2);
    }
}
