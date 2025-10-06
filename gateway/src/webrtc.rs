use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{auth, AppState};

// WebRTC Signaling Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcOfferRequest {
    pub room_id: String,
    pub peer_id: String,
    pub offer: RtcSdp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcOfferResponse {
    pub success: bool,
    pub error: Option<String>,
    pub answer: Option<RtcSdp>,
    pub ice_candidates: Vec<RtcIceCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcAnswerRequest {
    pub room_id: String,
    pub peer_id: String,
    pub answer: RtcSdp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcAnswerResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcIceRequest {
    pub room_id: String,
    pub peer_id: String,
    pub candidate: RtcIceCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcIceResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcSdp {
    #[serde(rename = "type")]
    pub sdp_type: String,
    pub sdp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcIceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u32>,
    pub username_fragment: Option<String>,
}

// WebRTC Session Management
#[derive(Debug, Clone)]
pub struct WebRTCSession {
    pub session_id: String,
    pub room_id: String,
    pub peer_id: String,
    pub offer: Option<RtcSdp>,
    pub answer: Option<RtcSdp>,
    pub ice_candidates: Vec<RtcIceCandidate>,
    pub state: WebRTCSessionState,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebRTCSessionState {
    New,
    OfferReceived,
    AnswerSent,
    Connected,
    Failed,
    Closed,
}

impl WebRTCSession {
    pub fn new(room_id: String, peer_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            room_id,
            peer_id,
            offer: None,
            answer: None,
            ice_candidates: Vec::new(),
            state: WebRTCSessionState::New,
            created_at: now,
            last_activity: now,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    pub fn is_expired(&self, timeout_minutes: i64) -> bool {
        let timeout = Duration::minutes(timeout_minutes);
        Utc::now() - self.last_activity > timeout
    }

    pub fn can_receive_offer(&self) -> bool {
        matches!(self.state, WebRTCSessionState::New)
    }

    pub fn can_receive_answer(&self) -> bool {
        matches!(self.state, WebRTCSessionState::OfferReceived)
    }

    pub fn can_receive_ice(&self) -> bool {
        matches!(self.state, WebRTCSessionState::OfferReceived | WebRTCSessionState::AnswerSent)
    }
}

pub type WebRTCSessionRegistry = Arc<RwLock<HashMap<String, WebRTCSession>>>;

// WebRTC Signaling Service
pub struct WebRTCSignalingService {
    sessions: WebRTCSessionRegistry,
}

impl WebRTCSignalingService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Session management
    pub async fn create_session(&self, room_id: String, peer_id: String) -> String {
        let session = WebRTCSession::new(room_id, peer_id);
        let session_id = session.session_id.clone();

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        info!("Created WebRTC session: {}", session_id);
        session_id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<WebRTCSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn update_session(&self, session_id: String, mut update_fn: impl FnMut(&mut WebRTCSession)) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            update_fn(session);
            session.update_activity();
        }
    }

    pub async fn remove_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).is_some()
    }

    // Cleanup expired sessions
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let before_count = sessions.len();

        sessions.retain(|_, session| !session.is_expired(30)); // 30 minutes timeout

        let removed = before_count - sessions.len();
        if removed > 0 {
            info!("Cleaned up {} expired WebRTC sessions", removed);
        }
    }

    // Handle SDP offer
    pub async fn handle_offer(&self, room_id: String, peer_id: String, offer: RtcSdp) -> Result<RtcOfferResponse, String> {
        // Check if we can accept offer for this peer in this room
        // For now, we'll create a new session for each offer
        let session_id = self.create_session(room_id.clone(), peer_id.clone()).await;

        self.update_session(session_id.clone(), |session| {
            session.offer = Some(offer.clone());
            session.state = WebRTCSessionState::OfferReceived;
        }).await;

        // In a real implementation, this would:
        // 1. Check if there's already a session for this peer/room
        // 2. Generate an answer SDP
        // 3. Set up local peer connection

        Ok(RtcOfferResponse {
            success: true,
            error: None,
            answer: Some(RtcSdp {
                sdp_type: "answer".to_string(),
                sdp: "v=0\r\no=- 123456789 987654321 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic: WMS\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=candidate:1 1 UDP 2130706431 127.0.0.1 9000 typ host\r\na=setup:active\r\na=mid:0\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n".to_string(),
            }),
            ice_candidates: vec![RtcIceCandidate {
                candidate: "candidate:1 1 UDP 2130706431 127.0.0.1 9000 typ host".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_m_line_index: Some(0),
                username_fragment: Some("1234abcd".to_string()),
            }],
        })
    }

    // Handle SDP answer
    pub async fn handle_answer(&self, room_id: String, peer_id: String, answer: RtcSdp) -> Result<RtcAnswerResponse, String> {
        // Find existing session
        let sessions = self.sessions.read().await;
        let session = sessions.values()
            .find(|s| s.room_id == room_id && s.peer_id == peer_id && s.state == WebRTCSessionState::OfferReceived)
            .cloned();

        drop(sessions);

        if let Some(mut session) = session {
            self.update_session(session.session_id.clone(), |s| {
                s.answer = Some(answer.clone());
                s.state = WebRTCSessionState::AnswerSent;
            }).await;

            Ok(RtcAnswerResponse {
                success: true,
                error: None,
            })
        } else {
            Err("No matching offer session found".to_string())
        }
    }

    // Handle ICE candidate
    pub async fn handle_ice_candidate(&self, room_id: String, peer_id: String, candidate: RtcIceCandidate) -> Result<RtcIceResponse, String> {
        // Find existing session
        let sessions = self.sessions.read().await;
        let session = sessions.values()
            .find(|s| s.room_id == room_id && s.peer_id == peer_id && s.can_receive_ice())
            .cloned();

        drop(sessions);

        if let Some(mut session) = session {
            self.update_session(session.session_id.clone(), |s| {
                s.ice_candidates.push(candidate.clone());
            }).await;

            Ok(RtcIceResponse {
                success: true,
                error: None,
            })
        } else {
            Err("No matching session found for ICE candidate".to_string())
        }
    }
}

