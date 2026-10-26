use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

pub struct LoadTestConfig {
    pub url: String,
    pub method: String,
    pub body: Option<String>,
    pub concurrency: usize,
    pub total_requests: usize,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct RequestResult {
    pub status: u16,
    pub duration: Duration,
    pub success: bool,
    pub error: Option<String>,
    pub bytes: usize,
}

/// Outcome of a load test: one result per request plus the wall-clock time of the whole run.
pub struct LoadTestRun {
    pub results: Vec<RequestResult>,
    pub elapsed: Duration,
}

pub async fn run_load_test(config: LoadTestConfig) -> LoadTestRun {
    let client = Client::builder()
        .timeout(config.timeout)
        .pool_max_idle_per_host(config.concurrency)
        .build()
        .expect("Failed to create HTTP client");

    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let client = Arc::new(client);
    let url = Arc::new(config.url);
    let method = Arc::new(config.method);
    let body = Arc::new(config.body);

    let mut handles = Vec::with_capacity(config.total_requests);

    let start = Instant::now();

    for _ in 0..config.total_requests {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = url.clone();
        let method = method.clone();
        let body = body.clone();

        let handle = tokio::spawn(async move {
            let req_start = Instant::now();

            let mut builder = match method.as_str() {
                "POST" => client.post(url.as_str()),
                "PUT" => client.put(url.as_str()),
                "DELETE" => client.delete(url.as_str()),
                "PATCH" => client.patch(url.as_str()),
                _ => client.get(url.as_str()),
            };

            if let Some(ref b) = *body {
                builder = builder
                    .header("Content-Type", "application/json")
                    .body(b.clone());
            }

            let result = match builder.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let bytes = response.bytes().await.map(|b| b.len()).unwrap_or(0);
                    RequestResult {
                        status,
                        duration: req_start.elapsed(),
                        success: (200..400).contains(&status),
                        error: None,
                        bytes,
                    }
                }
                Err(e) => RequestResult {
                    status: 0,
                    duration: req_start.elapsed(),
                    success: false,
                    error: Some(e.to_string()),
                    bytes: 0,
                },
            };

            drop(permit);
            result
        });

        handles.push(handle);
    }

    let mut results = Vec::with_capacity(config.total_requests);
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }

    let elapsed = start.elapsed();
    eprintln!(
        "  Completed {} requests in {:.2}s",
        results.len(),
        elapsed.as_secs_f64()
    );

    LoadTestRun { results, elapsed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Minimal local HTTP server that answers every request with `status` and a fixed body,
    /// so the runner can be exercised without depending on a public site.
    async fn spawn_server(status: u16, hits: Arc<AtomicUsize>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(_) => return,
                };
                let hits = hits.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let _ = socket.read(&mut buf).await;
                    hits.fetch_add(1, Ordering::SeqCst);
                    let body = "hello";
                    let response = format!(
                        "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status,
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        format!("http://{}/", addr)
    }

    fn config(url: String, total_requests: usize, concurrency: usize) -> LoadTestConfig {
        LoadTestConfig {
            url,
            method: "GET".to_string(),
            body: None,
            concurrency,
            total_requests,
            timeout: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn sends_exactly_the_requested_number_of_requests() {
        let hits = Arc::new(AtomicUsize::new(0));
        let url = spawn_server(200, hits.clone()).await;

        let run = run_load_test(config(url, 25, 5)).await;

        assert_eq!(run.results.len(), 25);
        assert_eq!(hits.load(Ordering::SeqCst), 25);
        assert!(run.results.iter().all(|r| r.success && r.status == 200));
        assert!(run.results.iter().all(|r| r.bytes == 5));
        assert!(run.elapsed > Duration::ZERO);
    }

    #[tokio::test]
    async fn marks_server_errors_as_failures() {
        let hits = Arc::new(AtomicUsize::new(0));
        let url = spawn_server(503, hits).await;

        let run = run_load_test(config(url, 4, 2)).await;

        assert_eq!(run.results.len(), 4);
        assert!(run.results.iter().all(|r| !r.success && r.status == 503));
        assert!(run.results.iter().all(|r| r.error.is_none()));
    }

    #[tokio::test]
    async fn records_connection_errors() {
        // Bind then drop a listener so the port is very likely closed.
        let addr = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap()
        };

        let run = run_load_test(config(format!("http://{}/", addr), 3, 3)).await;

        assert_eq!(run.results.len(), 3);
        assert!(run.results.iter().all(|r| !r.success && r.status == 0));
        assert!(run.results.iter().all(|r| r.error.is_some()));
    }
}
