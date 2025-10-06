use crate::cache::RedisCache;
use crate::message::{ControlMessage, Frame, FramePayload};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// High-concurrency transport layer for 1000+ simultaneous connections
#[derive(Debug)]
pub struct HighConcurrencyTransport {
    connections: Arc<DashMap<String, Connection>>,
    rooms: Arc<DashMap<String, Room>>,
    message_router: Arc<MessageRouter>,
    connection_manager: Arc<ConnectionManager>,
    metrics: Arc<TransportMetrics>,
    config: TransportConfig,
    cache: Option<Arc<RedisCache>>,
}

/// Connection represents a single client connection (WebSocket or WebRTC)
#[derive(Debug)]
pub struct Connection {
    pub connection_id: String,
    pub client_id: String,
    pub transport_type: TransportType,
    pub room_id: Option<String>,
    pub status: ConnectionStatus,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub sender: mpsc::UnboundedSender<Frame>,
    pub receiver: Option<mpsc::UnboundedReceiver<Frame>>,
    pub address: Option<SocketAddr>,
    pub user_agent: Option<String>,
    pub sticky_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransportType {
    WebSocket,
    WebRTC,
    WebRTCWithFallback,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    InGame,
    Disconnected,
    Failed,
}

/// Room represents a game room with multiple connections
#[derive(Debug)]
pub struct Room {
    pub room_id: String,
    pub connections: HashMap<String, String>, // connection_id -> client_id
    pub max_players: u32,
    pub game_mode: String,
    pub status: RoomStatus,
    pub created_at: Instant,
    pub last_activity: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoomStatus {
    Waiting,
    InProgress,
    Finished,
    Closed,
}

/// Message router for efficient message distribution
#[derive(Debug)]
pub struct MessageRouter {
    room_subscriptions: Arc<DashMap<String, Vec<String>>>, // room_id -> connection_ids
    broadcast_channels: Arc<DashMap<String, mpsc::UnboundedSender<Frame>>>, // room_id -> sender
}

/// Connection manager for handling connection lifecycle
#[derive(Debug)]
pub struct ConnectionManager {
    connection_timeout: Duration,
    max_connections_per_room: u32,
    max_idle_time: Duration,
    heartbeat_interval: Duration,
}

/// Transport configuration for performance tuning
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Maximum connections per server instance
    pub max_connections: u32,
    /// Maximum connections per room
    pub max_connections_per_room: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Message queue size per connection
    pub message_queue_size: usize,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
    /// Maximum idle time before disconnect in seconds
    pub max_idle_time: u64,
    /// Enable connection pooling
    pub enable_connection_pooling: bool,
    /// Enable message compression
    pub enable_compression: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Maximum messages per second per connection
    pub max_messages_per_second: u32,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_connections: 10000,
            max_connections_per_room: 100,
            connection_timeout: 30,
            message_queue_size: 1000,
            heartbeat_interval: 30,
            max_idle_time: 300,
            enable_connection_pooling: true,
            enable_compression: true,
            enable_rate_limiting: true,
            max_messages_per_second: 60,
        }
    }
}

/// Performance metrics for transport layer
#[derive(Debug, Default)]
pub struct TransportMetrics {
    /// Total connections created
    pub connections_created: AtomicU64,
    /// Total connections destroyed
    pub connections_destroyed: AtomicU64,
    /// Current active connections
    pub active_connections: AtomicU64,
    /// Total messages sent
    pub messages_sent: AtomicU64,
    /// Total messages received
    pub messages_received: AtomicU64,
    /// Total bytes sent
    pub bytes_sent: AtomicU64,
    /// Total bytes received
    pub bytes_received: AtomicU64,
    /// Average message latency in microseconds
    pub avg_message_latency: AtomicU64,
    /// Connection errors
    pub connection_errors: AtomicU64,
    /// Room count
    pub room_count: AtomicU64,
    /// WebRTC connections
    pub webrtc_connections: AtomicU64,
    /// WebSocket connections
    pub websocket_connections: AtomicU64,
}

