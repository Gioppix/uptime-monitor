use super::Database;
use anyhow::Result;
use log::info;
use scylla::{response::query_result::QueryResult, statement::prepared::PreparedStatement};
use tokio::sync::Mutex;

pub struct CachedPreparedStatement {
    statement: &'static str,
    // Use Mutex to allow interior mutability (resetting the cache)
    prepared: Mutex<Option<PreparedStatement>>,
}

impl CachedPreparedStatement {
    pub const fn new(statement: &'static str) -> Self {
        Self {
            statement,
            prepared: Mutex::const_new(None),
        }
    }

    pub async fn get_prepared_statement(&self, db: &Database) -> Result<PreparedStatement> {
        let mut lock = self.prepared.lock().await;

        // Disable caching in tests. `CachedPreparedStatement` is used as static and shared among them.
        #[cfg(not(test))]
        if let Some(prepared) = &*lock {
            return Ok(prepared.clone());
        }

        info!("Preparing statement: {}", self.statement.replace('\n', " "));
        let prepared = db.prepare(self.statement).await?;
        *lock = Some(prepared.clone());

        Ok(prepared)
    }

    pub async fn optimistically_prepare(&self, db: &Database) -> Result<()> {
        self.get_prepared_statement(db).await?;
        Ok(())
    }

    /// A thin wrapper around [`Session::execute_unpaged`].
    ///
    /// Please refer to that function's documentation for details on arguments
    /// and behavior.
    pub async fn execute_unpaged(
        &self,
        db: &Database,
        values: impl scylla::serialize::row::SerializeRow,
    ) -> Result<QueryResult> {
        let prepared = self.get_prepared_statement(db).await?;

        db.execute_unpaged(&prepared, &values)
            .await
            .map_err(Into::into)
    }
}
