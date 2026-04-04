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

pub async fn run_load_test(config: LoadTestConfig) -> Vec<RequestResult> {
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
                        success: status >= 200 && status < 400,
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

    results
}
