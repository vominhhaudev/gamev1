// Th╞░ viß╗çn cho gateway: cung cß║Ñp router d├╣ng trong test/integration.
// Binary entrypoint vß║½n ß╗ƒ src/main.rs.

use std::net::SocketAddr;
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{extract::{State, Path, Query}, http::{StatusCode, Method, HeaderValue, HeaderMap}, response::{IntoResponse, Response}, routing::{get, post, delete}, Json, Router};
use chrono::{DateTime, Utc};
use hyper::{header::AUTHORIZATION, server::conn::AddrIncoming};
use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, register_int_gauge, register_int_gauge_vec, Encoder, IntCounterVec, IntGauge, IntGaugeVec, TextEncoder};
use tracing::error;
use metrics::{counter, histogram};
use tower_http::cors::{Any, CorsLayer};
use tower::{Layer, Service};
use tonic::transport::Endpoint;

use common_net::message::{self, ControlMessage, Frame, FramePayload, StateMessage};
use common_net::transport::{GameTransport, TransportKind, WebRtcTransport};
use common_net::quantization::QuantizationConfig;
use common_net::snapshot::{encode_snapshot, decode_snapshot, encode_delta, decode_delta};

pub mod auth;
pub mod types;
pub mod worker_client;

use proto::worker::v1::worker_client::WorkerClient;
use room_manager::{RoomManagerState, GameMode, RoomStatus};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub signaling: SignalingState,
    pub signaling_sessions: SignalingSessions,
    pub webrtc_sessions: WebRTCSessionRegistry,
    pub ws_registry: WebSocketRegistry,
    pub transport_registry: TransportRegistry,
    pub worker_client: WorkerClient<tonic::transport::Channel>,
    pub auth_service: auth::AuthService,
    pub room_manager: std::sync::Arc<tokio::sync::RwLock<RoomManagerState>>,
}

pub const HEALTHZ_PATH: &str = "/healthz";
pub const VERSION_PATH: &str = "/version";
pub const METRICS_PATH: &str = "/metrics";
pub const WS_PATH: &str = "/ws";
pub const GAME_INPUT_PATH: &str = "/game/input";
pub const GAME_JOIN_PATH: &str = "/game/join";
pub const GAME_LEAVE_PATH: &str = "/game/leave";
pub const CHAT_SEND_PATH: &str = "/chat/send";
pub const CHAT_HISTORY_PATH: &str = "/chat/history";

// Room Manager paths
pub const ROOMS_CREATE_PATH: &str = "/rooms/create";
pub const ROOMS_JOIN_PATH: &str = "/rooms/join";
pub const ROOMS_LIST_PATH: &str = "/rooms/list";
pub const ROOMS_ASSIGN_PATH: &str = "/rooms/assign";

static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "gateway_http_requests_total",
        "Tß╗òng sß╗æ HTTP request theo route",
        &["path"]
    )
    .expect("register gateway_http_requests_total")
});

static TRANSPORT_CONNECTIONS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "gateway_transport_connections_total",
        "Tß╗òng sß╗æ kß║┐t nß╗æi transport theo loß║íi",
        &["transport_type", "fallback_used"]
    )
    .expect("register gateway_transport_connections_total")
});

static WEBRTC_CONNECTIONS_CURRENT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "gateway_webrtc_connections_current",
        "Sß╗æ kß║┐t nß╗æi WebRTC hiß╗çn tß║íi",
        &["status"]
    )
    .expect("register gateway_webrtc_connections_current")
});

static ROOMS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "gateway_rooms_active",
        "Số lượng phòng chơi đang hoạt động"
    )
    .expect("register gateway_rooms_active")
});

static PLAYERS_IN_ROOMS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "gateway_players_in_rooms",
        "Số lượng người chơi đang ở trong phòng"
    )
    .expect("register gateway_players_in_rooms")
});

// CORS helper function
fn add_cors_headers(response: impl IntoResponse) -> axum::response::Response {
    use axum::response::{Response, IntoResponse};
    use axum::http::{HeaderMap, HeaderValue};

    let mut response = response.into_response();
    let headers = response.headers_mut();

    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"));
    headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("Content-Type, Authorization, Accept"));

    response
}

// Handle CORS preflight requests
async fn handle_cors_preflight() -> impl IntoResponse {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    let mut headers = HeaderMap::new();
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"));
    headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("Content-Type, Authorization, Accept"));
    (StatusCode::OK, headers)
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GatewaySettings {
    pub bind_addr: SocketAddr,
    pub worker_endpoint: String,
}

impl GatewaySettings {
    pub fn from_env() -> Result<Self, BoxError> {
        let bind_addr: SocketAddr = std::env::var("GATEWAY_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
            .parse()
            .map_err(|e| Box::new(e) as BoxError)?;
        let worker_endpoint = std::env::var("WORKER_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        Ok(Self {
            bind_addr,
            worker_endpoint,
        })
    }
}

#[derive(Debug)]
pub struct GatewayConfig {
    pub bind_addr: SocketAddr,
    pub worker_endpoint: String,
    pub ready_tx: Option<oneshot::Sender<SocketAddr>>,
}

