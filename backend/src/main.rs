mod collab;
mod database;
mod eager_env;
mod mutations;
mod regions;
mod server;
mod utils;
mod worker;

use crate::{
    collab::{decide_position, heartbeat::HeartbeatManager, range_manager::RangeManager},
    database::{connect_db, parse_database_urls},
    eager_env::check_env,
    regions::Region,
    server::{AppStateInner, start_server},
    worker::Worker,
};
use std::{env, net::TcpListener, sync::Arc, time::Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();
    check_env();

    let process_id = Uuid::new_v4();
    let node_urls = parse_database_urls(&eager_env::DATABASE_NODE_URLS);
    let replica_id = env::var("RAILWAY_REPLICA_ID").ok();
    let region: Region = eager_env::REGION
        .parse()
        .expect("Invalid REGION");
    let git_sha = env::var("RAILWAY_GIT_COMMIT_SHA").ok();

    let database = connect_db(&node_urls, &eager_env::DATABASE_KEYSPACE)
        .await
        .expect("failed to connect to the database");
    let database = Arc::new(database);

    let heartbeat = HeartbeatManager::new(
        process_id,
        replica_id,
        region,
        Duration::from_secs(*eager_env::HEARTBEAT_INTERVAL_SECONDS),
        database.clone(),
        git_sha,
    )
    .await
    .expect("msg");
    let heartbeat = Arc::new(heartbeat);

    let range_manager = RangeManager::new(process_id, *eager_env::REPLICATION_FACTOR);

    let position = decide_position(&heartbeat, *eager_env::CURRENT_BUCKETS_COUNT)
        .await
        .expect("msg");

    let state = Arc::new(AppStateInner {
        database: database.clone(),
    });
    let listener =
        TcpListener::bind(format!("0.0.0.0:{}", *eager_env::PORT)).expect("Failed to bind PORT");

    println!(
        "Listening on {}",
        listener.local_addr().expect("Failed to get local address")
    );

    heartbeat.start(position).await;

    let (stop_range_manager, range_updates) = range_manager
        .start(
            Duration::from_secs(*eager_env::HEARTBEAT_INTERVAL_SECONDS),
            heartbeat.clone(),
        )
        .await;

    let worker = Worker::new(
        database.clone(),
        region,
        *eager_env::CURRENT_BUCKET_VERSION as i16,
        *eager_env::CURRENT_BUCKETS_COUNT,
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
