mod calculator;
mod queries;

use crate::regions::Region;
use crate::{database::Database, eager_env};
use anyhow::{Result, bail};
use calculator::{calculate_by_region_metrics, calculate_overall_metrics};
use chrono::{DateTime, NaiveDate, Timelike, Utc};
use futures::{StreamExt, TryStreamExt};
use queries::get_raw_check_results_range;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsSummary {
    pub uptime_percent: f32,

    pub total_checks: u32,
    pub successful_checks: u32,
    pub failed_checks: u32,

    pub avg_response_time_micros: i64,
    pub min_response_time_micros: i64,
    pub max_response_time_micros: i64,

    pub p50_response_time_micros: i64,
    pub p95_response_time_micros: i64,
    pub p99_response_time_micros: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    #[serde(flatten)]
    pub overall: MetricsSummary,
    pub by_region: HashMap<Region, MetricsSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponseDate {
    pub by_region: HashMap<Region, MetricsSummary>,
    pub date: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, Deserialize, ToSchema)]
pub enum GraphGranularity {
    Hourly,
    Daily,
}

/// Main function to get metrics for a check
pub async fn get_check_metrics(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<MetricsResponse> {
    // TODO: Try to get pre-aggregated data

    // Query raw data and aggregate
    let mut raw_results = get_raw_check_results_range(db, check_id, regions, from, to).await?;
    raw_results.sort_by_key(|r| r.check_started_at);

    let overall = calculate_overall_metrics(&raw_results);
    let by_region = calculate_by_region_metrics(&raw_results);

    // TODO: Cache the computed metrics back to the database

    Ok(MetricsResponse { overall, by_region })
}

/// Gets check results metrics for the time range `[from, to)`
///
/// `from` and `to` must be aligned to the granularity.
/// `to` must be a past date.
/// Example: `Hourly`, `2017-01-01 01:00:00 UTC`
pub async fn get_check_metrics_graph(
    db: &Database,
    check_id: Uuid,
    regions: &[Region],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    granularity: GraphGranularity,
) -> Result<Vec<MetricsResponseDate>> {
    if !is_rounded_to_granularity(from, granularity) {
        bail!("'from' must be rounded");
    }
    if !is_rounded_to_granularity(to, granularity) {
        bail!("'to' must be rounded");
    }

    // Fetch cached results
    let cached_results =
        queries::get_cached_check_results(db, check_id, regions, from, to, granularity).await?;

    // Generate all expected dates based on granularity
    let expected_dates: Vec<DateTime<Utc>> = match granularity {
        GraphGranularity::Hourly => get_hours_in_range(from, to),
        GraphGranularity::Daily => get_days_in_range(from, to)
            .into_iter()
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .collect(),
    };

    // Find which dates are missing from cache
    let cached_dates: HashSet<_> = cached_results.iter().map(|r| r.date).collect();
    let missing_dates: Vec<_> = expected_dates
        .iter()
        .filter(|d| !cached_dates.contains(d))
        .copied()
        .collect();

    let mut all_results = cached_results;

    // Calculate missing dates from raw data in parallel
    let futures = missing_dates.iter().map(|date| async move {
        let range_from = *date;
        let range_to = match granularity {
            GraphGranularity::Hourly => range_from + chrono::Duration::hours(1),
            GraphGranularity::Daily => range_from + chrono::Duration::days(1),
        };

        // Query raw data for this period
        let mut raw_results =
            get_raw_check_results_range(db, check_id, regions, range_from, range_to).await?;
        raw_results.sort_by_key(|r| r.check_started_at);

        // Calculate metrics
        let by_region = calculate_by_region_metrics(&raw_results);

        // If the range is completed (to <= now), write to cache
        if range_to <= Utc::now() {
            queries::insert_cached_check_result(db, check_id, *date, &by_region, granularity)
                .await?;
        }

        // Convert to results grouped by region
        let results: Vec<queries::MetricsSummaryRegionDate> = by_region
            .into_iter()
            .map(|(region, metrics)| queries::MetricsSummaryRegionDate {
                metrics_summary: metrics,
                region,
                date: *date,
            })
            .collect();

        Ok::<_, anyhow::Error>(results)
    });

    let missing_results: Vec<_> = futures::stream::iter(futures)
        .buffer_unordered(*eager_env::DATABASE_CONCURRENT_REQUESTS)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .collect();

    all_results.extend(missing_results);

    // Convert MetricsSummaryRegionDate to MetricsResponseDate
    // Group by date and combine regions
    let mut final_results: Vec<_> = all_results
        .into_iter()
        .fold(HashMap::new(), |mut acc, result| {
            acc.entry(result.date)
                .or_insert_with(HashMap::new)
                .insert(result.region, result.metrics_summary);
            acc
        })
        .into_iter()
        .map(|(date, by_region)| MetricsResponseDate { by_region, date })
        .collect();

    // Sort by date
    final_results.sort_by_key(|r| r.date);

    Ok(final_results)
}

/// Check if a DateTime is rounded to the hour
pub fn is_rounded_to_granularity(dt: DateTime<Utc>, graph_granularity: GraphGranularity) -> bool {
    dt.minute() == 0
        && dt.second() == 0
        && dt.nanosecond() == 0
        && match graph_granularity {
            GraphGranularity::Hourly => true,
            GraphGranularity::Daily => dt.hour() == 0,
        }
}

/// Generate all hours in the range [from, to)
///
/// Leap seconds are ignored.
/// Expects `from` and `to` to be aligned.
pub fn get_hours_in_range(from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let mut hours = Vec::new();
    let mut current = from;
    while current < to {
        hours.push(current);
        current += chrono::Duration::hours(1);
    }
    hours
}

/// Generate all days in the range [from, to)
///
/// Returns NaiveDates for each day in the range.
/// Expects `from` and `to` to be aligned.
fn get_days_in_range(from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<NaiveDate> {
    let mut days = Vec::new();
    let mut current = from.date_naive();
    let to_date = to.date_naive();

    while current < to_date {
        days.push(current);
        current = current.succ_opt().expect("representable date");
    }

    days
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_get_check_metrics_integration() -> Result<()> {
        let (db, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let check_id = uuid!("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        let from = "2025-11-29T09:00:00Z".parse::<DateTime<Utc>>()?;
        let to = "2025-11-29T14:00:00Z".parse::<DateTime<Utc>>()?;

        // Test: Get metrics for all regions with 100% uptime
        let metrics = get_check_metrics(
            &db,
            check_id,
            &[Region::Fsn1, Region::Nbg1, Region::Hel1],
            from,
            to,
        )
        .await?;
        assert_eq!(metrics.overall.uptime_percent, 100.0);
        assert!(metrics.overall.avg_response_time_micros > 0);
        assert_eq!(metrics.by_region.len(), 3); // fsn1, hel1, nbg1

        // Test: Each region has flattened metrics
        for (_region, metrics) in metrics.by_region {
            assert!(metrics.uptime_percent >= 0.0);
            assert!(metrics.avg_response_time_micros > 0);
        }

        // Test: Specific region filter
        let metrics_fsn1 = get_check_metrics(&db, check_id, &[Region::Fsn1], from, to).await?;
        assert_eq!(metrics_fsn1.by_region.len(), 1);
        assert!(metrics_fsn1.by_region.contains_key(&Region::Fsn1));
        assert_eq!(metrics_fsn1.by_region[&Region::Fsn1].uptime_percent, 100.0);

        // Test: Mixed success/failure check (time-weighted uptime)
        let check_mixed = uuid!("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        let metrics_mixed = get_check_metrics(
            &db,
            check_mixed,
            &[Region::Fsn1, Region::Nbg1, Region::Hel1],
            from,
            "2025-11-29T20:00:00Z".parse::<DateTime<Utc>>()?,
        )
        .await?;
        // Time-weighted: 7/9 intervals successful = 77.78%
        assert!((metrics_mixed.overall.uptime_percent - 77.78).abs() < 0.01);
        assert!(metrics_mixed.overall.avg_response_time_micros > 0);

        // Test: Empty result for non-existent check
        let nonexistent = uuid!("99999999-9999-9999-9999-999999999999");
        let empty = get_check_metrics(&db, nonexistent, &[], from, to).await?;
        assert_eq!(empty.overall.uptime_percent, 0.0);
        assert!(empty.by_region.is_empty());

        Ok(())
    }

    #[test]
    fn test_is_rounded_to_gran() {
        // Rounded to hour
        let dt = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(is_rounded_to_granularity(dt, GraphGranularity::Hourly));

        // Not rounded - has minutes
        let dt = "2025-11-29T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(!is_rounded_to_granularity(dt, GraphGranularity::Hourly));

        // Not rounded - has seconds
        let dt = "2025-11-29T10:00:30Z".parse::<DateTime<Utc>>().unwrap();
        assert!(!is_rounded_to_granularity(dt, GraphGranularity::Hourly));

        // Not rounded - has nanoseconds
        let dt = "2025-11-29T10:00:00.001Z".parse::<DateTime<Utc>>().unwrap();
        assert!(!is_rounded_to_granularity(dt, GraphGranularity::Hourly));

        // Midnight is rounded to hour
        let dt = "2025-11-29T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(is_rounded_to_granularity(dt, GraphGranularity::Hourly));

        // Rounded to day (midnight)
        let dt = "2025-11-29T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(is_rounded_to_granularity(dt, GraphGranularity::Daily));

        // Not rounded to day - has hours
        let dt = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(!is_rounded_to_granularity(dt, GraphGranularity::Daily));
    }
}
