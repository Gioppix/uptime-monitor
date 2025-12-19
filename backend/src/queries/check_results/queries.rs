use std::collections::HashMap;

use crate::database::preparer::CachedPreparedStatement;
use crate::eager_env;
use crate::queries::check_results::GraphGranularity;
use crate::regions::Region;
use crate::{database::Database, queries::check_results::MetricsSummary};
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use futures::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CheckResultRow {
    pub check_started_at: DateTime<Utc>,
    pub response_time_micros: i64,
    pub matches_expected: bool,
    pub region: Region,
}

static GET_RAW_CHECK_RESULTS_QUERY_RANGE: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT region,
           check_started_at,
           response_time_micros,
           status_code,
           matches_expected
    FROM check_results
    WHERE service_check_id = ?
      AND region IN ?
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
pub async fn get_raw_check_results_range(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<CheckResultRow>> {
    let dates = get_dates_in_range(from, to);
    let regions_vec: Vec<_> = regions.iter().map(|r| r.to_identifier()).collect();

    let futures =
        dates
            .iter()
            .map(|date| (date, &regions_vec))
            .map(move |(&day, regions_vec)| async move {
                let result = GET_RAW_CHECK_RESULTS_QUERY_RANGE
                    .execute_unpaged(db, (check_id, &regions_vec, day, from, to))
                    .await?
                    .into_rows_result()?;

                let rows = result.rows::<(String, DateTime<Utc>, i64, Option<i32>, bool)>()?;

                rows.map(|row| {
                    let (
                        region_id,
                        check_started_at,
                        response_time_micros,
                        _status_code,
                        matches_expected,
                    ) = row?;
                    let region = Region::from_identifier(&region_id)?;
                    Ok(CheckResultRow {
                        check_started_at,
                        response_time_micros,
                        matches_expected,
                        region,
                    })
                })
                .collect::<Result<Vec<_>>>()
            });

    stream::iter(futures)
        .buffer_unordered(*eager_env::DATABASE_CONCURRENT_REQUESTS)
        .try_collect::<Vec<_>>()
        .await
        .map(|results| results.into_iter().flatten().collect())
}

static GET_CACHED_HOURLY_CHECK_RESULTS_QUERY: CachedPreparedStatement =
    CachedPreparedStatement::new(
        "
        SELECT region,
               hour,
               successful_checks,
               failed_checks,
               avg_response_time_micros,
               min_response_time_micros,
               max_response_time_micros,
               p50_response_time_micros,
               p95_response_time_micros,
               p99_response_time_micros,
               uptime_percent
        FROM check_results_hourly
        WHERE service_check_id = ?
          AND region IN ?
          AND hour >= ?
          AND hour < ?
        ",
    );

static GET_CACHED_DAILY_CHECK_RESULTS_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT region,
           day,
           successful_checks,
           failed_checks,
           avg_response_time_micros,
           min_response_time_micros,
           max_response_time_micros,
           p50_response_time_micros,
           p95_response_time_micros,
           p99_response_time_micros,
           uptime_percent
    FROM check_results_daily
    WHERE service_check_id = ?
      AND region IN ?
      AND day >= ?
      AND day < ?
    ",
);

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsSummaryRegionDate {
    pub metrics_summary: MetricsSummary,
    pub date: DateTime<Utc>,
    pub region: Region,
}

/// Get cached check results for the time range `[from, to)`.
///
/// Assumes `from` and `to` are already rounded to granularity.
pub async fn get_cached_check_results(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    granularity: GraphGranularity,
) -> Result<Vec<MetricsSummaryRegionDate>> {
    match granularity {
        GraphGranularity::Hourly => {
            get_hourly_cached_check_results(db, check_id, regions, from, to).await
        }
        GraphGranularity::Daily => {
            get_daily_cached_check_results(db, check_id, regions, from, to).await
        }
    }
}

