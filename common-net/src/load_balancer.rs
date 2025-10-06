use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Load balancer for distributing traffic across multiple server instances
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    servers: Arc<RwLock<HashMap<String, ServerInstance>>>,
    config: LoadBalancerConfig,
    metrics: Arc<LoadBalancerMetrics>,
    strategy: LoadBalancingStrategy,
}

/// Configuration for the load balancer
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Server timeout in seconds
    pub server_timeout: u64,
    /// Maximum server failures before removal
    pub max_failures: u32,
    /// Enable sticky sessions
    pub sticky_sessions: bool,
    /// Session timeout in seconds
    pub session_timeout: u64,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            health_check_interval: 30,
            server_timeout: 10,
            max_failures: 3,
            sticky_sessions: true,
            session_timeout: 300,
        }
    }
}

/// Load balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Round Robin - distributes requests evenly
    RoundRobin,
    /// Least Connections - routes to server with fewest active connections
    LeastConnections,
    /// Weighted Round Robin - distributes based on server weights
    WeightedRoundRobin,
    /// Random - random selection
    Random,
    /// IP Hash - consistent hashing based on client IP
    IpHash,
}

/// Server instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInstance {
    pub id: String,
    pub address: SocketAddr,
    pub region: String,
    pub capacity: u32,
    pub current_load: u32,
    pub status: ServerStatus,
    pub last_health_check: u64,
    pub failure_count: u32,
    pub weight: u32, // For weighted algorithms
    pub tags: HashMap<String, String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Healthy,
    Unhealthy,
    Starting,
    Stopping,
    Maintenance,
}

/// Performance metrics for load balancer
#[derive(Debug, Default)]
pub struct LoadBalancerMetrics {
    /// Total requests handled
    pub total_requests: AtomicU64,
    /// Requests per second
    pub requests_per_second: AtomicU64,
    /// Average response time in microseconds
    pub avg_response_time: AtomicU64,
    /// Active connections
    pub active_connections: AtomicU64,
    /// Failed requests
    pub failed_requests: AtomicU64,
    /// Server health checks performed
    pub health_checks: AtomicU64,
    /// Servers marked unhealthy
    pub unhealthy_servers: AtomicU64,
    /// Load distribution statistics
    pub load_distribution: RwLock<HashMap<String, u64>>,
}

impl LoadBalancerMetrics {
    pub fn record_request(&self, server_id: &str, micros: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // Update load distribution
        if let Ok(mut dist) = self.load_distribution.try_write() {
            let count = dist.entry(server_id.to_string()).or_insert(0);
            *count += 1;
        }

        // Update average response time
        let current = self.avg_response_time.load(Ordering::Relaxed);
        let new_avg = (current + micros) / 2;
        self.avg_response_time.store(new_avg, Ordering::Relaxed);
    }

    pub fn record_failed_request(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_health_check(&self) {
        self.health_checks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_unhealthy_server(&self) {
        self.unhealthy_servers.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64) {
        (
            self.total_requests.load(Ordering::Relaxed),
            self.requests_per_second.load(Ordering::Relaxed),
            self.avg_response_time.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::Relaxed),
            self.failed_requests.load(Ordering::Relaxed),
        )
    }

    pub fn get_load_distribution(&self) -> HashMap<String, u64> {
        self.load_distribution.try_read().unwrap_or_default().clone()
    }
}

/// Sticky session manager for maintaining client-server affinity
#[derive(Debug)]
pub struct StickySessionManager {
    sessions: RwLock<HashMap<String, String>>, // client_id -> server_id
    last_access: RwLock<HashMap<String, Instant>>,
}

impl StickySessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            last_access: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_server_for_client(&self, client_id: &str, load_balancer: &LoadBalancer) -> Option<String> {
        // Check if client has existing session
        if let Some(server_id) = self.sessions.read().await.get(client_id).cloned() {
            // Check if session is still valid and server is healthy
            if let Some(server) = load_balancer.get_server(&server_id).await {
                if server.status == ServerStatus::Healthy {
                    // Update last access time
                    if let Ok(mut access) = self.last_access.try_write() {
                        access.insert(client_id.to_string(), Instant::now());
                    }
                    return Some(server_id);
                }
            }
        }

        None
    }

    pub async fn assign_client_to_server(&self, client_id: &str, server_id: &str) {
        if let Ok(mut sessions) = self.sessions.try_write() {
            sessions.insert(client_id.to_string(), server_id.to_string());
        }
        if let Ok(mut access) = self.last_access.try_write() {
            access.insert(client_id.to_string(), Instant::now());
        }
    }

