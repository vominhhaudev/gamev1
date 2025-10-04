use std::{collections::HashMap, net::TcpListener, sync::Arc};

use proto::worker::v1::{
    worker_client::WorkerClient,
    worker_server::{Worker, WorkerServer},
    JoinRoomRequest, JoinRoomResponse, LeaveRoomRequest, LeaveRoomResponse, PushInputRequest,
    PushInputResponse, Snapshot,
    // Room management
    CreateRoomRequest, CreateRoomResponse, ListRoomsRequest, ListRoomsResponse,
    GetRoomInfoRequest, GetRoomInfoResponse, JoinRoomAsPlayerRequest, JoinRoomAsPlayerResponse,
    JoinRoomAsSpectatorRequest, JoinRoomAsSpectatorResponse, LeaveRoomAsPlayerRequest,
    LeaveRoomAsPlayerResponse,
    // Note: LeaveRoomAsSpectatorRequest/Response not implemented in proto yet
    StartGameRequest, StartGameResponse, EndGameRequest, EndGameResponse, SetPlayerReadyRequest,
    SetPlayerReadyResponse, UpdatePlayerPingRequest, UpdatePlayerPingResponse,
};
use tokio::sync::RwLock;
use tonic::{
    transport::{Channel, Endpoint, Server},
    Response, Status,
};
use tracing::{error, info, warn};

use crate::{simulation::{GameWorld, PlayerInput, SpectatorCameraMode}, simulation_metrics, validation::{InputValidator, ValidationError}, room::{RoomManager, RoomSettings, GameMode, RoomListFilter, RoomInfo, RoomState}};

pub struct WorkerState {
    pub game_world: RwLock<GameWorld>,
    pub room_manager: RwLock<RoomManager>,
}

impl WorkerState {
    pub fn new() -> Self {
        Self {
            game_world: RwLock::new(GameWorld::new()),
            room_manager: RwLock::new(RoomManager::default()),
        }
    }
}

impl Default for WorkerState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct WorkerService {
    state: Arc<WorkerState>,
}
impl WorkerService {
    pub fn new(state: Arc<WorkerState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn join_room(
        &self,
        request: tonic::Request<JoinRoomRequest>,
    ) -> Result<Response<JoinRoomResponse>, Status> {
        let req = request.into_inner();
        let room_id = req.room_id.clone();
        let player_id = req.player_id.clone();

        info!(%room_id, %player_id, "worker: player joining room");

        let mut game_world = self.state.game_world.write().await;

        // Add player vào game world
        let player_entity = game_world.add_player(player_id.clone());

        // Create initial AOI snapshot cho player mới
        let player_position = [0.0, 5.0, 0.0]; // Player spawn position
        let view_distance = 50.0; // Default view distance

        // For now, use full snapshot until AOI is properly implemented
        let snapshot = game_world.create_snapshot();

        // Update metrics
        let active_players = 1; // For now, just count this player
        simulation_metrics().set_active_players(active_players);

        info!(%room_id, %player_id, "worker: player joined successfully");

        let snapshot_json = serde_json::to_string(&snapshot)
            .unwrap_or_else(|_| json::empty_snapshot().to_string());

        Ok(Response::new(JoinRoomResponse {
            ok: true,
            room_id,
            snapshot: Some(Snapshot {
                tick: snapshot.tick,
                payload_json: snapshot_json,
            }),
            error: String::new(),
        }))
    }

    async fn leave_room(
        &self,
        request: tonic::Request<LeaveRoomRequest>,
    ) -> Result<Response<LeaveRoomResponse>, Status> {
        let req = request.into_inner();
        let room_id = req.room_id;

        // For now, just update metrics (in real implementation would remove player entity)
        let active_players = 0; // Simplified for MVP
        simulation_metrics().set_active_players(active_players);

        info!(%room_id, "worker: player left room");
        Ok(Response::new(LeaveRoomResponse {
            ok: true,
            room_id,
            error: String::new(),
        }))
    }

    async fn push_input(
        &self,
        request: tonic::Request<PushInputRequest>,
    ) -> Result<Response<PushInputResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, sequence = %req.sequence, "worker: processing input");

        let mut game_world = self.state.game_world.write().await;

        // Parse input từ JSON
        let input: PlayerInput = match serde_json::from_str(&req.payload_json) {
            Ok(input) => input,
            Err(e) => {
                warn!("Failed to parse player input: {}", e);
                return Ok(Response::new(PushInputResponse {
                    ok: false,
                    room_id: req.room_id,
                    snapshot: None,
                    error: format!("invalid_json: {}", e),
                }));
            }
        };

        let player_id = input.player_id.clone();

