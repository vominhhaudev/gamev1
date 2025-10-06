use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Advanced matchmaking system for skill-based matching and tournaments
#[derive(Debug)]
pub struct MatchmakingSystem {
    queues: Arc<RwLock<HashMap<String, MatchmakingQueue>>>,
    tournaments: Arc<RwLock<HashMap<String, Tournament>>>,
    leagues: Arc<RwLock<HashMap<String, League>>>,
    player_ratings: Arc<RwLock<HashMap<String, PlayerRating>>>,
    metrics: Arc<MatchmakingMetrics>,
    config: MatchmakingConfig,
}

/// Matchmaking queue for different game modes
#[derive(Debug)]
pub struct MatchmakingQueue {
    game_mode: String,
    players: BinaryHeap<QueuedPlayer>,
    max_wait_time: Duration,
    min_skill_diff: f32,
    max_players_per_match: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedPlayer {
    pub player_id: String,
    pub skill_rating: f32,
    pub queued_at: u64,
    pub region: String,
    pub preferred_latency: u32,
    pub priority: i32, // Higher priority players get matched first
}

impl Ord for QueuedPlayer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then lower skill rating for balanced matches
        other.priority.cmp(&self.priority)
            .then(other.skill_rating.partial_cmp(&self.skill_rating).unwrap_or(std::cmp::Ordering::Equal))
    }
}

impl PartialOrd for QueuedPlayer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for QueuedPlayer {
    fn eq(&self, other: &Self) -> bool {
        self.player_id == other.player_id
    }
}

impl Eq for QueuedPlayer {}

