mod calculator;
mod queries;

use crate::database::Database;
use crate::regions::Region;
use anyhow::Result;
use calculator::{calculate_by_region_metrics, calculate_overall_metrics};
use chrono::{DateTime, Utc};
use queries::get_raw_check_results;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsSummary {
    pub uptime_percent: f64,
    pub avg_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegionMetrics {
    pub region: Region,
    #[serde(flatten)]
    pub metrics: MetricsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    #[serde(flatten)]
    pub overall: MetricsSummary,
    pub by_region: Vec<RegionMetrics>,
}

/// Main function to get metrics for a check
pub async fn get_check_metrics(
    db: &Database,
    check_id: Uuid,
    regions: Option<Vec<Region>>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<MetricsResponse> {
    // Determine which regions to query
    let query_regions = match regions {
        Some(r) => r,
        None => Region::iter().collect(),
    };

    // TODO: Try to get pre-aggregated data first
    // For now, we always query raw data

    // Query raw data and aggregate
    let mut raw_results =
        get_raw_check_results(db, check_id, query_regions.as_slice(), from, to).await?;
    raw_results.sort_by_key(|r| r.check_started_at);

    let overall = calculate_overall_metrics(&raw_results);
    let by_region = calculate_by_region_metrics(&raw_results);

    // TODO: Cache the computed metrics back to the database

    Ok(MetricsResponse { overall, by_region })
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
        let metrics = get_check_metrics(&db, check_id, None, from, to).await?;
        assert_eq!(metrics.overall.uptime_percent, 100.0);
        assert!(metrics.overall.avg_response_time_ms > 0.0);
        assert_eq!(metrics.by_region.len(), 3); // fsn1, hel1, nbg1

        // Test: Each region has flattened metrics
        for region_metrics in &metrics.by_region {
            assert!(region_metrics.metrics.uptime_percent >= 0.0);
            assert!(region_metrics.metrics.avg_response_time_ms > 0.0);
        }

        // Test: Specific region filter
        let metrics_fsn1 =
            get_check_metrics(&db, check_id, Some(vec![Region::Fsn1]), from, to).await?;
        assert_eq!(metrics_fsn1.by_region.len(), 1);
        assert_eq!(metrics_fsn1.by_region[0].region, Region::Fsn1);
        assert_eq!(metrics_fsn1.by_region[0].metrics.uptime_percent, 100.0);

        // Test: Mixed success/failure check (time-weighted uptime)
        let check_mixed = uuid!("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        let metrics_mixed = get_check_metrics(
            &db,
            check_mixed,
            None,
            from,
            "2025-11-29T20:00:00Z".parse::<DateTime<Utc>>()?,
        )
        .await?;
        // Time-weighted: 7/9 intervals successful = 77.78%
        assert!((metrics_mixed.overall.uptime_percent - 77.78).abs() < 0.01);
        assert!(metrics_mixed.overall.avg_response_time_ms > 0.0);

        // Test: Empty result for non-existent check
        let nonexistent = uuid!("99999999-9999-9999-9999-999999999999");
        let empty = get_check_metrics(&db, nonexistent, None, from, to).await?;
        assert_eq!(empty.overall.uptime_percent, 0.0);
        assert!(empty.by_region.is_empty());

        Ok(())
    }
}
