use std::time::Duration;

use common_net::{metrics, telemetry};
use reqwest::StatusCode;

#[tokio::test]
async fn metrics_endpoint_returns_expected_keys(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    telemetry::init("worker-test");
    let _ = worker::simulation_metrics();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        if let Err(err) = metrics::serve_metrics(listener, worker::METRICS_PATH).await {
            panic!("metrics server failed: {err}");
        }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;

    let resp = client
        .get(format!("http://{}{}", addr, worker::METRICS_PATH))
        .send()
        .await?;
    assert_eq!(StatusCode::OK, resp.status());

    let body = resp.text().await?;
    assert!(body.contains("worker_ticks_total"));
    assert!(body.contains("worker_active_players"));

    server.abort();
    Ok(())
}
