use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};
use crate::{message::Frame, compression::{CompressionConfig, CompressedData}};
use super::{TransportError, TransportType, MessageType, TransportMessage, ConnectionState, TransportEvent};

/// Enhanced transport trait với support cho multiple channels và monitoring
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get transport type
    fn transport_type(&self) -> TransportType;

    /// Get unique transport ID
    fn id(&self) -> &str;

    /// Check if transport is connected and ready
    fn is_connected(&self) -> bool;

    /// Get current connection state
    fn connection_state(&self) -> ConnectionState;

    /// Send message through specific channel
    async fn send_message(&self, message: TransportMessage) -> Result<(), TransportError>;

    /// Send compressed message
    async fn send_compressed_message(&self, compressed_data: CompressedData) -> Result<(), TransportError>;

    /// Receive message from any channel
    async fn receive_message(&self) -> Result<TransportMessage, TransportError>;

    /// Receive compressed message
    async fn receive_compressed_message(&self) -> Result<Option<CompressedData>, TransportError>;

    /// Set compression configuration
    fn set_compression_config(&mut self, config: CompressionConfig);

    /// Get current compression configuration
    fn get_compression_config(&self) -> &CompressionConfig;

    /// Send control message (ordered, reliable)
    async fn send_control(&self, payload: serde_json::Value) -> Result<(), TransportError> {
        let message = TransportMessage {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: MessageType::Control,
            payload,
            timestamp: chrono::Utc::now(),
            transport_type: self.transport_type(),
            session_id: None,
        };
        self.send_message(message).await
    }

    /// Send state message (unordered, unreliable for position/physics)
    async fn send_state(&self, payload: serde_json::Value) -> Result<(), TransportError> {
        let message = TransportMessage {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: MessageType::State,
            payload,
            timestamp: chrono::Utc::now(),
            transport_type: self.transport_type(),
            session_id: None,
        };
        self.send_message(message).await
    }

    /// Get transport statistics
    fn get_stats(&self) -> TransportStats;

    /// Subscribe to transport events
    fn subscribe_to_events(&self) -> tokio::sync::broadcast::Receiver<TransportEvent>;

    /// Close transport connection
    async fn close(&self) -> Result<(), TransportError>;
}

/// Transport factory trait để tạo các transport instances
#[async_trait]
pub trait TransportFactory: Send + Sync {
    /// Create new transport instance với configuration
    async fn create_transport(&self, config: TransportConfig) -> Result<Box<dyn Transport>, TransportError>;

    /// Check if factory supports specific transport type
    fn supports_transport_type(&self, transport_type: &TransportType) -> bool;

    /// Get priority for transport type (lower = higher priority)
    fn get_priority(&self, transport_type: &TransportType) -> u8;
}

/// Configuration for transport creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub transport_type: TransportType,
    pub endpoint: String,
    pub session_id: Option<String>,
    pub ice_servers: Vec<String>,
    pub max_reconnect_attempts: u32,
    pub heartbeat_interval_ms: u64,
    pub connection_timeout_ms: u64,
    pub buffer_size: usize,
    pub enable_compression: bool,
}

/// Statistics for transport monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStats {
    pub transport_id: String,
    pub transport_type: TransportType,
    pub connection_state: ConnectionState,
    pub uptime_seconds: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub average_latency_ms: f64,
    pub packet_loss_rate: f64,
    pub reconnect_count: u32,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub error_count: u32,
}

/// Transport manager trait để quản lý multiple transports
#[async_trait]
pub trait TransportManager: Send + Sync {
    /// Add transport to manager
    async fn add_transport(&self, transport: Box<dyn Transport>) -> Result<String, TransportError>;

    /// Remove transport from manager
    async fn remove_transport(&self, transport_id: &str) -> Result<(), TransportError>;

    /// Get transport by ID
    fn get_transport(&self, transport_id: &str) -> Option<&dyn Transport>;

    /// Get all active transports
    fn get_active_transports(&self) -> Vec<&dyn Transport>;

    /// Get best transport for message type (load balancing)
    fn get_best_transport(&self, message_type: &MessageType) -> Option<&dyn Transport>;

    /// Broadcast message to all transports
    async fn broadcast_message(&self, message: TransportMessage) -> Result<(), TransportError>;

    /// Subscribe to transport events from all transports
    fn subscribe_to_all_events(&self) -> tokio::sync::broadcast::Receiver<TransportEvent>;

    /// Get manager statistics
    fn get_manager_stats(&self) -> TransportManagerStats;
}

/// Manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportManagerStats {
    pub total_transports: usize,
    pub active_transports: usize,
    pub transports_by_type: HashMap<TransportType, usize>,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub average_latency_ms: f64,
    pub total_failovers: u32,
}
