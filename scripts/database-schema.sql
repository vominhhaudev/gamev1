-- GameV1 Database Schema for Alpha Release
-- Supports 1000+ concurrent players with performance optimizations

-- Enable necessary extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";

-- Players table - stores player information and statistics
CREATE TABLE IF NOT EXISTS players (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255), -- Optional for external auth systems
    skill_rating DECIMAL(5,2) DEFAULT 1200.00,
    games_played INTEGER DEFAULT 0,
    wins INTEGER DEFAULT 0,
    losses INTEGER DEFAULT 0,
    draws INTEGER DEFAULT 0,
    total_score BIGINT DEFAULT 0,
    is_online BOOLEAN DEFAULT false,
    last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    region VARCHAR(50) DEFAULT 'unknown',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Games table - stores game session information
CREATE TABLE IF NOT EXISTS games (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    game_mode VARCHAR(50) NOT NULL DEFAULT 'deathmatch',
    max_players INTEGER NOT NULL DEFAULT 8,
    current_players INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'waiting', -- waiting, in_progress, finished, cancelled
    map_name VARCHAR(100) DEFAULT 'default',
    host_id UUID REFERENCES players(id),
    settings JSONB, -- Game-specific settings
    started_at TIMESTAMP,
    finished_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Game sessions table - stores individual player game sessions
CREATE TABLE IF NOT EXISTS game_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_id UUID REFERENCES games(id) ON DELETE CASCADE,
    player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    position JSONB, -- Player position data
    health DECIMAL(5,2) DEFAULT 100.00,
    score INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active', -- active, finished, disconnected
    joined_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    left_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(game_id, player_id)
);

-- Matchmaking tickets table - for advanced matchmaking system
CREATE TABLE IF NOT EXISTS matchmaking_tickets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    game_mode VARCHAR(50) NOT NULL,
    preferred_latency INTEGER DEFAULT 50,
    region VARCHAR(50) DEFAULT 'unknown',
    skill_rating DECIMAL(5,2) NOT NULL,
    status VARCHAR(20) DEFAULT 'queued', -- queued, searching, matched, cancelled
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Tournaments table - for tournament system
CREATE TABLE IF NOT EXISTS tournaments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    game_mode VARCHAR(50) NOT NULL,
    format VARCHAR(50) NOT NULL, -- single_elimination, double_elimination, round_robin, swiss
    max_participants INTEGER NOT NULL,
    current_participants INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'registration', -- registration, in_progress, completed, cancelled
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    prize_pool DECIMAL(10,2) DEFAULT 0.00,
    entry_fee DECIMAL(10,2) DEFAULT 0.00,
    rules JSONB, -- Tournament-specific rules
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Tournament participants table
CREATE TABLE IF NOT EXISTS tournament_participants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tournament_id UUID REFERENCES tournaments(id) ON DELETE CASCADE,
    player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    seed INTEGER,
    current_round INTEGER DEFAULT 1,
    wins INTEGER DEFAULT 0,
    losses INTEGER DEFAULT 0,
    draws INTEGER DEFAULT 0,
    points INTEGER DEFAULT 0,
    eliminated BOOLEAN DEFAULT false,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tournament_id, player_id)
);

-- Beta testing feedback table
CREATE TABLE IF NOT EXISTS beta_feedback (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(255), -- Can be anonymous
    session_id VARCHAR(255),
    feedback_type VARCHAR(50) NOT NULL, -- bug, feature, improvement, other
    category VARCHAR(50), -- Automatically categorized
    severity VARCHAR(20) DEFAULT 'low', -- low, medium, high, critical
    priority VARCHAR(20) DEFAULT 'low', -- low, medium, high
    description TEXT NOT NULL,
    system_info JSONB, -- Browser, OS, hardware info
    game_state JSONB, -- Game state at time of feedback
    region VARCHAR(50),
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(20) DEFAULT 'new', -- new, investigating, resolved, closed
    github_issue_url VARCHAR(500) -- Link to GitHub issue if created
);

-- Player statistics table for advanced analytics
CREATE TABLE IF NOT EXISTS player_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    player_id UUID REFERENCES players(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    games_played INTEGER DEFAULT 0,
    games_won INTEGER DEFAULT 0,
    total_score BIGINT DEFAULT 0,
    average_score DECIMAL(8,2) DEFAULT 0.00,
    playtime_minutes INTEGER DEFAULT 0,
    region VARCHAR(50),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(player_id, date)
);

-- Create indexes for performance optimization
CREATE INDEX IF NOT EXISTS idx_players_online_rating ON players (is_online, skill_rating);
CREATE INDEX IF NOT EXISTS idx_players_region ON players (region);
CREATE INDEX IF NOT EXISTS idx_players_last_seen ON players (last_seen);

