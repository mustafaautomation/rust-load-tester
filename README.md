# Rust Load Tester

[![CI](https://github.com/mustafaautomation/rust-load-tester/actions/workflows/ci.yml/badge.svg)](https://github.com/mustafaautomation/rust-load-tester/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)

High-performance HTTP load testing CLI built with Rust and Tokio. Sends thousands of concurrent requests with minimal overhead, reports latency percentiles, and outputs colored terminal or JSON results.

---

## Why Rust?

| Feature | Benefit |
|---------|---------|
| Zero-cost abstractions | Near-native speed, no GC pauses |
| Tokio async runtime | Efficient concurrent I/O without thread-per-request |
| Memory safety | No segfaults, no data races in the load generator itself |
| Single binary | Ship one file, no runtime dependencies |

---

## Quick Start

```bash
# Build
cargo build --release

# Basic load test — 100 requests, 10 concurrent
./target/release/rust-load-tester https://reqres.in/api/users

# Custom load
./target/release/rust-load-tester https://reqres.in/api/users -n 1000 -c 50

# POST with body
./target/release/rust-load-tester https://reqres.in/api/users \
  -m POST -b '{"name":"test","job":"qa"}' -n 200 -c 20

# JSON output (for CI integration)
./target/release/rust-load-tester https://reqres.in/api/users --json
```

---

## CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `<url>` | Target URL (required) | — |
| `-n, --requests` | Total number of requests | 100 |
| `-c, --concurrency` | Concurrent connections | 10 |
| `-m, --method` | HTTP method (GET/POST/PUT/DELETE) | GET |
| `-b, --body` | Request body (JSON) | — |
| `-t, --timeout` | Request timeout (seconds) | 30 |
| `--json` | Output as JSON | false |

---

## Output

### Console (default)

```
╔══════════════════════════════════════╗
║     Rust Load Tester v1.0.0          ║
╚══════════════════════════════════════╝

  Target: https://reqres.in/api/users
  Method: GET
  Load:   100 requests, 10 concurrent

─── Results ───────────────────────────

  Status: 100 OK / 100 requests  (0 failed)

  Latency:
    Min:      45.23ms
    Mean:    120.56ms
    Median:  115.34ms
    p90:     180.12ms
    p95:     210.45ms
    p99:     350.67ms
    Max:     420.89ms
    Stdev:    52.31ms

  Status Codes:
    200 → 100

  Throughput:
    832.45 req/s
    1045678.00 bytes/s
```

### JSON (`--json`)

Machine-readable output for CI pipelines and trend tracking.

---

## Project Structure

```
rust-load-tester/
├── src/
│   ├── main.rs       # CLI args (clap), entrypoint
│   ├── runner.rs      # Tokio async load runner with semaphore concurrency
│   ├── stats.rs       # Latency percentiles, throughput, status code aggregation
│   └── report.rs      # Console (colored) and JSON output formatters
├── Cargo.toml
└── .github/workflows/ci.yml
```

---

## Architecture

```
CLI (clap) → Runner (tokio + reqwest + semaphore)
                ↓
           RequestResult[]
                ↓
         Stats (percentiles, throughput)
                ↓
         Report (console / JSON)
```

- **Semaphore-based concurrency** — caps active connections without spawning unlimited tasks
- **Connection pooling** — reqwest reuses connections via `pool_max_idle_per_host`
- **Non-blocking I/O** — Tokio's event loop handles thousands of concurrent requests efficiently

---

## Stack

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime |
| reqwest | HTTP client |
| clap | CLI argument parsing |
| colored | Terminal colors |
| serde/serde_json | JSON serialization |

---

## CI Pipeline

- `cargo fmt -- --check` — formatting
- `cargo clippy -- -D warnings` — lints
- `cargo build --release` — optimized build
- `cargo test` — unit tests

---

## License

MIT

---

Built by [Quvantic](https://quvantic.com)
