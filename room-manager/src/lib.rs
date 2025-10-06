use std::{
    collections::HashMap,
    env,
    sync::Arc,
    time::Duration,
};

use common_net::{
    metrics::{self, MatchmakingMetrics},
    shutdown,
};
use pocketbase::PocketBaseClient;
use serde::{Deserialize, Serialize};
use tokio::{sync::{oneshot, RwLock}, time::interval};
use tracing::{error, info, warn};
use uuid::Uuid;

pub type BoxError = metrics::BoxError;

const DEFAULT_METRICS_ADDR: &str = "127.0.0.1:3200";

pub const METRICS_PATH: &str = "/metrics";

// Room và Player structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub game_mode: GameMode,
    pub max_players: u32,
    pub current_players: u32,
    pub status: RoomStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub host_player_id: String,
    pub worker_endpoint: Option<String>, // Worker được assign để chạy game này
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameMode {
    #[serde(rename = "deathmatch")]
    Deathmatch,
    #[serde(rename = "team_deathmatch")]
    TeamDeathmatch,
    #[serde(rename = "capture_the_flag")]
    CaptureTheFlag,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoomStatus {
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "finished")]
    Finished,
    #[serde(rename = "closed")]
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub room_id: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub status: PlayerStatus,
    pub team: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlayerStatus {
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "disconnected")]
    Disconnected,
    #[serde(rename = "left")]
    Left,
}

// Room Manager state
#[derive(Debug)]
pub struct RoomManagerState {
    pub rooms: HashMap<String, Room>,
    pub players: HashMap<String, Player>,
    pub pocketbase: PocketBaseClient,
    pub heartbeat_interval: Duration,
    pub room_ttl: Duration,
}

impl RoomManagerState {
    pub fn new(pocketbase_url: &str) -> Result<Self, BoxError> {
        let pocketbase = PocketBaseClient::new(pocketbase_url);

        Ok(Self {
            rooms: HashMap::new(),
            players: HashMap::new(),
            pocketbase,
            heartbeat_interval: Duration::from_secs(30),
            room_ttl: Duration::from_secs(300), // 5 minutes
        })
    }

