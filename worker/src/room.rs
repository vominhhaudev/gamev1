use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use uuid::Uuid;

/// Room state enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoomState {
    Waiting,    // Đang chờ người chơi
    Starting,   // Đang khởi động game
    Playing,    // Đang chơi
    Finished,   // Đã kết thúc
    Closed,     // Đã đóng
}

/// Game mode enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameMode {
    Deathmatch,    // Mọi người chơi tự do
    TeamDeathmatch, // Chia đội
    CaptureTheFlag, // Cướp cờ
    KingOfTheHill,  // Vua đồi
}

/// Room settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    pub max_players: u32,
    pub game_mode: GameMode,
    pub map_name: String,
    pub time_limit: Option<Duration>, // None = unlimited
    pub has_password: bool,
    pub is_private: bool,
    pub allow_spectators: bool,
    pub auto_start: bool,
    pub min_players_to_start: u32,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            max_players: 8,
            game_mode: GameMode::Deathmatch,
            map_name: "default_map".to_string(),
            time_limit: Some(Duration::from_secs(300)), // 5 minutes
            has_password: false,
            is_private: false,
            allow_spectators: true,
            auto_start: true,
            min_players_to_start: 2,
        }
    }
}

/// Player info trong room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPlayer {
    pub id: String,
    pub name: String,
    pub joined_at: u64, // Unix timestamp in seconds
    pub is_ready: bool,
    pub is_host: bool,
    pub team: Option<String>, // For team modes
    pub score: u32,
    pub ping: u32, // milliseconds
    pub last_seen: u64, // Unix timestamp in seconds
}

impl RoomPlayer {
    pub fn new(id: String, name: String, is_host: bool) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            name,
            joined_at: now,
            is_ready: false,
            is_host,
            team: None,
            score: 0,
            ping: 0,
            last_seen: now,
        }
    }

    pub fn update_ping(&mut self, ping: u32) {
        self.ping = ping;
        self.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Spectator info trong room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSpectator {
    pub id: String,
    pub name: String,
    pub joined_at: u64, // Unix timestamp in seconds
}

impl RoomSpectator {
    pub fn new(id: String, name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            name,
            joined_at: now,
        }
    }
}

/// Room entity đại diện cho một phòng chơi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub settings: RoomSettings,
    pub state: RoomState,
    pub players: HashMap<String, RoomPlayer>,
    pub spectators: HashMap<String, RoomSpectator>,
    pub host_id: String,
    pub created_at: u64, // Unix timestamp in seconds
    pub started_at: Option<u64>, // Unix timestamp in seconds
    pub ended_at: Option<u64>, // Unix timestamp in seconds
    pub password_hash: Option<String>, // Hashed password for private rooms
    pub game_world_id: Option<String>, // Link to game world instance
}

impl Room {
    pub fn new(name: String, host_id: String, host_name: String, settings: RoomSettings) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut players = HashMap::new();
        players.insert(host_id.clone(), RoomPlayer::new(host_id.clone(), host_name, true));

