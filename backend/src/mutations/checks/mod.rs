use crate::database::Database;
use crate::database::preparer::CachedPreparedStatement;
use crate::regions::Region;
use crate::{collab::get_bucket_for_check, worker::Method};
use anyhow::Result;
use chrono::{DateTime, Utc};
use scylla::statement::batch::Batch;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckData {
    pub check_name: String,
    pub url: String,
    pub http_method: Method,
    pub check_frequency_seconds: i32,
    pub timeout_seconds: i32,
    pub expected_status_code: i32,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<String>,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Check {
    pub check_id: Uuid,
    pub regions: Vec<Region>,
    #[serde(flatten)]
    pub data: CheckData,
}

static GET_CHECK_BY_ID_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT check_id,
           region,
           bucket_version,
           bucket,
           check_name,
           url,
           http_method,
           check_frequency_seconds,
           timeout_seconds,
           expected_status_code,
           request_headers,
           request_body,
           is_enabled,
           created_at
    FROM checks
    WHERE region IN ?
      AND bucket_version = ?
      AND bucket = ?
      AND check_id = ?
    ",
);

pub async fn get_check_by_id(session: &Database, check_id: Uuid) -> Result<Option<Check>> {
    let (bucket_version, bucket) = get_bucket_for_check(check_id);
    let all_regions = Region::get_all_region_identifiers();

    let result = GET_CHECK_BY_ID_QUERY
        .execute_unpaged(session, (all_regions, bucket_version, bucket, check_id))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(
        Uuid,
        String,
        i16,
        i32,
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
    )>()?;

    let mut regions_found = Vec::new();
    let mut check_data = None;

    for row in rows {
        let (
            _check_id,
            region,
            _bucket_version,
            _bucket,
            check_name,
            url,
            http_method_str,
            check_frequency_seconds,
            timeout_seconds,
            expected_status_code,
            request_headers,
            request_body,
            is_enabled,
            created_at,
        ) = row?;

        if let Ok(region_enum) = Region::from_identifier(&region) {
            regions_found.push(region_enum);
        }

        if check_data.is_none() {
            let http_method = serde_plain::from_str(&http_method_str)?;
            check_data = Some((
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
            ));
        }
    }

    match check_data {
        Some((
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
        )) => Ok(Some(Check {
            check_id,
            regions: regions_found,
            data: CheckData {
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
            },
        })),
        None => Ok(None),
    }
}

static CREATE_CHECK_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO checks (check_id, region, bucket_version, bucket, check_name, url,
                        http_method, check_frequency_seconds, timeout_seconds, expected_status_code,
                        request_headers, request_body, is_enabled, created_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ",
);

pub async fn create_check(db: &Database, regions: Vec<Region>, data: CheckData) -> Result<Check> {
    if regions.is_empty() {
        anyhow::bail!("At least one region must be specified");
    }

    let check_id = Uuid::new_v4();
    let (bucket_version, bucket) = get_bucket_for_check(check_id);

    // Use batched writes for multiple regions
    let mut batch = Batch::default();
    let mut batch_values = Vec::new();
    let query = CREATE_CHECK_QUERY.get_prepared_statement(db).await?;

    let http_method_str = serde_plain::to_string(&data.http_method)?;

    for region in &regions {
        batch.append_statement(query.clone());
        batch_values.push((
            check_id,
            region.to_identifier(),
            bucket_version,
            bucket,
            data.check_name.clone(),
            data.url.clone(),
            http_method_str.clone(),
            data.check_frequency_seconds,
            data.timeout_seconds,
            data.expected_status_code,
            data.request_headers.clone(),
            data.request_body.clone(),
            data.is_enabled,
            data.created_at,
        ));
    }

    db.batch(&batch, batch_values).await?;

    Ok(Check {
        check_id,
        regions,
        data,
    })
}