    pub async fn cleanup_expired_sessions(&self, timeout: Duration) {
        let cutoff = Instant::now() - timeout;

        if let Ok(mut access) = self.last_access.try_write() {
            let expired_clients: Vec<String> = access
                .iter()
                .filter(|(_, &last_access)| last_access < cutoff)
                .map(|(client_id, _)| client_id.clone())
                .collect();

            for client_id in expired_clients {
                access.remove(&client_id);
                if let Ok(mut sessions) = self.sessions.try_write() {
                    sessions.remove(&client_id);
                }
            }
        }
    }
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(config: LoadBalancerConfig, strategy: LoadBalancingStrategy) -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(LoadBalancerMetrics::default()),
            strategy,
        }
    }

    /// Add a server to the load balancer
    pub async fn add_server(&self, server: ServerInstance) -> Result<(), BoxError> {
        let mut servers = self.servers.write().await;
        servers.insert(server.id.clone(), server);

        info!("Added server {} at {} to load balancer", server.id, server.address);
        Ok(())
    }

    /// Remove a server from the load balancer
    pub async fn remove_server(&self, server_id: &str) -> Result<(), BoxError> {
        let mut servers = self.servers.write().await;

        if let Some(server) = servers.remove(server_id) {
            info!("Removed server {} from load balancer", server_id);

            // Clean up sticky sessions for this server
            // In a real implementation, this would redistribute sessions

            Ok(())
        } else {
            Err(format!("Server {} not found", server_id).into())
        }
    }

    /// Get server by ID
    pub async fn get_server(&self, server_id: &str) -> Option<ServerInstance> {
        let servers = self.servers.read().await;
        servers.get(server_id).cloned()
    }

    /// Get all servers
    pub async fn get_servers(&self) -> Vec<ServerInstance> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Select server based on load balancing strategy
    pub async fn select_server(&self, client_id: Option<&str>) -> Result<Option<ServerInstance>, BoxError> {
        let servers = self.servers.read().await;

        // Filter healthy servers
        let healthy_servers: Vec<ServerInstance> = servers
            .values()
            .filter(|server| server.status == ServerStatus::Healthy)
            .cloned()
            .collect();

        if healthy_servers.is_empty() {
            warn!("No healthy servers available");
            return Ok(None);
        }

        // If sticky sessions are enabled and client_id is provided, try to find existing session
        if self.config.sticky_sessions && client_id.is_some() {
            // This would need access to StickySessionManager
            // For now, we'll implement basic sticky session logic
        }

        // Select server based on strategy
        let selected_server = match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.select_round_robin(&healthy_servers).await
            }
            LoadBalancingStrategy::LeastConnections => {
                self.select_least_connections(&healthy_servers).await
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(&healthy_servers).await
            }
            LoadBalancingStrategy::Random => {
                self.select_random(&healthy_servers).await
            }
            LoadBalancingStrategy::IpHash => {
                self.select_ip_hash(client_id.unwrap_or("unknown"), &healthy_servers).await
            }
        };

        if let Some(ref server) = selected_server {
            if self.config.enable_metrics {
                self.metrics.record_request(&server.id, 0); // Response time would be measured later
            }
        }

        Ok(selected_server)
    }

    /// Round Robin selection
    async fn select_round_robin(&self, servers: &[ServerInstance]) -> Option<ServerInstance> {
        static mut COUNTER: u64 = 0;

        let index = unsafe {
            let current = COUNTER % servers.len() as u64;
            COUNTER += 1;
            current as usize
        };

        servers.get(index).cloned()
    }

    /// Least Connections selection
    async fn select_least_connections(&self, servers: &[ServerInstance]) -> Option<ServerInstance> {
        servers
            .iter()
            .min_by_key(|server| server.current_load)
            .cloned()
    }

    /// Weighted Round Robin selection
    async fn select_weighted_round_robin(&self, servers: &[ServerInstance]) -> Option<ServerInstance> {
        // Simplified weighted selection
        let total_weight: u32 = servers.iter().map(|s| s.weight).sum();

        if total_weight == 0 {
            return self.select_round_robin(servers).await;
        }

        // This is a simplified implementation
        servers
            .iter()
            .max_by_key(|server| server.weight)
            .cloned()
    }

    /// Random selection
    async fn select_random(&self, servers: &[ServerInstance]) -> Option<ServerInstance> {
        use rand::Rng;

        if servers.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..servers.len());
        servers.get(index).cloned()
    }

    /// IP Hash selection (consistent hashing)
    async fn select_ip_hash(&self, client_ip: &str, servers: &[ServerInstance]) -> Option<ServerInstance> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        client_ip.hash(&mut hasher);
        let hash = hasher.finish();

        let index = (hash % servers.len() as u64) as usize;
        servers.get(index).cloned()
    }

    /// Update server load information
    pub async fn update_server_load(&self, server_id: &str, current_load: u32) -> Result<(), BoxError> {
        let mut servers = self.servers.write().await;

        if let Some(server) = servers.get_mut(server_id) {
            server.current_load = current_load;
            server.last_health_check = chrono::Utc::now().timestamp() as u64;
            debug!("Updated load for server {}: {}", server_id, current_load);
            Ok(())
        } else {
            Err(format!("Server {} not found", server_id).into())
        }
    }

    /// Health check all servers
    pub async fn perform_health_checks(&self) -> Result<(), BoxError> {
        let servers = self.servers.read().await;
        let mut unhealthy_count = 0;

        for server in servers.values() {
            let is_healthy = self.check_server_health(server).await;

            if !is_healthy {
                unhealthy_count += 1;
                if self.config.enable_metrics {
                    self.metrics.record_unhealthy_server();
                }
            }

            if self.config.enable_metrics {
                self.metrics.record_health_check();
            }
        }

        if unhealthy_count > 0 {
            warn!("{} servers are unhealthy", unhealthy_count);
        }

        Ok(())
    }

    /// Check individual server health
    async fn check_server_health(&self, server: &ServerInstance) -> bool {
        // In a real implementation, this would make HTTP requests to health endpoints
        // For now, we'll simulate health checks based on load and failure count

        if server.failure_count >= self.config.max_failures {
            return false;
        }

        // Simulate health check failure based on load
        let load_ratio = server.current_load as f32 / server.capacity as f32;
        if load_ratio > 0.95 { // 95% capacity
            return false;
        }

        // Simulate random failures for testing
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_bool(0.95) // 95% success rate
    }

    /// Get load balancer metrics
    pub fn get_metrics(&self) -> Arc<LoadBalancerMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Get current server count
    pub async fn server_count(&self) -> usize {
        self.servers.read().await.len()
    }

    /// Get healthy server count
    pub async fn healthy_server_count(&self) -> usize {
        let servers = self.servers.read().await;
        servers
            .values()
            .filter(|server| server.status == ServerStatus::Healthy)
            .count()
    }
}

