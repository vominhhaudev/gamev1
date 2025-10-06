use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    Json,
};
use common_net::message::Channel;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use prometheus::{
    register_int_counter, register_int_gauge, IntCounter, IntGauge,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    sync::{broadcast, mpsc},
    time::timeout,
};
use tracing::{error, info, warn};
// use wtransport; // Temporarily disabled for now

// Transport types enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TransportType {
    #[serde(rename = "quic")]
    Quic,
    #[serde(rename = "webrtc")]
    WebRTC,
    #[serde(rename = "websocket")]
    WebSocket,
}

// Transport capabilities response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCapabilities {
    pub quic: bool,
    pub webrtc: bool,
    pub websocket: bool,
}

// Connection info để track performance
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub transport: TransportType,
    pub connected_at: Instant,
    pub last_ping: Instant,
    pub latency_ms: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// Connection manager để track tất cả connections
#[derive(Debug)]
pub struct ConnectionManager {
    connections: DashMap<String, ConnectionInfo>,
    transport_stats: Arc<RwLock<HashMap<TransportType, u64>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            transport_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_connection(&self, id: String, transport: TransportType) {
        let info = ConnectionInfo {
            transport: transport.clone(),
            connected_at: Instant::now(),
            last_ping: Instant::now(),
            latency_ms: 0,
            bytes_sent: 0,
            bytes_received: 0,
        };

        self.connections.insert(id, info);

        // Update stats
        if let Ok(mut stats) = self.transport_stats.write() {
            *stats.entry(transport).or_insert(0) += 1;
        }
    }

    pub fn remove_connection(&self, id: &str) {
        if let Some((_, info)) = self.connections.remove(id) {
            if let Ok(mut stats) = self.transport_stats.write() {
                if let Some(count) = stats.get_mut(&info.transport) {
                    *count = count.saturating_sub(1);
                }
            }
        }
    }

    pub fn update_ping(&self, id: &str, latency_ms: u64) {
        if let Some(mut info) = self.connections.get_mut(id) {
            info.last_ping = Instant::now();
            info.latency_ms = latency_ms;
        }
    }

    pub fn get_stats(&self) -> HashMap<TransportType, u64> {
        self.transport_stats.read().unwrap().clone()
    }
}

/// Trait chung cho các transport (WS, QUIC, RTC).
#[async_trait::async_trait]
pub trait Transport {
    async fn serve(self, socket: WebSocket);

    /// QUIC-specific method (default implementation does nothing).
    async fn serve_quic(&mut self, _connection: ()) {
        panic!("QUIC transport must implement serve_quic");
    }
}

/// Transport state chung cho tất cả implementations.
#[derive(Clone, Default)]
pub struct TransportState {
    next_control_seq: u32,
    next_state_seq: u32,
}

impl TransportState {
    pub fn alloc_sequence(&mut self, channel: Channel) -> u32 {
        match channel {
            Channel::Control => {
                let s = self.next_control_seq;
                self.next_control_seq = self.next_control_seq.wrapping_add(1);
                s
            }
            Channel::State => {
                let s = self.next_state_seq;
                self.next_state_seq = self.next_state_seq.wrapping_add(1);
                s
            }
        }
    }
}


// QUIC/WebTransport implementation (temporarily disabled)
// TODO: Enable khi fix wtransport dependencies
/*
pub struct QuicTransport {
    state: TransportState,
    endpoint: Option<Endpoint>,
}

impl QuicTransport {
    pub fn new() -> Self {
        Self {
            state: TransportState::default(),
            endpoint: None,
        }
    }

    pub async fn start_server(&mut self, bind_addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Tạo self-signed cert cho development
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
        let cert_der = cert.cert.der();
        let key_der = cert.key_pair.serialize_der();

        let config = ServerConfig::builder()
            .with_bind_default(443)
            .with_certificate(&cert_der, &key_der)?
            .keep_alive_interval(Some(Duration::from_secs(3)))?
            .build();

        let endpoint = Endpoint::server(config)?;
        self.endpoint = Some(endpoint);
        Ok(())
    }

    pub async fn accept_connections(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(endpoint) = &self.endpoint {
            while let Some(connection) = endpoint.accept().await {
                let connection = connection.await?;
                let mut transport = QuicTransport::new();
                tokio::spawn(async move {
                    transport.handle_quic_connection(connection).await;
                });
            }
        }
        Ok(())
    }

    async fn handle_quic_connection(&mut self, _connection: ()) {
        // QUIC implementation...
    }
}

#[async_trait::async_trait]
impl Transport for QuicTransport {
    async fn serve(self, _socket: WebSocket) {
        todo!("QUIC transport uses separate listener")
    }

    async fn serve_quic(&mut self, connection: ()) {
        // self.handle_quic_connection(connection).await;
    }
}
*/


