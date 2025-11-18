mod collab;
mod database;
mod mutations;
mod regions;
mod server;
mod utils;
mod worker;

use crate::{
    collab::{decide_position, heartbeat::HeartbeatManager, range_manager::RangeManager},
    database::{connect_db, parse_database_urls},
    regions::Region,
    server::{AppStateInner, start_server},
    worker::Worker,
};
use std::{env, net::TcpListener, sync::Arc, time::Duration};
use uuid::Uuid;

const PORT: u32 = env_u32!("PORT");
const DATABASE_NODE_URLS: &str = env_str!("DATABASE_NODE_URLS");
const DATABASE_KEYSPACE: &str = env_str!("DATABASE_KEYSPACE");
const HEARTBEAT_INTERVAL_SECONDS: u64 = env_u64!("HEARTBEAT_INTERVAL_SECONDS");
const DEV_MODE: bool = env_bool!("DEV_MODE");

const CURRENT_BUCKET_VERSION: u32 = env_u32!("CURRENT_BUCKET_VERSION");
const CURRENT_BUCKETS_COUNT: u32 = env_u32!("CURRENT_BUCKETS_COUNT");
const REPLICATION_FACTOR: u32 = env_u32!("REPLICATION_FACTOR");

fn get_runtime_envs() -> (
    Uuid,
    Vec<&'static str>,
    Option<String>,
    Region,
    Option<String>,
) {
    let process_id = Uuid::new_v4();
    let node_urls = parse_database_urls(DATABASE_NODE_URLS);
    let replica_id = env::var("RAILWAY_REPLICA_ID").ok();
    let region: Region = env::var("RAILWAY_REPLICA_REGION")
        .expect("RAILWAY_REPLICA_REGION must be set")
        .parse()
        .expect("Invalid RAILWAY_REPLICA_REGION");

    let git_sha = env::var("RAILWAY_GIT_COMMIT_SHA").ok();

    (process_id, node_urls, replica_id, region, git_sha)
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    let (process_id, node_urls, replica_id, region, git_sha) = get_runtime_envs();

    let database = connect_db(&node_urls, DATABASE_KEYSPACE)
        .await
        .expect("failed to connect to the database");
    let database = Arc::new(database);

    let heartbeat = HeartbeatManager::new(
        process_id,
        replica_id,
        region,
        Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS),
        database.clone(),
        git_sha,
    )
    .await
    .expect("msg");
    let heartbeat = Arc::new(heartbeat);

    let range_manager = RangeManager::new(process_id, REPLICATION_FACTOR);

    let position = decide_position(&heartbeat, CURRENT_BUCKETS_COUNT)
        .await
        .expect("msg");

    let state = Arc::new(AppStateInner {
        database: database.clone(),
    });
    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).expect("Failed to bind PORT");

    println!(
        "Listening on {}",
        listener.local_addr().expect("Failed to get local address")
    );

    heartbeat.start(position).await;

    let (stop_range_manager, range_updates) = range_manager
        .start(
            Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS),
            heartbeat.clone(),
        )
        .await;

    let worker = Worker::new(
        database.clone(),
        region,
        CURRENT_BUCKET_VERSION as i16,
        CURRENT_BUCKETS_COUNT,
        range_updates,
    )
    .await
    .expect("worker initialization failed");

    let stop_worker = worker.start(database.clone());

    start_server(state, listener)
        .await
        .expect("error while running server");

    heartbeat.stop().await;
    stop_range_manager();
    stop_worker.await;
}
