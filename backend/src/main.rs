mod collab;
mod database;
mod mutations;
mod server;
mod utils;

use crate::{
    database::{connect_db, parse_database_urls},
    server::{AppStateInner, start_server},
};
use std::{net::TcpListener, sync::Arc};

const PORT: u32 = env_u32!("PORT");
const DATABASE_NODE_URLS: &str = env_str!("DATABASE_NODE_URLS");
const DATABASE_KEYSPACE: &str = env_str!("DATABASE_KEYSPACE");
const DEV_MODE: bool = env_bool!("DEV_MODE");

#[tokio::main]
async fn main() {
    let node_urls = parse_database_urls(DATABASE_NODE_URLS);

    let database = connect_db(&node_urls, DATABASE_KEYSPACE)
        .await
        .expect("failed to connect to the database");

    let state = Arc::new(AppStateInner { database });
    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).expect("Failed to bind PORT");

    println!(
        "Listening on {}",
        listener.local_addr().expect("Failed to get local address")
    );

    start_server(state, listener)
        .await
        .expect("error while running server");
}