        Self {
            id,
            name,
            settings,
            state: RoomState::Waiting,
            players,
            spectators: HashMap::new(),
            host_id,
            created_at: now,
            started_at: None,
            ended_at: None,
            password_hash: None,
            game_world_id: None,
        }
    }

    /// Check if room can accept new players
    pub fn can_join(&self, player_id: &str) -> Result<(), RoomError> {
        // Check if already in room
        if self.players.contains_key(player_id) || self.spectators.contains_key(player_id) {
            return Err(RoomError::AlreadyInRoom);
        }

        // Check room state
        if self.state != RoomState::Waiting {
            return Err(RoomError::RoomNotAcceptingPlayers);
        }

        // Check max players
        if self.players.len() >= self.settings.max_players as usize {
            return Err(RoomError::RoomFull);
        }

        Ok(())
    }

    /// Add player to room
    pub fn add_player(&mut self, player_id: String, player_name: String) -> Result<(), RoomError> {
        self.can_join(&player_id)?;

        let player_id_clone = player_id.clone();
        let player = RoomPlayer::new(player_id, player_name, false);
        self.players.insert(player_id_clone.clone(), player);

        info!("Player {} joined room {}", player_id_clone, self.id);
        Ok(())
    }

    /// Remove player from room
    pub fn remove_player(&mut self, player_id: &str) -> Result<(), RoomError> {
        if !self.players.contains_key(player_id) {
            return Err(RoomError::PlayerNotInRoom);
        }

        self.players.remove(player_id);

        // If host left, assign new host or close room
        if player_id == self.host_id {
            if let Some(new_host) = self.players.keys().next().cloned() {
                self.host_id = new_host.clone();
                if let Some(host_player) = self.players.get_mut(&new_host) {
                    host_player.is_host = true;
                }
            } else {
                // No players left, close room
                self.state = RoomState::Closed;
            }
        }

        info!("Player {} left room {}", player_id, self.id);
        Ok(())
    }

    /// Add spectator to room
    pub fn add_spectator(&mut self, spectator_id: String, spectator_name: String) -> Result<(), RoomError> {
        if !self.settings.allow_spectators {
            return Err(RoomError::SpectatorsNotAllowed);
        }

        if self.spectators.contains_key(&spectator_id) {
            return Err(RoomError::AlreadyInRoom);
        }

        let spectator_id_clone = spectator_id.clone();
        let spectator = RoomSpectator::new(spectator_id, spectator_name);
        self.spectators.insert(spectator_id_clone.clone(), spectator);

        info!("Spectator {} joined room {}", spectator_id_clone, self.id);
        Ok(())
    }

    /// Remove spectator from room
    pub fn remove_spectator(&mut self, spectator_id: &str) -> Result<(), RoomError> {
        if !self.spectators.contains_key(spectator_id) {
            return Err(RoomError::SpectatorNotInRoom);
        }

        self.spectators.remove(spectator_id);
        Ok(())
    }

    /// Check if player can start game
    pub fn can_start_game(&self, player_id: &str) -> Result<(), RoomError> {
        // Only host can start
        if player_id != self.host_id {
            return Err(RoomError::NotHost);
        }

        // Must be in waiting state
        if self.state != RoomState::Waiting {
            return Err(RoomError::InvalidState);
        }

        // Check minimum players
        if self.players.len() < self.settings.min_players_to_start as usize {
            return Err(RoomError::NotEnoughPlayers);
        }

        Ok(())
    }

    /// Start the game
    pub fn start_game(&mut self) -> Result<(), RoomError> {
        self.can_start_game(&self.host_id)?;

        self.state = RoomState::Starting;
        self.started_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());

        info!("Room {} starting game", self.id);
        Ok(())
    }

    /// End the game
    pub fn end_game(&mut self) -> Result<(), RoomError> {
        if self.state != RoomState::Playing {
            return Err(RoomError::InvalidState);
        }

        self.state = RoomState::Finished;
        self.ended_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());

        info!("Room {} game ended", self.id);
        Ok(())
    }

    /// Set player as ready
    pub fn set_player_ready(&mut self, player_id: &str, ready: bool) -> Result<(), RoomError> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.is_ready = ready;
            Ok(())
        } else {
            Err(RoomError::PlayerNotInRoom)
        }
    }

    /// Update player ping
    pub fn update_player_ping(&mut self, player_id: &str, ping: u32) -> Result<(), RoomError> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.update_ping(ping);
            Ok(())
        } else {
            Err(RoomError::PlayerNotInRoom)
        }
    }

    /// Get room info for client
    pub fn get_room_info(&self) -> RoomInfo {
        RoomInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            settings: self.settings.clone(),
            state: self.state.clone(),
            player_count: self.players.len() as u32,
            spectator_count: self.spectators.len() as u32,
            max_players: self.settings.max_players,
            has_password: self.settings.has_password,
            game_mode: self.settings.game_mode.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() - self.created_at,
        }
    }

    /// Check if room is empty (no players or spectators)
    pub fn is_empty(&self) -> bool {
        self.players.is_empty() && self.spectators.is_empty()
    }

    /// Get room age in seconds
    pub fn age_seconds(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - self.created_at
    }
}

