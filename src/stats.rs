use crate::runner::RequestResult;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct TestSummary {
    pub total_requests: usize,
    pub successful: usize,
    pub failed: usize,
    pub error_rate: f64,

    pub latency: LatencyStats,
    pub throughput: ThroughputStats,
    pub status_codes: HashMap<u16, usize>,
    /// Transport errors (timeouts, refused connections, ...) grouped by message.
    pub errors: HashMap<String, usize>,
    pub total_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct LatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub stdev_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct ThroughputStats {
    pub requests_per_sec: f64,
    pub bytes_per_sec: f64,
    pub total_duration_secs: f64,
}

/// Aggregate per-request results. `wall_clock` is the elapsed time of the whole run and is
/// what throughput is measured against (summing per-request durations would ignore concurrency).
pub fn compute_stats(results: &[RequestResult], wall_clock: Duration) -> TestSummary {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = total - successful;
    let error_rate = if total > 0 {
        failed as f64 / total as f64
    } else {
        0.0
    };

    let mut durations: Vec<f64> = results
        .iter()
        .map(|r| r.duration.as_secs_f64() * 1000.0)
        .collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let latency = if durations.is_empty() {
        LatencyStats {
            min_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
            median_ms: 0.0,
            p90_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            stdev_ms: 0.0,
        }
    } else {
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let variance =
            durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / durations.len() as f64;

        LatencyStats {
            min_ms: round2(durations[0]),
            max_ms: round2(*durations.last().unwrap()),
            mean_ms: round2(mean),
            median_ms: round2(percentile(&durations, 50.0)),
            p90_ms: round2(percentile(&durations, 90.0)),
            p95_ms: round2(percentile(&durations, 95.0)),
            p99_ms: round2(percentile(&durations, 99.0)),
            stdev_ms: round2(variance.sqrt()),
        }
    };

    let total_bytes: usize = results.iter().map(|r| r.bytes).sum();
    let wall_secs = wall_clock.as_secs_f64();

    let throughput = ThroughputStats {
        requests_per_sec: if wall_secs > 0.0 {
            round2(total as f64 / wall_secs)
        } else {
            0.0
        },
        bytes_per_sec: if wall_secs > 0.0 {
            round2(total_bytes as f64 / wall_secs)
        } else {
            0.0
        },
        total_duration_secs: round2(wall_secs),
    };

    let mut status_codes: HashMap<u16, usize> = HashMap::new();
    let mut errors: HashMap<String, usize> = HashMap::new();
    for r in results {
        *status_codes.entry(r.status).or_insert(0) += 1;
        if let Some(e) = &r.error {
            *errors.entry(e.clone()).or_insert(0) += 1;
        }
    }

    TestSummary {
        total_requests: total,
        successful,
        failed,
        error_rate: round2(error_rate),
        latency,
        throughput,
        status_codes,
        errors,
        total_bytes,
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(status: u16, duration_ms: u64, success: bool, bytes: usize) -> RequestResult {
        RequestResult {
            status,
            duration: Duration::from_millis(duration_ms),
            success,
            error: if success {
                None
            } else {
                Some("error".to_string())
            },
            bytes,
        }
    }

    #[test]
    fn test_compute_stats_all_successful() {
        let results = vec![
            make_result(200, 100, true, 500),
            make_result(200, 150, true, 600),
            make_result(200, 200, true, 700),
        ];

        let stats = compute_stats(&results, Duration::from_secs(1));
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.successful, 3);
        assert_eq!(stats.failed, 0);
        assert!(stats.error_rate < 0.01);
        assert!(stats.latency.min_ms <= stats.latency.max_ms);
        assert!(stats.latency.mean_ms > 0.0);
        assert_eq!(stats.total_bytes, 1800);
    }

    #[test]
    fn test_compute_stats_mixed_results() {
        let results = vec![
            make_result(200, 100, true, 500),
            make_result(500, 50, false, 0),
            make_result(200, 200, true, 600),
            make_result(503, 30, false, 0),
        ];

        let stats = compute_stats(&results, Duration::from_secs(1));
        assert_eq!(stats.total_requests, 4);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.failed, 2);
        assert!((stats.error_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_compute_stats_empty_results() {
        let stats = compute_stats(&[], Duration::ZERO);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 0);
        assert!(stats.error_rate < 0.01);
        assert!(stats.latency.min_ms < 0.01);
        assert!(stats.latency.max_ms < 0.01);
    }

    #[test]
    fn test_compute_stats_status_codes() {
        let results = vec![
            make_result(200, 100, true, 100),
            make_result(200, 100, true, 100),
            make_result(404, 50, false, 0),
            make_result(500, 50, false, 0),
        ];

        let stats = compute_stats(&results, Duration::from_secs(1));
        assert_eq!(*stats.status_codes.get(&200).unwrap_or(&0), 2);
        assert_eq!(*stats.status_codes.get(&404).unwrap_or(&0), 1);
        assert_eq!(*stats.status_codes.get(&500).unwrap_or(&0), 1);
    }

    #[test]
    fn test_compute_stats_latency_ordering() {
        let results = vec![
            make_result(200, 10, true, 100),
            make_result(200, 50, true, 100),
            make_result(200, 100, true, 100),
            make_result(200, 200, true, 100),
            make_result(200, 500, true, 100),
        ];

        let stats = compute_stats(&results, Duration::from_secs(1));
        assert!(stats.latency.min_ms <= stats.latency.median_ms);
        assert!(stats.latency.median_ms <= stats.latency.p90_ms);
        assert!(stats.latency.p90_ms <= stats.latency.p95_ms);
        assert!(stats.latency.p95_ms <= stats.latency.p99_ms);
        assert!(stats.latency.p99_ms <= stats.latency.max_ms);
    }

    #[test]
    fn test_compute_stats_single_request() {
        let results = vec![make_result(200, 42, true, 256)];

        let stats = compute_stats(&results, Duration::from_secs(1));
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful, 1);
        assert!((stats.latency.min_ms - 42.0).abs() < 0.1);
        assert!((stats.latency.max_ms - 42.0).abs() < 0.1);
        assert!((stats.latency.mean_ms - 42.0).abs() < 0.1);
        assert!(stats.latency.stdev_ms < 0.01);
    }

    #[test]
    fn test_throughput_uses_wall_clock_time() {
        // 10 requests of 1s each, run 10-wide in parallel, finish in ~1s of wall time:
        // throughput is 10 req/s, not 1 req/s as summing request durations would suggest.
        let results: Vec<_> = (0..10).map(|_| make_result(200, 1000, true, 100)).collect();

        let stats = compute_stats(&results, Duration::from_secs(1));
        assert!((stats.throughput.requests_per_sec - 10.0).abs() < 0.01);
        assert!((stats.throughput.bytes_per_sec - 1000.0).abs() < 0.01);
        assert!((stats.throughput.total_duration_secs - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_zero_wall_clock_gives_zero_throughput() {
        let results = vec![make_result(200, 10, true, 10)];
        let stats = compute_stats(&results, Duration::ZERO);
        assert!(stats.throughput.requests_per_sec < 0.01);
    }

    #[test]
    fn test_errors_are_grouped_by_message() {
        let results = vec![
            make_result(0, 10, false, 0),
            make_result(0, 10, false, 0),
            make_result(200, 10, true, 10),
        ];
        let stats = compute_stats(&results, Duration::from_secs(1));
        assert_eq!(stats.errors.get("error"), Some(&2));
        assert_eq!(stats.errors.len(), 1);
    }

    #[test]
    fn test_percentile_function() {
        let sorted = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert!((percentile(&sorted, 50.0) - 30.0).abs() < 0.01);
        assert!((percentile(&sorted, 0.0) - 10.0).abs() < 0.01);
        assert!((percentile(&sorted, 100.0) - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_percentile_empty() {
        assert!(percentile(&[], 50.0) < 0.01);
    }

    #[test]
    fn test_round2() {
        assert!((round2(1.23456) - 1.23).abs() < 0.001);
        assert!((round2(0.0) - 0.0).abs() < 0.001);
        assert!((round2(99.999) - 100.0).abs() < 0.001);
    }
}
