/// API endpoints for game services
/// Provides REST APIs for leaderboard, user stats, inventory, etc.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::collections::{LeaderboardEntry, User, Match, Participant, InventoryItem, UserStats};

/// API state containing database connections and caches
#[derive(Clone)]
pub struct ApiState {
    pub pocketbase_url: String,
    pub leaderboard_cache: Arc<RwLock<HashMap<String, Vec<LeaderboardEntry>>>>,
    pub user_cache: Arc<RwLock<HashMap<String, User>>>,
}

/// Query parameters for leaderboard API
#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub season: Option<String>,
    pub tier: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Query parameters for user stats API
#[derive(Debug, Deserialize)]
pub struct UserStatsQuery {
    pub user_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Leaderboard API response
#[derive(Debug, Serialize)]
pub struct LeaderboardResponse {
    pub season: String,
    pub entries: Vec<LeaderboardEntry>,
    pub total_count: u32,
    pub user_rank: Option<u32>,
    pub last_updated: DateTime<Utc>,
}

/// User stats API response
#[derive(Debug, Serialize)]
pub struct UserStatsResponse {
    pub user_id: String,
    pub stats: Vec<UserStats>,
    pub summary: UserStatsSummary,
}

/// User stats summary
#[derive(Debug, Serialize)]
pub struct UserStatsSummary {
    pub total_games: u32,
    pub total_score: u64,
    pub total_playtime_minutes: u32,
    pub avg_accuracy: f32,
    pub best_streak: u32,
    pub win_rate: f32,
}

/// Create API router with all endpoints
pub fn create_api_router(pocketbase_url: String) -> Router {
    let state = ApiState {
        pocketbase_url,
        leaderboard_cache: Arc::new(RwLock::new(HashMap::new())),
        user_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/leaderboard", get(get_leaderboard))
        .route("/leaderboard/:user_id/rank", get(get_user_rank))
        .route("/users/:user_id/stats", get(get_user_stats))
        .route("/users/:user_id/inventory", get(get_user_inventory))
        .route("/matches/:match_id/results", get(get_match_results))
        .route("/seasons", get(get_seasons))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({
        "status": "healthy",
        "service": "services",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Get leaderboard with filtering and pagination
async fn get_leaderboard(
    State(state): State<ApiState>,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, (StatusCode, Json<serde_json::Value>)> {
    let season = params.season.unwrap_or_else(|| "season_1".to_string());
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 entries
    let offset = params.offset.unwrap_or(0);

    // Check cache first
    {
        let cache = state.leaderboard_cache.read().await;
        if let Some(cached_entries) = cache.get(&season) {
            let total_count = cached_entries.len() as u32;
            let entries = cached_entries
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect::<Vec<_>>();

            return Ok(Json(LeaderboardResponse {
                season,
                entries,
                total_count,
                user_rank: None, // Would need user_id to calculate this
                last_updated: Utc::now(),
            }));
        }
    }

    // Fetch from PocketBase (mock implementation)
    match fetch_leaderboard_from_db(&state.pocketbase_url, &season, limit, offset).await {
        Ok((entries, total_count)) => {
            // Update cache
            {
                let mut cache = state.leaderboard_cache.write().await;
                cache.insert(season.clone(), entries.clone());
            }

            Ok(Json(LeaderboardResponse {
                season,
                entries,
                total_count,
                user_rank: None,
                last_updated: Utc::now(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch leaderboard: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to fetch leaderboard",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

/// Get user's rank in leaderboard
async fn get_user_rank(
    State(state): State<ApiState>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Implementation would fetch user's rank from leaderboard
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "rank": 42,
        "score": 1500,
        "percentile": 85.5
    })))
}

/// Get user statistics
async fn get_user_stats(
    State(state): State<ApiState>,
    Path(user_id): Path<String>,
    Query(params): Query<UserStatsQuery>,
) -> Result<Json<UserStatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Mock implementation - in real app would fetch from database
    let stats = vec![
        UserStats {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            date: "2024-01-15".to_string(),
            games_played: 5,
            total_score: 2500,
            total_playtime_seconds: 3600,
            avg_accuracy: 0.85,
            best_streak: 8,
            achievements_unlocked: 3,
            items_acquired: 2,
        },
        UserStats {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            date: "2024-01-14".to_string(),
            games_played: 3,
            total_score: 1800,
            total_playtime_seconds: 2700,
            avg_accuracy: 0.82,
            best_streak: 5,
            achievements_unlocked: 1,
            items_acquired: 1,
        },
    ];

    let summary = UserStatsSummary {
        total_games: stats.iter().map(|s| s.games_played).sum(),
        total_score: stats.iter().map(|s| s.total_score).sum(),
        total_playtime_minutes: stats.iter().map(|s| s.total_playtime_seconds / 60).sum(),
        avg_accuracy: if stats.is_empty() { 0.0 } else { stats.iter().map(|s| s.avg_accuracy).sum::<f32>() / stats.len() as f32 },
        best_streak: stats.iter().map(|s| s.best_streak).max().unwrap_or(0),
        win_rate: 0.75, // Would calculate from actual data
    };

    Ok(Json(UserStatsResponse {
        user_id,
        stats,
        summary,
    }))
}

/// Get user inventory
async fn get_user_inventory(
    State(_state): State<ApiState>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<InventoryItem>>, (StatusCode, Json<serde_json::Value>)> {
    // Mock implementation - in real app would fetch from database
    let inventory = vec![
        InventoryItem {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            item_type: "currency".to_string(),
            item_id: "coins".to_string(),
            quantity: 1500,
            rarity: "common".to_string(),
            acquired_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({}),
        },
        InventoryItem {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            item_type: "skin".to_string(),
            item_id: "golden_armor".to_string(),
            quantity: 1,
            rarity: "legendary".to_string(),
            acquired_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({
                "color": "#FFD700",
                "effects": ["glow", "particles"]
            }),
        },
    ];

    Ok(Json(inventory))
}

/// Get match results
async fn get_match_results(
    State(_state): State<ApiState>,
    Path(match_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Mock implementation - in real app would fetch from database
    Ok(Json(serde_json::json!({
        "match_id": match_id,
        "status": "finished",
        "duration": 1200, // seconds
        "participants": [
            {
                "user_id": "player1",
                "username": "Player One",
                "score": 2500,
                "position": 1,
                "kills": 15,
                "deaths": 3,
                "accuracy": 0.85
            },
            {
                "user_id": "player2",
                "username": "Player Two",
                "score": 1800,
                "position": 2,
                "kills": 12,
                "deaths": 5,
                "accuracy": 0.78
            }
        ]
    })))
}

/// Get available seasons
async fn get_seasons() -> Json<Vec<String>> {
    Json(vec![
        "season_1".to_string(),
        "season_2".to_string(),
        "season_3".to_string(),
    ])
}

/// Mock function to fetch leaderboard from database
async fn fetch_leaderboard_from_db(
    _pocketbase_url: &str,
    season: &str,
    limit: u32,
    offset: u32,
) -> Result<(Vec<LeaderboardEntry>, u32), Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would query PocketBase
    let mut entries = Vec::new();

    for i in 0..limit {
        let rank = offset + i + 1;
        entries.push(LeaderboardEntry {
            id: Uuid::new_v4().to_string(),
            user_id: format!("user_{}", rank),
            username: format!("Player{}", rank),
            rank,
            score: 2000 - (rank as u64 * 50),
            games_played: 50 + (rank as u32 * 2),
            win_rate: 0.8 - (rank as f32 * 0.01),
            avg_score: 1800.0 - (rank as f32 * 20.0),
            best_score: 2500 - (rank as u64 * 30),
            streak_current: 5,
            streak_best: 12,
            last_played: Utc::now(),
            tier: if rank <= 10 { "diamond" } else if rank <= 50 { "platinum" } else if rank <= 100 { "gold" } else { "silver" }.to_string(),
            season: season.to_string(),
        });
    }

    Ok((entries, 1000)) // Mock total count
}

/// Update leaderboard cache after game completion
pub async fn update_leaderboard_cache(
    state: &ApiState,
    season: &str,
    entries: Vec<LeaderboardEntry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut cache = state.leaderboard_cache.write().await;
    cache.insert(season.to_string(), entries);
    Ok(())
}

/// Save match result and update related data
pub async fn save_match_result(
    state: &ApiState,
    match_record: Match,
    participants: Vec<Participant>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would save to PocketBase
    tracing::info!(
        "Saving match result: {} - {} participants",
        match_record.id,
        participants.len()
    );

    // Update leaderboard cache if this was a ranked match
    if matches!(match_record.game_mode.as_str(), "deathmatch" | "team_deathmatch") {
        // Recalculate leaderboard for this season
        let _ = refresh_leaderboard(state, &match_record.game_mode).await;
    }

    Ok(())
}

/// Refresh leaderboard for a specific game mode/season
async fn refresh_leaderboard(
    _state: &ApiState,
    _game_mode: &str,
) -> Result<Vec<LeaderboardEntry>, Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would query and recalculate leaderboard
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_state_creation() {
        let state = ApiState {
            pocketbase_url: "http://localhost:8090".to_string(),
            leaderboard_cache: Arc::new(RwLock::new(HashMap::new())),
            user_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        assert_eq!(state.pocketbase_url, "http://localhost:8090");
    }

    #[test]
    fn test_leaderboard_response() {
        let response = LeaderboardResponse {
            season: "season_1".to_string(),
            entries: vec![],
            total_count: 0,
            user_rank: None,
            last_updated: Utc::now(),
        };

        assert_eq!(response.season, "season_1");
        assert_eq!(response.total_count, 0);
    }

    #[tokio::test]
    async fn test_mock_leaderboard_fetch() {
        let entries = fetch_leaderboard_from_db("http://localhost:8090", "season_1", 10, 0).await.unwrap();
        assert_eq!(entries.0.len(), 10);
        assert_eq!(entries.1, 1000);

        // Check ranking order
        for (i, entry) in entries.0.iter().enumerate() {
            assert_eq!(entry.rank, (i + 1) as u32);
            assert_eq!(entry.season, "season_1");
        }
    }
}
