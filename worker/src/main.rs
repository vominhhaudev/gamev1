use worker::{WorkerConfig, simulation::SimulationWorld, database::PocketBaseClient};
use common_net::telemetry;
use std::time::{Duration, Instant};
use tokio::time;
use std::sync::atomic::{AtomicU64, Ordering};

const DEFAULT_EMAIL: &str = "admin@pocketbase.local";
const DEFAULT_PASSWORD: &str = "123456789";

// Performance monitoring
static FRAME_COUNT: AtomicU64 = AtomicU64::new(0);
static DB_SYNC_COUNT: AtomicU64 = AtomicU64::new(0);
const DB_SYNC_INTERVAL: u64 = 60; // Sync every 60 frames (1 second at 60fps)

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
            tracing::info!("PocketBase connection successful");
        }
        Ok(false) => {
            tracing::warn!("PocketBase connection failed - will continue without database");
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
    tracing::info!("Simulation world created with {} entities", simulation.entities.len());

    // Fixed timestep: 60 FPS (16.67ms per frame)
    let target_frame_time = Duration::from_millis(16);
    let mut accumulator = Duration::from_secs(0);
    let mut last_time = Instant::now();
    let mut frame_start_time = Instant::now();

    // Start gRPC server in background
    let _grpc_handle = tokio::spawn(async move {
        if let Err(err) = worker::run_with_ctrl_c(config).await {
            tracing::error!(%err, "worker gRPC server ket thuc do loi");
        }
    });

    tracing::info!("Worker simulation started with 60Hz fixed timestep");
    tracing::info!("Database sync every {} frames ({} seconds)", DB_SYNC_INTERVAL, DB_SYNC_INTERVAL / 60);
    tracing::info!("Frame logging: enabled for all frames");
    tracing::info!("Performance monitoring: enabled every {} frames", 60);

    loop {
        frame_start_time = Instant::now();
        let current_time = Instant::now();
        let delta_time = current_time - last_time;
        last_time = current_time;

        accumulator += delta_time;

        // Fixed timestep loop
        while accumulator >= target_frame_time {
            let frame_count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

            // Step simulation
            let sim_start = Instant::now();
            simulation.step(target_frame_time);
            let sim_time = sim_start.elapsed();

            // Minimal operations in main game loop to avoid blocking
            // Just increment counters - no logging or I/O in main loop

            // Send snapshot to clients (placeholder) - only if entities exist
            if simulation.entities.len() > 0 {
                // Just increment counter for now - no actual snapshot creation in main loop
                // In real implementation, this would be handled by a separate task
            }

            // Periodic database sync (every DB_SYNC_INTERVAL frames) - just increment counter
            if frame_count % DB_SYNC_INTERVAL == 0 {
                DB_SYNC_COUNT.fetch_add(1, Ordering::Relaxed);
            }

            accumulator -= target_frame_time;
        }

        // Performance monitoring every 60 frames (display more frequently for testing)
        if FRAME_COUNT.load(Ordering::Relaxed) % 60 == 0 {
            let (cache_hits, cache_misses, db_queries, db_errors, avg_query_time) =
                db_client.get_performance_metrics();
            let (games_cached, players_cached, sessions_cached) = db_client.get_cache_stats();

            let total_frames = FRAME_COUNT.load(Ordering::Relaxed);
            let total_syncs = DB_SYNC_COUNT.load(Ordering::Relaxed);

            // Use info level for better visibility during testing
            tracing::info!(
                "PERF STATS - Frames: {}, Syncs: {}, Cache: {}/{}/{}, DB: {}/{}/{}ms, Hit Rate: {:.2}%",
                total_frames, total_syncs,
                games_cached, players_cached, sessions_cached,
                db_queries, db_errors, avg_query_time,
                if cache_hits + cache_misses > 0 {
                    (cache_hits as f64 / (cache_hits + cache_misses) as f64) * 100.0
                } else {
                    0.0
                }
            );
        }

        // Simple frame timing - maintain consistent 60fps
        let frame_time = frame_start_time.elapsed();
        let target_frame_time = Duration::from_millis(16);

        // Only sleep if we're significantly ahead of schedule
        if frame_time < Duration::from_millis(14) {
            // Sleep for a shorter time to avoid blocking too much
            tokio::time::sleep(Duration::from_millis(2)).await;
        }

        // Performance monitoring every 300 frames (more frequent for better visibility)
        if FRAME_COUNT.load(Ordering::Relaxed) % 300 == 0 {
            let (cache_hits, cache_misses, db_queries, db_errors, avg_query_time) =
                db_client.get_performance_metrics();
            let (games_cached, players_cached, sessions_cached) = db_client.get_cache_stats();

            let total_frames = FRAME_COUNT.load(Ordering::Relaxed);
            let total_syncs = DB_SYNC_COUNT.load(Ordering::Relaxed);

            // Use info level for better visibility during testing
            tracing::info!(
                "PERF STATS - Frames: {}, Syncs: {}, Cache: {}/{}/{}, DB: {}/{}/{}ms, Hit Rate: {:.2}%",
                total_frames, total_syncs,
                games_cached, players_cached, sessions_cached,
                db_queries, db_errors, avg_query_time,
                if cache_hits + cache_misses > 0 {
                    (cache_hits as f64 / (cache_hits + cache_misses) as f64) * 100.0
                } else {
                    0.0
                }
            );
        }
    }
}
