use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Body cho POST /inputs
#[derive(Debug, Deserialize)]
pub struct InputReq {
    pub player_id: String,
    pub room_id: String,
    pub seq: u64,             // map sang u32 của proto
    pub payload_json: String, // map sang payload_json của proto
}

/// WebRTC signaling session
#[derive(Debug, Clone, serde::Serialize)]
pub struct SignalingSession {
    pub session_id: String,
    pub user_id: String,
    pub peer_user_id: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub transport_type: String,
}
