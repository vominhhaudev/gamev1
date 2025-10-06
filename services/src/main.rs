use common_net::telemetry;
use axum::{Router};
use hyper::{server::conn::AddrIncoming, Server};
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod api;
mod collections;
mod jobs;
mod persistence;

use api::create_api_router;
use jobs::JobSystem;
use persistence::create_persistence_state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    telemetry::init("services");
    tracing::info!("Starting services server...");

    // Get configuration
    let pocketbase_url = std::env::var("POCKETBASE_URL")
        .unwrap_or_else(|_| "http://localhost:8090".to_string());

    let api_port = std::env::var("SERVICES_API_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()?;

    let bind_addr = std::env::var("SERVICES_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let addr: SocketAddr = format!("{}:{}", bind_addr, api_port).parse()?;

    // Initialize persistence state
    let persistence_state = create_persistence_state(pocketbase_url.clone());

    // Initialize job system
    let job_system = JobSystem::new(persistence_state.clone());

    // Start background job scheduler
    tokio::spawn(async move {
        if let Err(e) = job_system.start_scheduler().await {
            tracing::error!("Failed to start job scheduler: {:?}", e);
        }
    });

    // Create API router
    let app = create_api_router(pocketbase_url);

    // Start server
    tracing::info!("Services API server listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    let incoming = AddrIncoming::from_listener(listener)?;

    Server::builder(incoming)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
