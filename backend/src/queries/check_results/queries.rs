use crate::database::Database;
use crate::database::preparer::CachedPreparedStatement;
use crate::eager_env;
use crate::regions::Region;
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use futures::{StreamExt, TryStreamExt, stream};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CheckResultRow {
    pub check_started_at: DateTime<Utc>,
    pub response_time_micros: i64,
    pub matches_expected: bool,
    pub region: Region,
}

static GET_RAW_CHECK_RESULTS_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT check_started_at, response_time_micros, status_code, matches_expected
    FROM check_results
    WHERE service_check_id = ?
      AND region = ?
      AND day = ?
      AND check_started_at >= ?
      AND check_started_at < ?
    ",
);

/// Get all dates in the range [from, to)
fn get_dates_in_range(from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut current = from.date_naive();
    let end = to.date_naive();

    while current < end {
        dates.push(current);
        current = current.succ_opt().expect("representable date");
    }

    // Include the end date if 'to' is at the start of a new day
    if to != to.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc() {
        dates.push(end);
    }

    dates
}

/// Query raw check results for a given time range
pub async fn get_raw_check_results(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<CheckResultRow>> {
    let dates = get_dates_in_range(from, to);

    let futures = regions.iter().flat_map(|&region| {
        dates.iter().map(move |&day| async move {
            let result = GET_RAW_CHECK_RESULTS_QUERY
                .execute_unpaged(db, (check_id, region.to_identifier(), day, from, to))
                .await?
                .into_rows_result()?;

            let rows = result.rows::<(DateTime<Utc>, i64, Option<i32>, bool)>()?;

            rows.map(|row| {
                let (check_started_at, response_time_micros, _status_code, matches_expected) = row?;
                Ok(CheckResultRow {
                    check_started_at,
                    response_time_micros,
                    matches_expected,
                    region,
                })
            })
            .collect::<Result<Vec<_>>>()
        })
    });

    stream::iter(futures)
        .buffer_unordered(*eager_env::DATABASE_CONCURRENT_REQUESTS)
        .try_collect::<Vec<_>>()
        .await
        .map(|results| results.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_get_raw_check_results() -> Result<()> {
        let (db, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let check_id = uuid!("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        let from = "2025-11-29T09:00:00Z".parse::<DateTime<Utc>>()?;
        let to = "2025-11-29T14:00:00Z".parse::<DateTime<Utc>>()?;

        // Test: Query all regions
        let results = get_raw_check_results(
            &db,
            check_id,
            &[Region::Fsn1, Region::Hel1, Region::Nbg1],
            from,
            to,
        )
        .await?;
        assert_eq!(results.len(), 8); // 4 fsn1 + 2 hel1 + 2 nbg1

        // Test: Query single region
        let results_fsn1 = get_raw_check_results(&db, check_id, &[Region::Fsn1], from, to).await?;
        assert_eq!(results_fsn1.len(), 4);
        assert!(results_fsn1.iter().all(|r| r.region == Region::Fsn1));

        // Test: Query non-existent check returns empty
        let nonexistent = uuid!("99999999-9999-9999-9999-999999999999");
        let empty = get_raw_check_results(&db, nonexistent, &[Region::Fsn1], from, to).await?;
        assert!(empty.is_empty());

        // Test: Time range filtering works
        let narrow_from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>()?;
        let narrow_to = "2025-11-29T12:00:00Z".parse::<DateTime<Utc>>()?;
        let results_narrow =
            get_raw_check_results(&db, check_id, &[Region::Fsn1], narrow_from, narrow_to).await?;
        assert_eq!(results_narrow.len(), 2); // 10:00 and 11:00

        Ok(())
    }

    #[test]
    fn test_get_dates_in_range() {
        // Single day
        let from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to = "2025-11-29T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let dates = get_dates_in_range(from, to);
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0].to_string(), "2025-11-29");

        // Multiple days
        let from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to = "2025-12-02T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let dates = get_dates_in_range(from, to);
        assert_eq!(dates.len(), 4);
        assert_eq!(dates[0].to_string(), "2025-11-29");
        assert_eq!(dates[3].to_string(), "2025-12-02");

        // Edge: to at midnight (start of day)
        let from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to = "2025-11-30T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let dates = get_dates_in_range(from, to);
        assert_eq!(dates.len(), 1);

        // Edge
        let from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to = "2025-11-30T00:00:00.001Z".parse::<DateTime<Utc>>().unwrap();
        let dates = get_dates_in_range(from, to);
        assert_eq!(dates.len(), 2);

        // Edge: same datetime
        let from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to = from;
        let dates = get_dates_in_range(from, to);
        assert_eq!(dates.len(), 1);
    }
}
