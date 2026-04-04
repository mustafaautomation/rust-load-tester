use crate::stats::TestSummary;
use colored::Colorize;

pub fn print_console(summary: &TestSummary) {
    println!();
    println!("{}", "─── Results ───────────────────────────".dimmed());
    println!();

    // Status
    let status_color = if summary.error_rate < 0.01 {
        "green"
    } else if summary.error_rate < 0.1 {
        "yellow"
    } else {
        "red"
    };

    println!(
        "  {} {} / {} requests  ({} failed)",
        "Status:".bold(),
        match status_color {
            "green" => format!("{} OK", summary.successful).green(),
            "yellow" => format!("{} OK", summary.successful).yellow(),
            _ => format!("{} OK", summary.successful).red(),
        },
        summary.total_requests,
        summary.failed,
    );
    println!();

    // Latency
    println!("  {}", "Latency:".bold());
    println!(
        "    Min:    {:>8.2}ms",
        summary.latency.min_ms
    );
    println!(
        "    Mean:   {:>8.2}ms",
        summary.latency.mean_ms
    );
    println!(
        "    Median: {:>8.2}ms",
        summary.latency.median_ms
    );
    println!(
        "    p90:    {:>8.2}ms",
        summary.latency.p90_ms
    );
    println!(
        "    p95:    {:>8.2}ms",
        summary.latency.p95_ms
    );
    println!(
        "    p99:    {:>8.2}ms",
        summary.latency.p99_ms
    );
    println!(
        "    Max:    {:>8.2}ms",
        summary.latency.max_ms
    );
    println!(
        "    Stdev:  {:>8.2}ms",
        summary.latency.stdev_ms
    );
    println!();

    // Status codes
    println!("  {}", "Status Codes:".bold());
    let mut codes: Vec<_> = summary.status_codes.iter().collect();
    codes.sort_by_key(|(code, _)| *code);
    for (code, count) in codes {
        let label = if *code >= 200 && *code < 300 {
            format!("{}", code).green()
        } else if *code >= 400 {
            format!("{}", code).red()
        } else {
            format!("{}", code).yellow()
        };
        println!("    {} → {}", label, count);
    }
    println!();

    // Throughput
    println!("  {}", "Throughput:".bold());
    println!(
        "    {:.2} req/s",
        summary.throughput.requests_per_sec
    );
    println!(
        "    {:.2} bytes/s",
        summary.throughput.bytes_per_sec
    );
    println!();
}

pub fn print_json(summary: &TestSummary) {
    println!(
        "{}",
        serde_json::to_string_pretty(summary).unwrap_or_else(|_| "{}".to_string())
    );
}