// Handler functions
pub async fn handle_rtc_offer(
    State(state): State<AppState>,
    Json(req): Json<RtcOfferRequest>,
) -> Json<RtcOfferResponse> {
    // For now, we'll use a simple in-memory signaling service
    // In production, this would be injected as a dependency
    let signaling_service = WebRTCSignalingService::new();

    match signaling_service.handle_offer(req.room_id, req.peer_id, req.offer).await {
        Ok(response) => {
            Json(response)
        }
        Err(error) => {
            Json(RtcOfferResponse {
                success: false,
                error: Some(error),
                answer: None,
                ice_candidates: vec![],
            })
        }
    }
}

pub async fn handle_rtc_answer(
    State(state): State<AppState>,
    Json(req): Json<RtcAnswerRequest>,
) -> Json<RtcAnswerResponse> {
    let signaling_service = WebRTCSignalingService::new();

    match signaling_service.handle_answer(req.room_id, req.peer_id, req.answer).await {
        Ok(response) => {
            Json(response)
        }
        Err(error) => {
            Json(RtcAnswerResponse {
                success: false,
                error: Some(error),
            })
        }
    }
}

pub async fn handle_rtc_ice(
    State(state): State<AppState>,
    Json(req): Json<RtcIceRequest>,
) -> Json<RtcIceResponse> {
    let signaling_service = WebRTCSignalingService::new();

    match signaling_service.handle_ice_candidate(req.room_id, req.peer_id, req.candidate).await {
        Ok(response) => {
            Json(response)
        }
        Err(error) => {
            Json(RtcIceResponse {
                success: false,
                error: Some(error),
            })
        }
    }
}

// Session management endpoints
pub async fn list_webrtc_sessions(
    State(state): State<AppState>,
) -> Json<Vec<String>> {
    let signaling_service = WebRTCSignalingService::new();
    let sessions = signaling_service.sessions.read().await;
    let session_ids: Vec<String> = sessions.keys().cloned().collect();

    Json(session_ids)
}

pub async fn close_webrtc_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let signaling_service = WebRTCSignalingService::new();

    if signaling_service.remove_session(&session_id).await {
        (StatusCode::OK, "Session closed successfully")
    } else {
        (StatusCode::NOT_FOUND, "Session not found")
    }
}

// Cleanup task (should be run periodically)
pub async fn cleanup_webrtc_sessions_task() {
    let signaling_service = WebRTCSignalingService::new();
    signaling_service.cleanup_expired_sessions().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webrtc_session_lifecycle() {
        let service = WebRTCSignalingService::new();

        // Create session
        let session_id = service.create_session("room1".to_string(), "peer1".to_string()).await;

        // Get session
        let session = service.get_session(&session_id).await.unwrap();
        assert_eq!(session.room_id, "room1");
        assert_eq!(session.peer_id, "peer1");
        assert!(session.can_receive_offer());

        // Update session
        service.update_session(session_id.clone(), |s| {
            s.state = WebRTCSessionState::OfferReceived;
        }).await;

        let updated_session = service.get_session(&session_id).await.unwrap();
        assert_eq!(updated_session.state, WebRTCSessionState::OfferReceived);
        assert!(!updated_session.can_receive_offer());
        assert!(updated_session.can_receive_answer());

        // Remove session
        assert!(service.remove_session(&session_id).await);
        assert!(service.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_offer_handling() {
        let service = WebRTCSignalingService::new();

        let offer = RtcSdp {
            sdp_type: "offer".to_string(),
            sdp: "test offer".to_string(),
        };

        let response = service.handle_offer("room1".to_string(), "peer1".to_string(), offer).await.unwrap();

        assert!(response.success);
        assert!(response.answer.is_some());
        assert!(!response.ice_candidates.is_empty());
    }
}
