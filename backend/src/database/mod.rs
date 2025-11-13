#[cfg(test)]
pub mod testing;

use anyhow::Result;
use scylla::client::PoolSize;
use scylla::client::{session::Session, session_builder::SessionBuilder};
use scylla::{client::execution_profile::ExecutionProfile, statement::Consistency};
use std::num::NonZeroUsize;
use std::time::Duration;

pub type Database = Session;

pub fn parse_database_urls(urls: &str) -> Vec<&str> {
    urls.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

async fn connect_db_optional_ks(
    database_nodes_urls: &[&str],
    keyspace_name: Option<&str>,
) -> Result<Database> {
    let profile = ExecutionProfile::builder()
        .consistency(Consistency::One)
        .request_timeout(Some(Duration::from_secs(5)))
        .build();
    let handle = profile.clone().into_handle();

    let mut builder = SessionBuilder::new()
        .known_nodes(database_nodes_urls)
        .default_execution_profile_handle(handle)
        .pool_size(PoolSize::PerShard(
            NonZeroUsize::new(1).expect("non-zero pool size"),
        ));

    if let Some(keyspace) = keyspace_name {
        builder = builder.use_keyspace(keyspace, true);
    }

    let session = builder.build().await?;

    Ok(session)
}

pub async fn connect_db(database_nodes_urls: &[&str], keyspace_name: &str) -> Result<Session> {
    connect_db_optional_ks(database_nodes_urls, Some(keyspace_name)).await
}