impl GatewayConfig {
    pub fn from_settings(s: GatewaySettings) -> Self {
        Self {
            bind_addr: s.bind_addr,
            worker_endpoint: s.worker_endpoint,
            ready_tx: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerConnection {
    pub peer_id: String,
    pub offer: Option<String>,
    pub answer: Option<String>,
    pub ice_candidates: Vec<RtcIceCandidate>,
}

impl PeerConnection {
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            offer: None,
            answer: None,
            ice_candidates: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct RoomSignaling {
    pub peers: HashMap<String, PeerConnection>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RoomQuery {
    pub room_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcOfferRequest {
    pub sdp: String,
    pub room_id: String,
    pub peer_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Default)]
pub struct RtcOfferResponse {
    pub success: bool,
    pub session_id: Option<String>,
    pub sdp: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcAnswerResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcAnswerRequest {
    pub sdp: String,
    pub session_id: String,
    pub room_id: String,
    pub peer_id: String,
    pub target_peer_id: String, // Peer m├á answer n├áy nhß║»m tß╗¢i
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcIceCandidate {
    pub candidate: String,
    pub sdp_mid: String,
    pub sdp_mline_index: u32,
    pub room_id: String,
    pub peer_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatSendRequest {
    pub room_id: String,
    pub message: String,
    pub message_type: String, // "global", "team", "whisper"
    pub target_player_id: Option<String>, // For whisper messages
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatSendResponse {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatHistoryRequest {
    pub room_id: String,
    pub count: Option<usize>, // Number of recent messages to retrieve
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatHistoryResponse {
    pub messages: Vec<ChatMessage>,
    pub total: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub player_id: String,
    pub player_name: String,
    pub message: String,
    pub timestamp: u64,
    pub message_type: String, // "global", "team", "whisper", "system"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebRTCSession {
    pub session_id: String,
    pub room_id: String,
    pub user_id: String,
    pub peer_connections: HashMap<String, PeerConnection>,
    pub status: WebRTCSessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WebRTCSessionStatus {
    Initializing,
    Negotiating,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeerConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

type SignalingState = Arc<RwLock<HashMap<String, RoomSignaling>>>;
type SignalingSessions = Arc<RwLock<HashMap<String, crate::types::SignalingSession>>>;
type WebRTCSessionRegistry = Arc<RwLock<HashMap<String, WebRTCSession>>>;

#[derive(Debug)]
pub struct WebSocketConnection {
    pub peer_id: String,
    pub room_id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<axum::extract::ws::Message>,
}

pub type WebSocketRegistry = Arc<RwLock<HashMap<String, WebSocketConnection>>>; // key: connection_id

pub struct TransportConnection {
    pub peer_id: String,
    pub room_id: String,
    pub transport: Box<dyn GameTransport + Send + Sync>,
    pub fallback_used: bool,
}

impl std::fmt::Debug for TransportConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransportConnection")
            .field("peer_id", &self.peer_id)
            .field("room_id", &self.room_id)
            .field("transport_kind", &self.transport.kind())
            .field("fallback_used", &self.fallback_used)
            .finish()
    }
}

pub type TransportRegistry = Arc<RwLock<HashMap<String, TransportConnection>>>; // key: connection_id

// Helper function to extract user_id from JWT token in Authorization header
async fn extract_user_id_from_request(
    request: &axum::http::Request<axum::body::Body>,
    auth_service: &auth::AuthService,
) -> Result<String, String> {
    let headers = request.headers();
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| if s.starts_with("Bearer ") { Some(&s[7..]) } else { None });

    if let Some(token) = auth_header {
        match auth_service.verify_token(token) {
            Ok(token_data) => {
                return Ok(token_data.claims.sub);
            }
            Err(e) => {
                tracing::warn!("Invalid token: {}", e);
            }
        }
    }

    Err("No valid token found".to_string())
}

// Handler cho /rtc/offer (c├│ state)
async fn handle_rtc_offer(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    Json(req): Json<RtcOfferRequest>,
) -> Json<RtcOfferResponse> {
    // Extract user_id from JWT token
    let user_id = "anonymous".to_string();

    // Create or update WebRTC session
    let session_id = format!("webrtc_{}", chrono::Utc::now().timestamp_millis());
    let webrtc_session = WebRTCSession {
        session_id: session_id.clone(),
        room_id: req.room_id.clone(),
        user_id: user_id.clone(),
        peer_connections: HashMap::new(),
        status: WebRTCSessionStatus::Negotiating,
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };

    // Store WebRTC session
    {
        let mut sessions = state.webrtc_sessions.write().await;
        sessions.insert(session_id.clone(), webrtc_session);
    }

    // Update legacy signaling state for compatibility
    let mut map = state.signaling.write().await;
    let room = map.entry(req.room_id.clone()).or_default();
    let peer = room.peers.entry(req.peer_id.clone()).or_insert_with(|| PeerConnection::new(req.peer_id.clone()));
    peer.offer = Some(req.sdp.clone());

    // TODO: Relay offer tß╗¢i c├íc peers kh├íc trong room qua transport abstraction
    Json(RtcOfferResponse {
        success: true,
        session_id: Some(session_id),
        sdp: Some(req.sdp),
        error: None,
    })
}

// Handler cho /rtc/ice (c├│ state)
async fn handle_rtc_ice(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    Json(ice): Json<RtcIceCandidate>,
) -> Json<RtcAnswerResponse> {
    // Extract user_id from JWT token
    let user_id = "anonymous".to_string();

    // Update WebRTC session activity
    {
        let mut sessions = state.webrtc_sessions.write().await;
        // Find session by room_id and user_id (ICE candidates are associated with sessions)
        for session in sessions.values_mut() {
            if session.room_id == ice.room_id && session.user_id == user_id {
                session.last_activity = chrono::Utc::now();
                break;
            }
        }
    }

    // Update legacy signaling state for compatibility
    let mut map = state.signaling.write().await;
    let room = map.entry(ice.room_id.clone()).or_default();
    let peer = room.peers.entry(ice.peer_id.clone()).or_insert_with(|| PeerConnection::new(ice.peer_id.clone()));
    peer.ice_candidates.push(ice);

    Json(RtcAnswerResponse {
        success: true,
        error: None,
    })
}

// Handler cho /rtc/answer (c├│ state)
async fn handle_rtc_answer(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    Json(req): Json<RtcAnswerRequest>,
) -> Json<RtcAnswerResponse> {
    // Extract user_id from JWT token
    let user_id = "anonymous".to_string();

    // Update WebRTC session status
    {
        let mut sessions = state.webrtc_sessions.write().await;
        if let Some(session) = sessions.get_mut(&req.session_id) {
            session.status = WebRTCSessionStatus::Connected;
            session.last_activity = chrono::Utc::now();
        }
    }

    // Update legacy signaling state for compatibility
    let mut map = state.signaling.write().await;
    if let Some(room) = map.get_mut(&req.room_id) {
        if let Some(target_peer) = room.peers.get_mut(&req.target_peer_id) {
            target_peer.answer = Some(req.sdp);
            // TODO: Relay answer tß╗¢i target peer
            return Json(RtcAnswerResponse {
                success: true,
                error: None,
            });
        }
    }

    Json(RtcAnswerResponse {
        success: false,
        error: Some("Target peer not found".to_string()),
    })
}

// CORS middleware layer
#[derive(Clone)]
pub struct CorsMiddleware;

impl<S> Layer<S> for CorsMiddleware {
    type Service = CorsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorsService { inner }
    }
}

#[derive(Clone)]
pub struct CorsService<S> {
    inner: S,
}

impl<S> Service<axum::http::Request<axum::body::Body>> for CorsService<S>
where
    S: Service<axum::http::Request<axum::body::Body>, Response = axum::http::Response<axum::body::Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: axum::http::Request<axum::body::Body>) -> Self::Future {
        // Handle preflight requests
        if request.method() == axum::http::Method::OPTIONS {
            let mut response = axum::http::Response::builder()
                .status(axum::http::StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
                .header("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept")
                .header("Access-Control-Max-Age", "86400")
                .body(axum::body::Body::empty())
                .unwrap();
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response = future.await?;
            let headers = response.headers_mut();
            headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
            headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept".parse().unwrap());
            Ok(response)
        })
    }
}

// Helper function to add CORS headers to responses (deprecated - using CORS middleware instead)
// Helper function to create CORS-enabled response
fn cors_response<T: IntoResponse>(response: T) -> impl IntoResponse {
    let mut resp = response.into_response();
    let headers = resp.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
    headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept".parse().unwrap());
    resp
}

pub async fn build_router(worker_endpoint: String) -> Router {
    let signaling_state: SignalingState = Arc::new(RwLock::new(HashMap::new()));
    let signaling_sessions: SignalingSessions = Arc::new(RwLock::new(HashMap::new()));
    let webrtc_sessions: WebRTCSessionRegistry = Arc::new(RwLock::new(HashMap::new()));
    let ws_registry: WebSocketRegistry = Arc::new(RwLock::new(HashMap::new()));
    let transport_registry: TransportRegistry = Arc::new(RwLock::new(HashMap::new()));
    let auth_service = auth::AuthService::new().expect("Failed to create auth service");

    // Initialize Room Manager
    let pocketbase_url = std::env::var("POCKETBASE_URL").unwrap_or_else(|_| "http://localhost:8090".to_string());
    let room_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
        RoomManagerState::new(&pocketbase_url).expect("Failed to create room manager")
    ));

    // Configure CORS layer - allow all origins for development
    // let cors_layer = CorsLayer::new()
    //     .allow_origin(Any)
    //     .allow_methods(Any)
    //     .allow_headers(Any)
    //     .allow_credentials(true);

    // Create worker client - temporarily disabled for authentication testing
    // TODO: Re-enable when worker is available
    let worker_client = {
        tracing::warn!("Worker client disabled for authentication-only testing");
        // For now, create a dummy client that will fail on use but allows gateway to run
        // This is a temporary solution to allow authentication testing

        // Create a non-functional client for testing
        tracing::warn!("Creating dummy worker client for authentication-only mode");
        // Use a dummy endpoint that won't work but allows compilation
        let dummy_endpoint = Endpoint::from_static("http://127.0.0.1:0");
        // Create a dummy channel that won't actually connect
        let dummy_channel = dummy_endpoint.connect_lazy();
        WorkerClient::new(dummy_channel)
    };

    let state = AppState {
        signaling: signaling_state,
        signaling_sessions,
        webrtc_sessions,
        ws_registry,
        transport_registry,
        worker_client,
        auth_service,
        room_manager,
    };

    Router::new()
        .route(HEALTHZ_PATH, get(healthz))
        .route(VERSION_PATH, get(version))
        .route(METRICS_PATH, get(metrics))
        .route(WS_PATH, get(ws_handler))
        .route("/auth/login", post(auth_login))
        // Room management routes (v2 - using Room Manager)
        .route(ROOMS_CREATE_PATH, post(create_room_v2_handler))
        .route(ROOMS_LIST_PATH, get(list_rooms_v2_handler))
        .route(ROOMS_JOIN_PATH, post(join_room_v2_handler))
        .route(ROOMS_ASSIGN_PATH, post(assign_room_v2_handler))
        .route("/auth/refresh", post(auth_refresh))
        .route("/inputs", post(post_inputs))
        // TODO: Uncomment when axum version conflicts are resolved
        // .route("/rtc/offer", post(handle_rtc_offer))
        // .route("/rtc/answer", post(handle_rtc_answer))
        // .route("/rtc/ice", post(handle_rtc_ice))
        // .route("/rtc/sessions", get(list_webrtc_sessions))
        // .route("/rtc/sessions/:session_id", delete(close_webrtc_session))
        .route("/test", get(test_handler))
        .route("/api/leaderboard", get(leaderboard_handler))
        .route("/api/leaderboard/submit", post(submit_score_handler))
        .route(GAME_JOIN_PATH, post(game_join_handler))
        .route(GAME_LEAVE_PATH, post(game_leave_handler))
        .route(GAME_INPUT_PATH, post(game_input_handler))
        // TODO: Uncomment when axum version conflicts are resolved
        // .route(CHAT_SEND_PATH, post(chat_send_handler))
        // .route(CHAT_HISTORY_PATH, post(chat_history_handler))
        .with_state(state)
}

// ===== ROOM MANAGEMENT HANDLERS =====

// Create a new room (Room Manager integration)
async fn create_room_v2_handler(
    State(state): State<AppState>,
    Json(create_req): Json<room_manager::CreateRoomRequest>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[ROOMS_CREATE_PATH]).inc();

    match room_manager::create_room(state.room_manager, create_req).await {
        Ok(response) => {
            counter!("gateway.rooms.created").increment(1);
            Json(response).into_response()
        }
        Err(e) => {
            error!("Failed to create room: {}", e);
            counter!("gateway.rooms.create_failed").increment(1);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to create room: {}", e)
                }))
            ).into_response()
        }
    }
}

