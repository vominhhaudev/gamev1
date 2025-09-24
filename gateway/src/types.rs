use serde::Deserialize;

/// Body cho POST /inputs
#[derive(Debug, Deserialize)]
pub struct InputReq {
    pub player_id: String,
    pub room_id: String,
    pub seq: u64,             // map sang u32 của proto
    pub payload_json: String, // map sang payload_json của proto
}
