use super::queries::CheckResultRow;
use super::{MetricsSummary, RegionMetrics};
use crate::regions::Region;
use chrono::Duration;
use statrs::statistics::{Data, OrderStatistics, Statistics};
use std::borrow::Borrow;
use std::collections::HashMap;

/// Calculate time-weighted uptime percentage from check results.
///
/// Each check's status applies to the time interval from that check until the next check.
/// The bounds are defined by the first and last check timestamps.
///
/// **Expects data sorted by `check_started_at` in ascending order.**
fn calculate_uptime_percent<T>(sorted: &[T]) -> f64
where
    T: Borrow<CheckResultRow>,
{
    match sorted {
        [] => 0.0,
        [single] => {
            if single.borrow().matches_expected {
                100.0
            } else {
                0.0
            }
        }
        [first, .., last] => {
            let total_duration = last.borrow().check_started_at - first.borrow().check_started_at;

            if total_duration == Duration::zero() {
                // All checks at the same time, fall back to simple percentage
                let successful = sorted
                    .iter()
                    .filter(|r| Borrow::<CheckResultRow>::borrow(*r).matches_expected)
                    .count();
                return (successful as f64 / sorted.len() as f64) * 100.0;
            }

            // Calculate uptime by weighting each check by its time interval
            let uptime_duration: Duration = sorted
                .windows(2)
                .filter(|w| Borrow::<CheckResultRow>::borrow(&w[0]).matches_expected)
                .map(|w| {
                    Borrow::<CheckResultRow>::borrow(&w[1]).check_started_at
                        - Borrow::<CheckResultRow>::borrow(&w[0]).check_started_at
                })
                .sum();

            (uptime_duration.num_milliseconds() as f64 / total_duration.num_milliseconds() as f64)
                * 100.0
        }
    }
}

/// Calculate metrics from a slice of results.
///
/// **Expects data sorted by `check_started_at` in ascending order.**
fn calculate_metrics<T>(sorted: &[T]) -> MetricsSummary
where
    T: Borrow<CheckResultRow>,
{
    debug_assert!(
        sorted
            .windows(2)
            .all(|w| w[0].borrow().check_started_at <= w[1].borrow().check_started_at),
        "results must be sorted by check_started_at"
    );

    if sorted.is_empty() {
        return MetricsSummary {
            uptime_percent: 0.0,
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
        };
    }

    let uptime_percent = calculate_uptime_percent(sorted);

    let response_times: Vec<f64> = sorted
        .iter()
        .map(|r| r.borrow().response_time_micros as f64)
        .collect();

    let avg_response_time_ms = Statistics::mean(&response_times) / 1000.0;

    let mut data = Data::new(response_times);
    let p95_response_time_ms = data.percentile(95) / 1000.0;
    let p99_response_time_ms = data.percentile(99) / 1000.0;

    MetricsSummary {
        uptime_percent,
        avg_response_time_ms,
        p95_response_time_ms,
        p99_response_time_ms,
    }
}

/// Calculate overall metrics across all results.
///
/// **Expects data sorted by `check_started_at` in ascending order.**
pub fn calculate_overall_metrics(sorted: &[CheckResultRow]) -> MetricsSummary {
    debug_assert!(
        sorted
            .windows(2)
            .all(|w| w[0].check_started_at <= w[1].check_started_at),
        "results must be sorted by check_started_at"
    );

    calculate_metrics(sorted)
}

