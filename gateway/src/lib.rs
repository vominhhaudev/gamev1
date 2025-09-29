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
use prometheus::{register_int_counter_vec, Encoder, IntCounterVec, TextEncoder};
use tracing::error;

use common_net::message::{self, ControlMessage, Frame, FramePayload};

#[cfg(not(feature = "wallet_disabled"))]
pub mod auth;
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

// no state struct required for this simple router

pub fn build_router(_worker_endpoint: &str) -> Result<Router, BoxError> {
    Ok(Router::new()
        .route(HEALTHZ_PATH, get(healthz))
        .route(VERSION_PATH, get(version))
        .route(METRICS_PATH, get(metrics))
        .route(WS_PATH, get(ws_handler)))
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

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws_session)
}

async fn ws_session(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(WsMessage::Binary(bytes)) => {
                match message::decode(&bytes) {
                    // Bỏ qua các field khác của Frame bằng `..` để khớp với struct hiện tại
                    Ok(Frame { payload, .. }) => {
                        match payload {
                            FramePayload::Control {
                                message: ControlMessage::Ping { nonce },
                            } => {
                                let frame = Frame::control(0, 0, ControlMessage::Pong { nonce });
                                if let Ok(reply) = message::encode(&frame) {
                                    let _ = socket.send(WsMessage::Binary(reply)).await;
                                }
                            }
                            _ => {
                                // echo nguyên gốc nếu không phải Ping
                                let _ = socket.send(WsMessage::Binary(bytes)).await;
                            }
                        }
                    }
                    Err(_) => {
                        let _ = socket.send(WsMessage::Binary(bytes)).await;
                    }
                }
            }
            Ok(WsMessage::Text(s)) => {
                let _ = socket.send(WsMessage::Text(s)).await;
            }
            Ok(WsMessage::Ping(p)) => {
                let _ = socket.send(WsMessage::Pong(p)).await;
            }
            Ok(WsMessage::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
    let _ = socket.close().await;
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
