use crate::database::DATABASE_CONCURRENT_REQUESTS;
use crate::{database::Database, regions::Region, worker::check::execute::CheckResult};
use anyhow::Result;
use futures::StreamExt;
use scylla::statement::prepared::PreparedStatement;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct ResultSaveManager {
    sender: mpsc::UnboundedSender<CheckResult>,
    worker_handle: JoinHandle<()>,
}

impl ResultSaveManager {
    pub async fn new(db: Arc<Database>, region: Region) -> Result<Self> {
        let query = "
            INSERT INTO check_results (result_id,
                                       service_check_id,
                                       region,
                                       day,
                                       check_started_at,
                                       response_time_micros,
                                       status_code,
                                       matches_expected,
                                       response_body_fetched,
                                       response_body)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ";

        let prepared_statement = db.prepare(query).await?;

        let (sender, receiver) = mpsc::unbounded_channel();

        let worker_handle = tokio::spawn(Self::worker(db, prepared_statement, receiver, region));

        Ok(Self {
            sender,
            worker_handle,
        })
    }

    async fn worker(
        db: Arc<Database>,
        prepared_statement: PreparedStatement,
        receiver: mpsc::UnboundedReceiver<CheckResult>,
        region: Region,
    ) {
        UnboundedReceiverStream::new(receiver)
            .for_each_concurrent(DATABASE_CONCURRENT_REQUESTS as usize, |result| {
                let db = db.clone();
                let prepared_statement = prepared_statement.clone();
                async move {
                    if let Err(e) =
                        Self::save_single(&db, &prepared_statement, result, region).await
                    {
                        log::error!("Failed to save check result: {:?}", e);
                    }
                }
            })
            .await
    }

    async fn save_single(
        db: &Database,
        prepared_statement: &PreparedStatement,
        result: CheckResult,
        region: Region,
    ) -> Result<()> {
        let region_str = region.to_identifier();
        let day = result.check_started_at.date_naive();

        db.execute_unpaged(
            prepared_statement,
            (
                result.result_id,
                result.service_check_id,
                region_str,
                day,
                result.check_started_at,
                result.response_time_micros,
                result.status_code,
                result.matches_expected,
                result.response_body_fetched,
                result.response_body.as_ref(),
            ),
        )
        .await?;

        Ok(())
    }

    pub fn save(&self, result: CheckResult) -> Result<()> {
        self.sender.send(result)?;

        Ok(())
    }

    pub async fn close(self) {
        // Drop the sender to signal the worker to stop
        drop(self.sender);

        if let Err(e) = self.worker_handle.await {
            log::error!("Worker handle join error: {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use crate::worker::check::execute::CheckResult;
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_save_result() -> Result<()> {
        let (session, _keyspace) = create_test_database(None).await?;
        let session = Arc::new(session);

        let manager = ResultSaveManager::new(session.clone(), Region::UsEast).await?;

        let result = CheckResult {
            result_id: Uuid::new_v4(),
            service_check_id: Uuid::new_v4(),
            check_started_at: Utc::now(),
            response_time_micros: 1500,
            status_code: Some(200),
            matches_expected: true,
            response_body_fetched: false,
            response_body: None,
        };

        manager.save(result)?;

        // Close manager to flush and stop worker
        manager.close().await;

        let count: i64 = session
            .query_unpaged("SELECT COUNT(*) FROM check_results", &[])
            .await?
            .into_rows_result()?
            .single_row::<(i64,)>()?
            .0;

        assert_eq!(count, 1);
        Ok(())
    }
}
