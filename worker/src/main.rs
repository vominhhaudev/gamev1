use worker::{WorkerConfig, simulation::SimulationWorld, database::PocketBaseClient};
use common_net::telemetry;
use std::time::{Duration, Instant};
use tokio::time;

const DEFAULT_EMAIL: &str = "admin@pocketbase.local";
const DEFAULT_PASSWORD: &str = "123456789";

#[tokio::main]
async fn main() {
    telemetry::init("worker");

    let config = match WorkerConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(%err, "worker: cau hinh khong hop le");
            return;
        }
    };

    // Initialize PocketBase connection
    let mut db_client = PocketBaseClient::new();

    // Test PocketBase connection
    match db_client.test_connection().await {
        Ok(true) => {
            tracing::info!("✅ PocketBase connection successful");
        }
        Ok(false) => {
            tracing::warn!("❌ PocketBase connection failed - will continue without database");
        }
        Err(err) => {
            tracing::error!(%err, "PocketBase connection error");
        }
    }

    // Try to authenticate (optional for now)
    if let Err(err) = db_client.authenticate(&DEFAULT_EMAIL, &DEFAULT_PASSWORD).await {
        tracing::warn!(%err, "PocketBase authentication failed - continuing without auth");
    }

    // Create simulation world
    let mut simulation = SimulationWorld::new();

    // Fixed timestep: 60 FPS (16.67ms per frame)
    let target_frame_time = Duration::from_millis(16);
    let mut accumulator = Duration::from_secs(0);
    let mut last_time = Instant::now();

    // Start gRPC server in background
    let grpc_handle = tokio::spawn(async move {
        if let Err(err) = worker::run_with_ctrl_c(config).await {
            tracing::error!(%err, "worker gRPC server ket thuc do loi");
        }
    });

    tracing::info!("Worker simulation started with 60Hz fixed timestep");

    loop {
        let current_time = Instant::now();
        let delta_time = current_time - last_time;
        last_time = current_time;

        accumulator += delta_time;

        // Fixed timestep loop
        while accumulator >= target_frame_time {
            // Step simulation
            simulation.step(target_frame_time);

            // Send snapshot to clients (placeholder)
            if simulation.entities.len() > 0 {
                let snapshot = simulation.create_snapshot();
                tracing::debug!("Created snapshot with {} entities", snapshot.entities.len());
                // TODO: Send snapshot to clients via transport abstraction
            }

            accumulator -= target_frame_time;
        }

        // Small sleep to prevent busy waiting
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
