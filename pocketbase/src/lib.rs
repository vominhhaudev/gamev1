use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error, info};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Error, Debug)]
pub enum PocketBaseError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] ReqwestError),
    #[error("API error: {message} (code: {code})")]
    Api { message: String, code: String },
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid URL: {0}")]
    Url(String),
}

#[derive(Debug, Clone)]
pub struct PocketBaseClient {
    client: Client,
    base_url: String,
    admin_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub schema: Vec<FieldSchema>,
    pub indexes: Vec<String>,
    pub rules: Option<CollectionRules>,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub options: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionRules {
    pub create: Option<String>,
    pub update: Option<String>,
    pub delete: Option<String>,
    pub view: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub created: String,
    pub updated: String,
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRecord {
    pub token: String,
    pub record: Record,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionCreateRequest {
    pub name: String,
    pub schema: Vec<FieldSchema>,
    pub indexes: Option<Vec<String>>,
    pub rules: Option<CollectionRules>,
}

impl PocketBaseClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            admin_token: None,
        }
    }

    pub fn with_admin_token(mut self, token: String) -> Self {
        self.admin_token = Some(token);
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn get_auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(token) = &self.admin_token {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// Health check
    pub async fn health(&self) -> Result<String, PocketBaseError> {
        let url = format!("{}/api/health", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok("PocketBase is healthy".to_string())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(PocketBaseError::Api {
                message: format!("Health check failed: {}", error_text),
                code: status.to_string(),
            })
        }
    }

    /// Create collection
    pub async fn create_collection(&self, collection: CollectionCreateRequest) -> Result<Collection, PocketBaseError> {
        let url = format!("{}/api/collections", self.base_url);
        let response = self
            .client
            .post(&url)
            .headers(self.get_auth_headers())
            .json(&collection)
            .send()
            .await?;

        if response.status().is_success() {
            let collection: Collection = response.json().await?;
            info!("Created collection: {}", collection.name);
            Ok(collection)
        } else {
            let status = response.status();
            let error: Value = response.json().await.unwrap_or_default();
            Err(PocketBaseError::Api {
                message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
                code: status.to_string(),
            })
        }
    }

    /// Get collection
    pub async fn get_collection(&self, name: &str) -> Result<Collection, PocketBaseError> {
        let url = format!("{}/api/collections/{}", self.base_url, name);
        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let collection: Collection = response.json().await?;
            Ok(collection)
        } else {
            let status = response.status();
            Err(PocketBaseError::Api {
                message: format!("Collection '{}' not found", name),
                code: status.to_string(),
            })
        }
    }

    /// List collections
    pub async fn list_collections(&self) -> Result<Vec<Collection>, PocketBaseError> {
        let url = format!("{}/api/collections", self.base_url);
        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let collections: Vec<Collection> = response.json().await?;
            Ok(collections)
        } else {
            let status = response.status();
            Err(PocketBaseError::Api {
                message: "Failed to list collections".to_string(),
                code: status.to_string(),
            })
        }
    }

    /// Create record
    pub async fn create_record(&self, collection: &str, data: Value) -> Result<Record, PocketBaseError> {
        let url = format!("{}/api/collections/{}/records", self.base_url, collection);
        let response = self
            .client
            .post(&url)
            .headers(self.get_auth_headers())
            .json(&data)
            .send()
            .await?;

        if response.status().is_success() {
            let record: Record = response.json().await?;
            debug!("Created record in collection '{}': {}", collection, record.id);
            Ok(record)
        } else {
            let status = response.status();
            let error: Value = response.json().await.unwrap_or_default();
            Err(PocketBaseError::Api {
                message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
                code: status.to_string(),
            })
        }
    }

    /// Get record
    pub async fn get_record(&self, collection: &str, id: &str) -> Result<Record, PocketBaseError> {
        let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let record: Record = response.json().await?;
            Ok(record)
        } else {
            let status = response.status();
            Err(PocketBaseError::Api {
                message: format!("Record '{}' not found in collection '{}'", id, collection),
                code: status.to_string(),
            })
        }
    }

    /// Update record
    pub async fn update_record(&self, collection: &str, id: &str, data: Value) -> Result<Record, PocketBaseError> {
        let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
        let response = self
            .client
            .patch(&url)
            .headers(self.get_auth_headers())
            .json(&data)
            .send()
            .await?;

        if response.status().is_success() {
            let record: Record = response.json().await?;
            debug!("Updated record '{}' in collection '{}'", id, collection);
            Ok(record)
        } else {
            let status = response.status();
            let error: Value = response.json().await.unwrap_or_default();
            Err(PocketBaseError::Api {
                message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
                code: status.to_string(),
            })
        }
    }

    /// Delete record
    pub async fn delete_record(&self, collection: &str, id: &str) -> Result<(), PocketBaseError> {
        let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
        let response = self
            .client
            .delete(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Deleted record '{}' from collection '{}'", id, collection);
            Ok(())
        } else {
            let status = response.status();
            let error: Value = response.json().await.unwrap_or_default();
            Err(PocketBaseError::Api {
                message: error["message"].as_str().unwrap_or("Unknown error").to_string(),
                code: status.to_string(),
            })
        }
    }

    /// List records
    pub async fn list_records(&self, collection: &str, filter: Option<&str>, sort: Option<&str>) -> Result<Vec<Record>, PocketBaseError> {
        let mut url = format!("{}/api/collections/{}/records", self.base_url, collection);

        let mut params = Vec::new();
        if let Some(f) = filter {
            params.push(format!("filter={}", f));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let response_data: Value = response.json().await?;
            let records: Vec<Record> = serde_json::from_value(response_data["items"].clone())?;
            Ok(records)
        } else {
            let status = response.status();
            Err(PocketBaseError::Api {
                message: format!("Failed to list records from collection '{}'", collection),
                code: status.to_string(),
            })
        }
    }

    /// Authenticate admin
    pub async fn auth_admin(&mut self, email: &str, password: &str) -> Result<AuthRecord, PocketBaseError> {
        let url = format!("{}/api/admins/auth-with-password", self.base_url);
        let auth_data = json!({
            "identity": email,
            "password": password
        });

        let response = self
            .client
            .post(&url)
            .json(&auth_data)
            .send()
            .await?;

        if response.status().is_success() {
            let auth_record: AuthRecord = response.json().await?;
            self.admin_token = Some(auth_record.token.clone());
            info!("Authenticated as admin");
            Ok(auth_record)
        } else {
            let status = response.status();
            let error: Value = response.json().await.unwrap_or_default();
            Err(PocketBaseError::Api {
                message: error["message"].as_str().unwrap_or("Authentication failed").to_string(),
                code: status.to_string(),
            })
        }
    }

    /// Subscribe to real-time updates
    pub async fn subscribe(&self, collection: &str, callback: impl Fn(Value) + Send + 'static) -> Result<(), PocketBaseError> {
        let url = format!("{}/api/realtime", self.base_url);

        // For now, just log subscription (real implementation would use WebSocket)
        info!("Subscribing to collection: {}", collection);
        // TODO: Implement WebSocket subscription

        Ok(())
    }
}

