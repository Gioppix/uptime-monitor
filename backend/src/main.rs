mod dabatase;
mod server;
mod utils;

use crate::{
    dabatase::connect_db,
    server::{AppStateInner, start_server},
};
use std::{net::TcpListener, sync::Arc};

const PORT: u32 = env_u32!("PORT");
const DATABASE_URL: &str = env_str!("DATABASE_URL");

#[tokio::main]
async fn main() {
    let db = connect_db(DATABASE_URL)
        .await
        .expect("failed to connect to the database");

    let state = Arc::new(AppStateInner {});
    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).expect("Failed to bind PORT");

    start_server(state, listener)
        .await
        .expect("error while running server");
}
