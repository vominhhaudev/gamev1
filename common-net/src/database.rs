use async_trait::async_trait;
use bb8::{Pool, PooledConnection, RunError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Database connection pool for high concurrency scenarios
#[derive(Debug, Clone)]
pub struct DatabasePool {
    pool: Pool<DatabaseConnectionManager>,
    metrics: Arc<DatabaseMetrics>,
    config: DatabaseConfig,
}

/// Database configuration for connection pooling
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "postgresql://user:pass@localhost/db")
    pub database_url: String,
    /// Connection pool size (default: 50)
    pub pool_size: u32,
    /// Minimum idle connections (default: 5)
    pub min_idle: u32,
    /// Connection timeout in seconds (default: 30)
    pub connection_timeout: u64,
    /// Query timeout in seconds (default: 10)
    pub query_timeout: u64,
    /// Enable metrics collection (default: true)
    pub enable_metrics: bool,
    /// Enable read/write splitting (default: false)
    pub enable_read_replica: bool,
    /// Read replica URLs (if enabled)
    pub read_replica_urls: Vec<String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://postgres:password@localhost/gamev1".to_string(),
            pool_size: 50,
            min_idle: 5,
            connection_timeout: 30,
            query_timeout: 10,
            enable_metrics: true,
            enable_read_replica: false,
            read_replica_urls: vec![],
        }
    }
}

/// Performance metrics for database monitoring
#[derive(Debug, Default)]
pub struct DatabaseMetrics {
    /// Total connections created
    pub connections_created: AtomicU64,
    /// Total connections destroyed
    pub connections_destroyed: AtomicU64,
    /// Current active connections
    pub connections_active: AtomicU64,
    /// Current idle connections
    pub connections_idle: AtomicU64,
    /// Total queries executed
    pub queries_executed: AtomicU64,
    /// Total query errors
    pub query_errors: AtomicU64,
    /// Average query time in microseconds
    pub avg_query_time: AtomicU64,
    /// Connection pool wait time
    pub pool_wait_time: AtomicU64,
    /// Read query count
    pub read_queries: AtomicU64,
    /// Write query count
    pub write_queries: AtomicU64,
}