/// Room info gửi cho client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub settings: RoomSettings,
    pub state: RoomState,
    pub player_count: u32,
    pub spectator_count: u32,
    pub max_players: u32,
    pub has_password: bool,
    pub game_mode: GameMode,
    pub created_at: u64, // seconds ago
}

/// Room errors
#[derive(Debug, Clone)]
pub enum RoomError {
    RoomNotFound,
    RoomFull,
    RoomNotAcceptingPlayers,
    PlayerNotInRoom,
    SpectatorNotInRoom,
    AlreadyInRoom,
    NotHost,
    NotEnoughPlayers,
    SpectatorsNotAllowed,
    InvalidState,
    InvalidPassword,
    RoomNameTaken,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomError::RoomNotFound => write!(f, "Room not found"),
            RoomError::RoomFull => write!(f, "Room is full"),
            RoomError::RoomNotAcceptingPlayers => write!(f, "Room is not accepting players"),
            RoomError::PlayerNotInRoom => write!(f, "Player not in room"),
            RoomError::SpectatorNotInRoom => write!(f, "Spectator not in room"),
            RoomError::AlreadyInRoom => write!(f, "Already in room"),
            RoomError::NotHost => write!(f, "Not the host"),
            RoomError::NotEnoughPlayers => write!(f, "Not enough players to start"),
            RoomError::SpectatorsNotAllowed => write!(f, "Spectators not allowed"),
            RoomError::InvalidState => write!(f, "Invalid room state"),
            RoomError::InvalidPassword => write!(f, "Invalid password"),
            RoomError::RoomNameTaken => write!(f, "Room name already taken"),
        }
    }
}

impl std::error::Error for RoomError {}

/// Room list filter
#[derive(Debug, Clone, Default)]
pub struct RoomListFilter {
    pub game_mode: Option<GameMode>,
    pub has_password: Option<bool>,
    pub min_players: Option<u32>,
    pub max_players: Option<u32>,
    pub state: Option<RoomState>,
}

impl RoomListFilter {
    pub fn matches(&self, room: &Room) -> bool {
        if let Some(game_mode) = &self.game_mode {
            if room.settings.game_mode != *game_mode {
                return false;
            }
        }

        if let Some(has_password) = self.has_password {
            if room.settings.has_password != has_password {
                return false;
            }
        }

        if let Some(min_players) = self.min_players {
            if (room.players.len() as u32) < min_players {
                return false;
            }
        }

        if let Some(max_players) = self.max_players {
            if room.players.len() as u32 > max_players {
                return false;
            }
        }

        if let Some(state) = &self.state {
            if room.state != *state {
                return false;
            }
        }

        true
    }
}

/// RoomManager để quản lý nhiều rooms
#[derive(Debug)]
pub struct RoomManager {
    rooms: HashMap<String, Room>,
    max_rooms: usize,
    cleanup_interval: Duration,
    last_cleanup: u64, // Unix timestamp in seconds
}

