mod runner;
mod stats;
mod report;

use clap::Parser;
use colored::Colorize;
use runner::LoadTestConfig;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "rlt", about = "High-performance HTTP load tester", version)]
struct Cli {
    /// Target URL to test
    url: String,

    /// Number of concurrent connections
    #[arg(short, long, default_value = "10")]
    concurrency: usize,

    /// Total number of requests
    #[arg(short = 'n', long, default_value = "100")]
    requests: usize,

    /// HTTP method (GET, POST, PUT, DELETE)
    #[arg(short, long, default_value = "GET")]
    method: String,

    /// Request body (for POST/PUT)
    #[arg(short, long)]
    body: Option<String>,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!();
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║     Rust Load Tester v1.0.0          ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();
    println!("  {} {}", "Target:".bold(), cli.url);
    println!("  {} {}", "Method:".bold(), cli.method.to_uppercase());
    println!(
        "  {} {} requests, {} concurrent",
        "Load:".bold(),
        cli.requests,
        cli.concurrency
    );
    println!();

    let config = LoadTestConfig {
        url: cli.url,
        method: cli.method.to_uppercase(),
        body: cli.body,
        concurrency: cli.concurrency,
        total_requests: cli.requests,
        timeout: Duration::from_secs(cli.timeout),
    };

    let results = runner::run_load_test(config).await;
    let summary = stats::compute_stats(&results);

    if cli.json {
        report::print_json(&summary);
    } else {
        report::print_console(&summary);
    }

    if summary.error_rate > 0.1 {
        std::process::exit(1);
    }
}
