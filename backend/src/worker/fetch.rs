use crate::{
    collab::{NodePosition, RingRange},
    database::DATABASE_CONCURRENT_REQUESTS,
    regions::Region,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::{StreamExt, stream};
use scylla::{client::session::Session, response::query_result::QueryRowsResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
}

#[derive(Clone)]
pub struct ServiceCheck {
    pub check_id: Uuid,
    pub region: Region,
    pub check_name: String,
    pub url: String,
    pub http_method: Method,
    pub check_frequency_seconds: i32,
    pub timeout_seconds: i32,
    pub expected_status_code: i32,
    pub request_headers: std::collections::HashMap<String, String>,
    pub request_body: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
}

fn parse_service_check_rows(result: QueryRowsResult) -> Result<Vec<ServiceCheck>> {
    let rows = result.rows::<(
        Uuid,
        String,
        String,
        String,
        i32,
        i32,
        i32,
        HashMap<String, String>,
        String,
        bool,
        DateTime<Utc>,
        String,
    )>()?;

    let mut checks = Vec::new();
    for row in rows {
        let (
            check_id,
            check_name,
            url,
            http_method,
            check_frequency_seconds,
            timeout_seconds,
            expected_status_code,
            request_headers,
            request_body,
            is_enabled,
            created_at,
            region_str,
        ) = row?;

        checks.push(ServiceCheck {
            check_id,
            region: region_str.parse()?,
            check_name,
            url,
            http_method: serde_plain::from_str(&http_method)?,
            check_frequency_seconds,
            timeout_seconds,
            expected_status_code,
            request_headers,
            request_body,
            is_enabled,
            created_at,
        });
    }

    Ok(checks)
}

pub async fn fetch_health_checks(
    session: &Session,
    region: Region,
    bucket_version: i16,
    ring_range: RingRange,
    ring_size: NodePosition,
) -> Result<Vec<ServiceCheck>> {
    let query = "
        SELECT check_id,
               check_name,
               url,
               http_method,
               check_frequency_seconds,
               timeout_seconds,
               expected_status_code,
               request_headers,
               request_body,
               is_enabled,
               created_at,
               region
        FROM checks
        WHERE region = ?
          AND bucket_version = ?
          AND bucket = ?
    ";

    let prepared = session.prepare(query).await?;
    let region_str = region.to_identifier();

    let buckets = ring_range.iter(ring_size);

    let all_checks = stream::iter(buckets)
        .map(|b| (b, prepared.clone()))
        .map(|(bucket, prepared)| async move {
            let result = session
                .execute_unpaged(&prepared, (region_str, bucket_version, bucket as i32))
                .await?
                .into_rows_result()?;

            parse_service_check_rows(result)
        })
        .buffer_unordered(DATABASE_CONCURRENT_REQUESTS as usize)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(all_checks)
}

impl ServiceCheck {
    #[cfg(test)]
    pub fn example() -> Self {
        use std::collections::HashMap;

        ServiceCheck {
            check_id: Uuid::new_v4(),
            region: Region::UsEast,
            check_name: "Example Health Check".to_string(),
            url: "https://example.com/health".to_string(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 30,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: String::new(),
            is_enabled: true,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_fetch_health_checks() -> Result<()> {
        let (session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        // Case 1: Single bucket with all fields
        let checks = fetch_health_checks(
            &session,
            Region::UsEast,
            1,
            RingRange { start: 0, end: 1 },
            10,
        )
        .await?;
        assert_eq!(checks.len(), 1);
        assert_eq!(
            checks[0].check_id,
            uuid!("00000000-0000-0000-0000-000000000001")
        );
        assert_eq!(checks[0].check_name, "Test Health Check 1");
        assert_eq!(checks[0].http_method, Method::Get);
        assert_eq!(checks[0].request_headers.len(), 2);

        // Case 2: Multiple buckets
        let checks = fetch_health_checks(
            &session,
            Region::UsEast,
            1,
            RingRange { start: 0, end: 3 },
            10,
        )
        .await?;
        assert_eq!(checks.len(), 3);

        // Case 3: Different region
        let checks = fetch_health_checks(
            &session,
            Region::UsWest,
            1,
            RingRange { start: 0, end: 1 },
            10,
        )
        .await?;
        assert_eq!(checks.len(), 1);
        assert_eq!(
            checks[0].check_id,
            uuid!("00000000-0000-0000-0000-000000000004")
        );

        Ok(())
    }

    #[test]
    fn test_method_serialization() -> Result<()> {
        // Test serialization
        assert_eq!(serde_plain::to_string(&Method::Get)?, "GET");

        Ok(())
    }
}
