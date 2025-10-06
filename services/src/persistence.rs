/// Game persistence layer
/// Handles saving game results, updating leaderboards, and maintaining game history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::collections::{Match, Participant, LeaderboardEntry, User, UserStats, InventoryItem};

/// Persistence service state
pub struct PersistenceState {
    pub pocketbase_url: String,
    pub match_history: RwLock<HashMap<String, Match>>,
    pub participant_history: RwLock<HashMap<String, Vec<Participant>>>,
}

impl Clone for PersistenceState {
    fn clone(&self) -> Self {
        Self {
            pocketbase_url: self.pocketbase_url.clone(),
            match_history: RwLock::new(HashMap::new()),
            participant_history: RwLock::new(HashMap::new()),
        }
    }
}

/// Game result data structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    pub match_id: String,
    pub room_id: String,
    pub game_mode: String,
    pub map_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: u32,
    pub participants: Vec<GameParticipant>,
    pub winner_team: Option<String>,
    pub total_score: u64,
    pub settings: serde_json::Value,
}

/// Individual participant result in a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameParticipant {
    pub user_id: String,
    pub username: String,
    pub team: Option<String>,
    pub final_position: u32,
    pub score: u64,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub accuracy: f32,
    pub playtime_seconds: u32,
    pub is_winner: bool,
    pub stats: serde_json::Value,
}

impl From<&Participant> for GameParticipant {
    fn from(participant: &Participant) -> Self {
        Self {
            user_id: participant.user_id.clone(),
            username: participant.username.clone(),
            team: participant.team.clone(),
            final_position: participant.position,
            score: participant.score,
            kills: participant.kills,
            deaths: participant.deaths,
            assists: participant.assists,
            accuracy: participant.accuracy,
            playtime_seconds: participant.playtime_seconds,
            is_winner: participant.is_winner,
            stats: participant.stats.clone(),
        }
    }
}

/// Create persistence state
pub fn create_persistence_state(pocketbase_url: String) -> PersistenceState {
    PersistenceState {
        pocketbase_url,
        match_history: RwLock::new(HashMap::new()),
        participant_history: RwLock::new(HashMap::new()),
    }
}

/// Save complete game result
pub async fn save_game_result(
    state: &PersistenceState,
    game_result: GameResult,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        "Saving game result for match {} with {} participants",
        game_result.match_id,
        game_result.participants.len()
    );

    // Create match record
    let match_record = Match {
        id: game_result.match_id.clone(),
        room_id: game_result.room_id.clone(),
        game_mode: game_result.game_mode.clone(),
        map_name: game_result.map_name.clone(),
        max_players: game_result.participants.len() as u32,
        status: "finished".to_string(),
        start_time: Some(game_result.start_time),
        end_time: Some(game_result.end_time),
        duration_seconds: Some(game_result.duration_seconds),
        winner_team: game_result.winner_team.clone(),
        total_score: game_result.total_score,
        settings: game_result.settings.clone(),
        created: game_result.start_time,
        updated: game_result.end_time,
    };

    // Create participant records
    let mut participants = Vec::new();
    for (position, participant) in game_result.participants.iter().enumerate() {
        let participant_record = Participant {
            id: Uuid::new_v4().to_string(),
            match_id: game_result.match_id.clone(),
            user_id: participant.user_id.clone(),
            username: participant.username.clone(),
            team: participant.team.clone(),
            position: position as u32 + 1,
            score: participant.score,
            kills: participant.kills,
            deaths: participant.deaths,
            assists: participant.assists,
            accuracy: participant.accuracy,
            playtime_seconds: participant.playtime_seconds,
            joined_at: game_result.start_time,
            left_at: Some(game_result.end_time),
            is_winner: participant.is_winner,
            stats: participant.stats.clone(),
        };
        participants.push(participant_record);
    }

    // Save to database (mock implementation)
    save_match_to_database(&state.pocketbase_url, &match_record).await?;
    save_participants_to_database(&state.pocketbase_url, &participants).await?;

    // Update in-memory cache
    {
        let mut matches = state.match_history.write().await;
        matches.insert(game_result.match_id.clone(), match_record);
    }

    {
        let mut participant_map = state.participant_history.write().await;
        participant_map.insert(game_result.match_id.clone(), participants);
    }

    // Update leaderboards and user stats
    update_leaderboards_and_stats(state, &game_result).await?;

    Ok(())
}

/// Save match record to PocketBase
async fn save_match_to_database(
    _pocketbase_url: &str,
    match_record: &Match,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would POST to PocketBase API
    tracing::debug!("Saving match {} to database", match_record.id);
    Ok(())
}

/// Save participants to PocketBase
async fn save_participants_to_database(
    _pocketbase_url: &str,
    participants: &[Participant],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would POST batch to PocketBase API
    tracing::debug!("Saving {} participants to database", participants.len());
    Ok(())
}

/// Update leaderboards and user statistics after game completion
async fn update_leaderboards_and_stats(
    state: &PersistenceState,
    game_result: &GameResult,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Update user statistics for each participant
    for participant in &game_result.participants {
        update_user_stats(state, participant).await?;
    }

    // Update leaderboard rankings
    update_leaderboard_rankings(state, &game_result.game_mode).await?;

    Ok(())
}