/// Auto-scaling manager for dynamic server provisioning
#[derive(Debug)]
pub struct AutoScalingManager {
    load_balancer: Arc<LoadBalancer>,
    config: AutoScalingConfig,
    current_servers: RwLock<u32>,
}

#[derive(Debug, Clone)]
pub struct AutoScalingConfig {
    /// Minimum number of servers
    pub min_servers: u32,
    /// Maximum number of servers
    pub max_servers: u32,
    /// Target CPU usage percentage for scaling up
    pub scale_up_threshold: f32,
    /// Target CPU usage percentage for scaling down
    pub scale_down_threshold: f32,
    /// Scale up cooldown in seconds
    pub scale_up_cooldown: u64,
    /// Scale down cooldown in seconds
    pub scale_down_cooldown: u64,
}

impl Default for AutoScalingConfig {
    fn default() -> Self {
        Self {
            min_servers: 2,
            max_servers: 20,
            scale_up_threshold: 75.0,
            scale_down_threshold: 25.0,
            scale_up_cooldown: 300,
            scale_down_cooldown: 600,
        }
    }
}

impl AutoScalingManager {
    pub fn new(load_balancer: Arc<LoadBalancer>, config: AutoScalingConfig) -> Self {
        Self {
            load_balancer,
            config,
            current_servers: RwLock::new(0),
        }
    }