impl RoomManager {
    pub fn new(max_rooms: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            rooms: HashMap::new(),
            max_rooms,
            cleanup_interval: Duration::from_secs(60), // Cleanup every minute
            last_cleanup: now,
        }
    }

    /// Create a new room
    pub fn create_room(
        &mut self,
        name: String,
        host_id: String,
        host_name: String,
        settings: RoomSettings,
    ) -> Result<String, RoomError> {
        // Check if room name is taken
        if self.rooms.values().any(|room| room.name == name) {
            return Err(RoomError::RoomNameTaken);
        }

        // Check max rooms limit
        if self.rooms.len() >= self.max_rooms {
            return Err(RoomError::RoomNotFound); // Reuse for simplicity
        }

        let name_clone = name.clone();
        let room = Room::new(name, host_id, host_name, settings);
        let room_id = room.id.clone();

        self.rooms.insert(room_id.clone(), room);

        info!("Created room: {} ({})", room_id, name_clone);
        Ok(room_id)
    }

    /// Get room by ID
    pub fn get_room(&self, room_id: &str) -> Option<&Room> {
        self.rooms.get(room_id)
    }

    /// Get room by ID (mutable)
    pub fn get_room_mut(&mut self, room_id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(room_id)
    }

    /// List rooms with optional filter
    pub fn list_rooms(&self, filter: Option<&RoomListFilter>) -> Vec<RoomInfo> {
        self.rooms
            .values()
            .filter(|room| {
                if let Some(f) = filter {
                    f.matches(room)
                } else {
                    true
                }
            })
            .map(|room| room.get_room_info())
            .collect()
    }

    /// Join room as player
    pub fn join_room(&mut self, room_id: &str, player_id: String, player_name: String) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        let player_id_clone = player_id.to_string();
        room.add_player(player_id, player_name)?;
        info!("Player {} joined room {}", player_id_clone, room_id);
        Ok(())
    }

    /// Join room as spectator
    pub fn join_room_as_spectator(&mut self, room_id: &str, spectator_id: String, spectator_name: String) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        let spectator_id_clone = spectator_id.clone();
        room.add_spectator(spectator_id, spectator_name)?;
        info!("Spectator {} joined room {}", spectator_id_clone, room_id);
        Ok(())
    }

    /// Leave room as player
    pub fn leave_room(&mut self, room_id: &str, player_id: &str) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        let player_id_clone = player_id.to_string();
        room.remove_player(player_id)?;
        info!("Player {} left room {}", player_id_clone, room_id);
        Ok(())
    }

    /// Leave room as spectator
    pub fn leave_room_as_spectator(&mut self, room_id: &str, spectator_id: &str) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        let spectator_id_clone = spectator_id.to_string();
        room.remove_spectator(spectator_id)?;
        info!("Spectator {} left room {}", spectator_id_clone, room_id);
        Ok(())
    }

    /// Start game
    pub fn start_game(&mut self, room_id: &str, player_id: &str) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        let room_id_clone = room_id.to_string();
        room.start_game()?;
        room.state = RoomState::Playing; // Override to Playing after starting
        info!("Game started in room {}", room_id_clone);
        Ok(())
    }

    /// End game
    pub fn end_game(&mut self, room_id: &str) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        let room_id_clone = room_id.to_string();
        room.end_game()?;
        info!("Game ended in room {}", room_id_clone);
        Ok(())
    }

    /// Set player ready status
    pub fn set_player_ready(&mut self, room_id: &str, player_id: &str, ready: bool) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        room.set_player_ready(player_id, ready)
    }

    /// Update player ping
    pub fn update_player_ping(&mut self, room_id: &str, player_id: &str, ping: u32) -> Result<(), RoomError> {
        let room = self.get_room_mut(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        room.update_player_ping(player_id, ping)
    }

    /// Get room info
    pub fn get_room_info(&self, room_id: &str) -> Result<RoomInfo, RoomError> {
        let room = self.get_room(room_id)
            .ok_or(RoomError::RoomNotFound)?;

        Ok(room.get_room_info())
    }

    /// Cleanup empty and old rooms
    pub fn cleanup(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Don't cleanup too frequently
        if now - self.last_cleanup < self.cleanup_interval.as_secs() {
            return;
        }

        let mut rooms_to_remove = Vec::new();

        for (room_id, room) in &self.rooms {
            // Remove empty rooms after 5 minutes
            if room.is_empty() && room.age_seconds() > 300 {
                rooms_to_remove.push(room_id.clone());
            }

            // Remove finished rooms after 10 minutes
            if room.state == RoomState::Finished && room.age_seconds() > 600 {
                rooms_to_remove.push(room_id.clone());
            }

            // Remove closed rooms immediately
            if room.state == RoomState::Closed {
                rooms_to_remove.push(room_id.clone());
            }
        }

        for room_id in rooms_to_remove {
            if let Some(room) = self.rooms.remove(&room_id) {
                info!("Cleaned up room: {} ({})", room_id, room.name);
            }
        }

        self.last_cleanup = now;
    }

    /// Get total room count
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Get active player count across all rooms
    pub fn total_players(&self) -> usize {
        self.rooms.values().map(|room| room.players.len()).sum()
    }

    /// Get active spectator count across all rooms
    pub fn total_spectators(&self) -> usize {
        self.rooms.values().map(|room| room.spectators.len()).sum()
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new(1000) // Default max 1000 rooms
    }
}

/// Room management operations result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomOperationResult {
    pub success: bool,
    pub room_id: Option<String>,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}