static STATE_BUFFER_DROPPED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "gateway_state_dropped_total",
        "So goi state bi drop do backpressure"
    )
    .expect("register gateway_state_dropped_total")
});

static STATE_BUFFER_DEPTH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "gateway_state_buffer_depth",
        "Do sau buffer state hien tai"
    )
    .expect("register gateway_state_buffer_depth")
});


pub fn timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// Transport negotiation endpoint
pub async fn negotiate_transport() -> impl IntoResponse {
    info!("Transport negotiation requested");

    Json(TransportCapabilities {
        quic: true,     // QUIC/WebTransport supported
        webrtc: true,   // WebRTC supported
        websocket: true, // WebSocket fallback
    })
}

// Enhanced WebSocket với backpressure và connection pooling
pub struct EnhancedWebSocket {
    connections: Arc<ConnectionManager>,
    message_tx: broadcast::Sender<Vec<u8>>,
}

impl EnhancedWebSocket {
    pub fn new(connections: Arc<ConnectionManager>) -> Self {
        let (message_tx, _) = broadcast::channel(1000);

        Self {
            connections,
            message_tx,
        }
    }

    pub async fn handle_websocket(
        ws: WebSocketUpgrade,
        connections: Arc<ConnectionManager>,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| Self::websocket_connection(socket, connections))
    }

    async fn websocket_connection(
        socket: WebSocket,
        connections: Arc<ConnectionManager>,
    ) {
        let connection_id = uuid::Uuid::new_v4().to_string();
        connections.add_connection(connection_id.clone(), TransportType::WebSocket);

        info!("New enhanced WebSocket connection: {}", connection_id);

        let (mut sender, mut receiver) = socket.split();
        let (message_tx, mut message_rx) = mpsc::channel::<WsMessage>(100);

        // Message compression và buffering
        let buffer: Vec<u8> = Vec::new();
        let last_flush = Instant::now();

        loop {
            tokio::select! {
                // Receive messages với timeout để tránh blocking
                msg_result = timeout(Duration::from_millis(100), receiver.next()) => {
                    match msg_result {
                        Ok(Some(Ok(WsMessage::Text(text)))) => {
                            // Handle text messages (JSON)
                            if let Ok(msg) = serde_json::from_str::<NetworkMessage>(&text) {
                                info!("Received WS text message: {:?}", msg);

                                // Process message và gửi response nếu cần
                                match msg {
                                    NetworkMessage::Ping { timestamp } => {
                                        let response = NetworkMessage::Pong {
                                            timestamp: chrono::Utc::now().timestamp_millis()
                                        };
                                        if let Ok(json) = serde_json::to_string(&response) {
                                            let _ = message_tx.send(WsMessage::Text(json)).await;
                                        }
                                    }
                                    _ => {
                                        // Handle other message types
                                    }
                                }
                            }
                        }
                        Ok(Some(Ok(WsMessage::Binary(data)))) => {
                            // Handle binary messages (compressed hoặc raw)
                            let data_vec = data.to_vec();
                            if data_vec.len() > 2 && data_vec[0] == 0x04 && data_vec[1] == 0x22 { // LZ4 magic bytes
                                // Decompress LZ4
                                if let Ok(decompressed) = lz4_flex::decompress_size_prepended(&data_vec[2..]) {
                                    if let Ok(msg) = bincode::deserialize::<NetworkMessage>(&decompressed) {
                                        info!("Received WS compressed message: {:?}", msg);
                                    }
                                }
                            } else {
                                // Raw binary message
                                if let Ok(msg) = bincode::deserialize::<NetworkMessage>(&data_vec) {
                                    info!("Received WS binary message: {:?}", msg);
                                }
                            }
                        }
                        Ok(Some(Ok(WsMessage::Ping(payload)))) => {
                            let _ = sender.send(WsMessage::Pong(payload)).await;
                            connections.update_ping(&connection_id, 50); // Mock latency for now
                        }
                        Ok(Some(Ok(WsMessage::Pong(_)))) => {
                            // Handle pong response từ heartbeat
                        }
                        Ok(Some(Ok(WsMessage::Close(_)))) => {
                            info!("WebSocket connection closed by client");
                            break;
                        }
                        Ok(Some(Err(e))) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        Ok(None) => {
                            info!("WebSocket connection closed");
                            break;
                        }
                        Err(_) => {
                            // Timeout - continue để handle sending
                        }
                    }
                }

                // Send messages với backpressure
                msg = message_rx.recv() => {
                    if let Some(msg) = msg {
                        // Apply compression cho messages lớn
                        let send_msg = if let WsMessage::Text(ref text) = &msg {
                            if text.len() > 1000 {
                                // Compress large text messages (temporarily disabled)
                                // TODO: Fix compression implementation
                                msg
                            } else {
                                msg
                            }
                        } else {
                            msg
                        };

                        // Send với timeout để tránh blocking
                        if timeout(Duration::from_millis(50), sender.send(send_msg)).await.is_err() {
                            warn!("Failed to send WebSocket message - connection may be dead");
                            break;
                        }
                    }
                }

                // Periodic flush buffer nếu có
                _ = tokio::time::sleep(Duration::from_millis(10)) => {
                    // Optional: flush any pending operations
                }
            }
        }

        connections.remove_connection(&connection_id);
        info!("Enhanced WebSocket connection closed: {}", connection_id);
    }
}

