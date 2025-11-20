use crate::{
    collab::{NodePosition, RingRange},
    database::{DATABASE_CONCURRENT_REQUESTS, preparer::CachedPreparedStatement},
    regions::Region,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::{StreamExt, stream};
use itertools::Itertools;
use log::{error, warn};
use scylla::{client::session::Session, response::query_result::QueryRowsResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ServiceCheck {
    pub check_id: Uuid,
    pub region: Region,
    pub check_name: String,
    #[serde(deserialize_with = "deserialize_url")]
    pub url: Url,
    pub http_method: Method,
    pub check_frequency_seconds: i32,
    pub timeout_seconds: i32,
    pub expected_status_code: i32,
    pub request_headers: std::collections::HashMap<String, String>,
    pub request_body: Option<String>,
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
        Option<String>,
        bool,
        DateTime<Utc>,
        String,
    )>()?;

    let maybe_checks: Vec<Result<_>> = rows
        .into_iter()
        .map(|row| {
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

            let check = ServiceCheck {
                check_id,
                region: region_str.parse()?,
                check_name,
                url: url.parse()?,
                http_method: serde_plain::from_str(&http_method)?,
                check_frequency_seconds,
                timeout_seconds,
                expected_status_code,
                request_headers,
                request_body,
                is_enabled,
                created_at,
            };

            Ok(check)
        })
        .collect();

    let (checks, errors): (Vec<_>, Vec<_>) = maybe_checks.into_iter().partition_result();

    if !errors.is_empty() {
        error!(
            "Failed to parse [{}] checks. First 3 errors: {}",
            errors.len(),
            errors
                .iter()
                .take(3)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(checks)
}

static HEALTH_CHECKS_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
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
    ",
);

pub async fn fetch_health_checks(
    session: &Session,
    region: Region,
    bucket_version: i16,
    ring_range: RingRange,
    ring_size: NodePosition,
) -> Result<Vec<ServiceCheck>> {
    let region_str = region.to_identifier();

    let buckets = ring_range.iter(ring_size);

    let all_checks = stream::iter(buckets)
        .map(|bucket| async move {
            let result = HEALTH_CHECKS_QUERY
                .execute_unpaged(session, (region_str, bucket_version, bucket as i32))
                .await?
                .into_rows_result()?;

            warn!("Fetching bucket {bucket}");
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
            url: "https://example.com/health".parse().unwrap(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 30,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: None,
            is_enabled: true,
            created_at: Utc::now(),
        }
    }
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    ServiceCheck::parse_url(&s).map_err(serde::de::Error::custom)
}

impl ServiceCheck {
    fn parse_url(url_str: &str) -> Result<Url, anyhow::Error> {
        let url: Url = url_str.parse()?;

        match url.scheme() {
            "http" | "https" => Ok(url),
            scheme => anyhow::bail!(
                "Invalid URL scheme: {}. Only http and https are allowed",
                scheme
            ),
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

    #[tokio::test]
    async fn test_fetch_health_checks_with_malformed() -> Result<()> {
        let (session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        // Fetch checks from EuWest region which has 4 checks, one with empty URL
        // Should return only the 3 valid checks without erroring
        let checks = fetch_health_checks(
            &session,
            Region::EuWest,
            1,
            RingRange { start: 0, end: 4 },
            10,
        )
        .await?;

        let check_ids: Vec<_> = checks.iter().map(|c| c.check_id).collect();
        assert!(check_ids.contains(&uuid!("00000000-0000-0000-0000-000000000101")));
        assert!(check_ids.contains(&uuid!("00000000-0000-0000-0000-000000000102")));
        assert!(check_ids.contains(&uuid!("00000000-0000-0000-0000-000000000104")));
        assert!(!check_ids.contains(&uuid!("00000000-0000-0000-0000-000000000103"))); // malformed

        Ok(())
    }

    #[test]
    fn test_method_serialization() -> Result<()> {
        // Test serialization
        assert_eq!(serde_plain::to_string(&Method::Get)?, "GET");

        Ok(())
    }

    #[test]
    fn test_url_deserialization() -> Result<()> {
        ServiceCheck::parse_url("http://example.com")?;
        ServiceCheck::parse_url("https://api.example.com/v1/health")?;
        ServiceCheck::parse_url("http://localhost:8080/status")?;
        ServiceCheck::parse_url("https://example.com/search?q=test&limit=10")?;
        ServiceCheck::parse_url("https://docs.example.com/page#section")?;

        assert!(ServiceCheck::parse_url("ftp://example.com").is_err());
        assert!(ServiceCheck::parse_url("ws://example.com").is_err());
        assert!(ServiceCheck::parse_url("file:///etc/passwd").is_err());
        assert!(ServiceCheck::parse_url("javascript:alert(1)").is_err());

        assert!(ServiceCheck::parse_url("not a url").is_err());
        assert!(ServiceCheck::parse_url("").is_err());

        Ok(())
    }
}
