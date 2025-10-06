use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{broadcast, RwLock},
    time::interval,
};
use tracing::{info, warn};

use super::{
    traits::{Transport, TransportFactory, TransportManager, TransportManagerStats},
    TransportConfig, TransportError, TransportEvent, TransportMessage, TransportType, MessageType,
};
use crate::compression::CompressionConfig;

/// Default transport manager implementation
pub struct DefaultTransportManager {
    transports: Arc<RwLock<HashMap<String, Box<dyn Transport>>>>,
    event_sender: broadcast::Sender<TransportEvent>,
    factories: Vec<Box<dyn TransportFactory>>,
    stats: Arc<RwLock<TransportManagerStats>>,
    health_check_interval: Duration,
    compression_config: CompressionConfig,
}

impl DefaultTransportManager {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);

        Self {
            transports: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            factories: Vec::new(),
            stats: Arc::new(RwLock::new(TransportManagerStats {
                total_transports: 0,
                active_transports: 0,
                transports_by_type: HashMap::new(),
                total_messages_sent: 0,
                total_messages_received: 0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                average_latency_ms: 0.0,
                total_failovers: 0,
            })),
            health_check_interval: Duration::from_secs(30),
            compression_config: CompressionConfig::default(),
        }
    }

    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = interval;
        self
    }

    pub fn with_compression_config(mut self, config: CompressionConfig) -> Self {
        self.compression_config = config;
        self
    }

    pub fn set_compression_config(&mut self, config: CompressionConfig) {
        self.compression_config = config;
        // Update all active transports with new compression config
        // TODO: Implement this when transport trait supports compression config
    }

    pub fn get_compression_config(&self) -> &CompressionConfig {
        &self.compression_config
    }

    pub fn add_factory<F: TransportFactory + 'static>(mut self, factory: F) -> Self {
        self.factories.push(Box::new(factory));
        self
    }

    /// Start background health check task
    pub fn start_health_checks(&self) -> tokio::task::JoinHandle<()> {
        let transports = Arc::clone(&self.transports);
        let event_sender = self.event_sender.clone();
        let stats = Arc::clone(&self.stats);
        let interval_duration = self.health_check_interval;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Health check tất cả transports
                let transport_list: Vec<_> = {
                    let transports_lock = transports.read().await;
                    transports_lock.keys().cloned().collect()
                };

                for transport_id in transport_list {
                    let transports_lock = transports.read().await;
                    if let Some(transport) = transports_lock.get(&transport_id).map(|t| t.as_ref()) {
                        // Check connection health
                        if !transport.is_connected() {
                            let _ = event_sender.send(TransportEvent::Disconnected {
                                transport_type: transport.transport_type(),
                                session_id: transport_id.clone(),
                                reason: "Health check failed".to_string(),
                            });
                        }
                    }
                }

                // Update manager stats
                Self::update_manager_stats(&transports, &stats).await;
            }
        })
    }

    async fn update_manager_stats(
        transports: &Arc<RwLock<HashMap<String, Box<dyn Transport>>>>,
        stats: &Arc<RwLock<TransportManagerStats>>,
    ) {
        let transports_lock = transports.read().await;
        let active_count = transports_lock.values().filter(|t| t.is_connected()).count();

        let mut type_counts = HashMap::new();
        let mut total_sent = 0;
        let mut total_received = 0;
        let mut total_bytes_sent = 0;
        let mut total_bytes_received = 0;

        for transport in transports_lock.values() {
            let transport_type = transport.transport_type();
            *type_counts.entry(transport_type.clone()).or_insert(0) += 1;

            let transport_stats = transport.get_stats();
            total_sent += transport_stats.messages_sent;
            total_received += transport_stats.messages_received;
            total_bytes_sent += transport_stats.bytes_sent;
            total_bytes_received += transport_stats.bytes_received;
        }

        let mut stats_lock = stats.write().await;
        stats_lock.active_transports = active_count;
        stats_lock.transports_by_type = type_counts;
        stats_lock.total_messages_sent = total_sent;
        stats_lock.total_messages_received = total_received;
        stats_lock.total_bytes_sent = total_bytes_sent;
        stats_lock.total_bytes_received = total_bytes_received;
    }

    /// Find best transport for message type dựa trên priority và health
    fn select_best_transport<'a>(&self, transports: &'a HashMap<String, Box<dyn Transport>>, message_type: &MessageType) -> Option<&'a dyn Transport> {
        let mut best_transport: Option<&'a dyn Transport> = None;
        let mut best_priority = u8::MAX;

        for transport in transports.values() {
            if !transport.is_connected() {
                continue;
            }

            // Find factory for this transport type to get priority
            for factory in &self.factories {
                if factory.supports_transport_type(&transport.transport_type()) {
                    let priority = factory.get_priority(&transport.transport_type());
                    if priority < best_priority {
                        best_priority = priority;
                        best_transport = Some(transport.as_ref());
                    }
                    break;
                }
            }
        }

        best_transport
    }
}

