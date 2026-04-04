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

pub fn compute_stats(results: &[RequestResult]) -> TestSummary {
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

    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let wall_clock = if !results.is_empty() {
        let min_start = durations[0];
        let _ = min_start;
        total_duration.as_secs_f64() / results.len().max(1) as f64
            * results.len() as f64
            / results.len().max(1) as f64
    } else {
        0.0
    };

    let total_bytes: usize = results.iter().map(|r| r.bytes).sum();
    let actual_wall = total_duration.as_secs_f64();

    let throughput = ThroughputStats {
        requests_per_sec: if actual_wall > 0.0 {
            round2(total as f64 / actual_wall * results.len() as f64)
        } else {
            0.0
        },
        bytes_per_sec: if actual_wall > 0.0 {
            round2(total_bytes as f64 / actual_wall * results.len() as f64)
        } else {
            0.0
        },
        total_duration_secs: round2(actual_wall),
    };

    let mut status_codes: HashMap<u16, usize> = HashMap::new();
    for r in results {
        *status_codes.entry(r.status).or_insert(0) += 1;
    }

    TestSummary {
        total_requests: total,
        successful,
        failed,
        error_rate: round2(error_rate),
        latency,
        throughput,
        status_codes,
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