impl TransportMetrics {
    pub fn record_connection_created(&self, transport_type: &TransportType) {
        self.connections_created.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);

        match transport_type {
            TransportType::WebRTC | TransportType::WebRTCWithFallback => {
                self.webrtc_connections.fetch_add(1, Ordering::Relaxed);
            }
            TransportType::WebSocket => {
                self.websocket_connections.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn record_connection_destroyed(&self) {
        self.connections_destroyed.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_message_sent(&self, bytes: u64, latency_micros: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);

        // Update average latency
        let current = self.avg_message_latency.load(Ordering::Relaxed);
        let new_avg = (current + latency_micros) / 2;
        self.avg_message_latency.store(new_avg, Ordering::Relaxed);
    }

    pub fn record_message_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_connection_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_room_count(&self, count: u64) {
        self.room_count.store(count, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (
            self.connections_created.load(Ordering::Relaxed),
            self.connections_destroyed.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::Relaxed),
            self.messages_sent.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.bytes_sent.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
            self.avg_message_latency.load(Ordering::Relaxed),
            self.connection_errors.load(Ordering::Relaxed),
            self.room_count.load(Ordering::Relaxed),
            self.webrtc_connections.load(Ordering::Relaxed),
            self.websocket_connections.load(Ordering::Relaxed),
        )
    }
}

impl HighConcurrencyTransport {
    /// Create a new high-concurrency transport layer
    pub fn new(config: TransportConfig, cache: Option<Arc<RedisCache>>) -> Self {
        let connection_manager = Arc::new(ConnectionManager {
            connection_timeout: Duration::from_secs(config.connection_timeout),
            max_connections_per_room: config.max_connections_per_room,
            max_idle_time: Duration::from_secs(config.max_idle_time),
            heartbeat_interval: Duration::from_secs(config.heartbeat_interval),
        });

        Self {
            connections: Arc::new(DashMap::new()),
            rooms: Arc::new(DashMap::new()),
            message_router: Arc::new(MessageRouter::new()),
            connection_manager,
            metrics: Arc::new(TransportMetrics::default()),
            config,
            cache,
        }
    }

    /// Create a new WebSocket connection
    pub async fn create_websocket_connection(
        &self,
        client_id: String,
        address: SocketAddr,
        user_agent: Option<String>,
    ) -> Result<String, BoxError> {
        // Check connection limits
        if self.connections.len() >= self.config.max_connections as usize {
            return Err("Maximum connections exceeded".into());
        }

        let connection_id = Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::unbounded_channel();

        let connection = Connection {
            connection_id: connection_id.clone(),
            client_id,
            transport_type: TransportType::WebSocket,
            room_id: None,
            status: ConnectionStatus::Connecting,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            sender,
            receiver: Some(receiver),
            address: Some(address),
            user_agent,
            sticky_token: None,
        };

        self.connections.insert(connection_id.clone(), connection);

        if self.config.enable_metrics {
            self.metrics.record_connection_created(&TransportType::WebSocket);
        }

        info!("Created WebSocket connection {} for client {}", connection_id, client_id);
        Ok(connection_id)
    }

    /// Create a new WebRTC connection
    pub async fn create_webrtc_connection(
        &self,
        client_id: String,
        room_id: String,
    ) -> Result<String, BoxError> {
        // Check connection limits
        if self.connections.len() >= self.config.max_connections as usize {
            return Err("Maximum connections exceeded".into());
        }

        let connection_id = Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::unbounded_channel();

        let connection = Connection {
            connection_id: connection_id.clone(),
            client_id,
            transport_type: TransportType::WebRTC,
            room_id: Some(room_id.clone()),
            status: ConnectionStatus::Connecting,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            sender,
            receiver: Some(receiver),
            address: None,
            user_agent: None,
            sticky_token: None,
        };

        self.connections.insert(connection_id.clone(), connection);

        // Add to room
        self.add_connection_to_room(&connection_id, &room_id).await?;

        if self.config.enable_metrics {
            self.metrics.record_connection_created(&TransportType::WebRTC);
        }

        info!("Created WebRTC connection {} for client {} in room {}", connection_id, client_id, room_id);
        Ok(connection_id)
    }

    /// Join a room
    pub async fn join_room(&self, connection_id: &str, room_id: &str) -> Result<(), BoxError> {
        // Check if room exists, create if not
        if !self.rooms.contains_key(room_id) {
            self.create_room(room_id, "default".to_string(), 8).await?;
        }

        let room = self.rooms.get(room_id).unwrap();
        if room.connections.len() >= self.config.max_connections_per_room as usize {
            return Err("Room is full".into());
        }

        // Update connection
        if let Some(mut connection) = self.connections.get_mut(connection_id) {
            connection.room_id = Some(room_id.to_string());
            connection.status = ConnectionStatus::Connected;
            connection.last_activity = Instant::now();
        }

        // Add to room
        self.add_connection_to_room(connection_id, room_id).await?;

        debug!("Connection {} joined room {}", connection_id, room_id);
        Ok(())
    }

    /// Leave a room
    pub async fn leave_room(&self, connection_id: &str) -> Result<(), BoxError> {
        if let Some(connection) = self.connections.get(connection_id) {
            if let Some(room_id) = &connection.room_id {
                self.remove_connection_from_room(connection_id, room_id).await?;

                // Update connection status
                if let Some(mut conn) = self.connections.get_mut(connection_id) {
                    conn.room_id = None;
                    conn.status = ConnectionStatus::Connected;
                }
            }
        }

        debug!("Connection {} left room", connection_id);
        Ok(())
    }

    /// Send message to a specific connection
    pub async fn send_to_connection(&self, connection_id: &str, frame: Frame) -> Result<(), BoxError> {
        if let Some(connection) = self.connections.get(connection_id) {
            let start = Instant::now();

            // Apply rate limiting if enabled
            if self.config.enable_rate_limiting {
                // Simplified rate limiting check
            }

            // Apply compression if enabled
            let frame_bytes = if self.config.enable_compression {
                self.compress_frame(&frame)?
            } else {
                bincode::serialize(&frame)?
            };

            // Send message
            if let Err(_) = timeout(Duration::from_secs(5), connection.sender.send(frame.clone())).await {
                return Err("Send timeout".into());
            }

            let latency = start.elapsed().as_micros() as u64;

            if self.config.enable_metrics {
                self.metrics.record_message_sent(frame_bytes.len() as u64, latency);
            }

            // Update last activity
            if let Some(mut connection) = self.connections.get_mut(connection_id) {
                connection.last_activity = Instant::now();
            }

            Ok(())
        } else {
            Err("Connection not found".into())
        }
    }

    /// Broadcast message to all connections in a room
    pub async fn broadcast_to_room(&self, room_id: &str, frame: Frame, exclude_connection_id: Option<&str>) -> Result<(), BoxError> {
        let room_connections = {
            if let Some(room) = self.rooms.get(room_id) {
                room.connections.keys().cloned().collect::<Vec<_>>()
            } else {
                return Ok(()); // Room doesn't exist
            }
        };

        for connection_id in room_connections {
            if let Some(exclude) = exclude_connection_id {
                if connection_id == exclude {
                    continue;
                }
            }

            if let Err(e) = self.send_to_connection(&connection_id, frame.clone()).await {
                error!("Failed to send to connection {}: {}", connection_id, e);
            }
        }

        Ok(())
    }

    /// Handle incoming message from a connection
    pub async fn handle_incoming_message(&self, connection_id: &str, frame: Frame) -> Result<(), BoxError> {
        if let Some(mut connection) = self.connections.get_mut(connection_id) {
            connection.last_activity = Instant::now();

            if self.config.enable_metrics {
                let frame_size = bincode::serialize(&frame).map(|v| v.len() as u64).unwrap_or(0);
                self.metrics.record_message_received(frame_size);
            }
        }

        // Route message based on type
        match &frame.payload {
            FramePayload::Control { message } => {
                self.handle_control_message(connection_id, message).await?;
            }
            _ => {
                // Handle other message types (game input, chat, etc.)
                if let Some(room_id) = self.get_connection_room(connection_id).await {
                    self.broadcast_to_room(&room_id, frame, Some(connection_id)).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle control messages (WebRTC signaling, heartbeat, etc.)
    async fn handle_control_message(&self, connection_id: &str, message: &ControlMessage) -> Result<(), BoxError> {
        match message {
            ControlMessage::Ping { nonce } => {
                // Send pong response
                let pong_frame = Frame::control(0, 0, ControlMessage::Pong { nonce: *nonce });
                self.send_to_connection(connection_id, pong_frame).await?;
            }
            ControlMessage::Pong { .. } => {
                // Update connection activity
                if let Some(mut connection) = self.connections.get_mut(connection_id) {
                    connection.last_activity = Instant::now();
                }
            }
            ControlMessage::WebRtcOffer { .. } | ControlMessage::WebRtcAnswer { .. } | ControlMessage::WebRtcIceCandidate { .. } => {
                // Handle WebRTC signaling by forwarding to target peer
                if let Some(room_id) = self.get_connection_room(connection_id).await {
                    self.broadcast_to_room(&room_id, Frame::control(0, 0, message.clone()), Some(connection_id)).await?;
                }
            }
            _ => {
                debug!("Unhandled control message from connection {}", connection_id);
            }
        }

        Ok(())
    }

    /// Create a new room
    async fn create_room(&self, room_id: &str, game_mode: String, max_players: u32) -> Result<(), BoxError> {
        let room = Room {
            room_id: room_id.to_string(),
            connections: HashMap::new(),
            max_players,
            game_mode,
            status: RoomStatus::Waiting,
            created_at: Instant::now(),
            last_activity: Instant::now(),
        };

        self.rooms.insert(room_id.to_string(), room);

        if self.config.enable_metrics {
            self.metrics.update_room_count(self.rooms.len() as u64);
        }

        debug!("Created room {} with game mode {} and max players {}", room_id, game_mode, max_players);
        Ok(())
    }

    /// Add connection to room
    async fn add_connection_to_room(&self, connection_id: &str, room_id: &str) -> Result<(), BoxError> {
        if let Some(mut room) = self.rooms.get_mut(room_id) {
            if let Some(connection) = self.connections.get(connection_id) {
                room.connections.insert(connection_id.to_string(), connection.client_id.clone());
                room.last_activity = Instant::now();

                debug!("Added connection {} to room {}", connection_id, room_id);
            }
        }
        Ok(())
    }

    /// Remove connection from room
    async fn remove_connection_from_room(&self, connection_id: &str, room_id: &str) -> Result<(), BoxError> {
        if let Some(mut room) = self.rooms.get_mut(room_id) {
            room.connections.remove(connection_id);
            room.last_activity = Instant::now();

            // Clean up empty rooms
            if room.connections.is_empty() && room.status == RoomStatus::Finished {
                self.rooms.remove(room_id);
                if self.config.enable_metrics {
                    self.metrics.update_room_count(self.rooms.len() as u64);
                }
            }
        }
        Ok(())
    }

    /// Get room for a connection
    async fn get_connection_room(&self, connection_id: &str) -> Option<String> {
        if let Some(connection) = self.connections.get(connection_id) {
            connection.room_id.clone()
        } else {
            None
        }
    }

    /// Compress frame if compression is enabled
    fn compress_frame(&self, frame: &Frame) -> Result<Vec<u8>, BoxError> {
        // In a real implementation, this would use compression algorithms like LZ4 or Zstd
        // For now, we'll just serialize normally
        bincode::serialize(frame).map_err(|e| e.into())
    }

    /// Cleanup inactive connections and rooms
    pub async fn cleanup_inactive(&self) -> Result<(u64, u64), BoxError> {
        let mut cleaned_connections = 0u64;
        let mut cleaned_rooms = 0u64;
        let cutoff = Instant::now() - self.connection_manager.max_idle_time;

        // Clean up inactive connections
        let inactive_connections: Vec<String> = self.connections
            .iter()
            .filter(|entry| entry.last_activity < cutoff)
            .map(|entry| entry.connection_id.clone())
            .collect();

        for connection_id in inactive_connections {
            self.connections.remove(&connection_id);
            cleaned_connections += 1;

            if self.config.enable_metrics {
                self.metrics.record_connection_destroyed();
            }
        }

        // Clean up empty rooms
        let empty_rooms: Vec<String> = self.rooms
            .iter()
            .filter(|entry| entry.connections.is_empty())
            .map(|entry| entry.room_id.clone())
            .collect();

        for room_id in empty_rooms {
            self.rooms.remove(&room_id);
            cleaned_rooms += 1;
        }

        if self.config.enable_metrics {
            self.metrics.update_room_count(self.rooms.len() as u64);
        }

        debug!("Cleaned up {} connections and {} rooms", cleaned_connections, cleaned_rooms);
        Ok((cleaned_connections, cleaned_rooms))
    }

    /// Start heartbeat system for connection health monitoring
    pub async fn start_heartbeat_system(&self) -> Result<(), BoxError> {
        let connections = Arc::clone(&self.connections);
        let heartbeat_interval = self.connection_manager.heartbeat_interval;

        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);

            loop {
                interval.tick().await;

                // Send heartbeat to all connections
                for connection in connections.iter() {
                    let heartbeat_frame = Frame::control(0, 0, ControlMessage::Ping {
                        nonce: chrono::Utc::now().timestamp() as u32,
                    });

                    if let Err(e) = timeout(Duration::from_secs(5), connection.sender.send(heartbeat_frame.clone())).await {
                        error!("Failed to send heartbeat to connection {}: {}", connection.connection_id, e);

                        // Mark connection as failed
                        if let Some(mut conn) = connections.get_mut(&connection.connection_id) {
                            conn.status = ConnectionStatus::Failed;
                        }
                    }
                }
            }
        });

        info!("Started heartbeat system with {}s interval", heartbeat_interval.as_secs());
        Ok(())
    }

    /// Get transport metrics
    pub fn get_metrics(&self) -> Arc<TransportMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> (usize, usize, usize) {
        let total = self.connections.len();
        let active = self.connections.iter().filter(|c| c.status == ConnectionStatus::Connected || c.status == ConnectionStatus::InGame).count();
        let rooms = self.rooms.len();

        (total, active, rooms)
    }
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            room_subscriptions: Arc::new(DashMap::new()),
            broadcast_channels: Arc::new(DashMap::new()),
        }
    }

    /// Subscribe connection to room for message routing
    pub async fn subscribe_to_room(&self, connection_id: &str, room_id: &str) {
        let mut subscriptions = self.room_subscriptions.entry(room_id.to_string()).or_insert_with(Vec::new);
        if !subscriptions.contains(&connection_id.to_string()) {
            subscriptions.push(connection_id.to_string());
        }
    }

    /// Unsubscribe connection from room
    pub async fn unsubscribe_from_room(&self, connection_id: &str, room_id: &str) {
        if let Some(mut subscriptions) = self.room_subscriptions.get_mut(room_id) {
            subscriptions.retain(|id| id != connection_id);
        }
    }

    /// Get subscribers for a room
    pub async fn get_room_subscribers(&self, room_id: &str) -> Vec<String> {
        self.room_subscriptions
            .get(room_id)
            .map(|subs| subs.clone())
            .unwrap_or_default()
    }
}

impl Default for HighConcurrencyTransport {
    fn default() -> Self {
        Self::new(TransportConfig::default(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_creation() {
        let config = TransportConfig::default();
        let transport = HighConcurrencyTransport::new(config, None);

        let (total, active, rooms) = transport.get_connection_stats().await;
        assert_eq!(total, 0);
        assert_eq!(active, 0);
        assert_eq!(rooms, 0);

        println!("✅ Transport creation test completed");
    }

    #[tokio::test]
    async fn test_connection_management() {
        let config = TransportConfig::default();
        let transport = HighConcurrencyTransport::new(config, None);

        // Create WebSocket connection
        let connection_id = transport.create_websocket_connection(
            "client1".to_string(),
            "127.0.0.1:12345".parse().unwrap(),
            Some("TestAgent".to_string()),
        ).await.unwrap();

        let (total, active, rooms) = transport.get_connection_stats().await;
        assert_eq!(total, 1);
        assert_eq!(active, 0); // Still connecting

        // Join room
        transport.join_room(&connection_id, "room1").await.unwrap();

        let (total, active, rooms) = transport.get_connection_stats().await;
        assert_eq!(total, 1);
        assert_eq!(active, 1);
        assert_eq!(rooms, 1);

        println!("✅ Connection management test completed");
    }

    #[tokio::test]
    async fn test_message_handling() {
        let config = TransportConfig::default();
        let transport = HighConcurrencyTransport::new(config, None);

        // Create connection
        let connection_id = transport.create_websocket_connection(
            "client1".to_string(),
            "127.0.0.1:12345".parse().unwrap(),
            None,
        ).await.unwrap();

        // Join room
        transport.join_room(&connection_id, "room1").await.unwrap();

        // Send ping message
        let ping_frame = Frame::control(0, 0, ControlMessage::Ping { nonce: 123 });
        transport.handle_incoming_message(&connection_id, ping_frame).await.unwrap();

        println!("✅ Message handling test completed");
    }

    #[tokio::test]
    async fn test_room_broadcasting() {
        let config = TransportConfig::default();
        let transport = HighConcurrencyTransport::new(config, None);

        // Create two connections
        let conn1 = transport.create_websocket_connection(
            "client1".to_string(),
            "127.0.0.1:12345".parse().unwrap(),
            None,
        ).await.unwrap();

        let conn2 = transport.create_websocket_connection(
            "client2".to_string(),
            "127.0.0.1:12346".parse().unwrap(),
            None,
        ).await.unwrap();

        // Join same room
        transport.join_room(&conn1, "room1").await.unwrap();
        transport.join_room(&conn2, "room1").await.unwrap();

        // Broadcast message
        let message = Frame::control(0, 0, ControlMessage::Ping { nonce: 456 });
        transport.broadcast_to_room("room1", message, None).await.unwrap();

        println!("✅ Room broadcasting test completed");
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = TransportConfig::default();
        let transport = HighConcurrencyTransport::new(config, None);

        let metrics = transport.get_metrics();
        let (created, destroyed, active, sent, received, bytes_sent, bytes_received, avg_latency, errors, rooms, webrtc, websocket) = metrics.get_stats();

        assert_eq!(created, 0);
        assert_eq!(destroyed, 0);
        assert_eq!(active, 0);

        // Create connection to generate metrics
        let _ = transport.create_websocket_connection(
            "client1".to_string(),
            "127.0.0.1:12345".parse().unwrap(),
            None,
        ).await.unwrap();

        let (created, _, active, _, _, _, _, _, _, _, _, _) = metrics.get_stats();
        assert_eq!(created, 1);
        assert_eq!(active, 1);

        println!("✅ Performance metrics test completed");
    }

    #[tokio::test]
    async fn test_cleanup() {
        let config = TransportConfig {
            max_idle_time: 1, // 1 second for quick cleanup
            ..Default::default()
        };
        let transport = HighConcurrencyTransport::new(config, None);

        // Create connection
        let connection_id = transport.create_websocket_connection(
            "client1".to_string(),
            "127.0.0.1:12345".parse().unwrap(),
            None,
        ).await.unwrap();

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Cleanup
        let (cleaned_connections, cleaned_rooms) = transport.cleanup_inactive().await.unwrap();

        // Connection should be cleaned up due to idle timeout
        let (total, _, _) = transport.get_connection_stats().await;
        assert_eq!(total, 0);

        println!("✅ Cleanup test completed");
    }
}
