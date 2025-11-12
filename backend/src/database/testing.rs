use crate::database::connect_db_optional_ks;
use crate::{DATABASE_NODE_URLS, database::parse_database_urls};
use anyhow::Result;
use include_dir::{Dir, include_dir};
use rand::{Rng, rng};
use scylla::client::session::Session;

static MIGRATIONS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub fn get_migrations() -> Vec<(String, String)> {
    MIGRATIONS_DIR
        .files()
        .map(|file| {
            (
                file.path().to_str().expect("valid utf8").to_string(),
                file.contents_utf8().expect("valid utf8").to_string(),
            )
        })
        .collect()
}

// Test database setup utilities
//
// Returns a `Session` and the dedicated `keyspace`
#[cfg(test)]
pub async fn create_test_database() -> Result<(Session, String)> {
    let keyspace_name = format!("test_ks_{}", rng().random::<u32>());

    let database_urls = parse_database_urls(DATABASE_NODE_URLS);
    let session = connect_db_optional_ks(&database_urls, None).await?;

    // Create the keyspace
    session
        .query_unpaged(
            format!(
                "CREATE KEYSPACE IF NOT EXISTS {} WITH REPLICATION = {{'class': 'SimpleStrategy', 'replication_factor': 1}}",
                keyspace_name
            ),
            &[],
        )
        .await?;

    // Use the keyspace
    session.use_keyspace(&keyspace_name, true).await?;

    // Run migrations
    let migration_files = get_migrations();
    for (file, content) in migration_files {
        for statement in content.split(';').filter(|s| !s.trim().is_empty()) {
            session
                .query_unpaged(statement.trim(), &[])
                .await
                .map_err(|e| anyhow::anyhow!("Migration failed for file {}: {}", file, e))?;
        }
    }

    Ok((session, keyspace_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_create_database() -> Result<()> {
        let (_, keyspace_name) = create_test_database().await?;
        println!("Created test keyspace: {}", keyspace_name);

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn cleanup_test_keyspaces() -> Result<()> {
        let database_urls = parse_database_urls(DATABASE_NODE_URLS);
        let session = connect_db_optional_ks(&database_urls, None).await?;

        let rows = session
            .query_unpaged("SELECT keyspace_name FROM system_schema.keyspaces", &[])
            .await?
            .into_rows_result()?;

        for row in rows.rows::<(String,)>()? {
            let (keyspace_name,) = row?;
            if keyspace_name.starts_with("test_") {
                println!("Dropping keyspace: {}", keyspace_name);
                session
                    .query_unpaged(format!("DROP KEYSPACE IF EXISTS {}", keyspace_name), &[])
                    .await?;
            }
        }

        println!("Cleanup complete");
        Ok(())
    }
}
