/// PocketBase collection schemas for game persistence
/// Defines data structures for users, matches, participants, leaderboard, inventory

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// User collection schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub level: u32,
    pub xp: u64,
    pub total_games_played: u32,
    pub total_wins: u32,
    pub total_score: u64,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: String::new(),
            email: String::new(),
            username: String::new(),
            display_name: String::new(),
            avatar: None,
            level: 1,
            xp: 0,
            total_games_played: 0,
            total_wins: 0,
            total_score: 0,
            created: Utc::now(),
            updated: Utc::now(),
        }
    }
}

/// Match/Game session collection schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    pub room_id: String,
    pub game_mode: String, // "deathmatch", "team_deathmatch", "capture_the_flag"
    pub map_name: String,
    pub max_players: u32,
    pub status: String, // "waiting", "starting", "in_progress", "finished", "cancelled"
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u32>,
    pub winner_team: Option<String>,
    pub total_score: u64,
    pub settings: serde_json::Value, // Game-specific settings
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Default for Match {
    fn default() -> Self {
        Self {
            id: String::new(),
            room_id: String::new(),
            game_mode: "deathmatch".to_string(),
            map_name: "default".to_string(),
            max_players: 8,
            status: "waiting".to_string(),
            start_time: None,
            end_time: None,
            duration_seconds: None,
            winner_team: None,
            total_score: 0,
            settings: serde_json::json!({}),
            created: Utc::now(),
            updated: Utc::now(),
        }
    }
}

/// Participant (player in a match) collection schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub match_id: String,
    pub user_id: String,
    pub username: String,
    pub team: Option<String>,
    pub position: u32, // Final position (1st, 2nd, etc.)
    pub score: u64,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub accuracy: f32,
    pub playtime_seconds: u32,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub is_winner: bool,
    pub stats: serde_json::Value, // Additional game-specific stats
}

impl Default for Participant {
    fn default() -> Self {
        Self {
            id: String::new(),
            match_id: String::new(),
            user_id: String::new(),
            username: String::new(),
            team: None,
            position: 0,
            score: 0,
            kills: 0,
            deaths: 0,
            assists: 0,
            accuracy: 0.0,
            playtime_seconds: 0,
            joined_at: Utc::now(),
            left_at: None,
            is_winner: false,
            stats: serde_json::json!({}),
        }
    }
}

/// Leaderboard entry schema (computed/aggregated data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub rank: u32,
    pub score: u64,
    pub games_played: u32,
    pub win_rate: f32,
    pub avg_score: f32,
    pub best_score: u64,
    pub streak_current: u32,
    pub streak_best: u32,
    pub last_played: DateTime<Utc>,
    pub tier: String, // "bronze", "silver", "gold", "platinum", "diamond", "master"
    pub season: String, // Current season identifier
}

impl Default for LeaderboardEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            user_id: String::new(),
            username: String::new(),
            rank: 0,
            score: 0,
            games_played: 0,
            win_rate: 0.0,
            avg_score: 0.0,
            best_score: 0,
            streak_current: 0,
            streak_best: 0,
            last_played: Utc::now(),
            tier: "bronze".to_string(),
            season: "season_1".to_string(),
        }
    }
}

/// User inventory/item collection schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub user_id: String,
    pub item_type: String, // "skin", "weapon", "consumable", "currency"
    pub item_id: String,  // Unique identifier for the item type
    pub quantity: u32,
    pub rarity: String,   // "common", "rare", "epic", "legendary"
    pub acquired_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value, // Additional item-specific data
}

impl Default for InventoryItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            user_id: String::new(),
            item_type: "currency".to_string(),
            item_id: "coins".to_string(),
            quantity: 0,
            rarity: "common".to_string(),
            acquired_at: Utc::now(),
            expires_at: None,
            metadata: serde_json::json!({}),
        }
    }
}

/// Achievement/Progression collection schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub user_id: String,
    pub achievement_id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String, // "gameplay", "social", "progression"
    pub rarity: String,   // "common", "rare", "epic", "legendary"
    pub points: u32,
    pub unlocked_at: DateTime<Utc>,
    pub progress: serde_json::Value, // Progress towards unlocking
}