// List available rooms (Room Manager integration)
async fn list_rooms_v2_handler(
    State(state): State<AppState>,
    Query(params): Query<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[ROOMS_LIST_PATH]).inc();

    // Parse optional query parameters
    let game_mode = params.get("game_mode")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "deathmatch" => Some(GameMode::Deathmatch),
            "team_deathmatch" => Some(GameMode::TeamDeathmatch),
            "capture_the_flag" => Some(GameMode::CaptureTheFlag),
            _ => None,
        });

    let status = params.get("status")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "waiting" => Some(RoomStatus::Waiting),
            "starting" => Some(RoomStatus::Starting),
            "in_progress" => Some(RoomStatus::InProgress),
            "finished" => Some(RoomStatus::Finished),
            "closed" => Some(RoomStatus::Closed),
            _ => None,
        });

    let list_req = room_manager::ListRoomsRequest { game_mode, status };

    match room_manager::list_rooms(state.room_manager, list_req).await {
        Ok(response) => {
            Json(response).into_response()
        }
        Err(e) => {
            error!("Failed to list rooms: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "rooms": [],
                    "error": format!("Failed to list rooms: {}", e)
                }))
            ).into_response()
        }
    }
}

// Join a specific room (Room Manager integration)
async fn join_room_v2_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(join_req): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[ROOMS_JOIN_PATH]).inc();

    let player_id = join_req.get("player_id")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous")
        .to_string();

    let player_name = join_req.get("player_name")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("Player_{}", &player_id[..8]))
        .to_string();

    let request = room_manager::JoinRoomRequest {
        room_id,
        player_id,
        player_name,
    };

    match room_manager::join_room(state.room_manager, request).await {
        Ok(response) => {
            counter!("gateway.rooms.player_joined").increment(1);
            Json(response).into_response()
        }
        Err(e) => {
            error!("Failed to join room: {}", e);
            counter!("gateway.rooms.join_failed").increment(1);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to join room: {}", e)
                }))
            ).into_response()
        }
    }
}

// Assign player to an appropriate room (auto-matchmaking) (Room Manager integration)
async fn assign_room_v2_handler(
    State(state): State<AppState>,
    Json(assign_req): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[ROOMS_ASSIGN_PATH]).inc();

    let player_id = assign_req.get("player_id")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous")
        .to_string();

    let game_mode = assign_req.get("game_mode")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "deathmatch" => Some(GameMode::Deathmatch),
            "team_deathmatch" => Some(GameMode::TeamDeathmatch),
            "capture_the_flag" => Some(GameMode::CaptureTheFlag),
            _ => None,
        });

    let request = room_manager::AssignRoomRequest { player_id, game_mode };

    match room_manager::assign_room(state.room_manager, request).await {
        Ok(response) => {
            counter!("gateway.rooms.player_assigned").increment(1);
            Json(response).into_response()
        }
        Err(e) => {
            error!("Failed to assign room: {}", e);
            counter!("gateway.rooms.assign_failed").increment(1);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "room_id": null,
                    "worker_endpoint": null,
                    "error": format!("Failed to assign room: {}", e)
                }))
            ).into_response()
        }
    }
}