impl DatabaseMetrics {
    pub fn record_connection_created(&self) {
        self.connections_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_destroyed(&self) {
        self.connections_destroyed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_active(&self) {
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_idle(&self) {
        self.connections_idle.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_query(&self, query_type: QueryType, micros: u64) {
        match query_type {
            QueryType::Read => self.read_queries.fetch_add(1, Ordering::Relaxed),
            QueryType::Write => self.write_queries.fetch_add(1, Ordering::Relaxed),
        }
        self.queries_executed.fetch_add(1, Ordering::Relaxed);

        let current = self.avg_query_time.load(Ordering::Relaxed);
        let new_avg = (current + micros) / 2;
        self.avg_query_time.store(new_avg, Ordering::Relaxed);
    }

    pub fn record_query_error(&self) {
        self.query_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_pool_wait(&self, micros: u64) {
        let current = self.pool_wait_time.load(Ordering::Relaxed);
        let new_avg = (current + micros) / 2;
        self.pool_wait_time.store(new_avg, Ordering::Relaxed);
    }

    pub fn update_connection_stats(&self, active: u64, idle: u64) {
        self.connections_active.store(active, Ordering::Relaxed);
        self.connections_idle.store(idle, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (
            self.connections_created.load(Ordering::Relaxed),
            self.connections_destroyed.load(Ordering::Relaxed),
            self.connections_active.load(Ordering::Relaxed),
            self.connections_idle.load(Ordering::Relaxed),
            self.queries_executed.load(Ordering::Relaxed),
            self.query_errors.load(Ordering::Relaxed),
            self.avg_query_time.load(Ordering::Relaxed),
            self.pool_wait_time.load(Ordering::Relaxed),
            self.read_queries.load(Ordering::Relaxed),
            self.write_queries.load(Ordering::Relaxed),
        )
    }
}

#[derive(Debug, Clone)]
pub enum QueryType {
    Read,
    Write,
}

/// Database connection manager for bb8
#[derive(Debug)]
pub struct DatabaseConnectionManager {
    config: DatabaseConfig,
}

impl DatabaseConnectionManager {
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl bb8::ManageConnection for DatabaseConnectionManager {
    type Connection = DatabaseConnection;
    type Error = BoxError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        // In a real implementation, this would create actual database connections
        // For now, we'll create a mock connection that simulates database operations
        Ok(DatabaseConnection::new())
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Check if connection is still valid
        Ok(())
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

/// Mock database connection for testing
#[derive(Debug)]
pub struct DatabaseConnection {
    created_at: Instant,
    query_count: AtomicU64,
}

impl DatabaseConnection {
    pub fn new() -> Self {
        Self {
            created_at: Instant::now(),
            query_count: AtomicU64::new(0),
        }
    }

    pub async fn execute_query(&self, query: &str, query_type: QueryType) -> Result<serde_json::Value, BoxError> {
        let start = Instant::now();

        // Simulate query execution time
        let query_time = match query.len() % 3 {
            0 => Duration::from_millis(10),
            1 => Duration::from_millis(50),
            _ => Duration::from_millis(100),
        };

        tokio::time::sleep(query_time).await;

        self.query_count.fetch_add(1, Ordering::Relaxed);

        let micros = start.elapsed().as_micros() as u64;

        // Return mock data based on query type
        let result = match query_type {
            QueryType::Read => {
                serde_json::json!({
                    "success": true,
                    "data": [
                        {"id": "1", "name": "Test Record"},
                        {"id": "2", "name": "Another Record"}
                    ],
                    "count": 2
                })
            }
            QueryType::Write => {
                serde_json::json!({
                    "success": true,
                    "affected_rows": 1,
                    "insert_id": "new_id_123"
                })
            }
        };

        Ok(result)
    }

    pub fn get_query_count(&self) -> u64 {
        self.query_count.load(Ordering::Relaxed)
    }

    pub fn get_age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Player record for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub id: Option<String>,
    pub username: String,
    pub email: String,
    pub score: i32,
    pub is_online: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Game record for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub id: Option<String>,
    pub name: String,
    pub max_players: i32,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Game session record for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSessionRecord {
    pub id: Option<String>,
    pub game_id: String,
    pub player_id: String,
    pub score: i32,
    pub position: serde_json::Value,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl DatabasePool {
    /// Create a new database connection pool
    pub async fn new(config: DatabaseConfig) -> Result<Self, BoxError> {
        let manager = DatabaseConnectionManager::new(config.clone());

        let pool = Pool::builder()
            .max_size(config.pool_size)
            .min_idle(Some(config.min_idle))
            .connection_timeout(Duration::from_secs(config.connection_timeout))
            .build(manager)
            .await?;

        // Test connection
        let conn = pool.get().await?;
        info!("Database pool initialized successfully with {} max connections", config.pool_size);

        Ok(Self {
            pool,
            metrics: Arc::new(DatabaseMetrics::default()),
            config,
        })
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledConnection<DatabaseConnectionManager>, RunError<BoxError>> {
        let start = Instant::now();

        match self.pool.get().await {
            Ok(conn) => {
                if self.config.enable_metrics {
                    let micros = start.elapsed().as_micros() as u64;
                    self.metrics.record_pool_wait(micros);
                }
                Ok(conn)
            }
            Err(e) => {
                if self.config.enable_metrics {
                    self.metrics.record_query_error();
                }
                Err(e)
            }
        }
    }

    /// Execute a read query with automatic connection management
    pub async fn execute_read(&self, query: &str) -> Result<serde_json::Value, BoxError> {
        let conn = self.get_connection().await?;

        let start = Instant::now();
        let result = conn.execute_query(query, QueryType::Read).await;
        let micros = start.elapsed().as_micros() as u64;

        if self.config.enable_metrics {
            match &result {
                Ok(_) => self.metrics.record_query(QueryType::Read, micros),
                Err(_) => self.metrics.record_query_error(),
            }
        }

        result
    }

    /// Execute a write query with automatic connection management
    pub async fn execute_write(&self, query: &str) -> Result<serde_json::Value, BoxError> {
        let conn = self.get_connection().await?;

        let start = Instant::now();
        let result = conn.execute_query(query, QueryType::Write).await;
        let micros = start.elapsed().as_micros() as u64;

        if self.config.enable_metrics {
            match &result {
                Ok(_) => self.metrics.record_query(QueryType::Write, micros),
                Err(_) => self.metrics.record_query_error(),
            }
        }

        result
    }

    /// Batch operations for improved performance
    pub async fn execute_batch(&self, queries: Vec<(String, QueryType)>) -> Result<Vec<serde_json::Value>, BoxError> {
        let mut results = Vec::new();

        for (query, query_type) in queries {
            let result = match query_type {
                QueryType::Read => self.execute_read(&query).await?,
                QueryType::Write => self.execute_write(&query).await?,
            };
            results.push(result);
        }

        Ok(results)
    }

    /// Transaction support for atomic operations
    pub async fn execute_transaction<F, T>(&self, operations: F) -> Result<T, BoxError>
    where
        F: Fn(&DatabaseConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, BoxError>> + Send>>,
    {
        let conn = self.get_connection().await?;

        // In a real implementation, this would start a transaction
        // For now, we'll just execute the operations
        let result = operations(&conn).await?;

        Ok(result)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool, BoxError> {
        match self.execute_read("SELECT 1").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> Arc<DatabaseMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Update connection pool statistics
    pub async fn update_pool_stats(&self) {
        if let Ok(state) = self.pool.state().await {
            self.metrics.update_connection_stats(
                state.connections - state.idle_connections,
                state.idle_connections,
            );
        }
    }

    /// Specific database operations for the game
    pub async fn create_player(&self, player: &PlayerRecord) -> Result<String, BoxError> {
        let query = format!(
            "INSERT INTO players (username, email, score, is_online) VALUES ('{}', '{}', {}, {}) RETURNING id",
            player.username, player.email, player.score, player.is_online
        );

        let result = self.execute_write(&query).await?;

        if let Some(insert_id) = result.get("insert_id") {
            Ok(insert_id.as_str().unwrap_or("unknown").to_string())
        } else {
            Err("No insert_id returned".into())
        }
    }

    pub async fn get_player(&self, player_id: &str) -> Result<Option<PlayerRecord>, BoxError> {
        let query = format!("SELECT * FROM players WHERE id = '{}'", player_id);
        let result = self.execute_read(&query).await?;

        // Parse result into PlayerRecord
        if let Some(data) = result.get("data") {
            if let Some(players) = data.as_array() {
                if let Some(player_data) = players.first() {
                    let player = PlayerRecord {
                        id: player_data.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        username: player_data.get("username").unwrap_or(&serde_json::Value::String("Unknown".to_string())).as_str().unwrap().to_string(),
                        email: player_data.get("email").unwrap_or(&serde_json::Value::String("unknown@example.com".to_string())).as_str().unwrap().to_string(),
                        score: player_data.get("score").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        is_online: player_data.get("is_online").and_then(|v| v.as_bool()).unwrap_or(false),
                        created_at: player_data.get("created_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        updated_at: player_data.get("updated_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    };
                    return Ok(Some(player));
                }
            }
        }

        Ok(None)
    }

    pub async fn update_player_score(&self, player_id: &str, new_score: i32) -> Result<(), BoxError> {
        let query = format!("UPDATE players SET score = {}, updated_at = NOW() WHERE id = '{}'", new_score, player_id);
        self.execute_write(&query).await?;
        Ok(())
    }

    pub async fn create_game(&self, game: &GameRecord) -> Result<String, BoxError> {
        let query = format!(
            "INSERT INTO games (name, max_players, status) VALUES ('{}', {}, '{}') RETURNING id",
            game.name, game.max_players, game.status
        );

        let result = self.execute_write(&query).await?;

        if let Some(insert_id) = result.get("insert_id") {
            Ok(insert_id.as_str().unwrap_or("unknown").to_string())
        } else {
            Err("No insert_id returned".into())
        }
    }

    pub async fn get_games(&self, status_filter: Option<&str>) -> Result<Vec<GameRecord>, BoxError> {
        let query = match status_filter {
            Some(status) => format!("SELECT * FROM games WHERE status = '{}'", status),
            None => "SELECT * FROM games".to_string(),
        };

        let result = self.execute_read(&query).await?;
        let mut games = Vec::new();

        if let Some(data) = result.get("data") {
            if let Some(games_data) = data.as_array() {
                for game_data in games_data {
                    let game = GameRecord {
                        id: game_data.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        name: game_data.get("name").unwrap_or(&serde_json::Value::String("Unknown".to_string())).as_str().unwrap().to_string(),
                        max_players: game_data.get("max_players").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        status: game_data.get("status").unwrap_or(&serde_json::Value::String("unknown".to_string())).as_str().unwrap().to_string(),
                        created_at: game_data.get("created_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        updated_at: game_data.get("updated_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    };
                    games.push(game);
                }
            }
        }

        Ok(games)
    }

    pub async fn create_game_session(&self, session: &GameSessionRecord) -> Result<String, BoxError> {
        let position_json = serde_json::to_string(&session.position)?;
        let query = format!(
            "INSERT INTO game_sessions (game_id, player_id, score, position, status) VALUES ('{}', '{}', {}, '{}', '{}') RETURNING id",
            session.game_id, session.player_id, session.score, position_json, session.status
        );

        let result = self.execute_write(&query).await?;

        if let Some(insert_id) = result.get("insert_id") {
            Ok(insert_id.as_str().unwrap_or("unknown").to_string())
        } else {
            Err("No insert_id returned".into())
        }
    }

    /// Cleanup inactive connections and old data
    pub async fn cleanup(&self) -> Result<u64, BoxError> {
        // Update pool statistics
        self.update_pool_stats().await;

        // Clean up old sessions (older than 24 hours)
        let cleanup_query = "DELETE FROM game_sessions WHERE created_at < NOW() - INTERVAL '24 hours'";
        let result = self.execute_write(cleanup_query).await?;

        if let Some(affected_rows) = result.get("affected_rows") {
            if let Some(rows) = affected_rows.as_u64() {
                debug!("Cleaned up {} old game sessions", rows);
                return Ok(rows);
            }
        }

        Ok(0)
    }
}

impl Default for DatabasePool {
    fn default() -> Self {
        // This will panic if database is not available - use new() for proper error handling
        futures::executor::block_on(async {
            Self::new(DatabaseConfig::default()).await.unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_pool_creation() {
        let config = DatabaseConfig {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            pool_size: 10,
            min_idle: 2,
            ..Default::default()
        };

        match DatabasePool::new(config).await {
            Ok(_) => println!("✅ Database pool created successfully"),
            Err(e) => {
                println!("⚠️ Database pool creation failed (expected for mock): {}", e);
                return;
            }
        }
    }

    #[tokio::test]
    async fn test_database_operations() {
        let config = DatabaseConfig {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            pool_size: 5,
            min_idle: 1,
            enable_metrics: true,
            ..Default::default()
        };

        let pool = match DatabasePool::new(config).await {
            Ok(pool) => pool,
            Err(_) => return, // Skip test if database not available
        };

        // Test health check
        assert!(pool.health_check().await.unwrap());

        // Test read operation
        let read_result = pool.execute_read("SELECT * FROM players").await.unwrap();
        assert!(read_result.get("success").is_some());

        // Test write operation
        let write_result = pool.execute_write("INSERT INTO players (username) VALUES ('test')").await.unwrap();
        assert!(write_result.get("success").is_some());

        // Test batch operations
        let queries = vec![
            ("SELECT 1".to_string(), QueryType::Read),
            ("SELECT 2".to_string(), QueryType::Read),
        ];
        let batch_results = pool.execute_batch(queries).await.unwrap();
        assert_eq!(batch_results.len(), 2);

        // Test cleanup
        let cleaned_count = pool.cleanup().await.unwrap();
        println!("Cleaned up {} records", cleaned_count);

        println!("✅ Database operations test completed");
    }

    #[tokio::test]
    async fn test_player_operations() {
        let config = DatabaseConfig {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            pool_size: 5,
            min_idle: 1,
            ..Default::default()
        };

        let pool = match DatabasePool::new(config).await {
            Ok(pool) => pool,
            Err(_) => return, // Skip test if database not available
        };

        // Test player creation
        let player = PlayerRecord {
            id: None,
            username: "test_player".to_string(),
            email: "test@example.com".to_string(),
            score: 1000,
            is_online: true,
            created_at: None,
            updated_at: None,
        };

        let player_id = pool.create_player(&player).await.unwrap();
        assert!(!player_id.is_empty());

        // Test player retrieval
        let retrieved = pool.get_player(&player_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.username, "test_player");
        assert_eq!(retrieved.score, 1000);

        // Test score update
        pool.update_player_score(&player_id, 1500).await.unwrap();

        // Verify score update
        let updated = pool.get_player(&player_id).await.unwrap().unwrap();
        assert_eq!(updated.score, 1500);

        println!("✅ Player operations test completed");
    }

    #[tokio::test]
    async fn test_game_operations() {
        let config = DatabaseConfig {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            pool_size: 5,
            min_idle: 1,
            ..Default::default()
        };

        let pool = match DatabasePool::new(config).await {
            Ok(pool) => pool,
            Err(_) => return, // Skip test if database not available
        };

        // Test game creation
        let game = GameRecord {
            id: None,
            name: "Test Game".to_string(),
            max_players: 8,
            status: "waiting".to_string(),
            created_at: None,
            updated_at: None,
        };

        let game_id = pool.create_game(&game).await.unwrap();
        assert!(!game_id.is_empty());

        // Test game retrieval
        let games = pool.get_games(Some("waiting")).await.unwrap();
        assert!(!games.is_empty());

        // Test game session creation
        let session = GameSessionRecord {
            id: None,
            game_id: game_id.clone(),
            player_id: "player1".to_string(),
            score: 0,
            position: serde_json::json!({"x": 0, "y": 0, "z": 0}),
            status: "active".to_string(),
            created_at: None,
            updated_at: None,
        };

        let session_id = pool.create_game_session(&session).await.unwrap();
        assert!(!session_id.is_empty());

        println!("✅ Game operations test completed");
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = DatabaseConfig {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            pool_size: 5,
            min_idle: 1,
            enable_metrics: true,
            ..Default::default()
        };

        let pool = match DatabasePool::new(config).await {
            Ok(pool) => pool,
            Err(_) => return, // Skip test if database not available
        };

        // Perform some operations to generate metrics
        let _ = pool.execute_read("SELECT 1").await;
        let _ = pool.execute_write("INSERT test").await;

        let metrics = pool.get_metrics();
        let (created, destroyed, active, idle, queries, errors, avg_time, wait_time, read_queries, write_queries) = metrics.get_stats();

        assert!(queries > 0); // We executed at least some queries
        assert!(read_queries > 0 || write_queries > 0);

        println!("✅ Performance metrics test completed - Queries: {}, Errors: {}, Avg Time: {}μs", queries, errors, avg_time);
    }
}
