use worker::{WorkerConfig, simulation::{GameWorld, spawn_test_entities}, database::PocketBaseClient};
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

    // Create game world với ECS và Physics
    let mut game_world = GameWorld::new();

    // Spawn test entities
    spawn_test_entities(&mut game_world);
    tracing::info!("Game world created with ECS and Physics");

    // Fixed timestep: 60 FPS (16.67ms per frame)
    let target_frame_time = Duration::from_millis(16);
    let mut accumulator = Duration::from_secs(0);
    let mut last_time = Instant::now();
    let mut frame_start_time = Instant::now();

    // Giới hạn số frames tối đa có thể chạy trong một lần lặp để tránh quá tải
    const MAX_FRAMES_PER_CYCLE: u32 = 10;
    const MIN_FRAME_TIME: Duration = Duration::from_millis(10); // Minimum 10ms per frame

    // Start gRPC server in background
    let _grpc_handle = tokio::spawn(async move {
        if let Err(err) = worker::run_with_ctrl_c(config).await {
            tracing::error!(%err, "worker gRPC server ket thuc do loi");
        }
    });

    tracing::info!("Worker simulation started with 60Hz fixed timestep (max {} frames per cycle)", MAX_FRAMES_PER_CYCLE);
    tracing::info!("Database sync every {} frames ({} seconds)", DB_SYNC_INTERVAL, DB_SYNC_INTERVAL / 60);
    tracing::info!("Minimum frame time: {}ms to prevent CPU overload", MIN_FRAME_TIME.as_millis());
    tracing::info!("Performance monitoring: every 300 frames (5 seconds)");
    tracing::info!("Snapshot logging: every 600 frames (10 seconds)");

    loop {
        frame_start_time = Instant::now();
        let current_time = Instant::now();
        let delta_time = current_time - last_time;
        last_time = current_time;

        accumulator += delta_time;

        // Fixed timestep loop với giới hạn số frames tối đa
        let mut frames_this_cycle = 0;
        while accumulator >= target_frame_time && frames_this_cycle < MAX_FRAMES_PER_CYCLE {
            let frame_count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            frames_this_cycle += 1;

            // Step game simulation với ECS và Physics
            let sim_start = Instant::now();
            let snapshot = game_world.tick();
            let sim_time = sim_start.elapsed();

            // Send snapshot to clients (placeholder) - chỉ tick nếu có entities
            if frame_count % 600 == 0 && !snapshot.entities.is_empty() { // Log mỗi 10 giây
                tracing::debug!("Game snapshot: {} entities at tick {}", snapshot.entities.len(), snapshot.tick);
            }

            // Periodic database sync (every DB_SYNC_INTERVAL frames) - just increment counter
            if frame_count % DB_SYNC_INTERVAL == 0 {
                DB_SYNC_COUNT.fetch_add(1, Ordering::Relaxed);
            }

            accumulator -= target_frame_time;
        }

        // Nếu không có entities nào, giảm tần suất simulation để tiết kiệm tài nguyên
        if game_world.get_snapshot().entities.is_empty() && accumulator < target_frame_time {
            // Sleep lâu hơn khi không có game nào đang chạy
            time::sleep(Duration::from_millis(50)).await;
        }

        // Performance monitoring every 300 frames (5 seconds at 60fps) to reduce spam
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

        // Frame timing với giới hạn tối thiểu để tránh quá tải CPU
        let frame_time = frame_start_time.elapsed();

        // Đảm bảo mỗi frame ít nhất MIN_FRAME_TIME để tránh quá tải
        if frame_time < MIN_FRAME_TIME {
            let sleep_time = MIN_FRAME_TIME - frame_time;
            tokio::time::sleep(sleep_time).await;
        }

        // Performance monitoring every 600 frames (10 seconds at 60fps) - less frequent to reduce spam
        if FRAME_COUNT.load(Ordering::Relaxed) % 600 == 0 {
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
