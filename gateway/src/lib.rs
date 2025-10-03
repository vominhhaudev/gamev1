// Thư viện cho gateway: cung cấp router dùng trong test/integration.
// Binary entrypoint vẫn ở src/main.rs.

use std::net::SocketAddr;
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{extract::{State, Path}, response::IntoResponse, routing::{get, post, delete}, Json, Router};
use hyper::server::conn::AddrIncoming;
use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, register_int_gauge_vec, Encoder, IntCounterVec, IntGaugeVec, TextEncoder};
use tracing::error;
use metrics::{counter, histogram};

use common_net::message::{self, ControlMessage, Frame, FramePayload};
use common_net::transport::{GameTransport, TransportKind, WebRtcTransport};

pub mod auth;
pub mod types;
pub mod worker_client;

use proto::worker::v1::worker_client::WorkerClient;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub signaling: SignalingState,
    pub signaling_sessions: SignalingSessions,
    pub ws_registry: WebSocketRegistry,
    pub transport_registry: TransportRegistry,
    pub worker_client: WorkerClient<tonic::transport::Channel>,
    pub auth_config: auth::EmailAuthConfig,
}

pub const HEALTHZ_PATH: &str = "/healthz";
pub const VERSION_PATH: &str = "/version";
pub const METRICS_PATH: &str = "/metrics";
pub const WS_PATH: &str = "/ws";
pub const GAME_INPUT_PATH: &str = "/game/input";
pub const GAME_JOIN_PATH: &str = "/game/join";
pub const GAME_LEAVE_PATH: &str = "/game/leave";

static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "gateway_http_requests_total",
        "Tổng số HTTP request theo route",
        &["path"]
    )
    .expect("register gateway_http_requests_total")
});

static TRANSPORT_CONNECTIONS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "gateway_transport_connections_total",
        "Tổng số kết nối transport theo loại",
        &["transport_type", "fallback_used"]
    )
    .expect("register gateway_transport_connections_total")
});

static WEBRTC_CONNECTIONS_CURRENT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "gateway_webrtc_connections_current",
        "Số kết nối WebRTC hiện tại",
        &["status"]
    )
    .expect("register gateway_webrtc_connections_current")
});

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