// List WebRTC sessions for user
async fn list_webrtc_sessions(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
) -> Json<serde_json::Value> {
    // Extract user_id from JWT token
    let user_id = match extract_user_id_from_request(&request, &state.auth_service).await {
        Ok(id) => id,
        Err(_) => {
            return Json(serde_json::json!({
                "error": "Authentication failed",
                "sessions": []
            }));
        }
    };

    let sessions: Vec<_> = {
        let sessions_map = state.webrtc_sessions.read().await;
        sessions_map.values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    };

    Json(serde_json::json!({
        "sessions": sessions,
        "total": sessions.len()
    }))
}

// Close WebRTC session
async fn close_webrtc_session(
    State(mut state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    // Extract user_id from JWT token
    let user_id = match extract_user_id_from_request(&request, &state.auth_service).await {
        Ok(id) => id,
        Err(_) => {
            return Json(serde_json::json!({"error": "Authentication failed"}));
        }
    };

    {
        let mut sessions = state.webrtc_sessions.write().await;
        if let Some(session) = sessions.get(&session_id) {
            if session.user_id == user_id {
                sessions.remove(&session_id);
                counter!("gw.webrtc.sessions_closed").increment(1);
                return Json(serde_json::json!({"status": "session_closed"}));
            }
        }
    }

    Json(serde_json::json!({"error": "Session not found"}))
}

// ===== CHAT HANDLERS =====

async fn chat_send_handler(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    Json(chat_req): Json<ChatSendRequest>,
) -> Json<ChatSendResponse> {
    // Extract user_id from JWT token
    let user_id = match extract_user_id_from_request(&request, &state.auth_service).await {
        Ok(id) => id,
        Err(_) => {
            return Json(ChatSendResponse {
                success: false,
                message_id: None,
                error: Some("Authentication failed".to_string()),
            });
        }
    };

    // TODO: Get player name from user_id (could be stored in database or cache)
    let player_name = format!("Player_{}", &user_id[..8]);

    // Create chat message
    let message_id = format!("msg_{}", chrono::Utc::now().timestamp_millis());
    let chat_message = ChatMessage {
        id: message_id.clone(),
        player_id: user_id.clone(),
        player_name,
        message: chat_req.message.clone(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        message_type: chat_req.message_type.clone(),
    };

    // TODO: Send chat message to worker via gRPC
    // For now, just return success
    tracing::info!("Chat message sent: {:?}", chat_message);

    Json(ChatSendResponse {
        success: true,
        message_id: Some(message_id),
        error: None,
    })
}

async fn chat_history_handler(
    State(_state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    Json(history_req): Json<ChatHistoryRequest>,
) -> Json<ChatHistoryResponse> {
    // Extract user_id from JWT token
    let _user_id = "anonymous".to_string();

    // TODO: Get chat history from worker via gRPC
    // For now, return empty history
    Json(ChatHistoryResponse {
        messages: Vec::new(),
        total: 0,
    })
}

// Auth handlers
async fn auth_login(
    State(state): State<AppState>,
    Json(login_req): Json<auth::AuthRequest>,
) -> impl IntoResponse {
    match auth::login_handler(Json(login_req)).await {
        response => {
            counter!("gw.auth.login.success").increment(1);
            response
        }
    }
}

async fn auth_refresh(
    State(state): State<AppState>,
    Json(refresh_req): Json<auth::RefreshRequest>,
) -> impl IntoResponse {
    match auth::refresh_handler(Json(refresh_req)).await {
        response => {
            counter!("gw.auth.refresh.success").increment(1);
            response
        }
    }
}

// Game input handler
async fn post_inputs(
    State(mut state): State<AppState>,
    Json(body): Json<types::InputReq>,
) -> impl IntoResponse {
    let t0 = std::time::Instant::now();

    let req = proto::worker::v1::PushInputRequest {
        room_id: body.room_id,
        sequence: body.seq as u32,
        payload_json: body.payload_json,
    };

    match state.worker_client.push_input(req).await {
        Ok(_) => {
            histogram!("gw.inputs.push_ms").record(t0.elapsed().as_secs_f64() * 1000.0);
            counter!("gw.inputs.ok").increment(1);
            axum::http::StatusCode::OK
        }
        Err(e) => {
            error!(error=?e, "push_input failed");
            counter!("gw.inputs.err").increment(1);
            axum::http::StatusCode::BAD_GATEWAY
        }
    }
}

async fn healthz() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[HEALTHZ_PATH]).inc();

    let mut response = axum::http::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept, Cache-Control, Pragma")
        .header("Access-Control-Max-Age", "86400")
        .header("Vary", "Origin")
        .body(axum::body::Body::empty())
        .unwrap();

    response
}

async fn test_handler() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/test"]).inc();

    let mut response = Json(serde_json::json!({"message": "test endpoint works"})).into_response();
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
    headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept, Cache-Control, Pragma".parse().unwrap());
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    headers.insert("Vary", "Origin".parse().unwrap());
    response
}

async fn version() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[VERSION_PATH]).inc();
    let body = serde_json::json!({
        "name": "gateway",
        "version": env!("CARGO_PKG_VERSION"),
    });

    let mut response = Json(body).into_response();
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
    headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept, Cache-Control, Pragma".parse().unwrap());
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    headers.insert("Vary", "Origin".parse().unwrap());
    response
}

async fn metrics() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[METRICS_PATH]).inc();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
        error!(%err, "metrics encode failed");
        let mut response = (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "metrics encode failed"
        ).into_response();
        let headers = response.headers_mut();
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
        headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept, Cache-Control, Pragma".parse().unwrap());
        headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
        headers.insert("Vary", "Origin".parse().unwrap());
        return response;
    }
    let body = String::from_utf8(buffer).unwrap_or_default();
    let mut response = (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, encoder.format_type())],
        body
    ).into_response();
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
    headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept, Cache-Control, Pragma".parse().unwrap());
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    headers.insert("Vary", "Origin".parse().unwrap());
    response
}

async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| ws_session(socket, state.ws_registry, state.transport_registry))
}