#[async_trait]
impl TransportManager for DefaultTransportManager {
    async fn add_transport(&self, transport: Box<dyn Transport>) -> Result<String, TransportError> {
        let transport_id = transport.id().to_string();

        info!("Adding transport {} ({:?})", transport_id, transport.transport_type());

        {
            let mut transports = self.transports.write().await;
            transports.insert(transport_id.clone(), transport);
        }

        // Send event
        let _ = self.event_sender.send(TransportEvent::Connected {
            transport_type: TransportType::WebRTC, // TODO: Get from transport
            session_id: transport_id.clone(),
        });

        // Update stats
        Self::update_manager_stats(&self.transports, &self.stats).await;

        Ok(transport_id)
    }

    async fn remove_transport(&self, transport_id: &str) -> Result<(), TransportError> {
        info!("Removing transport {}", transport_id);

        {
            let mut transports = self.transports.write().await;
            if transports.remove(transport_id).is_some() {
                // Send event
                let _ = self.event_sender.send(TransportEvent::Disconnected {
                    transport_type: TransportType::WebRTC, // TODO: Get from transport
                    session_id: transport_id.to_string(),
                    reason: "Manually removed".to_string(),
                });
            }
        }

        // Update stats
        Self::update_manager_stats(&self.transports, &self.stats).await;

        Ok(())
    }

    fn get_transport(&self, transport_id: &str) -> Option<&dyn Transport> {
        // Note: This is a simplified implementation
        // In a real implementation, you'd want async access
        None
    }

    fn get_active_transports(&self) -> Vec<&dyn Transport> {
        // Note: This is a simplified implementation
        // In a real implementation, you'd want async access
        Vec::new()
    }

    fn get_best_transport(&self, message_type: &MessageType) -> Option<&dyn Transport> {
        // Note: This is a simplified implementation
        // In a real implementation, you'd want async access
        None
    }

    async fn broadcast_message(&self, message: TransportMessage) -> Result<(), TransportError> {
        let transports = self.transports.read().await;

        for (id, transport) in transports.iter() {
            if transport.is_connected() {
                if let Err(e) = transport.send_message(message.clone()).await {
                    warn!("Failed to send message to transport {}: {}", id, e);
                }
            }
        }

        Ok(())
    }

    fn subscribe_to_all_events(&self) -> broadcast::Receiver<TransportEvent> {
        self.event_sender.subscribe()
    }

    fn get_manager_stats(&self) -> TransportManagerStats {
        // Note: This is a simplified implementation
        // In a real implementation, you'd want async access
        TransportManagerStats {
            total_transports: 0,
            active_transports: 0,
            transports_by_type: HashMap::new(),
            total_messages_sent: 0,
            total_messages_received: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            average_latency_ms: 0.0,
            total_failovers: 0,
        }
    }
}

impl Default for DefaultTransportManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebRTC Transport Factory
pub struct WebRTCTransportFactory;

#[async_trait]
impl TransportFactory for WebRTCTransportFactory {
    async fn create_transport(&self, config: TransportConfig) -> Result<Box<dyn Transport>, TransportError> {
        // TODO: Implement WebRTC transport creation
        Err(TransportError::new(
            crate::transport::TransportErrorKind::Unsupported,
            "WebRTC transport not yet implemented",
        ))
    }

    fn supports_transport_type(&self, transport_type: &TransportType) -> bool {
        matches!(transport_type, TransportType::WebRTC)
    }

    fn get_priority(&self, transport_type: &TransportType) -> u8 {
        match transport_type {
            TransportType::WebRTC => 10,  // High priority for real-time
            _ => 255,
        }
    }
}

/// WebSocket Transport Factory
pub struct WebSocketTransportFactory;

#[async_trait]
impl TransportFactory for WebSocketTransportFactory {
    async fn create_transport(&self, config: TransportConfig) -> Result<Box<dyn Transport>, TransportError> {
        // TODO: Implement WebSocket transport creation
        Err(TransportError::new(
            crate::transport::TransportErrorKind::Unsupported,
            "WebSocket transport not yet implemented",
        ))
    }

    fn supports_transport_type(&self, transport_type: &TransportType) -> bool {
        matches!(transport_type, TransportType::WebSocket)
    }

    fn get_priority(&self, transport_type: &TransportType) -> u8 {
        match transport_type {
            TransportType::WebSocket => 20,  // Lower priority than WebRTC
            _ => 255,
        }
    }
}

/// QUIC Transport Factory
pub struct QUICTransportFactory;

#[async_trait]
impl TransportFactory for QUICTransportFactory {
    async fn create_transport(&self, config: TransportConfig) -> Result<Box<dyn Transport>, TransportError> {
        // TODO: Implement QUIC transport creation
        Err(TransportError::new(
            crate::transport::TransportErrorKind::Unsupported,
            "QUIC transport not yet implemented",
        ))
    }

    fn supports_transport_type(&self, transport_type: &TransportType) -> bool {
        matches!(transport_type, TransportType::QUIC)
    }

    fn get_priority(&self, transport_type: &TransportType) -> u8 {
        match transport_type {
            TransportType::QUIC => 15,  // Medium priority
            _ => 255,
        }
    }
}