    /// Evaluate scaling decisions based on current load
    pub async fn evaluate_scaling(&self) -> Result<ScalingDecision, BoxError> {
        let healthy_servers = self.load_balancer.healthy_server_count().await;
        let total_servers = self.load_balancer.server_count().await;

        let metrics = self.load_balancer.get_metrics();
        let (total_requests, _, avg_response_time, active_connections, _) = metrics.get_stats();

        // Calculate average load per server
        let avg_load_per_server = if healthy_servers > 0 {
            active_connections as f32 / healthy_servers as f32
        } else {
            0.0
        };

        // Calculate requests per second per server
        let avg_rps_per_server = if healthy_servers > 0 {
            total_requests as f32 / healthy_servers as f32
        } else {
            0.0
        };

        // Determine if we need to scale
        let current_load_percentage = (avg_load_per_server / 100.0) * 100.0; // Assuming max 100 connections per server

        let decision = if healthy_servers < self.config.min_servers {
            ScalingDecision::ScaleUp(1)
        } else if current_load_percentage > self.config.scale_up_threshold && healthy_servers < self.config.max_servers {
            ScalingDecision::ScaleUp(1)
        } else if current_load_percentage < self.config.scale_down_threshold && healthy_servers > self.config.min_servers {
            ScalingDecision::ScaleDown(1)
        } else {
            ScalingDecision::NoChange
        };

        debug!(
            "Scaling evaluation: healthy_servers={}, load_percentage={:.2}%, avg_rps={:.2}, decision={:?}",
            healthy_servers, current_load_percentage, avg_rps_per_server, decision
        );

        Ok(decision)
    }

    /// Execute scaling decision
    pub async fn execute_scaling(&self, decision: ScalingDecision) -> Result<(), BoxError> {
        match decision {
            ScalingDecision::ScaleUp(count) => {
                for _ in 0..count {
                    self.scale_up().await?;
                }
            }
            ScalingDecision::ScaleDown(count) => {
                for _ in 0..count {
                    self.scale_down().await?;
                }
            }
            ScalingDecision::NoChange => {
                // Do nothing
            }
        }

        Ok(())
    }

    async fn scale_up(&self) -> Result<(), BoxError> {
        // In a real implementation, this would:
        // 1. Provision new server instances (AWS EC2, Docker containers, etc.)
        // 2. Configure the servers
        // 3. Wait for them to become healthy
        // 4. Add them to the load balancer

        info!("Scaling up: provisioning new server instance");

        // For now, simulate adding a server
        let server_id = Uuid::new_v4().to_string();
        let server = ServerInstance {
            id: server_id.clone(),
            address: format!("127.0.0.1:{}", 3000 + self.load_balancer.server_count().await as u16).parse()?,
            region: "us-east".to_string(),
            capacity: 100,
            current_load: 0,
            status: ServerStatus::Starting,
            last_health_check: chrono::Utc::now().timestamp() as u64,
            failure_count: 0,
            weight: 1,
            tags: HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        self.load_balancer.add_server(server).await?;

        let mut current = self.current_servers.write().await;
        *current += 1;

        info!("Scaled up: added server {}", server_id);
        Ok(())
    }

    async fn scale_down(&self) -> Result<(), BoxError> {
        // In a real implementation, this would:
        // 1. Find the server with the least load
        // 2. Drain connections from that server
        // 3. Remove it from the load balancer
        // 4. Terminate the instance

        let servers = self.load_balancer.get_servers().await;

        if let Some(server_to_remove) = servers
            .iter()
            .min_by_key(|server| server.current_load)
        {
            info!("Scaling down: removing server {}", server_to_remove.id);

            self.load_balancer.remove_server(&server_to_remove.id).await?;

            let mut current = self.current_servers.write().await;
            *current = current.saturating_sub(1);
        }

        Ok(())
    }
}

/// Scaling decision result
#[derive(Debug, Clone)]
pub enum ScalingDecision {
    ScaleUp(u32),
    ScaleDown(u32),
    NoChange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_balancer_creation() {
        let config = LoadBalancerConfig::default();
        let lb = LoadBalancer::new(config, LoadBalancingStrategy::RoundRobin);

        assert_eq!(lb.server_count().await, 0);
        assert_eq!(lb.healthy_server_count().await, 0);

        println!("✅ Load balancer creation test completed");
    }

