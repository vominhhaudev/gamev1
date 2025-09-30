// Thư viện cho gateway: cung cấp router dùng trong test/integration.
// Binary entrypoint vẫn ở src/main.rs.

use std::net::SocketAddr;
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{extract::State, response::IntoResponse, routing::{get, post}, Json, Router};
use hyper::server::conn::AddrIncoming;
use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, Encoder, IntCounterVec, TextEncoder};
use tracing::error;

use common_net::message::{self, ControlMessage, Frame, FramePayload};

// pub mod auth; // Đã đóng băng, loại bỏ hoàn toàn
pub mod types;
pub mod worker_client;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub const HEALTHZ_PATH: &str = "/healthz";
pub const VERSION_PATH: &str = "/version";
pub const METRICS_PATH: &str = "/metrics";
pub const WS_PATH: &str = "/ws";

static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "gateway_http_requests_total",
        "Tổng số HTTP request theo route",
        &["path"]
    )
    .expect("register gateway_http_requests_total")
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

#[derive(Debug)]
pub struct WebSocketConnection {
    pub peer_id: String,
    pub room_id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<axum::extract::ws::Message>,
}

type WebSocketRegistry = Arc<RwLock<HashMap<String, WebSocketConnection>>>; // key: connection_id

// Handler cho /rtc/offer (có state)
async fn handle_rtc_offer(
    State(state): State<SignalingState>,
    Json(req): Json<RtcOfferRequest>,
) -> Json<RtcOfferResponse> {
    let mut map = state.write().await;
    let room = map.entry(req.room_id.clone()).or_default();
    let peer = room.peers.entry(req.peer_id.clone()).or_insert_with(|| PeerConnection::new(req.peer_id.clone()));
    peer.offer = Some(req.sdp.clone());

    // TODO: Relay offer tới các peers khác trong room
    Json(RtcOfferResponse { sdp: req.sdp })
}

// Handler cho /rtc/ice (có state)
async fn handle_rtc_ice(
    State(state): State<SignalingState>,
    Json(ice): Json<RtcIceCandidate>,
) -> axum::http::StatusCode {
    let mut map = state.write().await;
    let room = map.entry(ice.room_id.clone()).or_default();
    let peer = room.peers.entry(ice.peer_id.clone()).or_insert_with(|| PeerConnection::new(ice.peer_id.clone()));
    peer.ice_candidates.push(ice);
    axum::http::StatusCode::OK
}

// Handler cho /rtc/answer (có state)
async fn handle_rtc_answer(
    State(state): State<SignalingState>,
    Json(req): Json<RtcAnswerRequest>,
) -> axum::http::StatusCode {
    let mut map = state.write().await;
    if let Some(room) = map.get_mut(&req.room_id) {
        if let Some(target_peer) = room.peers.get_mut(&req.target_peer_id) {
            target_peer.answer = Some(req.sdp);
            // TODO: Relay answer tới target peer
            return axum::http::StatusCode::OK;
        }
    }
    axum::http::StatusCode::NOT_FOUND
}

pub fn build_router(_worker_endpoint: &str) -> Result<Router, BoxError> {
    let signaling_state: SignalingState = Arc::new(RwLock::new(HashMap::new()));
    let ws_registry: WebSocketRegistry = Arc::new(RwLock::new(HashMap::new()));

    Ok(Router::new()
        .route(HEALTHZ_PATH, get(healthz))
        .route(VERSION_PATH, get(version))
        .route(METRICS_PATH, get(metrics))
        .route(WS_PATH, get(move |ws| ws_handler(ws, ws_registry.clone())))
        .route("/rtc/offer", post(handle_rtc_offer))
        .route("/rtc/answer", post(handle_rtc_answer))
        .route("/rtc/ice", post(handle_rtc_ice))
        .with_state(signaling_state)
    )
}

async fn healthz() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[HEALTHZ_PATH]).inc();
    axum::http::StatusCode::OK
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
    registry: WebSocketRegistry,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| ws_session(socket, registry))
}

async fn ws_session(
    mut socket: axum::extract::ws::WebSocket,
    registry: WebSocketRegistry,
) {
    // Generate unique connection ID
    let connection_id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<axum::extract::ws::Message>();

    // Register connection (simplified - in real impl, need room_id and peer_id from handshake)
    {
        let mut reg = registry.write().await;
        reg.insert(connection_id.clone(), WebSocketConnection {
            peer_id: "unknown".to_string(), // TODO: Get from handshake
            room_id: "unknown".to_string(), // TODO: Get from handshake
            sender: tx.clone(),
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
                                            let mut reg = registry.write().await;
                                            if let Some(conn) = reg.get_mut(&connection_id) {
                                                conn.peer_id = peer_id.clone();
                                                conn.room_id = room_id.clone();
                                            }
                                        }

                                        // Broadcast offer to other peers in room
                                        broadcast_webrtc_message(&registry, &room_id, &peer_id, message::Frame::control(
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
                                        send_to_peer(&registry, &target_peer_id.clone(), message::Frame::control(
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
                                        broadcast_webrtc_message(&registry, &room_id, &peer_id, message::Frame::control(
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
        let mut reg = registry.write().await;
        reg.remove(&connection_id);
    }

    let _ = socket.close().await;
}

// Helper functions for WebRTC message relay
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

    let app = build_router(&config.worker_endpoint)?;
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

impl GatewayConfig {
    pub fn from_env() -> Result<Self, BoxError> {
        GatewaySettings::from_env().map(Self::from_settings)
    }
}
