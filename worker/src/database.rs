use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error, info};

const POCKETBASE_URL: &str = "http://127.0.0.1:8090";
const DEFAULT_EMAIL: &str = "admin@pocketbase.local";
const DEFAULT_PASSWORD: &str = "123456789";

#[derive(Debug, Clone)]
pub struct PocketBaseClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserRecord,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameRecord {
    pub id: Option<String>,
    pub name: String,
    pub max_players: i32,
    pub status: String, // "waiting", "playing", "finished"
    pub created: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSessionRecord {
    pub id: Option<String>,
    pub game_id: String,
    pub player_id: String,
    pub score: i32,
    pub position: serde_json::Value, // JSON position data
    pub status: String, // "active", "finished"
    pub created: Option<String>,
    pub updated: Option<String>,
}

impl PocketBaseClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: POCKETBASE_URL.to_string(),
            auth_token: None,
        }
    }

    pub async fn authenticate(&mut self, email: &str, password: &str) -> Result<()> {
        let auth_data = json!({
            "identity": email,
            "password": password
        });

        let response = self.client
            .post(&format!("{}/api/admins/auth-with-password", self.base_url))
            .json(&auth_data)
            .send()
            .await?;

        if response.status().is_success() {
            let auth_response: AuthResponse = response.json().await?;
            self.auth_token = Some(auth_response.token.clone());
            info!("Successfully authenticated with PocketBase");
            debug!("Auth token: {}", auth_response.token);
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Authentication failed: {}", error_text);
            Err(anyhow!("Authentication failed: {}", error_text))
        }
    }

    pub async fn test_connection(&self) -> Result<bool> {
        let response = self.client
            .get(&format!("{}/api/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn create_collection(&self, name: &str, schema: Value) -> Result<String> {
        if self.auth_token.is_none() {
            return Err(anyhow!("Not authenticated"));
        }

        let collection_data = json!({
            "name": name,
            "type": "base",
            "schema": schema,
            "indexes": [],
            "rules": {
                "createRule": null,
                "updateRule": null,
                "deleteRule": null,
                "viewRule": null
            }
        });

        let response = self.client
            .post(&format!("{}/api/collections", self.base_url))
            .header("Authorization", format!("Bearer {}", self.auth_token.as_ref().unwrap()))
            .json(&collection_data)
            .send()
            .await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            let collection_id = result["id"].as_str().unwrap_or("");
            info!("Created collection: {} (ID: {})", name, collection_id);
            Ok(collection_id.to_string())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to create collection {}: {}", name, error_text);
            Err(anyhow!("Failed to create collection: {}", error_text))
        }
    }

    pub async fn save_game(&self, game: &GameRecord) -> Result<String> {
        let game_data = json!({
            "name": game.name,
            "max_players": game.max_players,
            "status": game.status
        });

        let response = self.client
            .post(&format!("{}/api/collections/games/records", self.base_url))
            .json(&game_data)
            .send()
            .await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            let game_id = result["id"].as_str().unwrap_or("");
            info!("Saved game: {} (ID: {})", game.name, game_id);
            Ok(game_id.to_string())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to save game {}: {}", game.name, error_text);
            Err(anyhow!("Failed to save game: {}", error_text))
        }
    }

    pub async fn update_game_status(&self, game_id: &str, status: &str) -> Result<()> {
        let update_data = json!({
            "status": status
        });

        let response = self.client
            .patch(&format!("{}/api/collections/games/records/{}", self.base_url, game_id))
            .json(&update_data)
            .send()
            .await?;

        if response.status().is_success() {
            info!("Updated game {} status to: {}", game_id, status);
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to update game {}: {}", game_id, error_text);
            Err(anyhow!("Failed to update game: {}", error_text))
        }
    }

    pub async fn get_games(&self) -> Result<Vec<GameRecord>> {
        let response = self.client
            .get(&format!("{}/api/collections/games/records", self.base_url))
            .send()
            .await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            let items = result["items"].as_array().unwrap_or(&vec![]).clone();

            let games: Vec<GameRecord> = items
                .into_iter()
                .map(|item| GameRecord {
                    id: item["id"].as_str().map(|s| s.to_string()),
                    name: item["name"].as_str().unwrap_or("").to_string(),
                    max_players: item["max_players"].as_i64().unwrap_or(0) as i32,
                    status: item["status"].as_str().unwrap_or("").to_string(),
                    created: item["created"].as_str().map(|s| s.to_string()),
                    updated: item["updated"].as_str().map(|s| s.to_string()),
                })
                .collect();

            Ok(games)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to get games: {}", error_text);
            Err(anyhow!("Failed to get games: {}", error_text))
        }
    }
}

impl Default for PocketBaseClient {
    fn default() -> Self {
        Self::new()
    }
}
