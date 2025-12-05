mod collab;
mod database;
mod eager_env;
mod queries;
mod regions;
mod server;
mod utils;
mod worker;

use crate::{
    collab::{
        decide_position,
        heartbeat::HeartbeatManager,
        internode::{MessageWithFilters, messages::InterNodeMessage, standard_broadcast},
        range_manager::RangeManager,
    },
    database::{connect_db, parse_database_urls},
    eager_env::check_env,
    regions::Region,
    server::{AppStateInner, start_server},
    worker::Worker,
};
use anyhow::Result;
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;
use uuid::Uuid;

async fn communicate_shutdown(
    heartbeat: Arc<HeartbeatManager>,
    process_id: Uuid,
) -> Result<Vec<SocketAddr>> {
    let (ips, _) = standard_broadcast(
        &heartbeat,
        vec![MessageWithFilters {
            message: InterNodeMessage::ShuttingDown { process_id },
            filter_bucket: None,
        }],
    )
    .await?;
    Ok(ips)
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();
    check_env();

    let process_id = Uuid::new_v4();
    let node_urls = parse_database_urls(&eager_env::DATABASE_NODE_URLS);
    let region: Region = *eager_env::REGION;

    let database = connect_db(&node_urls, &eager_env::DATABASE_KEYSPACE)
        .await
        .expect("failed to connect to the database");
    let database = Arc::new(database);

    let heartbeat = HeartbeatManager::new(
        process_id,
        region,
        Duration::from_secs(*eager_env::HEARTBEAT_INTERVAL_SECONDS),
        database.clone(),
    )
    .await
    .expect("msg");
    let heartbeat = Arc::new(heartbeat);

    let range_manager = RangeManager::new(process_id, *eager_env::REPLICATION_FACTOR, region);

    let position = decide_position(&heartbeat, *eager_env::CURRENT_BUCKETS_COUNT)
        .await
        .expect("msg");

    let (task_updates_sender, task_updates_receiver) = mpsc::unbounded_channel();

    let state = Arc::new(AppStateInner {
        process_id,
        database: database.clone(),
        task_updates: task_updates_sender,
        heartbeat_manager: heartbeat.clone(),
    });
    let listener =
        TcpListener::bind(format!("0.0.0.0:{}", *eager_env::PORT)).expect("Failed to bind PORT");

    println!(
        "Listening on {}",
        listener.local_addr().expect("Failed to get local address")
    );

    let (alive_nodes_receiver, stop_heartbeat) = heartbeat.start(position).await.unwrap();

    let (stop_range_manager, range_updates) = range_manager.start(alive_nodes_receiver).await;

    let worker = Worker::new(
        database.clone(),
        region,
        *eager_env::CURRENT_BUCKET_VERSION as i16,
        *eager_env::CURRENT_BUCKETS_COUNT,
        range_updates,
        task_updates_receiver,
    )
    .await
    .expect("worker initialization failed");

    let stop_worker = worker.start();

    start_server(state, listener)
        .await
        .expect("error while running server");

    match communicate_shutdown(heartbeat.clone(), process_id).await {
        Err(e) => {
            log::error!("failed to communicate shutdown: {:?}", e);
        }
        Ok(ips) => {
            log::info!("shutdown communicated to {:?}", ips);
        }
    }

    stop_heartbeat.await;
    stop_range_manager();
    stop_worker.await;
}