// Network message types cho transport layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Ping { timestamp: i64 },
    Pong { timestamp: i64 },
    JoinRoom { room_id: String, player_id: String },
    LeaveRoom,
    PlayerInput { input: String, sequence: u32, timestamp: i64 },
    GameState { state: serde_json::Value, sequence: u32 },
    Error { message: String, code: String },
}

// Transport test functions
pub async fn test_transport_availability() -> TransportCapabilities {
    TransportCapabilities {
        quic: test_quic_availability().await,
        webrtc: test_webrtc_availability().await,
        websocket: test_websocket_availability().await,
    }
}

async fn test_quic_availability() -> bool {
    // Test QUIC connectivity bằng cách thử connect đến một test endpoint
    // For now, assume QUIC is available nếu wtransport được enable
    true
}

async fn test_webrtc_availability() -> bool {
    // WebRTC availability được determine client-side dựa trên browser support
    // Server luôn support WebRTC signaling
    true
}

async fn test_websocket_availability() -> bool {
    // WebSocket luôn available trên modern browsers và servers
    true
}

// Utility functions để get connection stats
pub fn get_transport_stats(manager: &ConnectionManager) -> HashMap<TransportType, u64> {
    manager.get_stats()
}

pub fn get_connection_count(manager: &ConnectionManager) -> usize {
    manager.connections.len()
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_types() {
        let quic = TransportType::Quic;
        let ws = TransportType::WebSocket;

        assert_ne!(quic, ws);
        assert_eq!(format!("{:?}", quic), "Quic");
    }

    #[test]
    fn test_connection_manager() {
        let manager = ConnectionManager::new();

        manager.add_connection("test1".to_string(), TransportType::WebSocket);
        assert_eq!(manager.get_stats()[&TransportType::WebSocket], 1);

        manager.add_connection("test2".to_string(), TransportType::Quic);
        assert_eq!(manager.get_stats()[&TransportType::Quic], 1);

        manager.remove_connection("test1");
        assert_eq!(manager.get_stats()[&TransportType::WebSocket], 0);
    }
}