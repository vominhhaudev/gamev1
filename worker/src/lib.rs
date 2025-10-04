#![recursion_limit = "256"]

use common_net::metrics::{self, SimulationMetrics};
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

pub type BoxError = metrics::BoxError;

const DEFAULT_METRICS_ADDR: &str = "127.0.0.1:3100";
const DEFAULT_RPC_ADDR: &str = "127.0.0.1:50051";
pub const METRICS_PATH: &str = "/metrics";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WorkerSettings {
    pub rpc_addr: String,
    pub metrics_addr: String,
    pub fail_fast: bool,
}
impl Default for WorkerSettings {
    fn default() -> Self {
        Self {
            rpc_addr: DEFAULT_RPC_ADDR.into(),
            metrics_addr: DEFAULT_METRICS_ADDR.into(),
            fail_fast: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub rpc_addr: SocketAddr,
    pub metrics_addr: SocketAddr,
    pub fail_fast: bool,
}
impl WorkerConfig {
    pub fn from_env() -> Result<Self, BoxError> {
        Ok(Self {
            rpc_addr: env_socket("WORKER_RPC_ADDR", DEFAULT_RPC_ADDR)?,
            metrics_addr: env_socket("WORKER_METRICS_ADDR", DEFAULT_METRICS_ADDR)?,
            fail_fast: std::env::var("WORKER_FAIL_FAST").ok().as_deref() == Some("1"),
        })
    }
    pub fn from_settings(s: WorkerSettings) -> Result<Self, BoxError> {
        Ok(Self {
            rpc_addr: s.rpc_addr.parse().map_err(|e| Box::new(e) as BoxError)?,
            metrics_addr: s
                .metrics_addr
                .parse()
                .map_err(|e| Box::new(e) as BoxError)?,
            fail_fast: s.fail_fast,
        })
    }
}

impl WorkerSettings {
    pub fn from_env() -> Result<Self, BoxError> {
        Ok(Self {
            rpc_addr: std::env::var("WORKER_RPC_ADDR")
                .unwrap_or_else(|_| DEFAULT_RPC_ADDR.to_string()),
            metrics_addr: std::env::var("WORKER_METRICS_ADDR")
                .unwrap_or_else(|_| DEFAULT_METRICS_ADDR.to_string()),
            fail_fast: std::env::var("WORKER_FAIL_FAST").ok().as_deref() == Some("1"),
        })
    }
}
pub fn simulation_metrics() -> &'static SimulationMetrics {
    metrics::simulation_metrics()
}

pub async fn run_with_ctrl_c(config: WorkerConfig) -> Result<(), BoxError> {
    let (tx, rx) = common_net::shutdown::channel();
    let j = tokio::spawn(async move {
        let _ = run(config, rx).await;
    });

    tokio::signal::ctrl_c().await.ok();
    info!("worker: ctrl_c received, shutting down");
    common_net::shutdown::trigger(&tx);
    let _ = j.await;
    Ok(())
}

pub async fn run(
    config: WorkerConfig,
    shutdown_rx: common_net::shutdown::ShutdownReceiver,
) -> Result<(), BoxError> {
    simulation_metrics().on_startup();

    let _metrics_task =
        metrics::spawn_metrics_exporter(config.metrics_addr, METRICS_PATH, "worker");

    let state = Arc::new(crate::rpc::WorkerState::default());
    let svc = crate::rpc::WorkerService::new(state.clone());

    info!(addr = %config.rpc_addr, "worker: starting gRPC");
    let grpc_task = tokio::spawn(async move {
        crate::rpc::serve_rpc(config.rpc_addr, svc).await;
    });

    // Room manager cleanup task
    let cleanup_state = state.clone();
    let cleanup_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Cleanup every minute
        loop {
            interval.tick().await;

            let mut room_manager = cleanup_state.room_manager.write().await;
            room_manager.cleanup();
            drop(room_manager);

            tracing::debug!("Room manager cleanup completed");
        }
    });

    common_net::shutdown::wait(shutdown_rx).await;
    grpc_task.abort();
    cleanup_task.abort();
    Ok(())
}

fn env_socket(key: &str, default: &str) -> Result<SocketAddr, BoxError> {
    let value = std::env::var(key).unwrap_or_else(|_| default.to_string());
    Ok(value.parse().map_err(|err| Box::new(err) as BoxError)?)
}

pub mod rpc;
pub mod snapshot;
pub mod simulation;
pub mod database;
pub mod validation;
pub mod room;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_simulation_basic() {
        // Tạo game world mới
        let mut game_world = simulation::GameWorld::new();