async fn ws_session(
    mut socket: axum::extract::ws::WebSocket,
    ws_registry: WebSocketRegistry,
    transport_registry: TransportRegistry,
) {
    // Generate unique connection ID
    let connection_id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<axum::extract::ws::Message>();

    // Try WebRTC first, fallback to WebSocket
    let mut webrtc_transport = WebRtcTransport::new("default_room".to_string(), connection_id.clone());
    let webrtc_connected = try_establish_webrtc(&mut webrtc_transport).await;

    // Update metrics
    let transport_type = if webrtc_connected { "webrtc" } else { "websocket" };
    let fallback_used = if !webrtc_connected { "true" } else { "false" };
    TRANSPORT_CONNECTIONS_TOTAL.with_label_values(&[transport_type, fallback_used]).inc();

    if webrtc_connected {
        WEBRTC_CONNECTIONS_CURRENT.with_label_values(&["connected"]).inc();
    }

    // Register WebSocket connection
    {
        let mut ws_reg = ws_registry.write().await;
        ws_reg.insert(connection_id.clone(), WebSocketConnection {
            peer_id: "unknown".to_string(), // TODO: Get from handshake
            room_id: "unknown".to_string(), // TODO: Get from handshake
            sender: tx.clone(),
        });
    }

    // Register transport connection
    {
        let mut transport_reg = transport_registry.write().await;
        transport_reg.insert(connection_id.clone(), TransportConnection {
            peer_id: "unknown".to_string(),
            room_id: "unknown".to_string(),
            transport: if webrtc_connected {
                Box::new(webrtc_transport)
            } else {
                // Fallback to WebSocket transport - mark as fallback
                // We'll use the existing WebSocket connection for transport
                let mut fallback_transport = WebRtcTransport::new("unknown".to_string(), "unknown".to_string());
                fallback_transport.fallback_to_websocket().await.unwrap();
                Box::new(fallback_transport)
            },
            fallback_used: !webrtc_connected,
        });
    }

    loop {
        tokio::select! {
            // Handle incoming messages from WebSocket
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        // Handle text messages (echo for now)
                        println!("Received text message: {}", text);
                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(format!("Echo: {}", text))).await {
                            eprintln!("Failed to send echo: {}", e);
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Binary(bytes))) => {
                        match message::decode(&bytes) {
                            Ok(Frame { payload, .. }) => {
                                match payload {
                                    FramePayload::Control {
                                        message: ControlMessage::Ping { nonce },
                                    } => {
                                        let frame = Frame::control(0, 0, ControlMessage::Pong { nonce });
                                        if let Ok(reply) = message::encode(&frame) {
                                            let _ = socket.send(axum::extract::ws::Message::Binary(reply)).await;
                                        }
                                    }
                                    FramePayload::Control {
                                        message: ControlMessage::WebRtcOffer { room_id, peer_id, target_peer_id, sdp },
                                    } => {
                                        // Update connection info
                                        {
                                            let mut ws_reg = ws_registry.write().await;
                                            if let Some(conn) = ws_reg.get_mut(&connection_id) {
                                                conn.peer_id = peer_id.clone();
                                                conn.room_id = room_id.clone();
                                            }
                                        }

                                // Broadcast offer to other peers in room
                                broadcast_to_transport(&transport_registry, &room_id, &peer_id, message::Frame::control(
                                    0, 0, ControlMessage::WebRtcOffer {
                                        room_id: room_id.clone(),
                                        peer_id: peer_id.clone(),
                                        target_peer_id,
                                        sdp,
                                    }
                                )).await;
                                    }
                                    FramePayload::Control {
                                        message: ControlMessage::WebRtcAnswer { room_id, peer_id, target_peer_id, sdp },
                                    } => {
                                // Send answer to target peer
                                send_to_transport(&transport_registry, &target_peer_id.clone(), message::Frame::control(
                                    0, 0, ControlMessage::WebRtcAnswer {
                                        room_id: room_id.clone(),
                                        peer_id: peer_id.clone(),
                                        target_peer_id: target_peer_id.clone(),
                                        sdp,
                                    }
                                )).await;
                                    }
                                    FramePayload::Control {
                                        message: ControlMessage::WebRtcIceCandidate { room_id, peer_id, target_peer_id, candidate, sdp_mid, sdp_mline_index },
                                    } => {
                                        // Broadcast ICE candidate
                                        broadcast_to_transport(&transport_registry, &room_id, &peer_id, message::Frame::control(
                                            0, 0, ControlMessage::WebRtcIceCandidate {
                                                room_id: room_id.clone(),
                                                peer_id: peer_id.clone(),
                                                target_peer_id,
                                                candidate,
                                                sdp_mid,
                                                sdp_mline_index,
                                            }
                                        )).await;
                                    }
                                    FramePayload::State { message: state_msg } => {
                                        // Handle quantized state messages (snapshot/delta)
                                        // For now, use default room_id since state messages don't carry room context
                                        let default_room_id = "default_room";
                                        match handle_quantized_state_message(&state_msg, &transport_registry, default_room_id, &connection_id).await {
                                            Ok(response_frame) => {
                                                if let Some(frame) = response_frame {
                                                    broadcast_to_transport(&transport_registry, default_room_id, &connection_id, frame).await;
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to handle quantized state message: {:?}", e);
                                            }
                                        }
                                    }
                                    _ => {
                                        // echo nguy├¬n gß╗æc nß║┐u kh├┤ng phß║úi c├íc message ─æß║╖c biß╗çt
                                        let _ = socket.send(axum::extract::ws::Message::Binary(bytes)).await;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to decode message: {:?}", e);
                                // Send error message back to client
                                let error_msg = format!("Error: Invalid message format (expected binary protocol)");
                                if let Err(send_err) = socket.send(axum::extract::ws::Message::Text(error_msg)).await {
                                    eprintln!("Failed to send error message: {}", send_err);
                                }
                            }
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Ping(p))) => {
                        let _ = socket.send(axum::extract::ws::Message::Pong(p)).await;
                    }
                    Some(Ok(axum::extract::ws::Message::Pong(_))) => {
                        // Handle Pong - do nothing for now
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) | Some(Err(_)) => break,
                    None => break,
                }
            }

            // Handle outgoing messages from channel
            Some(msg) = rx.recv() => {
                if socket.send(msg).await.is_err() {
                    break;
                }
            }
        }
    }

    // Cleanup
    {
        let mut ws_reg = ws_registry.write().await;
        ws_reg.remove(&connection_id);
    }

    {
        let mut transport_reg = transport_registry.write().await;
        if let Some(transport_conn) = transport_reg.remove(&connection_id) {
            // Update metrics on disconnect
            if transport_conn.transport.kind() == TransportKind::WebRtc {
                WEBRTC_CONNECTIONS_CURRENT.with_label_values(&["connected"]).dec();
            }
        }
    }

    let _ = socket.close().await;
}