/// Tournament system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub id: String,
    pub name: String,
    pub game_mode: String,
    pub format: TournamentFormat,
    pub max_participants: u32,
    pub current_participants: u32,
    pub status: TournamentStatus,
    pub start_time: u64,
    pub end_time: u64,
    pub prize_pool: Vec<Prize>,
    pub brackets: Vec<TournamentBracket>,
    pub participants: Vec<TournamentParticipant>,
    pub rules: TournamentRules,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TournamentFormat {
    SingleElimination,
    DoubleElimination,
    RoundRobin,
    Swiss,
    BattleRoyale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TournamentStatus {
    Registration,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prize {
    pub position: u32,
    pub amount: f32,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentBracket {
    pub round: u32,
    pub matches: Vec<TournamentMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentMatch {
    pub match_id: String,
    pub players: Vec<String>,
    pub winner: Option<String>,
    pub scores: HashMap<String, u32>,
    pub status: MatchStatus,
    pub scheduled_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchStatus {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentParticipant {
    pub player_id: String,
    pub player_name: String,
    pub seed: u32,
    pub current_round: u32,
    pub wins: u32,
    pub losses: u32,
    pub points: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentRules {
    pub max_round_time: u64,
    pub allow_rematches: bool,
    pub skill_range: (f32, f32),
    pub region_restriction: Option<String>,
}

/// League system for competitive play
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct League {
    pub id: String,
    pub name: String,
    pub season: u32,
    pub game_mode: String,
    pub divisions: Vec<LeagueDivision>,
    pub status: LeagueStatus,
    pub start_date: u64,
    pub end_date: u64,
    pub prize_pool: Vec<Prize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeagueStatus {
    Registration,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueDivision {
    pub division_id: String,
    pub name: String,
    pub skill_range: (f32, f32),
    pub participants: Vec<String>,
    pub standings: Vec<LeagueStanding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueStanding {
    pub player_id: String,
    pub rank: u32,
    pub points: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

/// Player rating and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRating {
    pub player_id: String,
    pub skill_rating: f32,
    pub rating_deviation: f32,
    pub volatility: f32,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub win_streak: u32,
    pub best_streak: u32,
    pub last_updated: u64,
    pub rank: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchmakingConfig {
    /// Maximum wait time for matchmaking in seconds
    pub max_wait_time: u64,
    /// Maximum skill difference for matching
    pub max_skill_diff: f32,
    /// Minimum players required for a match
    pub min_players_per_match: u32,
    /// Maximum players per match
    pub max_players_per_match: u32,
    /// Enable strict skill matching
    pub strict_skill_matching: bool,
    /// Enable region-based matching
    pub region_based_matching: bool,
    /// Enable priority queue for premium players
    pub priority_queue: bool,
}

impl Default for MatchmakingConfig {
    fn default() -> Self {
        Self {
            max_wait_time: 300, // 5 minutes
            max_skill_diff: 200.0,
            min_players_per_match: 2,
            max_players_per_match: 8,
            strict_skill_matching: false,
            region_based_matching: true,
            priority_queue: true,
        }
    }
}

/// Performance metrics for matchmaking
#[derive(Debug, Default)]
pub struct MatchmakingMetrics {
    /// Total matches created
    pub matches_created: AtomicU64,
    /// Average matchmaking time in milliseconds
    pub avg_matchmaking_time: AtomicU64,
    /// Total players queued
    pub players_queued: AtomicU64,
    /// Players currently waiting
    pub players_waiting: AtomicU64,
    /// Matches found per minute
    pub matches_per_minute: AtomicU64,
    /// Average queue size
    pub avg_queue_size: AtomicU64,
    /// Longest wait time in milliseconds
    pub longest_wait_time: AtomicU64,
}

impl MatchmakingMetrics {
    pub fn record_match_created(&self) {
        self.matches_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_matchmaking_time(&self, millis: u64) {
        let current = self.avg_matchmaking_time.load(Ordering::Relaxed);
        let new_avg = (current + millis) / 2;
        self.avg_matchmaking_time.store(new_avg, Ordering::Relaxed);

        // Update longest wait time
        let longest = self.longest_wait_time.load(Ordering::Relaxed);
        if millis > longest {
            self.longest_wait_time.store(millis, Ordering::Relaxed);
        }
    }

    pub fn record_player_queued(&self) {
        self.players_queued.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_player_waiting(&self, waiting: bool) {
        if waiting {
            self.players_waiting.fetch_add(1, Ordering::Relaxed);
        } else {
            self.players_waiting.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn update_queue_size(&self, size: u64) {
        let current = self.avg_queue_size.load(Ordering::Relaxed);
        let new_avg = (current + size) / 2;
        self.avg_queue_size.store(new_avg, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64, u64, u64) {
        (
            self.matches_created.load(Ordering::Relaxed),
            self.avg_matchmaking_time.load(Ordering::Relaxed),
            self.players_queued.load(Ordering::Relaxed),
            self.players_waiting.load(Ordering::Relaxed),
            self.matches_per_minute.load(Ordering::Relaxed),
            self.avg_queue_size.load(Ordering::Relaxed),
            self.longest_wait_time.load(Ordering::Relaxed),
        )
    }
}

impl MatchmakingSystem {
    /// Create a new matchmaking system
    pub fn new(config: MatchmakingConfig) -> Self {
        Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
            tournaments: Arc::new(RwLock::new(HashMap::new())),
            leagues: Arc::new(RwLock::new(HashMap::new())),
            player_ratings: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(MatchmakingMetrics::default()),
            config,
        }
    }

    /// Queue a player for matchmaking
    pub async fn queue_player(&self, player_id: &str, game_mode: &str, region: &str) -> Result<String, BoxError> {
        let player_rating = self.get_or_create_player_rating(player_id).await;

        let queued_player = QueuedPlayer {
            player_id: player_id.to_string(),
            skill_rating: player_rating.skill_rating,
            queued_at: chrono::Utc::now().timestamp() as u64,
            region: region.to_string(),
            preferred_latency: 50, // Default 50ms
            priority: if self.config.priority_queue { 1 } else { 0 },
        };

        let mut queues = self.queues.write().await;

        let queue = queues.entry(game_mode.to_string()).or_insert_with(|| {
            MatchmakingQueue {
                game_mode: game_mode.to_string(),
                players: BinaryHeap::new(),
                max_wait_time: Duration::from_secs(self.config.max_wait_time),
                min_skill_diff: self.config.max_skill_diff,
                max_players_per_match: self.config.max_players_per_match,
            }
        });

        queue.players.push(queued_player);

        if self.config.enable_metrics {
            self.metrics.record_player_queued();
            self.metrics.record_player_waiting(true);
            self.metrics.update_queue_size(queue.players.len() as u64);
        }

        debug!("Player {} queued for {} matchmaking", player_id, game_mode);
        Ok("queued".to_string())
    }

    /// Find matches for all game modes
    pub async fn find_matches(&self) -> Result<Vec<GameMatch>, BoxError> {
        let mut matches = Vec::new();
        let queues = self.queues.read().await;

        for (game_mode, queue) in queues.iter() {
            if let Some(new_matches) = self.find_matches_in_queue(queue).await {
                matches.extend(new_matches);
            }
        }

        if self.config.enable_metrics && !matches.is_empty() {
            for _ in &matches {
                self.metrics.record_match_created();
            }
        }

        Ok(matches)
    }

    /// Find matches within a specific queue
    async fn find_matches_in_queue(&self, queue: &MatchmakingQueue) -> Option<Vec<GameMatch>> {
        let mut players = Vec::new();
        let mut remaining_players = BinaryHeap::new();

        // Extract players that have been waiting too long or can be matched
        let now = chrono::Utc::now().timestamp() as u64;

        while let Some(player) = queue.players.peek().cloned() {
            if players.len() >= self.config.max_players_per_match as usize {
                remaining_players.push(player);
                break;
            }

            // Check if player has been waiting too long
            if now - player.queued_at > self.config.max_wait_time {
                players.push(queue.players.pop().unwrap());
                continue;
            }

            // Check if we can create a balanced match
            if self.can_create_balanced_match(&players, &player, queue).await {
                players.push(queue.players.pop().unwrap());
            } else {
                remaining_players.push(player);
                break;
            }
        }

        // Restore remaining players to queue
        while let Some(player) = remaining_players.pop() {
            // In a real implementation, this would push back to the queue
            // For now, we'll just continue
        }

        if players.len() >= self.config.min_players_per_match as usize {
            Some(vec![self.create_match_from_players(&players, &queue.game_mode)])
        } else {
            None
        }
    }

    /// Check if we can create a balanced match with the given players
    async fn can_create_balanced_match(&self, current_players: &[QueuedPlayer], new_player: &QueuedPlayer, queue: &MatchmakingQueue) -> bool {
        if current_players.is_empty() {
            return true;
        }

        let avg_skill = current_players.iter().map(|p| p.skill_rating).sum::<f32>() / current_players.len() as f32;
        let skill_diff = (new_player.skill_rating - avg_skill).abs();

        // Check skill balance
        if skill_diff > queue.min_skill_diff && self.config.strict_skill_matching {
            return false;
        }

        // Check region compatibility (simplified)
        if self.config.region_based_matching {
            let regions: std::collections::HashSet<&String> = current_players.iter().map(|p| &p.region).collect();
            if !regions.contains(&new_player.region) && regions.len() >= 2 {
                return false;
            }
        }

        true
    }

    /// Create a match from queued players
    fn create_match_from_players(&self, players: &[QueuedPlayer], game_mode: &str) -> GameMatch {
        GameMatch {
            match_id: Uuid::new_v4().to_string(),
            game_mode: game_mode.to_string(),
            players: players.iter().map(|p| p.player_id.clone()).collect(),
            max_players: players.len() as u32,
            status: MatchStatus::Scheduled,
            skill_range: self.calculate_skill_range(players),
            region: self.determine_match_region(players),
            created_at: chrono::Utc::now().timestamp() as u64,
            scheduled_start: chrono::Utc::now().timestamp() as u64 + 60, // Start in 1 minute
        }
    }

    /// Calculate skill range for the match
    fn calculate_skill_range(&self, players: &[QueuedPlayer]) -> (f32, f32) {
        let skill_ratings: Vec<f32> = players.iter().map(|p| p.skill_rating).collect();
        let min_skill = skill_ratings.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_skill = skill_ratings.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        (min_skill, max_skill)
    }

    /// Determine the best region for the match
    fn determine_match_region(&self, players: &[QueuedPlayer]) -> String {
        let mut region_count: HashMap<String, u32> = HashMap::new();

        for player in players {
            *region_count.entry(player.region.clone()).or_insert(0) += 1;
        }

        region_count
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(region, _)| region)
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// ELO Rating System Implementation
    pub async fn update_player_rating(&self, player_id: &str, game_result: &GameResult) -> Result<(), BoxError> {
        let mut ratings = self.player_ratings.write().await;

        let player_rating = ratings.entry(player_id.to_string()).or_insert_with(|| {
            PlayerRating {
                player_id: player_id.to_string(),
                skill_rating: 1200.0, // Default ELO rating
                rating_deviation: 200.0,
                volatility: 0.06,
                games_played: 0,
                wins: 0,
                losses: 0,
                draws: 0,
                win_streak: 0,
                best_streak: 0,
                last_updated: chrono::Utc::now().timestamp() as u64,
                rank: None,
                tier: None,
            }
        });

        // Calculate ELO rating change
        let rating_change = self.calculate_elo_change(player_rating, game_result);

        player_rating.skill_rating += rating_change;
        player_rating.games_played += 1;
        player_rating.last_updated = chrono::Utc::now().timestamp() as u64;

        // Update win/loss/draw statistics
        match game_result.outcome {
            GameOutcome::Win => {
                player_rating.wins += 1;
                player_rating.win_streak += 1;
                player_rating.best_streak = player_rating.best_streak.max(player_rating.win_streak);
            }
            GameOutcome::Loss => {
                player_rating.losses += 1;
                player_rating.win_streak = 0;
            }
            GameOutcome::Draw => {
                player_rating.draws += 1;
                player_rating.win_streak = 0;
            }
        }

        // Update rank and tier
        self.update_player_rank_and_tier(player_rating);

        debug!("Updated rating for player {}: {} -> {}", player_id, player_rating.skill_rating - rating_change, player_rating.skill_rating);
        Ok(())
    }

    /// Calculate ELO rating change based on game result
    fn calculate_elo_change(&self, player_rating: &PlayerRating, game_result: &GameResult) -> f32 {
        let mut total_opponent_rating = 0.0;
        let opponent_count = game_result.opponent_ratings.len();

        if opponent_count == 0 {
            return 0.0;
        }

        for opponent_rating in &game_result.opponent_ratings {
            total_opponent_rating += opponent_rating;
        }

        let avg_opponent_rating = total_opponent_rating / opponent_count as f32;
        let rating_diff = avg_opponent_rating - player_rating.skill_rating;

        // ELO formula: K * (S - E)
        // Where S is score (1 for win, 0.5 for draw, 0 for loss)
        // E is expected score
        let k_factor = self.get_k_factor(player_rating);
        let expected_score = 1.0 / (1.0 + 10.0_f32.powf(-rating_diff / 400.0));

        let actual_score = match game_result.outcome {
            GameOutcome::Win => 1.0,
            GameOutcome::Draw => 0.5,
            GameOutcome::Loss => 0.0,
        };

        k_factor * (actual_score - expected_score)
    }

    /// Get K-factor based on player rating and games played
    fn get_k_factor(&self, player_rating: &PlayerRating) -> f32 {
        if player_rating.games_played < 30 {
            40.0 // Higher K-factor for new players
        } else if player_rating.skill_rating > 2400.0 {
            16.0 // Lower K-factor for high-rated players
        } else {
            32.0 // Standard K-factor
        }
    }

    /// Update player rank and tier based on skill rating
    fn update_player_rank_and_tier(&self, player_rating: &mut PlayerRating) {
        // Determine tier based on skill rating
        player_rating.tier = Some(match player_rating.skill_rating {
            r if r >= 2500.0 => "Master".to_string(),
            r if r >= 2200.0 => "Diamond".to_string(),
            r if r >= 1900.0 => "Platinum".to_string(),
            r if r >= 1600.0 => "Gold".to_string(),
            r if r >= 1300.0 => "Silver".to_string(),
            _ => "Bronze".to_string(),
        });

        // Calculate rank within tier (simplified)
        player_rating.rank = Some(format!("{} {}", player_rating.tier.as_ref().unwrap(), player_rating.games_played));
    }

    /// Get or create player rating
    async fn get_or_create_player_rating(&self, player_id: &str) -> PlayerRating {
        let ratings = self.player_ratings.read().await;

        ratings.get(player_id).cloned().unwrap_or_else(|| {
            PlayerRating {
                player_id: player_id.to_string(),
                skill_rating: 1200.0, // Default ELO rating
                rating_deviation: 200.0,
                volatility: 0.06,
                games_played: 0,
                wins: 0,
                losses: 0,
                draws: 0,
                win_streak: 0,
                best_streak: 0,
                last_updated: chrono::Utc::now().timestamp() as u64,
                rank: None,
                tier: None,
            }
        })
    }

    /// Tournament Management
    pub async fn create_tournament(&self, tournament: Tournament) -> Result<(), BoxError> {
        let mut tournaments = self.tournaments.write().await;
        tournaments.insert(tournament.id.clone(), tournament);

        info!("Created tournament: {}", tournament.id);
        Ok(())
    }

    pub async fn get_tournament(&self, tournament_id: &str) -> Option<Tournament> {
        let tournaments = self.tournaments.read().await;
        tournaments.get(tournament_id).cloned()
    }

    pub async fn register_player_for_tournament(&self, tournament_id: &str, player_id: &str, player_name: &str) -> Result<(), BoxError> {
        let mut tournaments = self.tournaments.write().await;

        if let Some(tournament) = tournaments.get_mut(tournament_id) {
            if tournament.status != TournamentStatus::Registration {
                return Err("Tournament is not accepting registrations".into());
            }

            if tournament.current_participants >= tournament.max_participants {
                return Err("Tournament is full".into());
            }

            // Check player rating for skill requirements
            let player_rating = self.get_or_create_player_rating(player_id).await;
            if player_rating.skill_rating < tournament.rules.skill_range.0 ||
               player_rating.skill_rating > tournament.rules.skill_range.1 {
                return Err("Player skill rating outside tournament requirements".into());
            }

            // Add player to tournament
            let participant = TournamentParticipant {
                player_id: player_id.to_string(),
                player_name: player_name.to_string(),
                seed: tournament.current_participants + 1,
                current_round: 1,
                wins: 0,
                losses: 0,
                points: 0,
            };

            tournament.participants.push(participant);
            tournament.current_participants += 1;

            debug!("Player {} registered for tournament {}", player_id, tournament_id);
            Ok(())
        } else {
            Err(format!("Tournament {} not found", tournament_id).into())
        }
    }

    /// League Management
    pub async fn create_league(&self, league: League) -> Result<(), BoxError> {
        let mut leagues = self.leagues.write().await;
        leagues.insert(league.id.clone(), league);

        info!("Created league: {}", league.id);
        Ok(())
    }

    pub async fn get_league(&self, league_id: &str) -> Option<League> {
        let leagues = self.leagues.read().await;
        leagues.get(league_id).cloned()
    }

    /// Get matchmaking metrics
    pub fn get_metrics(&self) -> Arc<MatchmakingMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Get current queue sizes
    pub async fn get_queue_sizes(&self) -> HashMap<String, usize> {
        let queues = self.queues.read().await;
        queues
            .iter()
            .map(|(game_mode, queue)| (game_mode.clone(), queue.players.len()))
            .collect()
    }

    /// Get player rating
    pub async fn get_player_rating(&self, player_id: &str) -> Option<PlayerRating> {
        let ratings = self.player_ratings.read().await;
        ratings.get(player_id).cloned()
    }

    /// Cleanup expired players from queues
    pub async fn cleanup_expired_queues(&self) -> Result<u64, BoxError> {
        let mut cleaned_count = 0u64;
        let now = chrono::Utc::now().timestamp() as u64;

        {
            let mut queues = self.queues.write().await;

            for queue in queues.values_mut() {
                let initial_size = queue.players.len();
                queue.players.retain(|player| {
                    now - player.queued_at <= self.config.max_wait_time
                });
                cleaned_count += (initial_size - queue.players.len()) as u64;

                if self.config.enable_metrics {
                    self.metrics.update_queue_size(queue.players.len() as u64);
                    self.metrics.record_player_waiting(false);
                }
            }
        }

        debug!("Cleaned up {} expired players from matchmaking queues", cleaned_count);
        Ok(cleaned_count)
    }
}

/// Game match result
#[derive(Debug, Clone)]
pub struct GameMatch {
    pub match_id: String,
    pub game_mode: String,
    pub players: Vec<String>,
    pub max_players: u32,
    pub status: MatchStatus,
    pub skill_range: (f32, f32),
    pub region: String,
    pub created_at: u64,
    pub scheduled_start: u64,
}

/// Game result for rating updates
#[derive(Debug, Clone)]
pub struct GameResult {
    pub player_id: String,
    pub outcome: GameOutcome,
    pub opponent_ratings: Vec<f32>,
    pub game_mode: String,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_matchmaking_system_creation() {
        let config = MatchmakingConfig::default();
        let system = MatchmakingSystem::new(config);

        assert_eq!(system.get_queue_sizes().await.len(), 0);

        println!("✅ Matchmaking system creation test completed");
    }

    #[tokio::test]
    async fn test_player_queuing() {
        let config = MatchmakingConfig::default();
        let system = MatchmakingSystem::new(config);

        // Queue players for matchmaking
        system.queue_player("player1", "deathmatch", "us-east").await.unwrap();
        system.queue_player("player2", "deathmatch", "us-east").await.unwrap();
        system.queue_player("player3", "deathmatch", "us-west").await.unwrap();

        let queue_sizes = system.get_queue_sizes().await;
        assert_eq!(queue_sizes.get("deathmatch"), Some(&3));

        println!("✅ Player queuing test completed");
    }

    #[tokio::test]
    async fn test_match_creation() {
        let config = MatchmakingConfig {
            min_players_per_match: 2,
            max_players_per_match: 4,
            max_wait_time: 60,
            ..Default::default()
        };
        let system = MatchmakingSystem::new(config);

        // Queue minimum players for a match
        system.queue_player("player1", "deathmatch", "us-east").await.unwrap();
        system.queue_player("player2", "deathmatch", "us-east").await.unwrap();

        // Find matches
        let matches = system.find_matches().await.unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].players.len(), 2);

        println!("✅ Match creation test completed");
    }

    #[tokio::test]
    async fn test_elo_rating_system() {
        let config = MatchmakingConfig::default();
        let system = MatchmakingSystem::new(config);

        // Create initial player rating
        let game_result = GameResult {
            player_id: "player1".to_string(),
            outcome: GameOutcome::Win,
            opponent_ratings: vec![1200.0, 1300.0], // Two opponents
            game_mode: "deathmatch".to_string(),
            duration_seconds: 300,
        };

        system.update_player_rating("player1", &game_result).await.unwrap();

        let player_rating = system.get_player_rating("player1").await.unwrap();
        assert!(player_rating.skill_rating > 1200.0); // Should have gained rating
        assert_eq!(player_rating.wins, 1);
        assert_eq!(player_rating.games_played, 1);

        println!("✅ ELO rating system test completed");
    }

    #[tokio::test]
    async fn test_tournament_creation() {
        let config = MatchmakingConfig::default();
        let system = MatchmakingSystem::new(config);

        let tournament = Tournament {
            id: "tournament1".to_string(),
            name: "Test Tournament".to_string(),
            game_mode: "deathmatch".to_string(),
            format: TournamentFormat::SingleElimination,
            max_participants: 16,
            current_participants: 0,
            status: TournamentStatus::Registration,
            start_time: chrono::Utc::now().timestamp() as u64 + 3600,
            end_time: chrono::Utc::now().timestamp() as u64 + 7200,
            prize_pool: vec![Prize { position: 1, amount: 100.0, currency: "USD".to_string() }],
            brackets: vec![],
            participants: vec![],
            rules: TournamentRules {
                max_round_time: 600,
                allow_rematches: false,
                skill_range: (1000.0, 2000.0),
                region_restriction: Some("us-east".to_string()),
            },
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        system.create_tournament(tournament).await.unwrap();

        let retrieved = system.get_tournament("tournament1").await.unwrap();
        assert_eq!(retrieved.name, "Test Tournament");
        assert_eq!(retrieved.max_participants, 16);

        println!("✅ Tournament creation test completed");
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = MatchmakingConfig::default();
        let system = MatchmakingSystem::new(config);

        let metrics = system.get_metrics();
        let (matches, avg_time, queued, waiting, per_minute, avg_queue, longest) = metrics.get_stats();

        assert_eq!(matches, 0);
        assert_eq!(queued, 0);

        // Queue some players to generate metrics
        system.queue_player("player1", "deathmatch", "us-east").await.unwrap();
        system.queue_player("player2", "deathmatch", "us-east").await.unwrap();

        // Create a match to generate more metrics
        let matches = system.find_matches().await.unwrap();
        if !matches.is_empty() {
            let (matches_after, _, _, _, _, _, _) = metrics.get_stats();
            assert_eq!(matches_after, 1);
        }

        println!("✅ Performance metrics test completed");
    }
}