/// Calculate metrics grouped by region.
///
/// **Expects data sorted by `check_started_at` in ascending order.**
pub fn calculate_by_region_metrics(sorted: &[CheckResultRow]) -> Vec<RegionMetrics> {
    debug_assert!(
        sorted
            .windows(2)
            .all(|w| w[0].check_started_at <= w[1].check_started_at),
        "results must be sorted by check_started_at"
    );

    // Group by region (maintains sort order within each group)
    let by_region = sorted.iter().fold(
        HashMap::<Region, Vec<&CheckResultRow>>::new(),
        |mut acc, result| {
            acc.entry(result.region).or_default().push(result);
            acc
        },
    );

    // Calculate metrics for each region and sort
    let mut region_metrics: Vec<_> = by_region
        .into_iter()
        .filter(|(_, results)| !results.is_empty())
        .map(|(region, region_results)| RegionMetrics {
            region,
            metrics: calculate_metrics(&region_results),
        })
        .collect();

    region_metrics.sort_by_key(|r| r.region);
    region_metrics
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn create_test_results(
        response_times: Vec<(i64, bool)>,
        region: Region,
        start_time: DateTime<Utc>,
    ) -> Vec<CheckResultRow> {
        response_times
            .into_iter()
            .enumerate()
            .map(|(i, (rt, success))| CheckResultRow {
                check_started_at: start_time + chrono::Duration::hours(i as i64),
                response_time_micros: rt,
                matches_expected: success,
                region,
            })
            .collect()
    }

    #[test]
    fn test_calculate_overall_all_successful() {
        let start = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let results = create_test_results(
            vec![(100000, true), (150000, true), (200000, true)],
            Region::Fsn1,
            start,
        );
        let metrics = calculate_overall_metrics(&results);

        assert_eq!(metrics.uptime_percent, 100.0);
        assert_eq!(metrics.avg_response_time_ms, 150.0); // (100+150+200)/3 = 150
        assert!(metrics.p95_response_time_ms > 0.0);
        assert!(metrics.p99_response_time_ms >= metrics.p95_response_time_ms);
    }

    #[test]
    fn test_calculate_overall_mixed() {
        let start = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let results = create_test_results(
            vec![
                (100000, true),
                (150000, true),
                (500000, false),
                (600000, false),
            ],
            Region::Fsn1,
            start,
        );
        let metrics = calculate_overall_metrics(&results);

        // Time-weighted: 4 checks 1h apart, first 2 succeed
        // c0->c1: 1h up, c1->c2: 1h up, c2->c3: 1h down = 2h/3h = 66.67%
        assert!((metrics.uptime_percent - 66.67).abs() < 0.01);
        assert!(metrics.avg_response_time_ms > 0.0);
    }

    #[test]
    fn test_calculate_overall_empty() {
        let metrics = calculate_overall_metrics(&[]);

        assert_eq!(metrics.uptime_percent, 0.0);
        assert_eq!(metrics.avg_response_time_ms, 0.0);
    }

    #[test]
    fn test_calculate_by_region() {
        let start = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let mut results =
            create_test_results(vec![(100000, true), (150000, true)], Region::Fsn1, start);
        results.extend(create_test_results(
            vec![(110000, true), (130000, true)],
            Region::Hel1,
            start,
        ));
        results.sort_by_key(|r| r.check_started_at);

        let by_region = calculate_by_region_metrics(&results);

        assert_eq!(by_region.len(), 2);
        assert_eq!(by_region[0].region, Region::Fsn1);
        assert_eq!(by_region[1].region, Region::Hel1);

        // Check Fsn1 metrics
        assert_eq!(by_region[0].metrics.uptime_percent, 100.0);
        assert_eq!(by_region[0].metrics.avg_response_time_ms, 125.0); // (100+150)/2

        // Check Hel1 metrics
        assert_eq!(by_region[1].metrics.uptime_percent, 100.0);
        assert_eq!(by_region[1].metrics.avg_response_time_ms, 120.0); // (110+130)/2
    }

    #[test]
    fn test_calculate_by_region_mixed_success() {
        let start = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let results = create_test_results(
            vec![
                (100000, true),
                (150000, true),
                (200000, false),
                (300000, false),
            ],
            Region::Fsn1,
            start,
        );

        let by_region = calculate_by_region_metrics(&results);

        assert_eq!(by_region.len(), 1);
        assert_eq!(by_region[0].region, Region::Fsn1);
        // Time-weighted: 4 checks 1h apart, first 2 succeed = 2h/3h = 66.67%
        assert!((by_region[0].metrics.uptime_percent - 66.67).abs() < 0.01);
        assert!(by_region[0].metrics.avg_response_time_ms > 0.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let start = "2025-11-29T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let results = create_test_results(
            vec![
                (100000, true),
                (200000, true),
                (300000, true),
                (400000, true),
                (500000, true),
            ],
            Region::Fsn1,
            start,
        );

        let metrics = calculate_overall_metrics(&results);

        // With sorted [100, 200, 300, 400, 500] microseconds
        assert_eq!(metrics.avg_response_time_ms, 300.0);
        assert!(metrics.p95_response_time_ms >= metrics.avg_response_time_ms);
        assert!(metrics.p99_response_time_ms >= metrics.p95_response_time_ms);
    }
}