// Handle quantized state messages (snapshot/delta encoding)
async fn handle_quantized_state_message(
    state_msg: &StateMessage,
    _transport_registry: &TransportRegistry,
    _room_id: &str,
    _sender_peer_id: &str,
) -> Result<Option<Frame>, Box<dyn std::error::Error + Send + Sync>> {
    let quantization_config = QuantizationConfig::default();

    match state_msg {
        StateMessage::Snapshot { tick, entities } => {
            // TODO: Implement quantized snapshot encoding when binary protocol is ready
            // For now, forward as regular event for testing
            let event_frame = Frame::state(
                0,
                chrono::Utc::now().timestamp_millis() as u64,
                StateMessage::Event {
                    name: "snapshot".to_string(),
                    data: serde_json::json!({
                        "tick": tick,
                        "entity_count": entities.len(),
                        "quantized": false // Mark as not quantized for now
                    })
                }
            );
            Ok(Some(event_frame))
        }
        StateMessage::Delta { tick, changes } => {
            // TODO: Implement quantized delta encoding when binary protocol is ready
            // For now, forward as regular event for testing
            let event_frame = Frame::state(
                0,
                chrono::Utc::now().timestamp_millis() as u64,
                StateMessage::Event {
                    name: "delta".to_string(),
                    data: serde_json::json!({
                        "tick": tick,
                        "change_count": changes.len(),
                        "quantized": false // Mark as not quantized for now
                    })
                }
            );
            Ok(Some(event_frame))
        }
        StateMessage::Event { name, data } => {
            // Events are forwarded as-is (not quantized)
            let event_frame = Frame::state(
                0,
                chrono::Utc::now().timestamp_millis() as u64,
                StateMessage::Event {
                    name: name.clone(),
                    data: data.clone(),
                }
            );

            Ok(Some(event_frame))
        }
    }
}

// Helper function to establish WebRTC connection with fallback
async fn try_establish_webrtc(transport: &mut WebRtcTransport) -> bool {
    // In a real implementation, this would:
    // 1. Wait for WebRTC signaling to complete
    // 2. Establish DataChannels
    // 3. Return true if successful

    // For now, we'll simulate a successful connection
    // In production, this should implement actual WebRTC negotiation
    transport.set_connected(true).await;
    true
}

// Helper functions for transport-based message relay
async fn broadcast_to_transport(
    transport_registry: &TransportRegistry,
    room_id: &str,
    sender_peer_id: &str,
    frame: message::Frame,
) {
    let mut reg = transport_registry.write().await;

    for (_conn_id, transport_conn) in reg.iter_mut() {
        if transport_conn.room_id == room_id && transport_conn.peer_id != sender_peer_id {
            // Send frame through transport abstraction
            if let Err(e) = transport_conn.transport.send_frame(frame.clone()).await {
                eprintln!("Failed to send frame via transport: {:?}", e);
            }
        }
    }
}

async fn send_to_transport(
    transport_registry: &TransportRegistry,
    target_peer_id: &str,
    frame: message::Frame,
) {
    let mut reg = transport_registry.write().await;

    for (_conn_id, transport_conn) in reg.iter_mut() {
        if transport_conn.peer_id == target_peer_id {
            // Send frame through transport abstraction
            if let Err(e) = transport_conn.transport.send_frame(frame.clone()).await {
                eprintln!("Failed to send frame via transport: {:?}", e);
            }
            break;
        }
    }
}

// Legacy WebSocket helper functions (kept for backward compatibility)
async fn broadcast_webrtc_message(
    registry: &WebSocketRegistry,
    room_id: &str,
    sender_peer_id: &str,
    frame: message::Frame,
) {
    let reg = registry.read().await;
    let encoded = message::encode(&frame);

    match encoded {
        Ok(bytes) => {
            for (_conn_id, conn) in reg.iter() {
                if conn.room_id == room_id && conn.peer_id != sender_peer_id {
                    let _ = conn.sender.send(axum::extract::ws::Message::Binary(bytes.clone()));
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to encode WebRTC message: {:?}", e);
        }
    }
}

async fn send_to_peer(
    registry: &WebSocketRegistry,
    target_peer_id: &str,
    frame: message::Frame,
) {
    let reg = registry.read().await;
    let encoded = message::encode(&frame);

    match encoded {
        Ok(bytes) => {
            for (_conn_id, conn) in reg.iter() {
                if conn.peer_id == target_peer_id {
                    let _ = conn.sender.send(axum::extract::ws::Message::Binary(bytes.clone()));
                    break;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to encode WebRTC message: {:?}", e);
        }
    }
}

// ===== LEADERBOARD HANDLERS =====

// Get leaderboard data
async fn leaderboard_handler(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/leaderboard"]).inc();

    let game_mode = params.get("game_mode").map(|s| s.as_str());
    let time_range = params.get("time_range").map(|s| s.as_str()).unwrap_or("all_time");
    let limit = params.get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    // For now, return mock leaderboard data since we don't have PocketBase integration yet
    // In a real implementation, this would query PocketBase for actual leaderboard data
    let leaderboard_data = match game_mode {
        Some("endless_runner") | None => {
            vec![
                serde_json::json!({
                    "rank": 1,
                    "player_id": "player_001",
                    "player_name": "Speed Demon",
                    "score": 15420,
                    "game_mode": "endless_runner",
                    "timestamp": chrono::Utc::now().timestamp()
                }),
                serde_json::json!({
                    "rank": 2,
                    "player_id": "player_002",
                    "player_name": "Track Master",
                    "score": 12850,
                    "game_mode": "endless_runner",
                    "timestamp": chrono::Utc::now().timestamp()
                }),
                serde_json::json!({
                    "rank": 3,
                    "player_id": "player_003",
                    "player_name": "Jump King",
                    "score": 11200,
                    "game_mode": "endless_runner",
                    "timestamp": chrono::Utc::now().timestamp()
                }),
            ]
        }
        _ => Vec::new(),
    };

    let response = serde_json::json!({
        "success": true,
        "leaderboard": leaderboard_data,
        "game_mode": game_mode.unwrap_or("all"),
        "time_range": time_range,
        "total": leaderboard_data.len()
    });

    Json(response).into_response()
}

// Submit score to leaderboard
async fn submit_score_handler(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/leaderboard/submit"]).inc();

    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");
    let player_name = request.get("player_name").and_then(|v| v.as_str()).unwrap_or("Anonymous");
    let score = request.get("score").and_then(|v| v.as_u64()).unwrap_or(0);
    let game_mode = request.get("game_mode").and_then(|v| v.as_str()).unwrap_or("endless_runner");

    // Validate inputs
    if score == 0 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Score must be greater than 0"
        })).into_response();
    }

    if player_id.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Player ID is required"
        })).into_response();
    }

    // For now, just log the score submission since we don't have PocketBase integration yet
    // In a real implementation, this would save the score to PocketBase
    tracing::info!(
        player_id,
        player_name,
        score,
        game_mode,
        "Score submitted to leaderboard"
    );

    Json(serde_json::json!({
        "success": true,
        "message": "Score submitted successfully",
        "rank": 1, // Mock rank - in reality would be calculated based on other scores
        "score": score
    })).into_response()
}

