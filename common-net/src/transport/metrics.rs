use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{
    TransportEvent, TransportType, ConnectionState, MessageType,
    traits::{Transport, TransportManager},
};

/// Transport metrics collector
pub struct TransportMetrics {
    metrics: Arc<RwLock<HashMap<String, TransportMetricsData>>>,
    global_stats: Arc<RwLock<GlobalTransportStats>>,
}

#[derive(Debug, Clone)]
pub struct TransportMetricsData {
    pub transport_id: String,
    pub transport_type: TransportType,
    pub connection_state: ConnectionState,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors: u64,
    pub reconnects: u64,
    pub last_activity: Option<Instant>,
    pub created_at: Instant,
}

impl TransportMetricsData {
    pub fn new(transport_id: String, transport_type: TransportType) -> Self {
        Self {
            transport_id,
            transport_type,
            connection_state: ConnectionState::Disconnected,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            errors: 0,
            reconnects: 0,
            last_activity: None,
            created_at: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GlobalTransportStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_errors: u64,
    pub total_reconnects: u64,
    pub total_failovers: u64,
    pub average_latency_ms: f64,
    pub connections_by_type: HashMap<TransportType, u64>,
}

impl TransportMetrics {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            global_stats: Arc::new(RwLock::new(GlobalTransportStats::default())),
        }
    }

    /// Record transport event và update metrics
    pub async fn record_event(&self, event: TransportEvent) {
        match event.clone() {
            TransportEvent::Connected { transport_type, session_id } => {
                self.add_transport(transport_type, session_id).await;
            }
            TransportEvent::Disconnected { transport_type, session_id, .. } => {
                self.remove_transport(&session_id).await;
            }
            TransportEvent::MessageSent { transport_type, message_type, size } => {
                self.record_message_sent(&transport_type, size).await;
            }
            TransportEvent::MessageReceived { transport_type, message_type, size } => {
                self.record_message_received(&transport_type, size).await;
            }
            TransportEvent::Error { transport_type, .. } => {
                self.record_error(&transport_type).await;
            }
            TransportEvent::Reconnecting { transport_type, .. } => {
                self.record_reconnect(&transport_type).await;
            }
            TransportEvent::Failover { from_transport, to_transport } => {
                self.record_failover(&from_transport, &to_transport).await;
            }
        }

        // Log event for debugging
        debug!("Transport event: {:?}", event);
    }

    async fn add_transport(&self, transport_type: TransportType, transport_id: String) {
        let mut metrics = self.metrics.write().await;

        let transport_data = TransportMetricsData {
            transport_id: transport_id.clone(),
            transport_type: transport_type.clone(),
            connection_state: ConnectionState::Connected,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            errors: 0,
            reconnects: 0,
            last_activity: Some(Instant::now()),
            created_at: Instant::now(),
        };

        metrics.insert(transport_id.clone(), transport_data);

        // Update global stats
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_connections += 1;
        global_stats.active_connections += 1;
        *global_stats.connections_by_type.entry(transport_type).or_insert(0) += 1;

        info!("Transport {} ({:?}) connected", transport_id, transport_type);
    }

    async fn remove_transport(&self, transport_id: &str) {
        let mut metrics = self.metrics.write().await;
        if let Some(_) = metrics.remove(transport_id) {
            let mut global_stats = self.global_stats.write().await;
            global_stats.active_connections = global_stats.active_connections.saturating_sub(1);

            info!("Transport {} disconnected", transport_id);
        }
    }

    async fn record_message_sent(&self, transport_type: &TransportType, size: usize) {
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_messages_sent += 1;
        global_stats.total_bytes_sent += size as u64;

        // Update prometheus metrics nếu có
        #[cfg(feature = "metrics")]
        {
            use metrics::{counter, histogram};
            counter!("transport_messages_sent_total", 1, "transport_type" => transport_type.to_string());
            histogram!("transport_message_size_bytes", size as f64, "transport_type" => transport_type.to_string());
        }
    }