        // Validate input trước khi xử lý
        if let Err(validation_error) = game_world.input_validator.validate_input(&input) {
            warn!("Input validation failed for player {}: {}", player_id, validation_error);
            return Ok(Response::new(PushInputResponse {
                ok: false,
                room_id: req.room_id,
                snapshot: None,
                error: format!("validation_error: {}", validation_error),
            }));
        }

        // Add input vào buffer sau khi validation thành công
        game_world.input_buffers
            .entry(player_id.clone())
            .or_insert_with(|| crate::simulation::InputBuffer::new())
            .add_input(input);

        // Run game tick để process input
        game_world.tick();

        // Get current snapshot with AOI optimization
        let snapshot = game_world.get_snapshot_for_player(&player_id);

        // Serialize snapshot
        let snapshot_json = serde_json::to_string(&snapshot)
            .unwrap_or_else(|_| json::empty_snapshot().to_string());

        info!(room_id = %req.room_id, tick = %snapshot.tick, "worker: input processed, snapshot generated");

        Ok(Response::new(PushInputResponse {
            ok: true,
            room_id: req.room_id,
            snapshot: Some(Snapshot {
                tick: snapshot.tick,
                payload_json: snapshot_json,
            }),
            error: String::new(),
        }))
    }

    // Room management methods

    async fn create_room(
        &self,
        request: tonic::Request<CreateRoomRequest>,
    ) -> Result<Response<CreateRoomResponse>, Status> {
        let req = request.into_inner();

        info!(room_name = %req.room_name, host_id = %req.host_id, "worker: creating room");

        let mut room_manager = self.state.room_manager.write().await;

        // Convert proto RoomSettings to internal RoomSettings
        let settings = RoomSettings {
            max_players: req.settings.as_ref().map_or(8, |s| s.max_players),
            game_mode: req.settings.as_ref()
                .and_then(|s| match s.game_mode {
                    0 => Some(GameMode::Deathmatch),
                    1 => Some(GameMode::TeamDeathmatch),
                    2 => Some(GameMode::CaptureTheFlag),
                    3 => Some(GameMode::KingOfTheHill),
                    _ => None,
                })
                .unwrap_or(GameMode::Deathmatch),
            map_name: req.settings.as_ref().map_or("default_map".to_string(), |s| s.map_name.clone()),
            time_limit: req.settings.as_ref()
                .and_then(|s| if s.time_limit_seconds > 0 {
                    Some(std::time::Duration::from_secs(s.time_limit_seconds as u64))
                } else {
                    None
                }),
            has_password: req.settings.as_ref().map_or(false, |s| s.has_password),
            is_private: req.settings.as_ref().map_or(false, |s| s.is_private),
            allow_spectators: req.settings.as_ref().map_or(true, |s| s.allow_spectators),
            auto_start: req.settings.as_ref().map_or(true, |s| s.auto_start),
            min_players_to_start: req.settings.as_ref().map_or(2, |s| s.min_players_to_start),
        };

        match room_manager.create_room(req.room_name, req.host_id, req.host_name, settings) {
            Ok(room_id) => {
                info!("Room created successfully: {}", room_id);
                Ok(Response::new(CreateRoomResponse {
                    success: true,
                    room_id,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to create room: {}", e);
                Ok(Response::new(CreateRoomResponse {
                    success: false,
                    room_id: String::new(),
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn list_rooms(
        &self,
        request: tonic::Request<ListRoomsRequest>,
    ) -> Result<Response<ListRoomsResponse>, Status> {
        let req = request.into_inner();

        info!("worker: listing rooms");

        let room_manager = self.state.room_manager.read().await;

        // Convert proto filter to internal filter
        let filter = if req.filter.is_some() {
            let f = req.filter.unwrap();
            Some(RoomListFilter {
                game_mode: Some(match f.game_mode {
                    0 => GameMode::Deathmatch,
                    1 => GameMode::TeamDeathmatch,
                    2 => GameMode::CaptureTheFlag,
                    3 => GameMode::KingOfTheHill,
                    _ => GameMode::Deathmatch,
                }),
                has_password: if f.has_password { Some(true) } else { None },
                min_players: if f.min_players > 0 { Some(f.min_players) } else { None },
                max_players: if f.max_players > 0 { Some(f.max_players) } else { None },
                state: Some(match f.state {
                    0 => RoomState::Waiting,
                    1 => RoomState::Starting,
                    2 => RoomState::Playing,
                    3 => RoomState::Finished,
                    4 => RoomState::Closed,
                    _ => RoomState::Waiting,
                }),
            })
        } else {
            None
        };

        let rooms = room_manager.list_rooms(filter.as_ref());

        // Convert internal RoomInfo to proto RoomInfo
        let proto_rooms: Vec<proto::worker::v1::RoomInfo> = rooms.into_iter().map(|room| {
            proto::worker::v1::RoomInfo {
                id: room.id,
                name: room.name,
                settings: Some(proto::worker::v1::RoomSettings {
                    max_players: room.max_players,
                    game_mode: match room.game_mode {
                        GameMode::Deathmatch => 0,
                        GameMode::TeamDeathmatch => 1,
                        GameMode::CaptureTheFlag => 2,
                        GameMode::KingOfTheHill => 3,
                    },
                    map_name: room.settings.map_name,
                    time_limit_seconds: room.settings.time_limit.map_or(0, |d| d.as_secs() as u32),
                    has_password: room.has_password,
                    is_private: room.settings.is_private,
                    allow_spectators: room.settings.allow_spectators,
                    auto_start: room.settings.auto_start,
                    min_players_to_start: room.settings.min_players_to_start,
                }),
                state: match room.state {
                    RoomState::Waiting => 0,
                    RoomState::Starting => 1,
                    RoomState::Playing => 2,
                    RoomState::Finished => 3,
                    RoomState::Closed => 4,
                },
                player_count: room.player_count,
                spectator_count: room.spectator_count,
                max_players: room.max_players,
                has_password: room.has_password,
                game_mode: match room.game_mode {
                    GameMode::Deathmatch => 0,
                    GameMode::TeamDeathmatch => 1,
                    GameMode::CaptureTheFlag => 2,
                    GameMode::KingOfTheHill => 3,
                },
                created_at_seconds_ago: room.created_at,
            }
        }).collect();

        Ok(Response::new(ListRoomsResponse {
            success: true,
            rooms: proto_rooms,
            error: String::new(),
        }))
    }

    async fn get_room_info(
        &self,
        request: tonic::Request<GetRoomInfoRequest>,
    ) -> Result<Response<GetRoomInfoResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, "worker: getting room info");

        let room_manager = self.state.room_manager.read().await;

        match room_manager.get_room_info(&req.room_id) {
            Ok(room_info) => {
                let proto_room = proto::worker::v1::RoomInfo {
                    id: room_info.id,
                    name: room_info.name,
                    settings: Some(proto::worker::v1::RoomSettings {
                        max_players: room_info.max_players,
                        game_mode: match room_info.game_mode {
                            GameMode::Deathmatch => 0,
                            GameMode::TeamDeathmatch => 1,
                            GameMode::CaptureTheFlag => 2,
                            GameMode::KingOfTheHill => 3,
                        },
                        map_name: room_info.settings.map_name,
                        time_limit_seconds: room_info.settings.time_limit.map_or(0, |d| d.as_secs() as u32),
                        has_password: room_info.has_password,
                        is_private: room_info.settings.is_private,
                        allow_spectators: room_info.settings.allow_spectators,
                        auto_start: room_info.settings.auto_start,
                        min_players_to_start: room_info.settings.min_players_to_start,
                    }),
                    state: match room_info.state {
                        RoomState::Waiting => 0,
                        RoomState::Starting => 1,
                        RoomState::Playing => 2,
                        RoomState::Finished => 3,
                        RoomState::Closed => 4,
                    },
                    player_count: room_info.player_count,
                    spectator_count: room_info.spectator_count,
                    max_players: room_info.max_players,
                    has_password: room_info.has_password,
                    game_mode: match room_info.game_mode {
                        GameMode::Deathmatch => 0,
                        GameMode::TeamDeathmatch => 1,
                        GameMode::CaptureTheFlag => 2,
                        GameMode::KingOfTheHill => 3,
                    },
                    created_at_seconds_ago: room_info.created_at,
                };

                Ok(Response::new(GetRoomInfoResponse {
                    success: true,
                    room: Some(proto_room),
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to get room info: {}", e);
                Ok(Response::new(GetRoomInfoResponse {
                    success: false,
                    room: None,
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn join_room_as_player(
        &self,
        request: tonic::Request<JoinRoomAsPlayerRequest>,
    ) -> Result<Response<JoinRoomAsPlayerResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, player_id = %req.player_id, "worker: player joining room");

        let mut room_manager = self.state.room_manager.write().await;

        match room_manager.join_room(&req.room_id, req.player_id, req.player_name) {
            Ok(_) => {
                info!("Player joined room successfully");
                Ok(Response::new(JoinRoomAsPlayerResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to join room: {}", e);
                Ok(Response::new(JoinRoomAsPlayerResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn join_room_as_spectator(
        &self,
        request: tonic::Request<JoinRoomAsSpectatorRequest>,
    ) -> Result<Response<JoinRoomAsSpectatorResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, spectator_id = %req.spectator_id, "worker: spectator joining room");

        let mut room_manager = self.state.room_manager.write().await;

        // First, join the room as spectator
        match room_manager.join_room_as_spectator(&req.room_id, req.spectator_id.clone(), req.spectator_name) {
            Ok(_) => {
                // Then, add spectator to the game world
                let mut game_world = self.state.game_world.write().await;
                let spectator_entity = game_world.add_spectator(
                    req.spectator_id.clone(),
                    SpectatorCameraMode::Overview, // Default camera mode
                );

                info!(room_id = %req.room_id, spectator_id = %req.spectator_id, "Spectator joined room successfully");
                Ok(Response::new(JoinRoomAsSpectatorResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to join room as spectator: {}", e);
                Ok(Response::new(JoinRoomAsSpectatorResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn leave_room_as_player(
        &self,
        request: tonic::Request<LeaveRoomAsPlayerRequest>,
    ) -> Result<Response<LeaveRoomAsPlayerResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, player_id = %req.player_id, "worker: player leaving room");

        let mut room_manager = self.state.room_manager.write().await;

        match room_manager.leave_room(&req.room_id, &req.player_id) {
            Ok(_) => {
                info!("Player left room successfully");
                Ok(Response::new(LeaveRoomAsPlayerResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to leave room: {}", e);
                Ok(Response::new(LeaveRoomAsPlayerResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }

    // TODO: Fix LeaveRoomAsSpectatorRequest/Response in proto file
    // async fn leave_room_as_spectator(&self, request: tonic::Request<()>) -> Result<Response<()>, Status> {
    //     // Implementation here...
    // }

    async fn start_game(
        &self,
        request: tonic::Request<StartGameRequest>,
    ) -> Result<Response<StartGameResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, player_id = %req.player_id, "worker: starting game");

        let mut room_manager = self.state.room_manager.write().await;

        match room_manager.start_game(&req.room_id, &req.player_id) {
            Ok(_) => {
                info!("Game started successfully");
                Ok(Response::new(StartGameResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to start game: {}", e);
                Ok(Response::new(StartGameResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn end_game(
        &self,
        request: tonic::Request<EndGameRequest>,
    ) -> Result<Response<EndGameResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, "worker: ending game");

        let mut room_manager = self.state.room_manager.write().await;

        match room_manager.end_game(&req.room_id) {
            Ok(_) => {
                info!("Game ended successfully");
                Ok(Response::new(EndGameResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to end game: {}", e);
                Ok(Response::new(EndGameResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn set_player_ready(
        &self,
        request: tonic::Request<SetPlayerReadyRequest>,
    ) -> Result<Response<SetPlayerReadyResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, player_id = %req.player_id, ready = %req.ready, "worker: setting player ready");

        let mut room_manager = self.state.room_manager.write().await;

        match room_manager.set_player_ready(&req.room_id, &req.player_id, req.ready) {
            Ok(_) => {
                info!("Player ready status updated");
                Ok(Response::new(SetPlayerReadyResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to set player ready: {}", e);
                Ok(Response::new(SetPlayerReadyResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn update_player_ping(
        &self,
        request: tonic::Request<UpdatePlayerPingRequest>,
    ) -> Result<Response<UpdatePlayerPingResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, player_id = %req.player_id, ping = %req.ping, "worker: updating player ping");

        let mut room_manager = self.state.room_manager.write().await;

        match room_manager.update_player_ping(&req.room_id, &req.player_id, req.ping) {
            Ok(_) => {
                Ok(Response::new(UpdatePlayerPingResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to update player ping: {}", e);
                Ok(Response::new(UpdatePlayerPingResponse {
                    success: false,
                    error: e.to_string(),
                }))
            }
        }
    }
}

pub async fn serve_rpc(addr: std::net::SocketAddr, svc: WorkerService) {
    info!(%addr, "starting gRPC");
    if let Err(e) = Server::builder()
        .add_service(WorkerServer::new(svc))
        .serve_with_shutdown(addr, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
    {
        error!(?e, "gRPC server error");
    }
}

pub fn channel(endpoint: &str) -> Result<Channel, tonic::transport::Error> {
    Ok(Endpoint::from_shared(endpoint.to_string())?.connect_lazy())
}
pub type Client = WorkerClient<Channel>;
pub fn client(endpoint: &str) -> Result<Client, tonic::transport::Error> {
    Ok(WorkerClient::new(channel(endpoint)?))
}

pub async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind worker test");
    let addr = listener.local_addr().expect("addr");
    drop(listener);

    let endpoint = format!("http://{}", addr);
    let state = Arc::new(WorkerState::default());
    let svc = WorkerService::new(state);

    let handle = tokio::spawn(async move {
        serve_rpc(addr, svc).await;
    });
    (endpoint, handle)
}

mod json {
    use serde_json::{json, Value};
    pub fn empty_snapshot() -> Value {
        json!({ "entities": [] })
    }
    pub fn input_snapshot(tick: u64, sequence: u32, input_json: &str) -> String {
        let parsed_input = serde_json::from_str(input_json).unwrap_or_else(|_| json!(input_json));
        json!({ "tick": tick, "sequence": sequence, "input": parsed_input }).to_string()
    }
}