    // Tạo phòng mới
    pub async fn create_room(&mut self, req: CreateRoomRequest) -> Result<CreateRoomResponse, BoxError> {
        let room_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let room = Room {
            id: room_id.clone(),
            name: req.name,
            game_mode: req.game_mode,
            max_players: req.max_players,
            current_players: 1,
            status: RoomStatus::Waiting,
            created_at: now,
            updated_at: now,
            host_player_id: req.host_player_id.clone(),
            worker_endpoint: None,
            settings: req.settings.unwrap_or(serde_json::json!({})),
        };

        // Lưu vào PocketBase
        let room_data = serde_json::json!({
            "id": room.id,
            "name": room.name,
            "game_mode": serde_json::to_string(&room.game_mode)?,
            "max_players": room.max_players,
            "current_players": room.current_players,
            "status": serde_json::to_string(&room.status)?,
            "created_at": room.created_at,
            "updated_at": room.updated_at,
            "host_player_id": room.host_player_id,
            "worker_endpoint": room.worker_endpoint,
            "settings": room.settings,
        });

        match self.pocketbase.create_record("rooms", room_data).await {
            Ok(_) => {
                self.rooms.insert(room_id.clone(), room);

                matchmaking_metrics().inc_rooms_created();
                info!("Created room: {}", room_id);

                Ok(CreateRoomResponse {
                    room_id,
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                error!("Failed to create room in database: {}", e);
                Ok(CreateRoomResponse {
                    room_id: String::new(),
                    success: false,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        }
    }

    // Join phòng
    pub async fn join_room(&mut self, req: JoinRoomRequest) -> Result<JoinRoomResponse, BoxError> {
        if let Some(room) = self.rooms.get_mut(&req.room_id) {
            if room.current_players >= room.max_players {
                return Ok(JoinRoomResponse {
                    success: false,
                    error: Some("Room is full".to_string()),
                    room: None,
                });
            }

            if room.status != RoomStatus::Waiting {
                return Ok(JoinRoomResponse {
                    success: false,
                    error: Some("Room is not accepting new players".to_string()),
                    room: None,
                });
            }

            let now = chrono::Utc::now();
            let player = Player {
                id: req.player_id.clone(),
                name: req.player_name,
                room_id: req.room_id.clone(),
                joined_at: now,
                last_seen: now,
                status: PlayerStatus::Connected,
                team: None,
            };

            room.current_players += 1;
            room.updated_at = now;

            // Lưu player vào database
            let player_data = serde_json::json!({
                "id": player.id,
                "name": player.name,
                "room_id": player.room_id,
                "joined_at": player.joined_at,
                "last_seen": player.last_seen,
                "status": serde_json::to_string(&player.status)?,
                "team": player.team,
            });

            match self.pocketbase.create_record("players", player_data).await {
                Ok(_) => {
                    self.players.insert(req.player_id.clone(), player);
                    // Player joined - we could add a counter for this in the future

                    Ok(JoinRoomResponse {
                        success: true,
                        error: None,
                        room: Some(room.clone()),
                    })
                }
                Err(e) => {
                    // Rollback room state
                    room.current_players -= 1;
                    error!("Failed to save player to database: {}", e);
                    Ok(JoinRoomResponse {
                        success: false,
                        error: Some(format!("Database error: {}", e)),
                        room: None,
                    })
                }
            }
        } else {
            Ok(JoinRoomResponse {
                success: false,
                error: Some("Room not found".to_string()),
                room: None,
            })
        }
    }

    // Lấy danh sách phòng
    pub async fn list_rooms(&self, req: ListRoomsRequest) -> Result<ListRoomsResponse, BoxError> {
        let mut rooms: Vec<Room> = self.rooms.values().cloned().collect();

        // Filter theo game_mode nếu có
        if let Some(game_mode) = req.game_mode {
            rooms.retain(|room| room.game_mode == game_mode);
        }

        // Filter theo status nếu có
        if let Some(status) = req.status {
            rooms.retain(|room| room.status == status);
        }

        Ok(ListRoomsResponse { rooms })
    }

    // Assign player vào phòng phù hợp
    pub async fn assign_room(&mut self, req: AssignRoomRequest) -> Result<AssignRoomResponse, BoxError> {
        let mut best_room_id: Option<String> = None;
        let mut best_player_count = u32::MAX;

        // Tìm phòng phù hợp
        for (room_id, room) in &self.rooms {
            if room.status != RoomStatus::Waiting {
                continue;
            }

            if room.current_players >= room.max_players {
                continue;
            }

            // Nếu có game_mode filter
            if let Some(ref game_mode) = req.game_mode {
                if room.game_mode != *game_mode {
                    continue;
                }
            }

            // Chọn phòng có ít player nhất hoặc phù hợp nhất
            if room.current_players < best_player_count {
                best_room_id = Some(room_id.clone());
                best_player_count = room.current_players;
            }
        }

        if let Some(room_id) = best_room_id {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                let now = chrono::Utc::now();
                let player = Player {
                    id: req.player_id.clone(),
                    name: format!("Player_{}", &req.player_id[..8]),
                    room_id: room.id.clone(),
                    joined_at: now,
                    last_seen: now,
                    status: PlayerStatus::Connected,
                    team: None,
                };

                room.current_players += 1;
                room.updated_at = now;

                self.players.insert(req.player_id.clone(), player);

                // Player assigned - we could add a counter for this in the future

                Ok(AssignRoomResponse {
                    room_id: Some(room.id.clone()),
                    worker_endpoint: room.worker_endpoint.clone(),
                })
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Room not found after assignment",
                )))
            }
        } else {
            // Không tìm thấy phòng phù hợp, tạo phòng mới
            let create_req = CreateRoomRequest {
                name: format!("Auto Room {}", Uuid::new_v4().to_string()[..8].to_string()),
                game_mode: req.game_mode.unwrap_or(GameMode::Deathmatch),
                max_players: 4,
                host_player_id: req.player_id.clone(),
                settings: Some(serde_json::json!({})),
            };

            match self.create_room(create_req).await {
                Ok(create_resp) => {
                    if create_resp.success {
                        // Tự động join vào phòng vừa tạo
                        let join_req = JoinRoomRequest {
                            room_id: create_resp.room_id.clone(),
                            player_id: req.player_id.clone(),
                            player_name: format!("Player_{}", &req.player_id[..8]),
                        };

                        match self.join_room(join_req).await {
                            Ok(_) => Ok(AssignRoomResponse {
                                room_id: Some(create_resp.room_id),
                                worker_endpoint: None,
                            }),
                            Err(e) => Err(e),
                        }
                    } else {
                        Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            create_resp.error.unwrap_or_else(|| "Failed to create room".to_string()),
                        )))
                    }
                }
                Err(e) => Err(e),
            }
        }
    }

    // Heartbeat để cleanup
    pub async fn heartbeat(&mut self) -> Result<(), BoxError> {
        let now = chrono::Utc::now();
        let mut rooms_to_remove = Vec::new();

        // Cleanup players không hoạt động
        let mut players_to_remove = Vec::new();
        for (player_id, player) in &self.players {
            if player.status == PlayerStatus::Disconnected &&
               (now - player.last_seen).num_seconds() > 60 { // 1 minute timeout
                players_to_remove.push(player_id.clone());
            }
        }

        for player_id in players_to_remove {
            if let Some(player) = self.players.remove(&player_id) {
                if let Some(room) = self.rooms.get_mut(&player.room_id) {
                    room.current_players = room.current_players.saturating_sub(1);
                    room.updated_at = now;
                }
            }
        }

        // Cleanup phòng trống hoặc quá cũ
        for (room_id, room) in &self.rooms {
            let should_remove = match room.status {
                RoomStatus::Waiting => {
                    (now - room.updated_at).num_seconds() > 300 // 5 minutes for waiting rooms
                }
                RoomStatus::Finished => {
                    (now - room.updated_at).num_seconds() > 60 // 1 minute for finished rooms
                }
                _ => false,
            };

            if should_remove {
                rooms_to_remove.push(room_id.clone());
            }
        }

        for room_id in rooms_to_remove {
            self.rooms.remove(&room_id);
            // Room removed - we could add a counter for this in the future
        }

        Ok(())
    }

    // Đồng bộ với database
    pub async fn sync_with_database(&mut self) -> Result<(), BoxError> {
        // Load rooms từ database
        match self.pocketbase.list_records("rooms", None, None).await {
            Ok(records) => {
                for record in records {
                    let room_id = record.fields.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if !room_id.is_empty() {
                        // Convert database record to Room struct
                        // TODO: Implement proper conversion
                    }
                }
            }
            Err(e) => {
                warn!("Failed to sync rooms from database: {}", e);
            }
        }

        Ok(())
    }
}