    async fn record_message_received(&self, transport_type: &TransportType, size: usize) {
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_messages_received += 1;
        global_stats.total_bytes_received += size as u64;

        // Update prometheus metrics nếu có
        #[cfg(feature = "metrics")]
        {
            use metrics::{counter, histogram};
            counter!("transport_messages_received_total", 1, "transport_type" => transport_type.to_string());
            histogram!("transport_message_size_bytes", size as f64, "transport_type" => transport_type.to_string(), "direction" => "received");
        }
    }

    async fn record_error(&self, transport_type: &TransportType) {
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_errors += 1;

        #[cfg(feature = "metrics")]
        {
            use metrics::counter;
            counter!("transport_errors_total", 1, "transport_type" => transport_type.to_string());
        }
    }

    async fn record_reconnect(&self, transport_type: &TransportType) {
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_reconnects += 1;

        #[cfg(feature = "metrics")]
        {
            use metrics::counter;
            counter!("transport_reconnects_total", 1, "transport_type" => transport_type.to_string());
        }
    }

    async fn record_failover(&self, from_transport: &TransportType, to_transport: &TransportType) {
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_failovers += 1;

        #[cfg(feature = "metrics")]
        {
            use metrics::counter;
            counter!("transport_failovers_total", 1,
                "from_transport" => from_transport.to_string(),
                "to_transport" => to_transport.to_string()
            );
        }
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> GlobalTransportStats {
        self.global_stats.read().await.clone()
    }

    /// Get metrics for specific transport
    pub async fn get_transport_metrics(&self, transport_id: &str) -> Option<TransportMetricsData> {
        self.metrics.read().await.get(transport_id).cloned()
    }

    /// Get all transport metrics
    pub async fn get_all_transport_metrics(&self) -> HashMap<String, TransportMetricsData> {
        self.metrics.read().await.clone()
    }

    /// Calculate average latency across all transports
    pub async fn calculate_average_latency(&self) -> f64 {
        let metrics = self.metrics.read().await;
        let active_transports: Vec<_> = metrics.values()
            .filter(|m| matches!(m.connection_state, ConnectionState::Connected))
            .collect();

        if active_transports.is_empty() {
            return 0.0;
        }

        // TODO: Implement actual latency calculation
        // For now, return a placeholder value
        50.0
    }

    /// Get health status of transport layer
    pub async fn get_health_status(&self) -> TransportHealthStatus {
        let global_stats = self.global_stats.read().await.clone();
        let transport_metrics = self.metrics.read().await.clone();

        let healthy_transports = transport_metrics.values()
            .filter(|m| matches!(m.connection_state, ConnectionState::Connected))
            .count();

        let health_percentage = if global_stats.active_connections > 0 {
            (healthy_transports as f64 / global_stats.active_connections as f64) * 100.0
        } else {
            100.0
        };

        TransportHealthStatus {
            total_transports: global_stats.total_connections,
            active_transports: global_stats.active_connections,
            healthy_transports,
            health_percentage,
            total_errors: global_stats.total_errors,
            total_reconnects: global_stats.total_reconnects,
            average_latency_ms: global_stats.average_latency_ms,
        }
    }
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Health status của transport layer
#[derive(Debug, Clone)]
pub struct TransportHealthStatus {
    pub total_transports: u64,
    pub active_transports: u64,
    pub healthy_transports: usize,
    pub health_percentage: f64,
    pub total_errors: u64,
    pub total_reconnects: u64,
    pub average_latency_ms: f64,
}

impl TransportHealthStatus {
    pub fn is_healthy(&self) -> bool {
        self.health_percentage >= 90.0 && self.total_errors < 10
    }

    pub fn get_status_string(&self) -> &'static str {
        if self.health_percentage >= 95.0 {
            "EXCELLENT"
        } else if self.health_percentage >= 90.0 {
            "GOOD"
        } else if self.health_percentage >= 75.0 {
            "FAIR"
        } else {
            "POOR"
        }
    }
}