CREATE INDEX IF NOT EXISTS idx_games_status_created ON games (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_games_game_mode ON games (game_mode);
CREATE INDEX IF NOT EXISTS idx_games_host_id ON games (host_id);

CREATE INDEX IF NOT EXISTS idx_game_sessions_game_id ON game_sessions (game_id);
CREATE INDEX IF NOT EXISTS idx_game_sessions_player_id ON game_sessions (player_id);
CREATE INDEX IF NOT EXISTS idx_game_sessions_status ON game_sessions (status);

CREATE INDEX IF NOT EXISTS idx_matchmaking_tickets_status_expires ON matchmaking_tickets (status, expires_at);
CREATE INDEX IF NOT EXISTS idx_matchmaking_tickets_skill_rating ON matchmaking_tickets (skill_rating);
CREATE INDEX IF NOT EXISTS idx_matchmaking_tickets_game_mode ON matchmaking_tickets (game_mode);

CREATE INDEX IF NOT EXISTS idx_tournaments_status_start ON tournaments (status, start_time);
CREATE INDEX IF NOT EXISTS idx_tournaments_game_mode ON tournaments (game_mode);

CREATE INDEX IF NOT EXISTS idx_tournament_participants_tournament ON tournament_participants (tournament_id);
CREATE INDEX IF NOT EXISTS idx_tournament_participants_player ON tournament_participants (player_id);

CREATE INDEX IF NOT EXISTS idx_beta_feedback_status_timestamp ON beta_feedback (status, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_beta_feedback_category ON beta_feedback (category);
CREATE INDEX IF NOT EXISTS idx_beta_feedback_severity ON beta_feedback (severity);

CREATE INDEX IF NOT EXISTS idx_player_stats_player_date ON player_stats (player_id, date DESC);

-- Create functions for automatic timestamp updates
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for automatic timestamp updates
CREATE TRIGGER update_players_updated_at BEFORE UPDATE ON players FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_games_updated_at BEFORE UPDATE ON games FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_game_sessions_updated_at BEFORE UPDATE ON game_sessions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_matchmaking_tickets_updated_at BEFORE UPDATE ON matchmaking_tickets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_tournaments_updated_at BEFORE UPDATE ON tournaments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_beta_feedback_updated_at BEFORE UPDATE ON beta_feedback FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert sample data for testing (optional)
-- Uncomment the following lines if you want to populate with test data

-- INSERT INTO players (username, email, skill_rating, region) VALUES
--     ('test_player_1', 'test1@example.com', 1500.00, 'us-east'),
--     ('test_player_2', 'test2@example.com', 1400.00, 'us-west'),
--     ('test_player_3', 'test3@example.com', 1300.00, 'eu-west');

-- INSERT INTO games (name, game_mode, max_players, host_id, status) VALUES
--     ('Test Game 1', 'deathmatch', 8, (SELECT id FROM players WHERE username = 'test_player_1' LIMIT 1), 'waiting');

-- Performance optimization settings for high-concurrency
ALTER TABLE players SET (autovacuum_vacuum_scale_factor = 0.02);
ALTER TABLE games SET (autovacuum_vacuum_scale_factor = 0.02);
ALTER TABLE game_sessions SET (autovacuum_vacuum_scale_factor = 0.02);
ALTER TABLE matchmaking_tickets SET (autovacuum_vacuum_scale_factor = 0.01);

-- Redis Cluster Configuration for 10,000+ CCU
-- Key patterns for Redis (to be implemented in application code):
-- Session: session:{token} -> {user_id, expires_at, metadata}
-- User Profile Cache: user:{id}:profile -> {username, level, stats}
-- Leaderboard Cache: leaderboard:{season}:top -> sorted set of user scores
-- Game State Cache: game:{id}:state -> {current_players, status, settings}
-- Rate Limiting: ratelimit:{ip}:{action} -> counter with TTL
-- Matchmaking Queue: matchmaking:queue -> list of player tickets
-- Real-time Updates: pubsub:game:{id} -> publish game events
-- Distributed Locks: lock:{resource} -> mutex for coordination

-- Grant permissions
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO gamev1_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO gamev1_user;

-- Analyze tables for query planner optimization
ANALYZE players;
ANALYZE games;
ANALYZE game_sessions;
ANALYZE matchmaking_tickets;
ANALYZE tournaments;
ANALYZE tournament_participants;
ANALYZE beta_feedback;
ANALYZE player_stats;

COMMENT ON TABLE players IS 'Player information and statistics for matchmaking and leaderboards';
COMMENT ON TABLE games IS 'Game session management with support for multiple game modes';
COMMENT ON TABLE game_sessions IS 'Individual player participation in games';
COMMENT ON TABLE matchmaking_tickets IS 'Advanced matchmaking system with skill-based matching';
COMMENT ON TABLE tournaments IS 'Tournament system supporting multiple formats';
COMMENT ON TABLE beta_feedback IS 'Beta testing feedback collection and tracking';

log_success "Database schema created successfully for GameV1 Alpha Release"