/// Assumes `from` and `to` are already rounded to granularity.
pub async fn get_hourly_cached_check_results(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<MetricsSummaryRegionDate>> {
    let regions_vec: Vec<_> = regions.iter().map(|r| r.to_identifier()).collect();

    let result = GET_CACHED_HOURLY_CHECK_RESULTS_QUERY
        .execute_unpaged(db, (check_id, &regions_vec, from, to))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(
        String,
        DateTime<Utc>,
        i32,
        i32,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        f32,
    )>()?;

    rows.map(|row| {
        let (
            region_id,
            hour,
            successful_checks,
            failed_checks,
            avg_response_time_micros,
            min_response_time_micros,
            max_response_time_micros,
            p50_response_time_micros,
            p95_response_time_micros,
            p99_response_time_micros,
            uptime_percent,
        ) = row?;
        let region = Region::from_identifier(&region_id)?;
        Ok(MetricsSummaryRegionDate {
            metrics_summary: MetricsSummary {
                uptime_percent,
                total_checks: (successful_checks + failed_checks) as u32,
                successful_checks: successful_checks as u32,
                failed_checks: failed_checks as u32,
                avg_response_time_micros,
                min_response_time_micros,
                max_response_time_micros,
                p50_response_time_micros,
                p95_response_time_micros,
                p99_response_time_micros,
            },
            date: hour,
            region,
        })
    })
    .collect::<Result<Vec<_>>>()
}

/// Assumes `from` and `to` are already rounded to granularity.
pub async fn get_daily_cached_check_results(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<MetricsSummaryRegionDate>> {
    let regions_vec: Vec<_> = regions.iter().map(|r| r.to_identifier()).collect();

    let result = GET_CACHED_DAILY_CHECK_RESULTS_QUERY
        .execute_unpaged(
            db,
            (check_id, &regions_vec, from.date_naive(), to.date_naive()),
        )
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(
        String,
        NaiveDate,
        i32,
        i32,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        f32,
    )>()?;

    rows.map(|row| {
        let (
            region_id,
            day,
            successful_checks,
            failed_checks,
            avg_response_time_micros,
            min_response_time_micros,
            max_response_time_micros,
            p50_response_time_micros,
            p95_response_time_micros,
            p99_response_time_micros,
            uptime_percent,
        ) = row?;
        let region = Region::from_identifier(&region_id)?;
        Ok(MetricsSummaryRegionDate {
            metrics_summary: MetricsSummary {
                uptime_percent,
                total_checks: (successful_checks + failed_checks) as u32,
                successful_checks: successful_checks as u32,
                failed_checks: failed_checks as u32,
                avg_response_time_micros,
                min_response_time_micros,
                max_response_time_micros,
                p50_response_time_micros,
                p95_response_time_micros,
                p99_response_time_micros,
            },
            date: day.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            region,
        })
    })
    .collect::<Result<Vec<_>>>()
}

static INSERT_HOURLY_CACHED_CHECK_RESULTS: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO check_results_hourly (service_check_id,
                                      region,
                                      hour,
                                      successful_checks,
                                      failed_checks,
                                      avg_response_time_micros,
                                      min_response_time_micros,
                                      max_response_time_micros,
                                      p50_response_time_micros,
                                      p95_response_time_micros,
                                      p99_response_time_micros,
                                      uptime_percent,
                                      computed_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ",
);

static INSERT_DAILY_CACHED_CHECK_RESULTS: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO check_results_daily (service_check_id,
                                     region,
                                     day,
                                     successful_checks,
                                     failed_checks,
                                     avg_response_time_micros,
                                     min_response_time_micros,
                                     max_response_time_micros,
                                     p50_response_time_micros,
                                     p95_response_time_micros,
                                     p99_response_time_micros,
                                     uptime_percent,
                                     computed_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ",
);

pub async fn insert_hourly_cached_check_result(
    db: &Database,
    check_id: Uuid,
    region: Region,
    date: DateTime<Utc>,
    metrics: &MetricsSummary,
) -> Result<()> {
    INSERT_HOURLY_CACHED_CHECK_RESULTS
        .execute_unpaged(
            db,
            (
                check_id,
                region.to_identifier(),
                date,
                metrics.successful_checks as i32,
                metrics.failed_checks as i32,
                metrics.avg_response_time_micros,
                metrics.min_response_time_micros,
                metrics.max_response_time_micros,
                metrics.p50_response_time_micros,
                metrics.p95_response_time_micros,
                metrics.p99_response_time_micros,
                metrics.uptime_percent,
                Utc::now(),
            ),
        )
        .await?;

    Ok(())
}

pub async fn insert_daily_cached_check_result(
    db: &Database,
    check_id: Uuid,
    region: Region,
    date: DateTime<Utc>,
    metrics: &MetricsSummary,
) -> Result<()> {
    INSERT_DAILY_CACHED_CHECK_RESULTS
        .execute_unpaged(
            db,
            (
                check_id,
                region.to_identifier(),
                date.date_naive(),
                metrics.successful_checks as i32,
                metrics.failed_checks as i32,
                metrics.avg_response_time_micros,
                metrics.min_response_time_micros,
                metrics.max_response_time_micros,
                metrics.p50_response_time_micros,
                metrics.p95_response_time_micros,
                metrics.p99_response_time_micros,
                metrics.uptime_percent,
                Utc::now(),
            ),
        )
        .await?;

    Ok(())
}

pub async fn insert_cached_check_result(
    db: &Database,
    check_id: Uuid,
    date: DateTime<Utc>,
    summaries: &HashMap<Region, MetricsSummary>,
    granularity: GraphGranularity,
) -> Result<()> {
    let futures = summaries
        .iter()
        .map(|(region, metrics_summary)| async move {
            match granularity {
                GraphGranularity::Hourly => {
                    insert_hourly_cached_check_result(db, check_id, *region, date, metrics_summary)
                        .await
                }
                GraphGranularity::Daily => {
                    insert_daily_cached_check_result(db, check_id, *region, date, metrics_summary)
                        .await
                }
            }
        });

    stream::iter(futures)
        .buffer_unordered(*eager_env::DATABASE_CONCURRENT_REQUESTS)
        .try_collect::<Vec<_>>()
        .await?;
    Ok(())
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
        let results = get_raw_check_results_range(
            &db,
            check_id,
            &[Region::Fsn1, Region::Hel1, Region::Nbg1],
            from,
            to,
        )
        .await?;
        assert_eq!(results.len(), 8); // 4 fsn1 + 2 hel1 + 2 nbg1

        // Test: Query single region
        let results_fsn1 =
            get_raw_check_results_range(&db, check_id, &[Region::Fsn1], from, to).await?;
        assert_eq!(results_fsn1.len(), 4);
        assert!(results_fsn1.iter().all(|r| r.region == Region::Fsn1));

        // Test: Query non-existent check returns empty
        let nonexistent = uuid!("99999999-9999-9999-9999-999999999999");
        let empty =
            get_raw_check_results_range(&db, nonexistent, &[Region::Fsn1], from, to).await?;
        assert!(empty.is_empty());

        // Test: Time range filtering works
        let narrow_from = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>()?;
        let narrow_to = "2025-11-29T12:00:00Z".parse::<DateTime<Utc>>()?;
        let results_narrow =
            get_raw_check_results_range(&db, check_id, &[Region::Fsn1], narrow_from, narrow_to)
                .await?;
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

    #[tokio::test]
    async fn test_cached_metrics() -> Result<()> {
        let (db, _keyspace) = create_test_database(Some(FIXTURES)).await?;
        let check_id = uuid!("cccccccc-cccc-cccc-cccc-cccccccccccc");

        // Test hourly metrics - read existing
        let from_hourly = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to_hourly = "2025-11-29T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let hourly_results = get_hourly_cached_check_results(
            &db,
            check_id,
            &[Region::Fsn1, Region::Nbg1],
            from_hourly,
            to_hourly,
        )
        .await?;
        assert_eq!(hourly_results.len(), 2);

        // Test daily metrics - read existing
        let from_daily = "2025-11-27T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let to_daily = "2025-11-29T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let daily_results = get_daily_cached_check_results(
            &db,
            check_id,
            &[Region::Fsn1, Region::Nbg1],
            from_daily,
            to_daily,
        )
        .await?;
        assert_eq!(daily_results.len(), 3);

        // Insert new hourly metric
        let new_hourly_date = "2025-11-29T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let new_metrics = MetricsSummary {
            uptime_percent: 99.5,
            total_checks: 100,
            successful_checks: 99,
            failed_checks: 1,
            avg_response_time_micros: 105000,
            min_response_time_micros: 70000,
            max_response_time_micros: 250000,
            p50_response_time_micros: 100000,
            p95_response_time_micros: 200000,
            p99_response_time_micros: 240000,
        };
        insert_hourly_cached_check_result(
            &db,
            check_id,
            Region::Fsn1,
            new_hourly_date,
            &new_metrics,
        )
        .await?;

        // Verify hourly insertion
        let verify_from = "2025-11-29T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let verify_to = "2025-11-29T15:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let verified =
            get_hourly_cached_check_results(&db, check_id, &[Region::Fsn1], verify_from, verify_to)
                .await?;
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].metrics_summary.successful_checks, 99);

        // Insert new daily metric
        let new_daily_date = "2025-11-30T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        insert_daily_cached_check_result(&db, check_id, Region::Hel1, new_daily_date, &new_metrics)
            .await?;

        // Verify daily insertion
        let verify_daily_from = "2025-11-30T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let verify_daily_to = "2025-12-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let verified_daily = get_daily_cached_check_results(
            &db,
            check_id,
            &[Region::Hel1],
            verify_daily_from,
            verify_daily_to,
        )
        .await?;
        assert_eq!(verified_daily.len(), 1);
        assert_eq!(verified_daily[0].metrics_summary.uptime_percent, 99.5);

        Ok(())
    }
}