pub async fn run(
    config: GatewayConfig,
    shutdown_rx: common_net::shutdown::ShutdownReceiver,
) -> Result<(), BoxError> {
    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .map_err(|e| Box::new(e) as BoxError)?;
    let local_addr = listener.local_addr().map_err(|e| Box::new(e) as BoxError)?;
    if let Some(tx) = config.ready_tx {
        let _ = tx.send(local_addr);
    }

    let app = build_router(config.worker_endpoint.clone()).await;
    let server = tokio::spawn(async move {
        let incoming = AddrIncoming::from_listener(listener).expect("failed to create incoming");
        if let Err(err) = hyper::Server::builder(incoming)
            .serve(app.into_make_service())
            .await
        {
            error!(%err, "gateway server stopped unexpectedly");
        }
    });

    common_net::shutdown::wait(shutdown_rx).await;
    server.abort();
    Ok(())
}

// Game handlers
async fn game_join_handler(
    State(mut state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[GAME_JOIN_PATH]).inc();

    let room_id = request.get("room_id").and_then(|v| v.as_str()).unwrap_or("default");
    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");

    tracing::info!(room_id, player_id, "gateway: player joining game");

    // Call worker to join room
    match state.worker_client.join_room(proto::worker::v1::JoinRoomRequest {
        room_id: room_id.to_string(),
        player_id: player_id.to_string(),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.ok {
                tracing::info!(room_id, player_id, "gateway: player joined game successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": room_id,
                    "player_id": player_id,
                    "snapshot": response_inner.snapshot.map(|s| s.payload_json).unwrap_or_else(|| "{}".to_string())
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": "Failed to join room"
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to join room");
            Json(serde_json::json!({
                "success": false,
                "error": format!("Worker error: {}", e)
            })).into_response()
        }
    }
}

async fn game_leave_handler(
    State(mut state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[GAME_LEAVE_PATH]).inc();

    // Room management endpoints
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/create"]).inc();

    let room_id = request.get("room_id").and_then(|v| v.as_str()).unwrap_or("default");
    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");

    tracing::info!(room_id, player_id, "gateway: player leaving game");

    // Call worker to leave room
    match state.worker_client.leave_room(proto::worker::v1::LeaveRoomRequest {
        room_id: room_id.to_string(),
    }).await {
        Ok(response) => {
            if response.into_inner().ok {
                tracing::info!(room_id, player_id, "gateway: player left game successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": room_id,
                    "player_id": player_id
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": "Failed to leave room"
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to leave room");
            Json(serde_json::json!({
                "success": false,
                "error": format!("Worker error: {}", e)
            })).into_response()
        }
    }
}

async fn game_input_handler(
    State(mut state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[GAME_INPUT_PATH]).inc();

    let room_id = request.get("room_id").and_then(|v| v.as_str()).unwrap_or("default");
    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");
    let sequence = request.get("sequence").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let input_json = request.get("input").map(|v| v.to_string()).unwrap_or_default();

    tracing::debug!(room_id, player_id, sequence, "gateway: processing game input");

    // Call worker to push input
    match state.worker_client.push_input(proto::worker::v1::PushInputRequest {
        room_id: room_id.to_string(),
        sequence,
        payload_json: input_json,
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.ok {
                tracing::debug!(room_id, player_id, sequence, tick = %response_inner.snapshot.as_ref().map(|s| s.tick).unwrap_or(0), "gateway: input processed");
                Json(serde_json::json!({
                    "success": true,
                    "snapshot": response_inner.snapshot.map(|s| s.payload_json).unwrap_or_else(|| "{}".to_string())
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to push input");
            Json(serde_json::json!({
                "success": false,
                "error": format!("Worker error: {}", e)
            })).into_response()
        }
    }
}

// Room management handlers

async fn create_room_handler(
    State(mut state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/create"]).inc();

    let room_name = request.get("room_name").and_then(|v| v.as_str()).unwrap_or("New Room");
    let host_id = request.get("host_id").and_then(|v| v.as_str()).unwrap_or("anonymous");
    let host_name = request.get("host_name").and_then(|v| v.as_str()).unwrap_or("Host");

    // Validate inputs
    if room_name.trim().is_empty() || room_name.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Room name must be between 1 and 50 characters"
        })).into_response();
    }

    if host_id.trim().is_empty() || host_id.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Host ID must be between 1 and 50 characters"
        })).into_response();
    }

    if host_name.trim().is_empty() || host_name.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Host name must be between 1 and 50 characters"
        })).into_response();
    }

    // Parse room settings - for now use default settings
    let settings = proto::worker::v1::RoomSettings::default();

    tracing::info!(room_name, host_id, "gateway: creating room");

    // Call worker to create room
    match state.worker_client.create_room(proto::worker::v1::CreateRoomRequest {
        room_name: room_name.to_string(),
        host_id: host_id.to_string(),
        host_name: host_name.to_string(),
        settings: Some(settings),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.success {
                tracing::info!(room_id = %response_inner.room_id, "gateway: room created successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": response_inner.room_id,
                    "room_name": room_name
                })).into_response()
            } else {
                tracing::error!(error = %response_inner.error, "gateway: failed to create room");
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to create room");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to create room"
            })).into_response()
        }
    }
}