impl Default for Achievement {
    fn default() -> Self {
        Self {
            id: String::new(),
            user_id: String::new(),
            achievement_id: String::new(),
            name: String::new(),
            description: String::new(),
            icon: String::new(),
            category: "gameplay".to_string(),
            rarity: "common".to_string(),
            points: 0,
            unlocked_at: Utc::now(),
            progress: serde_json::json!({}),
        }
    }
}

/// Daily/Weekly stats aggregation for performance optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub id: String,
    pub user_id: String,
    pub date: String, // YYYY-MM-DD format
    pub games_played: u32,
    pub total_score: u64,
    pub total_playtime_seconds: u32,
    pub avg_accuracy: f32,
    pub best_streak: u32,
    pub achievements_unlocked: u32,
    pub items_acquired: u32,
}

impl Default for UserStats {
    fn default() -> Self {
        Self {
            id: String::new(),
            user_id: String::new(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            games_played: 0,
            total_score: 0,
            total_playtime_seconds: 0,
            avg_accuracy: 0.0,
            best_streak: 0,
            achievements_unlocked: 0,
            items_acquired: 0,
        }
    }
}

/// PocketBase collection configuration
pub struct CollectionConfig {
    pub name: &'static str,
    pub schema: Vec<FieldConfig>,
}

pub struct FieldConfig {
    pub name: &'static str,
    pub field_type: &'static str,
    pub required: bool,
    pub options: Option<serde_json::Value>,
}

/// Define all PocketBase collections needed for the game
pub fn get_collection_configs() -> Vec<CollectionConfig> {
    vec![
        CollectionConfig {
            name: "users",
            schema: vec![
                FieldConfig { name: "email", field_type: "email", required: true, options: None },
                FieldConfig { name: "username", field_type: "text", required: true, options: None },
                FieldConfig { name: "display_name", field_type: "text", required: true, options: None },
                FieldConfig { name: "avatar", field_type: "file", required: false, options: None },
                FieldConfig { name: "level", field_type: "number", required: false, options: None },
                FieldConfig { name: "xp", field_type: "number", required: false, options: None },
                FieldConfig { name: "total_games_played", field_type: "number", required: false, options: None },
                FieldConfig { name: "total_wins", field_type: "number", required: false, options: None },
                FieldConfig { name: "total_score", field_type: "number", required: false, options: None },
            ],
        },
        CollectionConfig {
            name: "matches",
            schema: vec![
                FieldConfig { name: "room_id", field_type: "text", required: true, options: None },
                FieldConfig { name: "game_mode", field_type: "select", required: true, options: Some(serde_json::json!(["deathmatch", "team_deathmatch", "capture_the_flag"])) },
                FieldConfig { name: "map_name", field_type: "text", required: true, options: None },
                FieldConfig { name: "max_players", field_type: "number", required: true, options: None },
                FieldConfig { name: "status", field_type: "select", required: true, options: Some(serde_json::json!(["waiting", "starting", "in_progress", "finished", "cancelled"])) },
                FieldConfig { name: "start_time", field_type: "date", required: false, options: None },
                FieldConfig { name: "end_time", field_type: "date", required: false, options: None },
                FieldConfig { name: "duration_seconds", field_type: "number", required: false, options: None },
                FieldConfig { name: "winner_team", field_type: "text", required: false, options: None },
                FieldConfig { name: "total_score", field_type: "number", required: false, options: None },
                FieldConfig { name: "settings", field_type: "json", required: false, options: None },
            ],
        },
        CollectionConfig {
            name: "participants",
            schema: vec![
                FieldConfig { name: "match_id", field_type: "relation", required: true, options: Some(serde_json::json!({"collectionName": "matches"})) },
                FieldConfig { name: "user_id", field_type: "relation", required: true, options: Some(serde_json::json!({"collectionName": "users"})) },
                FieldConfig { name: "username", field_type: "text", required: true, options: None },
                FieldConfig { name: "team", field_type: "text", required: false, options: None },
                FieldConfig { name: "position", field_type: "number", required: false, options: None },
                FieldConfig { name: "score", field_type: "number", required: false, options: None },
                FieldConfig { name: "kills", field_type: "number", required: false, options: None },
                FieldConfig { name: "deaths", field_type: "number", required: false, options: None },
                FieldConfig { name: "assists", field_type: "number", required: false, options: None },
                FieldConfig { name: "accuracy", field_type: "number", required: false, options: None },
                FieldConfig { name: "playtime_seconds", field_type: "number", required: false, options: None },
                FieldConfig { name: "joined_at", field_type: "date", required: true, options: None },
                FieldConfig { name: "left_at", field_type: "date", required: false, options: None },
                FieldConfig { name: "is_winner", field_type: "bool", required: false, options: None },
                FieldConfig { name: "stats", field_type: "json", required: false, options: None },
            ],
        },
        CollectionConfig {
            name: "leaderboard",
            schema: vec![
                FieldConfig { name: "user_id", field_type: "relation", required: true, options: Some(serde_json::json!({"collectionName": "users"})) },
                FieldConfig { name: "username", field_type: "text", required: true, options: None },
                FieldConfig { name: "rank", field_type: "number", required: true, options: None },
                FieldConfig { name: "score", field_type: "number", required: true, options: None },
                FieldConfig { name: "games_played", field_type: "number", required: false, options: None },
                FieldConfig { name: "win_rate", field_type: "number", required: false, options: None },
                FieldConfig { name: "avg_score", field_type: "number", required: false, options: None },
                FieldConfig { name: "best_score", field_type: "number", required: false, options: None },
                FieldConfig { name: "streak_current", field_type: "number", required: false, options: None },
                FieldConfig { name: "streak_best", field_type: "number", required: false, options: None },
                FieldConfig { name: "last_played", field_type: "date", required: true, options: None },
                FieldConfig { name: "tier", field_type: "select", required: true, options: Some(serde_json::json!(["bronze", "silver", "gold", "platinum", "diamond", "master"])) },
                FieldConfig { name: "season", field_type: "text", required: true, options: None },
            ],
        },
        CollectionConfig {
            name: "inventory",
            schema: vec![
                FieldConfig { name: "user_id", field_type: "relation", required: true, options: Some(serde_json::json!({"collectionName": "users"})) },
                FieldConfig { name: "item_type", field_type: "select", required: true, options: Some(serde_json::json!(["skin", "weapon", "consumable", "currency"])) },
                FieldConfig { name: "item_id", field_type: "text", required: true, options: None },
                FieldConfig { name: "quantity", field_type: "number", required: true, options: None },
                FieldConfig { name: "rarity", field_type: "select", required: true, options: Some(serde_json::json!(["common", "rare", "epic", "legendary"])) },
                FieldConfig { name: "acquired_at", field_type: "date", required: true, options: None },
                FieldConfig { name: "expires_at", field_type: "date", required: false, options: None },
                FieldConfig { name: "metadata", field_type: "json", required: false, options: None },
            ],
        },
        CollectionConfig {
            name: "achievements",
            schema: vec![
                FieldConfig { name: "user_id", field_type: "relation", required: true, options: Some(serde_json::json!({"collectionName": "users"})) },
                FieldConfig { name: "achievement_id", field_type: "text", required: true, options: None },
                FieldConfig { name: "name", field_type: "text", required: true, options: None },
                FieldConfig { name: "description", field_type: "text", required: true, options: None },
                FieldConfig { name: "icon", field_type: "text", required: true, options: None },
                FieldConfig { name: "category", field_type: "select", required: true, options: Some(serde_json::json!(["gameplay", "social", "progression"])) },
                FieldConfig { name: "rarity", field_type: "select", required: true, options: Some(serde_json::json!(["common", "rare", "epic", "legendary"])) },
                FieldConfig { name: "points", field_type: "number", required: true, options: None },
                FieldConfig { name: "unlocked_at", field_type: "date", required: true, options: None },
                FieldConfig { name: "progress", field_type: "json", required: false, options: None },
            ],
        },
        CollectionConfig {
            name: "user_stats",
            schema: vec![
                FieldConfig { name: "user_id", field_type: "relation", required: true, options: Some(serde_json::json!({"collectionName": "users"})) },
                FieldConfig { name: "date", field_type: "text", required: true, options: None },
                FieldConfig { name: "games_played", field_type: "number", required: false, options: None },
                FieldConfig { name: "total_score", field_type: "number", required: false, options: None },
                FieldConfig { name: "total_playtime_seconds", field_type: "number", required: false, options: None },
                FieldConfig { name: "avg_accuracy", field_type: "number", required: false, options: None },
                FieldConfig { name: "best_streak", field_type: "number", required: false, options: None },
                FieldConfig { name: "achievements_unlocked", field_type: "number", required: false, options: None },
                FieldConfig { name: "items_acquired", field_type: "number", required: false, options: None },
            ],
        },
    ]
}

/// Create SQL schema for PocketBase collections
pub fn generate_pocketbase_schema() -> String {
    let collections = get_collection_configs();
    let mut sql = String::new();

    for collection in collections {
        sql.push_str(&format!(
            "-- Collection: {}\nCREATE TABLE {} (\n    id TEXT PRIMARY KEY,\n    created DATETIME DEFAULT CURRENT_TIMESTAMP,\n    updated DATETIME DEFAULT CURRENT_TIMESTAMP",
            collection.name, collection.name
        ));

        for field in collection.schema {
            let field_def = match field.field_type {
                "text" => format!("    {} TEXT{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "email" => format!("    {} TEXT{} UNIQUE", field.name, if field.required { " NOT NULL" } else { "" }),
                "number" => format!("    {} INTEGER{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "date" => format!("    {} DATETIME{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "bool" => format!("    {} BOOLEAN{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "file" => format!("    {} TEXT{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "json" => format!("    {} TEXT{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "relation" => format!("    {} TEXT{}", field.name, if field.required { " NOT NULL" } else { "" }),
                "select" => format!("    {} TEXT{}", field.name, if field.required { " NOT NULL" } else { "" }),
                _ => format!("    {} TEXT{}", field.name, if field.required { " NOT NULL" } else { "" }),
            };
            sql.push_str(&format!(",\n{}", field_def));
        }

        sql.push_str("\n);\n\n");
    }

    sql
}

/// PocketBase collection creation JSON for API setup
pub fn generate_pocketbase_collections_json() -> serde_json::Value {
    let collections = get_collection_configs();
    let mut collections_json = Vec::new();

    for collection in collections {
        let mut fields = Vec::new();

        for field in collection.schema {
            let field_json = serde_json::json!({
                "name": field.name,
                "type": field.field_type,
                "required": field.required,
                "options": field.options.unwrap_or(serde_json::json!({}))
            });
            fields.push(field_json);
        }

        let collection_json = serde_json::json!({
            "name": collection.name,
            "type": "base",
            "schema": fields,
            "indexes": [],
            "rules": {
                "create": "true",
                "update": "true",
                "delete": "false"
            }
        });

        collections_json.push(collection_json);
    }

    serde_json::Value::Array(collections_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::default();
        assert_eq!(user.level, 1);
        assert_eq!(user.total_games_played, 0);
    }

    #[test]
    fn test_match_creation() {
        let match_record = Match::default();
        assert_eq!(match_record.status, "waiting");
        assert_eq!(match_record.max_players, 8);
    }

    #[test]
    fn test_collection_configs() {
        let configs = get_collection_configs();
        assert_eq!(configs.len(), 7); // users, matches, participants, leaderboard, inventory, achievements, user_stats

        let user_collection = configs.iter().find(|c| c.name == "users").unwrap();
        assert!(user_collection.schema.iter().any(|f| f.name == "email"));
        assert!(user_collection.schema.iter().any(|f| f.name == "username"));
    }

    #[test]
    fn test_schema_generation() {
        let sql = generate_pocketbase_schema();
        assert!(sql.contains("CREATE TABLE users"));
        assert!(sql.contains("CREATE TABLE matches"));
        assert!(sql.contains("CREATE TABLE leaderboard"));

        let json = generate_pocketbase_collections_json();
        if let serde_json::Value::Array(collections) = json {
            assert_eq!(collections.len(), 7);
        } else {
            panic!("Expected array of collections");
        }
    }
}