    #[tokio::test]
    async fn test_server_management() {
        let config = LoadBalancerConfig::default();
        let lb = LoadBalancer::new(config, LoadBalancingStrategy::RoundRobin);

        // Add servers
        let server1 = ServerInstance {
            id: "server1".to_string(),
            address: "127.0.0.1:3001".parse().unwrap(),
            region: "us-east".to_string(),
            capacity: 100,
            current_load: 0,
            status: ServerStatus::Healthy,
            last_health_check: chrono::Utc::now().timestamp() as u64,
            failure_count: 0,
            weight: 1,
            tags: HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        let server2 = ServerInstance {
            id: "server2".to_string(),
            address: "127.0.0.1:3002".parse().unwrap(),
            region: "us-west".to_string(),
            capacity: 100,
            current_load: 0,
            status: ServerStatus::Healthy,
            last_health_check: chrono::Utc::now().timestamp() as u64,
            failure_count: 0,
            weight: 1,
            tags: HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        lb.add_server(server1).await.unwrap();
        lb.add_server(server2).await.unwrap();

        assert_eq!(lb.server_count().await, 2);
        assert_eq!(lb.healthy_server_count().await, 2);

        // Test server selection
        let selected = lb.select_server(Some("client1")).await.unwrap().unwrap();
        assert!(selected.id == "server1" || selected.id == "server2");

        // Test load update
        lb.update_server_load("server1", 50).await.unwrap();

        // Test server removal
        lb.remove_server("server2").await.unwrap();
        assert_eq!(lb.server_count().await, 1);

        println!("✅ Server management test completed");
    }

    #[tokio::test]
    async fn test_load_balancing_strategies() {
        let config = LoadBalancerConfig::default();
        let lb = LoadBalancer::new(config.clone(), LoadBalancingStrategy::LeastConnections);

        // Add servers with different loads
        let server1 = ServerInstance {
            id: "server1".to_string(),
            address: "127.0.0.1:3001".parse().unwrap(),
            region: "us-east".to_string(),
            capacity: 100,
            current_load: 10,
            status: ServerStatus::Healthy,
            last_health_check: chrono::Utc::now().timestamp() as u64,
            failure_count: 0,
            weight: 1,
            tags: HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        let server2 = ServerInstance {
            id: "server2".to_string(),
            address: "127.0.0.1:3002".parse().unwrap(),
            region: "us-east".to_string(),
            capacity: 100,
            current_load: 80,
            status: ServerStatus::Healthy,
            last_health_check: chrono::Utc::now().timestamp() as u64,
            failure_count: 0,
            weight: 1,
            tags: HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        lb.add_server(server1).await.unwrap();
        lb.add_server(server2).await.unwrap();

        // Least connections should select server1 (load 10 vs 80)
        let selected = lb.select_server(None).await.unwrap().unwrap();
        assert_eq!(selected.id, "server1");

        println!("✅ Load balancing strategies test completed");
    }

    #[tokio::test]
    async fn test_auto_scaling() {
        let config = LoadBalancerConfig::default();
        let lb = Arc::new(LoadBalancer::new(config, LoadBalancingStrategy::RoundRobin));
        let scaling_config = AutoScalingConfig::default();
        let scaling_manager = AutoScalingManager::new(lb.clone(), scaling_config);

        // Test scaling evaluation with no servers
        let decision = scaling_manager.evaluate_scaling().await.unwrap();
        assert!(matches!(decision, ScalingDecision::ScaleUp(_)));

        // Add minimum servers
        for i in 0..2 {
            let server = ServerInstance {
                id: format!("server{}", i),
                address: format!("127.0.0.1:300{}", i).parse().unwrap(),
                region: "us-east".to_string(),
                capacity: 100,
                current_load: 0,
                status: ServerStatus::Healthy,
                last_health_check: chrono::Utc::now().timestamp() as u64,
                failure_count: 0,
                weight: 1,
                tags: HashMap::new(),
                created_at: chrono::Utc::now().timestamp() as u64,
            };
            lb.add_server(server).await.unwrap();
        }

        // Test scaling evaluation with adequate servers
        let decision = scaling_manager.evaluate_scaling().await.unwrap();
        // This might be NoChange or ScaleDown depending on load
        println!("Scaling decision with adequate servers: {:?}", decision);

        println!("✅ Auto scaling test completed");
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = LoadBalancerConfig::default();
        let lb = LoadBalancer::new(config, LoadBalancingStrategy::RoundRobin);

        let metrics = lb.get_metrics();
        let (total_requests, rps, avg_response, active_conn, failed) = metrics.get_stats();

        assert_eq!(total_requests, 0);
        assert_eq!(failed, 0);

        // Simulate some requests
        metrics.record_request("server1", 1000);
        metrics.record_request("server1", 2000);

        let (total_requests, _, avg_response, _, _) = metrics.get_stats();
        assert_eq!(total_requests, 2);
        assert_eq!(avg_response, 1500); // Average of 1000 and 2000

        println!("✅ Performance metrics test completed");
    }
}