pub async fn update_check(session: &Database, check: Check) -> Result<()> {
    let (bucket_version, bucket) = get_bucket_for_check(check.check_id);
    let all_regions = Region::get_all_region_identifiers();

    // First delete from all regions
    let delete_query = "
        DELETE
        FROM checks
        WHERE region = ?
          AND bucket_version = ?
          AND bucket = ?
          AND check_id = ?
    ";
    let mut delete_batch = Batch::default();
    let mut delete_values = Vec::new();

    for region in all_regions {
        delete_batch.append_statement(delete_query);
        delete_values.push((region, bucket_version, bucket, check.check_id));
    }

    session.batch(&delete_batch, delete_values).await?;

    // Then insert into the specified regions
    let insert_query = "
        INSERT INTO checks (check_id,
                                    region,
                                    bucket_version,
                                    bucket,
                                    check_name,
                                    url,
                                    http_method,
                                    check_frequency_seconds,
                                    timeout_seconds,
                                    expected_status_code,
                                    request_headers,
                                    request_body,
                                    is_enabled,
                                    created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ";

    let mut insert_batch = Batch::default();
    let mut insert_values = Vec::new();

    let http_method_str = serde_plain::to_string(&check.data.http_method)?;

    for region in &check.regions {
        insert_batch.append_statement(insert_query);
        insert_values.push((
            check.check_id,
            region.to_identifier(),
            bucket_version,
            bucket,
            &check.data.check_name,
            &check.data.url,
            &http_method_str,
            check.data.check_frequency_seconds,
            check.data.timeout_seconds,
            check.data.expected_status_code,
            &check.data.request_headers,
            &check.data.request_body,
            check.data.is_enabled,
            check.data.created_at,
        ));
    }

    session.batch(&insert_batch, insert_values).await?;

    Ok(())
}

static DELETE_CHECK_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    DELETE
    FROM checks
    WHERE region IN ?
      AND bucket_version = ?
      AND bucket = ?
      AND check_id = ?
    ",
);

pub async fn delete_check(session: &Database, check_id: Uuid) -> Result<()> {
    let (bucket_version, bucket) = get_bucket_for_check(check_id);
    let all_regions = Region::get_all_region_identifiers();

    DELETE_CHECK_QUERY
        .execute_unpaged(session, (all_regions, bucket_version, bucket, check_id))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;

    #[tokio::test]
    async fn test_checks_basic() -> Result<()> {
        let (session, _keyspace) = create_test_database(None).await?;

        // Create check in multiple regions
        let regions = vec![Region::UsWest, Region::UsEast];
        let data = CheckData {
            check_name: "Test Check".to_string(),
            url: "https://example.com".to_string(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 10,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: None,
            is_enabled: true,
            created_at: Utc::now(),
        };

        let check = create_check(&session, regions.clone(), data).await?;
        assert_eq!(check.regions.len(), 2);
        let check_id = check.check_id;

        // Verify we can get it by check_id
        let retrieved = get_check_by_id(&session, check_id).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().unwrap().regions.len(), 2);

        // Test update
        let mut updated_check = check.clone();
        updated_check.data.check_name = "Updated Check".to_string();
        // Update regions: remove UsWest, add EuWest
        updated_check.regions = vec![Region::UsEast, Region::EuWest];
        update_check(&session, updated_check).await?;

        // Verify update
        let retrieved = get_check_by_id(&session, check_id).await?;
        let retrieved_check = retrieved.unwrap();
        assert_eq!(retrieved_check.data.check_name, "Updated Check");
        assert_eq!(retrieved_check.regions.len(), 2);

        let mut actual_regions = retrieved_check.regions.clone();
        actual_regions.sort();

        let mut expected_regions = vec![Region::UsEast, Region::EuWest];
        expected_regions.sort();

        assert_eq!(actual_regions, expected_regions);

        // Test delete
        delete_check(&session, check_id).await?;
        let deleted = get_check_by_id(&session, check_id).await?;
        assert!(deleted.is_none());

        Ok(())
    }
}