#[derive(Debug)]
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

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcOfferResponse {
    pub sdp: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcAnswerRequest {
    pub sdp: String,
    pub room_id: String,
    pub peer_id: String,
    pub target_peer_id: String, // Peer mà answer này nhắm tới
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RtcIceCandidate {
    pub candidate: String,
    pub sdp_mid: String,
    pub sdp_mline_index: u32,
    pub room_id: String,
    pub peer_id: String,
}

type SignalingState = Arc<RwLock<HashMap<String, RoomSignaling>>>;
type SignalingSessions = Arc<RwLock<HashMap<String, crate::types::SignalingSession>>>;

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

// Handler cho /rtc/offer (có state)
async fn handle_rtc_offer(
    State(state): State<AppState>,
    Json(req): Json<RtcOfferRequest>,
) -> Json<RtcOfferResponse> {
    let mut map = state.signaling.write().await;
    let room = map.entry(req.room_id.clone()).or_default();
    let peer = room.peers.entry(req.peer_id.clone()).or_insert_with(|| PeerConnection::new(req.peer_id.clone()));
    peer.offer = Some(req.sdp.clone());

    // TODO: Relay offer tới các peers khác trong room qua transport abstraction
    Json(RtcOfferResponse { sdp: req.sdp })
}

// Handler cho /rtc/ice (có state)
async fn handle_rtc_ice(
    State(state): State<AppState>,
    Json(ice): Json<RtcIceCandidate>,
) -> axum::http::StatusCode {
    let mut map = state.signaling.write().await;
    let room = map.entry(ice.room_id.clone()).or_default();
    let peer = room.peers.entry(ice.peer_id.clone()).or_insert_with(|| PeerConnection::new(ice.peer_id.clone()));
    peer.ice_candidates.push(ice);
    axum::http::StatusCode::OK
}

// Handler cho /rtc/answer (có state)
async fn handle_rtc_answer(
    State(state): State<AppState>,
    Json(req): Json<RtcAnswerRequest>,
) -> axum::http::StatusCode {
    let mut map = state.signaling.write().await;
    if let Some(room) = map.get_mut(&req.room_id) {
        if let Some(target_peer) = room.peers.get_mut(&req.target_peer_id) {
            target_peer.answer = Some(req.sdp);
            // TODO: Relay answer tới target peer
            return axum::http::StatusCode::OK;
        }
    }
    axum::http::StatusCode::NOT_FOUND
}

pub async fn build_router(worker_endpoint: String) -> Router {
    let signaling_state: SignalingState = Arc::new(RwLock::new(HashMap::new()));
    let signaling_sessions: SignalingSessions = Arc::new(RwLock::new(HashMap::new()));
    let ws_registry: WebSocketRegistry = Arc::new(RwLock::new(HashMap::new()));
    let transport_registry: TransportRegistry = Arc::new(RwLock::new(HashMap::new()));
    let auth_config = auth::EmailAuthConfig::from_env();

    // Create worker client
    let worker_client = match WorkerClient::connect(worker_endpoint.clone()).await {
        Ok(client) => client,
        Err(e) => {
            tracing::warn!("Failed to connect to worker, will retry on demand: {}", e);
            match WorkerClient::connect(worker_endpoint.clone()).await {
                Ok(client) => client,
                Err(_) => {
                    // Fallback: create a dummy client that will fail on use
                    match WorkerClient::connect("http://127.0.0.1:1").await {
                        Ok(client) => client,
                        Err(_) => {
                            panic!("Cannot create fallback worker client")
                        }
                    }
                }
            }
        }
    };

    let state = AppState {
        signaling: signaling_state,
        signaling_sessions,
        ws_registry,
        transport_registry,
        worker_client,
        auth_config,
    };

    Router::new()
        .route(HEALTHZ_PATH, get(healthz))
        .route(VERSION_PATH, get(version))
        .route(METRICS_PATH, get(metrics))
        .route(WS_PATH, get(ws_handler))
        .route("/auth/login", post(auth_login))
        .route("/auth/refresh", post(auth_refresh))
        .route("/inputs", post(post_inputs))
        .route("/rtc/offer", post(handle_rtc_offer))
        .route("/rtc/answer", post(handle_rtc_answer))
        .route("/rtc/ice", post(handle_rtc_ice))
        .route("/rtc/sessions", get(list_webrtc_sessions))
        .route("/rtc/sessions/:session_id", delete(close_webrtc_session))
        .route("/test", get(test_handler))
        .route(GAME_JOIN_PATH, post(game_join_handler))
        .route(GAME_LEAVE_PATH, post(game_leave_handler))
        .route(GAME_INPUT_PATH, post(game_input_handler))
        .with_state(state)
}

// List WebRTC sessions for user
async fn list_webrtc_sessions(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = "temp_user_id"; // TODO: Extract from JWT

    let sessions: Vec<_> = {
        let sessions_map = state.signaling_sessions.read().await;
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
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let user_id = "temp_user_id"; // TODO: Extract from JWT

    {
        let mut sessions = state.signaling_sessions.write().await;
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

// Auth handlers
async fn auth_login(
    State(state): State<AppState>,
    Json(login_req): Json<auth::EmailLoginRequest>,
) -> impl IntoResponse {
    match auth::email_login_handler(&state.auth_config, login_req).await {
        Ok(response) => {
            counter!("gw.auth.login.success").increment(1);
            Json::<auth::EmailAuthResponse>(response).into_response()
        }
        Err(e) => {
            counter!("gw.auth.login.failed").increment(1);
            error!("Login failed: {}", e);
            (axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials").into_response()
        }
    }
}

async fn auth_refresh(
    State(state): State<AppState>,
    Json(refresh_req): Json<auth::RefreshTokenRequest>,
) -> impl IntoResponse {
    match auth::email_refresh_handler(&state.auth_config, refresh_req).await {
        Ok(response) => {
            counter!("gw.auth.refresh.success").increment(1);
            Json::<auth::EmailAuthResponse>(response).into_response()
        }
        Err(e) => {
            counter!("gw.auth.refresh.failed").increment(1);
            error!("Token refresh failed: {}", e);
            (axum::http::StatusCode::UNAUTHORIZED, "Invalid refresh token").into_response()
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
    axum::http::StatusCode::OK
}

async fn test_handler() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&["/test"]).inc();
    Json(serde_json::json!({"message": "test endpoint works"}))
}

async fn version() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[VERSION_PATH]).inc();
    let body = serde_json::json!({
        "name": "gateway",
        "version": env!("CARGO_PKG_VERSION"),
    });
    Json(body)
}

async fn metrics() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[METRICS_PATH]).inc();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
        error!(%err, "metrics encode failed");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "metrics encode failed"
        ).into_response();
    }
    let body = String::from_utf8(buffer).unwrap_or_default();
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, encoder.format_type())],
        body
    ).into_response()
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
                                    _ => {
                                        // echo nguyên gốc nếu không phải các message đặc biệt
                                        let _ = socket.send(axum::extract::ws::Message::Binary(bytes)).await;
                                    }
                                }
                            }
                            Err(_) => {
                                let _ = socket.send(axum::extract::ws::Message::Binary(bytes)).await;
                            }
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Text(s))) => {
                        let _ = socket.send(axum::extract::ws::Message::Text(s)).await;
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

impl GatewayConfig {
    pub fn from_env() -> Result<Self, BoxError> {
        GatewaySettings::from_env().map(Self::from_settings)
    }
}