/// Update individual user statistics
async fn update_user_stats(
    _state: &PersistenceState,
    participant: &GameParticipant,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would:
    // 1. Fetch current user stats
    // 2. Update with new game data
    // 3. Save back to database
    // 4. Update user level/XP if needed

    tracing::debug!(
        "Updating stats for user {}: score={}, kills={}, deaths={}",
        participant.user_id,
        participant.score,
        participant.kills,
        participant.deaths
    );

    Ok(())
}

/// Update leaderboard rankings for a game mode
async fn update_leaderboard_rankings(
    _state: &PersistenceState,
    _game_mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would:
    // 1. Query all users' recent performance
    // 2. Calculate new rankings
    // 3. Update leaderboard collection
    // 4. Update user tiers

    tracing::debug!("Updating leaderboard rankings for game mode: {}", _game_mode);
    Ok(())
}

/// Get match history for a user
pub async fn get_user_match_history(
    state: &PersistenceState,
    user_id: &str,
    limit: Option<usize>,
) -> Result<Vec<Match>, Box<dyn std::error::Error + Send + Sync>> {
    let matches = state.match_history.read().await;
    let participant_history = state.participant_history.read().await;

    let mut user_matches = Vec::new();

    // Find all matches where user participated
    for (match_id, participants) in participant_history.iter() {
        if participants.iter().any(|p| p.user_id == user_id) {
            if let Some(match_record) = matches.get(match_id) {
                user_matches.push(match_record.clone());
            }
        }
    }

    // Sort by end time (newest first)
    user_matches.sort_by(|a, b| {
        b.end_time.cmp(&a.end_time)
    });

    // Apply limit
    if let Some(limit) = limit {
        user_matches.truncate(limit);
    }

    Ok(user_matches)
}

/// Get user statistics summary
pub async fn get_user_stats_summary(
    _state: &PersistenceState,
    user_id: &str,
) -> Result<UserStatsSummary, Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would aggregate from user_stats collection
    Ok(UserStatsSummary {
        total_games: 150,
        total_score: 45000,
        total_playtime_minutes: 7200,
        avg_accuracy: 0.82,
        best_streak: 15,
        win_rate: 0.73,
    })
}

/// User statistics summary structure
#[derive(Debug, Serialize)]
pub struct UserStatsSummary {
    pub total_games: u32,
    pub total_score: u64,
    pub total_playtime_minutes: u32,
    pub avg_accuracy: f32,
    pub best_streak: u32,
    pub win_rate: f32,
}

/// Award items to user (e.g., after completing achievements or purchases)
pub async fn award_inventory_item(
    _state: &PersistenceState,
    user_id: &str,
    item_type: &str,
    item_id: &str,
    quantity: u32,
    rarity: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let item = InventoryItem {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.to_string(),
        item_type: item_type.to_string(),
        item_id: item_id.to_string(),
        quantity,
        rarity: rarity.to_string(),
        acquired_at: Utc::now(),
        expires_at: None,
        metadata: serde_json::json!({}),
    };

    // Mock implementation - in real app would save to inventory collection
    tracing::info!(
        "Awarded {} x {} ({}) to user {}",
        quantity, item_id, rarity, user_id
    );

    Ok(item.id)
}

/// Get user inventory
pub async fn get_user_inventory(
    _state: &PersistenceState,
    user_id: &str,
) -> Result<Vec<InventoryItem>, Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would query inventory collection
    Ok(vec![
        InventoryItem {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            item_type: "currency".to_string(),
            item_id: "coins".to_string(),
            quantity: 1500,
            rarity: "common".to_string(),
            acquired_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({}),
        }
    ])
}

/// Clean up old data (background job)
pub async fn cleanup_old_data(
    _state: &PersistenceState,
    older_than_days: u32,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    // Mock implementation - in real app would:
    // 1. Delete old matches and participants
    // 2. Archive old user stats
    // 3. Remove expired inventory items

    let cutoff_date = Utc::now() - chrono::Duration::days(older_than_days as i64);
    tracing::info!("Cleaning up data older than {}", cutoff_date);

    // Mock cleanup count
    Ok(150) // Records cleaned up
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_result_creation() {
        let participant = GameParticipant {
            user_id: "user1".to_string(),
            username: "Player1".to_string(),
            team: Some("red".to_string()),
            final_position: 1,
            score: 2500,
            kills: 15,
            deaths: 3,
            assists: 5,
            accuracy: 0.85,
            playtime_seconds: 600,
            is_winner: true,
            stats: serde_json::json!({}),
        };

        let game_result = GameResult {
            match_id: "match_123".to_string(),
            room_id: "room_456".to_string(),
            game_mode: "deathmatch".to_string(),
            map_name: "arena_1".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_seconds: 600,
            participants: vec![participant],
            winner_team: Some("red".to_string()),
            total_score: 2500,
            settings: serde_json::json!({}),
        };

        assert_eq!(game_result.match_id, "match_123");
        assert_eq!(game_result.participants.len(), 1);
        assert_eq!(game_result.winner_team, Some("red".to_string()));
    }

    #[test]
    fn test_persistence_state_creation() {
        let state = create_persistence_state("http://localhost:8090".to_string());
        assert_eq!(state.pocketbase_url, "http://localhost:8090");
    }

    #[tokio::test]
    async fn test_user_stats_summary() {
        let summary = UserStatsSummary {
            total_games: 100,
            total_score: 50000,
            total_playtime_minutes: 3600,
            avg_accuracy: 0.85,
            best_streak: 12,
            win_rate: 0.75,
        };

        assert_eq!(summary.total_games, 100);
        assert_eq!(summary.win_rate, 0.75);
    }
}
