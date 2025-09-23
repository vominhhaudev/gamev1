use std::time::Duration;

use common_net::{metrics, telemetry};
use reqwest::StatusCode;

#[tokio::test]
async fn metrics_endpoint_contains_room_manager_counters(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    telemetry::init("room-manager-test");
    let _ = room_manager::matchmaking_metrics();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        if let Err(err) = metrics::serve_metrics(listener, room_manager::METRICS_PATH).await {
            panic!("metrics server failed: {err}");
        }
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;

    let resp = client
        .get(format!("http://{}{}", addr, room_manager::METRICS_PATH))
        .send()
        .await?;
    assert_eq!(StatusCode::OK, resp.status());

    let body = resp.text().await?;
    assert!(body.contains("room_manager_rooms_created_total"));
    assert!(body.contains("room_manager_active_rooms"));
    assert!(body.contains("room_manager_matchmaking_queue_depth"));

    server.abort();
    Ok(())
}
