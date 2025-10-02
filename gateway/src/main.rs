// Don't declare modules here since we're using lib.rs as the main library module

// WebRTC functionality is integrated directly in main.rs for now

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hyper::{server::conn::AddrIncoming, Server as HyperServer};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use serde_json::{json, Value};
use std::net::SocketAddr;
// Tạm thời loại bỏ các layer gây lỗi với axum 0.6
// use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

use gateway::types::InputReq;
use proto::worker::v1::worker_client::WorkerClient;

// Nếu không có v1: use proto::worker::PushInputRequest;
use proto::worker::v1::PushInputRequest;

#[derive(Clone)]
struct AppState {
    build: &'static str,
    worker: WorkerClient<tonic::transport::Channel>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common_net::telemetry::init("gateway");

    let metrics_handle = PrometheusBuilder::new().install_recorder().unwrap();

    let worker = WorkerClient::connect("http://127.0.0.1:50051").await?;

    let state = AppState {
        build: env!("CARGO_PKG_VERSION"),
        worker,
    };

    let app = Router::new()
        .route("/healthz", get(|| async { axum::http::StatusCode::OK }))
        .route(
            "/version",
            get(|State(state): State<AppState>| async move {
                (axum::http::StatusCode::OK, Json(json!({
                    "name": "gateway",
                    "version": state.build,
                })))
            }),
        )
        .route(
            "/metrics",
            get(move || {
                let h = metrics_handle.clone();
                async move { h.render() }
            }),
        )
        .route("/inputs", post(post_inputs))
        .route("/ws", get(ws_echo))
        // WebRTC Signaling endpoints (simplified for testing)
        .route("/rtc/offer", post(handle_webrtc_offer))
        .route("/rtc/answer", post(handle_webrtc_answer))
        .route("/rtc/ice", post(handle_webrtc_ice))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    info!(%addr, "gateway listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let incoming = AddrIncoming::from_listener(listener).expect("failed to create incoming");
    HyperServer::builder(incoming)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn post_inputs(
    State(mut state): State<AppState>,
    Json(body): Json<InputReq>,
) -> impl IntoResponse {
    let t0 = std::time::Instant::now();

    let req = PushInputRequest {
        room_id: body.room_id,
        sequence: body.seq as u32,
        payload_json: body.payload_json,
    };

    match state.worker.push_input(req).await {
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

async fn ws_echo(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| echo_session(socket))
}

async fn echo_session(mut socket: WebSocket) {
    gauge!("gw.ws.clients").increment(1.0);

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(s)) => {
                let _ = socket.send(Message::Text(s)).await;
                counter!("gw.ws.echo_text").increment(1);
            }
            Ok(Message::Binary(b)) => {
                let _ = socket.send(Message::Binary(b)).await;
                counter!("gw.ws.echo_bin").increment(1);
            }
            Ok(Message::Ping(p)) => {
                let _ = socket.send(Message::Pong(p)).await;
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    gauge!("gw.ws.clients").decrement(1.0);
    let _ = socket.close().await;
}

// Simplified WebRTC signaling handlers for testing
async fn handle_webrtc_offer(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    info!("WebRTC offer received: {:?}", req);
    Json(json!({
        "status": "offer_received",
        "sdp": req.get("sdp").unwrap_or(&serde_json::Value::Null)
    }))
}

async fn handle_webrtc_answer(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    info!("WebRTC answer received: {:?}", req);
    axum::http::StatusCode::OK
}

async fn handle_webrtc_ice(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    info!("WebRTC ICE candidate received: {:?}", req);
    axum::http::StatusCode::OK
}
