mod messages;

use crate::collab::{heartbeat::Heartbeat, internode::messages::InterNodeMessage};
use anyhow::Result;
use log::error;
use reqwest::Client;
use std::collections::BTreeSet;

async fn broadcast(
    alive_nodes: &BTreeSet<Heartbeat>,
    messages: Vec<InterNodeMessage>,
) -> Result<()> {
    let client = Client::new();

    for node in alive_nodes {
        if let Some(socket_addr) = node.socket_address {
            let url = format!("http://{}/private/message", socket_addr);

            if let Err(e) = client.post(&url).json(&messages).send().await {
                error!("failed to send message to {}: {}", socket_addr, e);
            }
        }
    }

    Ok(())
}