async fn list_rooms_handler(
    State(mut state): State<AppState>,
    Query(query): Query<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/list"]).inc();

    // For now, use default filter - can be extended later
    let filter = proto::worker::v1::RoomListFilter::default();

    tracing::info!("gateway: listing rooms");

    // Call worker to list rooms
    match state.worker_client.list_rooms(proto::worker::v1::ListRoomsRequest {
        filter: Some(filter),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.success {
                let rooms_json: Vec<serde_json::Value> = response_inner.rooms.iter().map(|room| {
                    serde_json::json!({
                        "id": room.id,
                        "name": room.name,
                        "settings": room.settings.as_ref().map(|s| serde_json::json!({
                            "max_players": s.max_players,
                            "game_mode": s.game_mode,
                            "map_name": s.map_name,
                            "time_limit_seconds": s.time_limit_seconds,
                            "has_password": s.has_password,
                            "is_private": s.is_private,
                            "allow_spectators": s.allow_spectators,
                            "auto_start": s.auto_start,
                            "min_players_to_start": s.min_players_to_start,
                        })).unwrap_or_default(),
                        "state": room.state,
                        "player_count": room.player_count,
                        "spectator_count": room.spectator_count,
                        "max_players": room.max_players,
                        "has_password": room.has_password,
                        "game_mode": room.game_mode,
                        "created_at_seconds_ago": room.created_at_seconds_ago,
                    })
                }).collect();

                let mut response = Json(serde_json::json!({
                    "success": true,
                    "rooms": rooms_json
                })).into_response();
                let headers = response.headers_mut();
                headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
                headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
                headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept".parse().unwrap());
                response
            } else {
                let mut response = Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response();
                let headers = response.headers_mut();
                headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
                headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
                headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept".parse().unwrap());
                response
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to list rooms");
            let mut response = Json(serde_json::json!({
                "success": false,
                "error": "Failed to list rooms"
            })).into_response();
            let headers = response.headers_mut();
            headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
            headers.insert("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
            headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Accept".parse().unwrap());
            response
        }
    }
}

async fn get_room_info_handler(
    State(mut state): State<AppState>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/info"]).inc();

    tracing::info!(room_id, "gateway: getting room info");

    // Call worker to get room info
    match state.worker_client.get_room_info(proto::worker::v1::GetRoomInfoRequest {
        room_id: room_id.clone(),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.success {
                let room_json = response_inner.room.as_ref().map(|room| {
                    serde_json::json!({
                        "id": room.id,
                        "name": room.name,
                        "settings": room.settings.as_ref().map(|s| serde_json::json!({
                            "max_players": s.max_players,
                            "game_mode": s.game_mode,
                            "map_name": s.map_name,
                            "time_limit_seconds": s.time_limit_seconds,
                            "has_password": s.has_password,
                            "is_private": s.is_private,
                            "allow_spectators": s.allow_spectators,
                            "auto_start": s.auto_start,
                            "min_players_to_start": s.min_players_to_start,
                        })).unwrap_or_default(),
                        "state": room.state,
                        "player_count": room.player_count,
                        "spectator_count": room.spectator_count,
                        "max_players": room.max_players,
                        "has_password": room.has_password,
                        "game_mode": room.game_mode,
                        "created_at_seconds_ago": room.created_at_seconds_ago,
                    })
                });

                Json(serde_json::json!({
                    "success": true,
                    "room": room_json
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to get room info");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to get room info"
            })).into_response()
        }
    }
}

async fn join_room_as_player_handler(
    State(mut state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/join-player"]).inc();

    let room_id = request.get("room_id").and_then(|v| v.as_str()).unwrap_or("default");
    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");
    let player_name = request.get("player_name").and_then(|v| v.as_str()).unwrap_or("Player");

    // Validate inputs
    if room_id.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Room ID is required"
        })).into_response();
    }

    if player_id.trim().is_empty() || player_id.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Player ID must be between 1 and 50 characters"
        })).into_response();
    }

    if player_name.trim().is_empty() || player_name.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Player name must be between 1 and 50 characters"
        })).into_response();
    }

    tracing::info!(room_id, player_id, "gateway: player joining room");

    // Call worker to join room as player
    match state.worker_client.join_room_as_player(proto::worker::v1::JoinRoomAsPlayerRequest {
        room_id: room_id.to_string(),
        player_id: player_id.to_string(),
        player_name: player_name.to_string(),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.success {
                tracing::info!("Player joined room successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": room_id,
                    "player_id": player_id
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to join room as player");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to join room"
            })).into_response()
        }
    }
}

async fn start_game_handler(
    State(mut state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/start-game"]).inc();

    let room_id = request.get("room_id").and_then(|v| v.as_str()).unwrap_or("default");
    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");

    // Validate inputs
    if room_id.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Room ID is required"
        })).into_response();
    }

    if player_id.trim().is_empty() || player_id.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Player ID must be between 1 and 50 characters"
        })).into_response();
    }

    tracing::info!(room_id, player_id, "gateway: starting game");

    // Call worker to start game
    match state.worker_client.start_game(proto::worker::v1::StartGameRequest {
        room_id: room_id.to_string(),
        player_id: player_id.to_string(),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.success {
                tracing::info!("Game started successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": room_id
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to start game");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to start game"
            })).into_response()
        }
    }
}

// Join room handler
async fn join_room_handler(
    State(mut state): State<AppState>,
    Path(room_id): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/{room_id}/join"]).inc();

    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");
    let player_name = request.get("player_name").and_then(|v| v.as_str()).unwrap_or(&player_id);

    // Validate inputs
    if room_id.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Room ID is required"
        })).into_response();
    }

    if player_id.trim().is_empty() || player_id.len() > 50 {
        return Json(serde_json::json!({
            "success": false,
            "error": "Player ID must be between 1 and 50 characters"
        })).into_response();
    }

    tracing::info!(room_id, player_id, "gateway: joining room");

    // Call worker to join room
    match state.worker_client.join_room(proto::worker::v1::JoinRoomRequest {
        room_id: room_id.clone(),
        player_id: player_id.to_string(),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.ok {
                tracing::info!("Player joined room successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": room_id,
                    "snapshot": response_inner.snapshot.map(|s| {
                        // Parse the snapshot JSON
                        serde_json::from_str::<serde_json::Value>(&s.payload_json).unwrap_or_default()
                    }).unwrap_or_default()
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to join room");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to join room"
            })).into_response()
        }
    }
}

// Get room snapshot handler
async fn get_room_snapshot_handler(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/{room_id}/snapshot"]).inc();

    let player_id = params.get("player_id").map(|s| s.as_str()).unwrap_or("anonymous");

    // Validate inputs
    if room_id.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Room ID is required"
        })).into_response();
    }

    tracing::debug!(room_id, player_id, "gateway: getting room snapshot");

    // For now, return a mock snapshot since we don't have a direct snapshot API in worker
    // In a real implementation, this would call a worker RPC to get the current snapshot
    Json(serde_json::json!({
        "success": true,
        "tick": 0,
        "entities": [],
        "chat_messages": [],
        "spectators": []
    })).into_response()
}

// Send room input handler
async fn send_room_input_handler(
    State(mut state): State<AppState>,
    Path(room_id): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/api/rooms/{room_id}/input"]).inc();

    let player_id = request.get("player_id").and_then(|v| v.as_str()).unwrap_or("anonymous");
    let input_sequence = request.get("input_sequence").and_then(|v| v.as_u64()).unwrap_or(0);
    let movement_value = request.get("movement");
    let timestamp = request.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);

    // Validate inputs
    if room_id.trim().is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Room ID is required"
        })).into_response();
    }

    tracing::debug!(room_id, player_id, input_sequence, "gateway: processing room input");

    // Call worker to push input
    match state.worker_client.push_input(proto::worker::v1::PushInputRequest {
        room_id: room_id.clone(),
        sequence: input_sequence as u32,
        payload_json: serde_json::json!({
            "player_id": player_id,
            "movement": movement_value,
            "timestamp": timestamp
        }).to_string(),
    }).await {
        Ok(response) => {
            let response_inner = response.into_inner();
            if response_inner.ok {
                tracing::debug!("Room input processed successfully");
                Json(serde_json::json!({
                    "success": true,
                    "room_id": room_id,
                    "snapshot": response_inner.snapshot.map(|s| {
                        // Parse the snapshot JSON
                        serde_json::from_str::<serde_json::Value>(&s.payload_json).unwrap_or_default()
                    }).unwrap_or_default()
                })).into_response()
            } else {
                Json(serde_json::json!({
                    "success": false,
                    "error": response_inner.error
                })).into_response()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "gateway: failed to push room input");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to send input"
            })).into_response()
        }
    }
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self, BoxError> {
        GatewaySettings::from_env().map(Self::from_settings)
    }
}
