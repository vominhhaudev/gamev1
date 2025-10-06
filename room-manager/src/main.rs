use room_manager::{RoomManagerConfig, RoomManagerState, CreateRoomRequest, GameMode};
use std::sync::Arc;
use tokio::sync::RwLock;

use common_net::telemetry;

#[tokio::main]
async fn main() {
    telemetry::init("room-manager");

    let config = match RoomManagerConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(%err, "room-manager: cau hinh khong hop le");
            return;
        }
    };

    // Test basic functionality if needed
    if std::env::var("ROOM_MANAGER_TEST").is_ok() {
        test_room_manager().await;
        return;
    }

    if let Err(err) = room_manager::run_with_ctrl_c(config).await {
        tracing::error!(%err, "room-manager ket thuc do loi");
    }
}

async fn test_room_manager() {
    println!("🧪 Testing Room Manager functionality...");

    // Initialize Room Manager state
    let pocketbase_url = std::env::var("POCKETBASE_URL").unwrap_or_else(|_| "http://localhost:8090".to_string());
    let room_state = Arc::new(RwLock::new(match RoomManagerState::new(&pocketbase_url) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("❌ Failed to initialize Room Manager: {}", e);
            return;
        }
    }));

    // Test tạo phòng
    println!("📝 Testing room creation...");
    let create_req = CreateRoomRequest {
        name: "Test Room".to_string(),
        game_mode: GameMode::Deathmatch,
        max_players: 4,
        host_player_id: "player_123".to_string(),
        settings: Some(serde_json::json!({
            "difficulty": "normal",
            "time_limit": 300
        })),
    };

    match room_manager::create_room(room_state.clone(), create_req).await {
        Ok(resp) => {
            if resp.success {
                println!("✅ Room created successfully: {}", resp.room_id);

                // Test join phòng
                println!("🚪 Testing room join...");
                let join_req = room_manager::JoinRoomRequest {
                    room_id: resp.room_id.clone(),
                    player_id: "player_456".to_string(),
                    player_name: "Test Player".to_string(),
                };

                match room_manager::join_room(room_state.clone(), join_req).await {
                    Ok(join_resp) => {
                        if join_resp.success {
                            println!("✅ Player joined room successfully");

                            // Test list rooms
                            println!("📋 Testing room listing...");
                            let list_req = room_manager::ListRoomsRequest {
                                game_mode: Some(GameMode::Deathmatch),
                                status: Some(room_manager::RoomStatus::Waiting),
                            };

                            match room_manager::list_rooms(room_state.clone(), list_req).await {
                                Ok(list_resp) => {
                                    println!("✅ Found {} rooms", list_resp.rooms.len());
                                    for room in &list_resp.rooms {
                                        println!("  - {}: {} players", room.name, room.current_players);
                                    }
                                }
                                Err(e) => eprintln!("❌ Failed to list rooms: {}", e),
                            }
                        } else {
                            eprintln!("❌ Failed to join room: {:?}", join_resp.error);
                        }
                    }
                    Err(e) => eprintln!("❌ Failed to join room: {}", e),
                }
            } else {
                eprintln!("❌ Failed to create room: {:?}", resp.error);
            }
        }
        Err(e) => eprintln!("❌ Failed to create room: {}", e),
    }

    println!("✨ Room Manager test completed!");
}
