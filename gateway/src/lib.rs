// Thư viện cho gateway: cung cấp router dùng trong test/integration.
// Binary entrypoint vẫn ở src/main.rs.

use std::net::SocketAddr;
use tokio::sync::oneshot;

use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use once_cell::sync::Lazy;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, Encoder, IntCounter,
    IntCounterVec, IntGauge, TextEncoder,
};
use tracing::error;

use common_net::message::{self, Channel, ControlMessage, Frame, FramePayload, StateMessage};

pub mod types;
pub mod worker_client;

pub type BoxError = common_net::metrics::BoxError;

pub const HEALTHZ_PATH: &str = "/healthz";
pub const VERSION_PATH: &str = "/version";
pub const METRICS_PATH: &str = "/metrics";
pub const WS_PATH: &str = "/ws";
pub const NEGOTIATE_PATH: &str = "/negotiate";

static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "gateway_http_requests_total",
        "Tổng số HTTP request theo route",
        &["path"]
    )
    .expect("register gateway_http_requests_total")
});

// Metrics for WS state backpressure
static STATE_BUFFER_DROPPED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "gateway_ws_state_dropped_total",
        "So goi state bi drop do backpressure"
    )
    .expect("register gateway_ws_state_dropped_total")
});

static STATE_BUFFER_DEPTH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "gateway_ws_state_buffer_depth",
        "Do sau buffer state hien tai"
    )
    .expect("register gateway_ws_state_buffer_depth")
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
        Ok(Self { bind_addr, worker_endpoint })
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
        Self { bind_addr: s.bind_addr, worker_endpoint: s.worker_endpoint, ready_tx: None }
    }
}

// no state struct required for this simple router

pub fn build_router(_worker_endpoint: &str) -> Result<Router, BoxError> {
    Ok(Router::new()
        .route(HEALTHZ_PATH, get(healthz))
        .route(VERSION_PATH, get(version))
        .route(METRICS_PATH, get(metrics))
        .route(NEGOTIATE_PATH, get(negotiate))
        .route(WS_PATH, get(ws_handler))
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
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let body = String::from_utf8(buffer).unwrap_or_default();
    axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, encoder.format_type())
        .body(axum::body::Body::from(body))
        .unwrap()
}

#[derive(serde::Serialize)]
struct TransportInfo<'a> {
    kind: &'a str,
    available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct NegotiationBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<Vec<&'a str>>, // server may omit to use client default
    transports: Vec<TransportInfo<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<serde_json::Value>,
}

async fn negotiate() -> impl IntoResponse {
    HTTP_REQUESTS_TOTAL.with_label_values(&[NEGOTIATE_PATH]).inc();
    let body = NegotiationBody {
        order: Some(vec!["webtransport", "webrtc", "websocket"]),
        transports: vec![
            TransportInfo { kind: "webtransport", available: false, endpoint: None },
            TransportInfo { kind: "webrtc", available: false, endpoint: None },
            TransportInfo { kind: "websocket", available: true, endpoint: Some(WS_PATH) },
        ],
        auth: None,
    };
    Json(body)
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws_session)
}

// Transport abstraction (WS implementation)
#[derive(Clone, Default)]
struct WsTransportState {
    next_control_seq: u32,
    next_state_seq: u32,
}

impl WsTransportState {
    fn alloc_sequence(&mut self, channel: Channel) -> u32 {
        match channel {
            Channel::Control => { let s = self.next_control_seq; self.next_control_seq = self.next_control_seq.wrapping_add(1); s }
            Channel::State => { let s = self.next_state_seq; self.next_state_seq = self.next_state_seq.wrapping_add(1); s }
        }
    }
}

async fn ws_session(mut socket: WebSocket) {
    let mut transport_state = WsTransportState::default();
    // simple bounded buffer with watermarks
    const STATE_BUFFER_CAPACITY: usize = 128;
    const STATE_LOW_WATERMARK: usize = 48;
    let mut state_buffer: std::collections::VecDeque<Vec<u8>> =
        std::collections::VecDeque::with_capacity(STATE_BUFFER_CAPACITY);
    let tick_ms: u64 = std::env::var("GATEWAY_TICK_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(250);
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(tick_ms));
    let mut state_seq = 0u32;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let seq = state_seq; state_seq = state_seq.wrapping_add(1);
                let msg = StateMessage::Event { name: "tick".into(), data: serde_json::json!({"seq": seq}) };
                if let Ok(bytes) = message::encode(&Frame::state(seq, timestamp_ms(), msg)) {
                    if state_buffer.len() >= STATE_BUFFER_CAPACITY {
                        let _ = state_buffer.pop_front();
                        STATE_BUFFER_DROPPED_TOTAL.inc();
                    }
                    state_buffer.push_back(bytes);
                    STATE_BUFFER_DEPTH.set(state_buffer.len() as i64);
                }
                // active draining when above low watermark
                while state_buffer.len() > STATE_LOW_WATERMARK {
                    if let Some(bytes) = state_buffer.pop_front() {
                        let _ = socket.send(WsMessage::Binary(bytes)).await;
                        STATE_BUFFER_DEPTH.set(state_buffer.len() as i64);
                    } else { break; }
                }
            }
            maybe_msg = socket.recv() => {
                let Some(msg) = maybe_msg else { break; };
        match msg {
            Ok(WsMessage::Binary(bytes)) => {
                match message::decode(&bytes) {
                    // Bỏ qua các field khác của Frame bằng `..` để khớp với struct hiện tại
                    Ok(Frame { payload, .. }) => {
                        match payload {
                            FramePayload::Control { message: ControlMessage::Ping { nonce } } => {
                                let seq = transport_state.alloc_sequence(Channel::Control);
                                let frame = Frame::control(seq, timestamp_ms(), ControlMessage::Pong { nonce });
                                if let Ok(reply) = message::encode(&frame) { let _ = socket.send(WsMessage::Binary(reply)).await; }
                            }
                            FramePayload::Control { message: ControlMessage::JoinRoom { room_id, reconnect_token: _ } } => {
                                // Trả về event state tối thiểu xác nhận join
                                let seq = transport_state.alloc_sequence(Channel::State);
                                let state = StateMessage::Event { name: "joined".into(), data: serde_json::json!({"room_id": room_id}) };
                                if let Ok(reply) = message::encode(&Frame::state(seq, timestamp_ms(), state)) { let _ = socket.send(WsMessage::Binary(reply)).await; }
                            }
                            _ => {
                                // echo nguyên gốc nếu không phải Ping
                                let _ = socket.send(WsMessage::Binary(bytes)).await;
                            }
                        }
                    }
                    Err(_) => { let _ = socket.send(WsMessage::Binary(bytes)).await; }
                }
            }
            Ok(WsMessage::Text(s)) => {
                let _ = socket.send(WsMessage::Text(s)).await;
            }
            Ok(WsMessage::Ping(p)) => { let _ = socket.send(WsMessage::Pong(p)).await; }
            Ok(WsMessage::Close(_)) | Err(_) => break,
            _ => {}
        }
            }
        }
    }
}

fn timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

pub async fn run(config: GatewayConfig, shutdown_rx: common_net::shutdown::ShutdownReceiver) -> Result<(), BoxError> {
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await
        .map_err(|e| Box::new(e) as BoxError)?;
    let local_addr = listener.local_addr().map_err(|e| Box::new(e) as BoxError)?;
    if let Some(tx) = config.ready_tx { let _ = tx.send(local_addr); }

    let app = build_router(&config.worker_endpoint)?;
    let make_svc = app.into_make_service();
    let server = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, make_svc).await {
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