// API Request/Response types
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub game_mode: GameMode,
    pub max_players: u32,
    pub host_player_id: String,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    pub room_id: String,
    pub player_id: String,
    pub player_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomResponse {
    pub success: bool,
    pub error: Option<String>,
    pub room: Option<Room>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListRoomsRequest {
    pub game_mode: Option<GameMode>,
    pub status: Option<RoomStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListRoomsResponse {
    pub rooms: Vec<Room>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignRoomRequest {
    pub player_id: String,
    pub game_mode: Option<GameMode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignRoomResponse {
    pub room_id: Option<String>,
    pub worker_endpoint: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RoomManagerSettings {
    pub metrics_addr: std::net::SocketAddr,
}

impl RoomManagerSettings {
    pub fn from_env() -> Result<Self, BoxError> {
        let metrics_addr = env::var("ROOM_MANAGER_METRICS_ADDR")
            .unwrap_or_else(|_| DEFAULT_METRICS_ADDR.to_string());
        let metrics_addr = metrics_addr
            .parse()
            .map_err(|err| Box::new(err) as BoxError)?;
        Ok(Self { metrics_addr })
    }
}

impl Default for RoomManagerSettings {
    fn default() -> Self {
        Self {
            metrics_addr: DEFAULT_METRICS_ADDR
                .parse()
                .expect("default room-manager metrics addr"),
        }
    }
}

#[derive(Debug)]
pub struct RoomManagerConfig {
    pub metrics_addr: std::net::SocketAddr,
    pub ready_tx: Option<oneshot::Sender<std::net::SocketAddr>>,
}

impl RoomManagerConfig {
    pub fn from_settings(settings: RoomManagerSettings) -> Self {
        Self {
            metrics_addr: settings.metrics_addr,
            ready_tx: None,
        }
    }

    pub fn from_env() -> Result<Self, BoxError> {
        RoomManagerSettings::from_env().map(Self::from_settings)
    }
}

pub fn matchmaking_metrics() -> &'static MatchmakingMetrics {
    metrics::matchmaking_metrics()
}

pub async fn run_with_ctrl_c(config: RoomManagerConfig) -> Result<(), BoxError> {
    let (shutdown_tx, shutdown_rx) = shutdown::channel();

    let ctrl_c = tokio::spawn(async move {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!(%err, "room-manager: khong the lang nghe ctrl_c");
        }
        shutdown::trigger(&shutdown_tx);
    });

    let result = run(config, shutdown_rx).await;

    ctrl_c.abort();
    result
}

pub async fn run(
    config: RoomManagerConfig,
    shutdown_rx: shutdown::ShutdownReceiver,
) -> Result<(), BoxError> {
    matchmaking_metrics().on_startup();

    let listener = tokio::net::TcpListener::bind(config.metrics_addr)
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    let local_addr = listener
        .local_addr()
        .map_err(|err| Box::new(err) as BoxError)?;

    if let Some(tx) = config.ready_tx {
        let _ = tx.send(local_addr);
    }

    info!(%local_addr, path = METRICS_PATH, "room-manager metrics exporter dang lang nghe");

    // Initialize Room Manager state
    let pocketbase_url = std::env::var("POCKETBASE_URL").unwrap_or_else(|_| "http://localhost:8090".to_string());
    let room_state = Arc::new(RwLock::new(RoomManagerState::new(&pocketbase_url)?));

    // Sync với database khi khởi động
    {
        let mut state = room_state.write().await;
        if let Err(e) = state.sync_with_database().await {
            error!("Failed to sync with database: {}", e);
        }
    }

    // Background heartbeat task
    let heartbeat_state = room_state.clone();
    let heartbeat_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let mut state = heartbeat_state.write().await;
            if let Err(e) = state.heartbeat().await {
                error!("Heartbeat failed: {}", e);
            }
        }
    });

    let server = tokio::spawn(async move {
        if let Err(err) = metrics::serve_metrics(listener, METRICS_PATH).await {
            error!(%err, "room-manager metrics exporter dung bat thuong");
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown::wait(shutdown_rx) => {
            info!("Room Manager shutting down...");
        }
    }

    // Cleanup
    heartbeat_task.abort();
    server.abort();

    Ok(())
}

// Helper functions để expose Room Manager functionality
pub async fn create_room(
    state: Arc<RwLock<RoomManagerState>>,
    request: CreateRoomRequest,
) -> Result<CreateRoomResponse, BoxError> {
    let mut state = state.write().await;
    state.create_room(request).await
}

pub async fn join_room(
    state: Arc<RwLock<RoomManagerState>>,
    request: JoinRoomRequest,
) -> Result<JoinRoomResponse, BoxError> {
    let mut state = state.write().await;
    state.join_room(request).await
}

pub async fn list_rooms(
    state: Arc<RwLock<RoomManagerState>>,
    request: ListRoomsRequest,
) -> Result<ListRoomsResponse, BoxError> {
    let state = state.read().await;
    state.list_rooms(request).await
}

pub async fn assign_room(
    state: Arc<RwLock<RoomManagerState>>,
    request: AssignRoomRequest,
) -> Result<AssignRoomResponse, BoxError> {
    let mut state = state.write().await;
    state.assign_room(request).await
}