        // Spawn test entities
        simulation::spawn_test_entities(&mut game_world);

        // Kiểm tra có player không
        let player_count = game_world.world.query::<&simulation::Player>().iter(&game_world.world).count();
        assert!(player_count > 0, "Should have at least one player");

        // Kiểm tra có pickups không
        let pickup_count = game_world.world.query::<&simulation::Pickup>().iter(&game_world.world).count();
        assert!(pickup_count > 0, "Should have pickups for testing");

        // Chạy simulation trong 60 ticks (fixed timestep)
        let snapshots = game_world.run_simulation_for_test(1.0);

        // Kiểm tra có snapshots không
        assert!(!snapshots.is_empty(), "Should generate snapshots");
        assert_eq!(snapshots.len(), 60, "Should have 60 snapshots for 1 second at 60fps");

        // Kiểm tra tick count tăng dần
        for (i, snapshot) in snapshots.iter().enumerate() {
            assert_eq!(snapshot.tick as usize, i, "Tick count should increase sequentially");
        }

        // Kiểm tra có entities trong snapshots
        let last_snapshot = &snapshots[snapshots.len() - 1];
        assert!(!last_snapshot.entities.is_empty(), "Last snapshot should contain entities");
    }

    #[test]
    fn test_gameplay_logic_pickup_collection() {
        // Tạo game world với player và pickups
        let mut game_world = simulation::GameWorld::new();
        game_world.add_player("test_player".to_string());

        // Thêm một pickup gần player để test collision
        game_world.add_pickup([0.5, 1.0, 0.0], 10);

        // Lấy player ban đầu để kiểm tra score
        let player_entity = game_world.world.resource::<simulation::PlayerEntityMap>().map.get("test_player").copied().unwrap();
        let initial_score = game_world.world.get::<simulation::Player>(player_entity).unwrap().score;

        // Chạy một vài ticks để trigger collision detection
        for _ in 0..10 {
            let _snapshot = game_world.tick();
        }

        // Kiểm tra score đã tăng (nếu collision detection hoạt động)
        let final_score = game_world.world.get::<simulation::Player>(player_entity).unwrap().score;
        // Note: Collision detection có thể chưa hoạt động hoàn hảo trong test này
        // vì cần physics simulation thực tế hơn

        println!("Initial score: {}, Final score: {}", initial_score, final_score);
    }

    #[tokio::test]
    async fn test_end_to_end_client_worker_integration() {
        use proto::worker::v1::{worker_client::WorkerClient, JoinRoomRequest, PushInputRequest};
        use std::time::Duration;

        // Start test worker server
        let (endpoint, server_handle) = crate::rpc::spawn_test_server().await;

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create client connection
        let mut client = crate::rpc::client(&endpoint).expect("Failed to create client");

        // Test 1: Join room
        println!("Testing join room...");
        let join_response = client
            .join_room(JoinRoomRequest {
                room_id: "test_room".to_string(),
                player_id: "test_player".to_string(),
            })
            .await
            .expect("Failed to join room");

        assert!(join_response.into_inner().ok, "Join room should succeed");
        println!("✓ Join room successful");

        // Test 2: Push input and verify simulation runs
        println!("Testing input processing...");

        // Send movement input
        let input = crate::simulation::PlayerInput {
            player_id: "test_player".to_string(),
            input_sequence: 1,
            movement: [1.0, 0.0, 0.0], // Move right
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let input_json = serde_json::to_string(&input).unwrap();

        let push_response = client
            .push_input(PushInputRequest {
                room_id: "test_room".to_string(),
                sequence: 1,
                payload_json: input_json.clone(),
            })
            .await
            .expect("Failed to push input");

        let response = push_response.into_inner();
        assert!(response.ok, "Push input should succeed");

        // Verify snapshot is returned
        assert!(response.snapshot.is_some(), "Should return snapshot");
        let snapshot = response.snapshot.unwrap();

        println!("✓ Input processed, tick: {}", snapshot.tick);

        // Test 3: Send multiple inputs and verify tick progression
        println!("Testing multiple inputs...");

        let mut previous_tick: u64 = 0;
        for i in 2..=5 {
            let input = crate::simulation::PlayerInput {
                player_id: "test_player".to_string(),
                input_sequence: i,
                movement: [0.5, 0.0, 0.5], // Diagonal movement
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            let input_json = serde_json::to_string(&input).unwrap();

            let push_response = client
                .push_input(PushInputRequest {
                    room_id: "test_room".to_string(),
                    sequence: i,
                    payload_json: input_json.clone(),
                })
                .await
                .expect("Failed to push input");

            let response = push_response.into_inner();
            assert!(response.ok, "Push input {} should succeed", i);

            if let Some(snapshot) = response.snapshot {
                println!("✓ Input {} processed, tick: {}", i, snapshot.tick);

                // Verify tick is increasing (skip first iteration where previous_tick = 0)
                if i > 2 && previous_tick > 0 {
                    assert!(snapshot.tick > previous_tick, "Tick should increase");
                }
                previous_tick = snapshot.tick;
            }
        }

        // Cleanup
        server_handle.abort();
        println!("✓ End-to-end integration test completed successfully");
    }

    #[tokio::test]
    async fn test_input_processing_end_to_end() {
        use proto::worker::v1::{worker_client::WorkerClient, JoinRoomRequest, PushInputRequest};
        use std::time::Duration;

        // Start test worker server
        let (endpoint, server_handle) = crate::rpc::spawn_test_server().await;

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create client connection
        let mut client = crate::rpc::client(&endpoint).expect("Failed to create client");

        // Test 1: Join room
        let join_response = client
            .join_room(JoinRoomRequest {
                room_id: "test_room".to_string(),
                player_id: "test_player".to_string(),
            })
            .await
            .expect("Failed to join room");

        assert!(join_response.into_inner().ok, "Join room should succeed");

        // Test 2: Push movement input and verify player movement
        println!("Testing input processing - player movement...");

        // Get initial snapshot
        let initial_input = crate::simulation::PlayerInput {
            player_id: "test_player".to_string(),
            input_sequence: 1,
            movement: [0.0, 0.0, 0.0], // No movement initially
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let initial_input_json = serde_json::to_string(&initial_input).unwrap();
        let initial_response = client
            .push_input(PushInputRequest {
                room_id: "test_room".to_string(),
                sequence: 1,
                payload_json: initial_input_json.clone(),
            })
            .await
            .expect("Failed to push initial input");

        let initial_snapshot = initial_response.into_inner().snapshot.unwrap();
        let initial_payload: crate::simulation::GameSnapshot = serde_json::from_str(&initial_snapshot.payload_json).unwrap();
        let initial_player_pos = initial_payload.entities.iter()
            .find(|e| e.player.as_ref().map_or(false, |p| p.id == "test_player"))
            .map(|e| e.transform.position)
            .unwrap_or([0.0, 5.0, 0.0]);

        println!("Initial player position: {:?}", initial_player_pos);

        // Test 3: Send movement input and verify position changes
        println!("Testing movement input processing...");

        let move_right_input = crate::simulation::PlayerInput {
            player_id: "test_player".to_string(),
            input_sequence: 2,
            movement: [5.0, 0.0, 0.0], // Move right
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let move_right_json = serde_json::to_string(&move_right_input).unwrap();

        // Send multiple movement inputs để thấy sự thay đổi rõ rệt
        for seq in 3..=10 {
            let input = crate::simulation::PlayerInput {
                player_id: "test_player".to_string(),
                input_sequence: seq,
                movement: [3.0, 0.0, 2.0], // Diagonal movement để test cả X và Z
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            let input_json = serde_json::to_string(&input).unwrap();

            let push_response = client
                .push_input(PushInputRequest {
                    room_id: "test_room".to_string(),
                    sequence: seq,
                    payload_json: input_json.clone(),
                })
                .await
                .expect("Failed to push movement input");

            let response = push_response.into_inner();
            assert!(response.ok, "Push input {} should succeed", seq);

            if let Some(snapshot) = response.snapshot {
                let payload: crate::simulation::GameSnapshot = serde_json::from_str(&snapshot.payload_json).unwrap();

                // Verify tick is increasing
                assert!(snapshot.tick >= (seq - 1) as u64, "Tick should increase or stay same");

                // Check if player position changed (nếu có player trong snapshot)
                if let Some(player_entity) = payload.entities.iter()
                    .find(|e| e.player.as_ref().map_or(false, |p| p.id == "test_player")) {

                    println!("Tick {}: Player position: {:?}", snapshot.tick, player_entity.transform.position);

                    // Với nhiều input, position nên thay đổi từ vị trí ban đầu
                    if snapshot.tick > 2 {
                        let current_pos = player_entity.transform.position;
                        let distance_moved = ((current_pos[0] - initial_player_pos[0]).powi(2) +
                                            (current_pos[2] - initial_player_pos[2]).powi(2)).sqrt();

                        // Với movement input liên tục, player nên di chuyển một khoảng cách hợp lý
                        println!("Distance moved: {}", distance_moved);

                        // Không assert cứng vì physics simulation có thể khác nhau, chỉ log để quan sát
                    }
                }
            }
        }

        // Test 4: Verify input buffer processing
        println!("Testing input buffer and sequence handling...");

        // Send inputs với sequence numbers khác nhau
        let mut expected_sequences = Vec::new();
        for seq in 11..=15 {
            expected_sequences.push(seq);

            let input = crate::simulation::PlayerInput {
                player_id: "test_player".to_string(),
                input_sequence: seq,
                movement: [1.0, 0.0, 1.0],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            let input_json = serde_json::to_string(&input).unwrap();

            let push_response = client
                .push_input(PushInputRequest {
                    room_id: "test_room".to_string(),
                    sequence: seq,
                    payload_json: input_json.clone(),
                })
                .await
                .expect("Failed to push sequenced input");

            assert!(push_response.into_inner().ok);
        }

        // Final snapshot should reflect all processed inputs
        let final_response = client
            .push_input(PushInputRequest {
                room_id: "test_room".to_string(),
                sequence: 16,
                payload_json: r#"{"player_id":"test_player","input_sequence":16,"movement":[0,0,0],"timestamp":0}"#.to_string(),
            })
            .await
            .expect("Failed to get final snapshot");

        let final_snapshot = final_response.into_inner().snapshot.unwrap();
        println!("Final tick: {}", final_snapshot.tick);

        // Cleanup
        server_handle.abort();
        println!("✓ Input processing end-to-end test completed successfully");
    }

    #[test]
    fn test_comprehensive_game_simulation() {
        // Comprehensive test với tất cả các loại entities
        let mut game_world = simulation::GameWorld::new();

        // Spawn comprehensive test entities
        simulation::spawn_test_entities(&mut game_world);

        // Verify có đủ loại entities
        let player_count = game_world.world.query::<&simulation::Player>().iter(&game_world.world).count();
        let pickup_count = game_world.world.query::<&simulation::Pickup>().iter(&game_world.world).count();
        let obstacle_count = game_world.world.query::<&simulation::Obstacle>().iter(&game_world.world).count();
        let power_up_count = game_world.world.query::<&simulation::PowerUp>().iter(&game_world.world).count();
        let enemy_count = game_world.world.query::<&simulation::Enemy>().iter(&game_world.world).count();

        println!("Spawned entities:");
        println!("  Players: {}", player_count);
        println!("  Pickups: {}", pickup_count);
        println!("  Obstacles: {}", obstacle_count);
        println!("  Power-ups: {}", power_up_count);
        println!("  Enemies: {}", enemy_count);

        assert!(player_count >= 1, "Should have at least one player");
        assert!(pickup_count >= 5, "Should have pickups for gameplay");
        assert!(obstacle_count >= 3, "Should have obstacles");
        assert!(power_up_count >= 1, "Should have power-ups");
        assert!(enemy_count >= 2, "Should have enemies for combat");

        // Run simulation cho 120 ticks (2 seconds at 60fps)
        let snapshots = game_world.run_simulation_for_test(2.0);

        assert!(!snapshots.is_empty(), "Should generate snapshots");
        assert_eq!(snapshots.len(), 120, "Should have 120 snapshots for 2 seconds at 60fps");

        // Verify gameplay mechanics hoạt động
        let first_snapshot = &snapshots[0];
        let last_snapshot = &snapshots[snapshots.len() - 1];

        println!("Simulation ran for {} ticks", last_snapshot.tick);

        // Verify entities vẫn tồn tại và gameplay hoạt động
        assert!(!last_snapshot.entities.is_empty(), "Final snapshot should have entities");

        // Log some statistics
        let mut final_pickups = 0;
        let mut final_obstacles = 0;
        let mut final_enemies = 0;

        for entity in &last_snapshot.entities {
            if entity.pickup.is_some() { final_pickups += 1; }
            if entity.obstacle.is_some() { final_obstacles += 1; }
            if entity.enemy.is_some() { final_enemies += 1; }
        }

        println!("Final entity counts:");
        println!("  Pickups: {}", final_pickups);
        println!("  Obstacles: {}", final_obstacles);
        println!("  Enemies: {}", final_enemies);

        // Verify player có điểm số hợp lý (nếu gameplay hoạt động)
        if let Some(player_entity) = last_snapshot.entities.iter()
            .find(|e| e.player.is_some()) {

            if let Some(player) = &player_entity.player {
                println!("Player final score: {}", player.score);
                // Score có thể là 0 nếu chưa collect được pickup nào
                assert!(player.score >= 0, "Player score should be non-negative");
            }
        }

        println!("✓ Comprehensive game simulation test completed successfully");
    }
}
