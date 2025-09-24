mod types;
mod worker_client;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    routing::{get, post},
    Json, Router,
};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

use types::InputReq;
use worker_client::WorkerClient;

// Nếu không có v1: use proto::worker::PushInputRequest;
use proto::worker::v1::PushInputRequest;

#[derive(Clone)]
struct AppState {
    build: &'static str,
    worker: WorkerClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common_net::telemetry::init("gateway");

    let metrics_handle = PrometheusBuilder::new().install_recorder().unwrap();

    let worker_uri =
        std::env::var("WORKER_GRPC_URI").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
    let worker = WorkerClient::connect(&worker_uri).await?;

    let state = AppState {
        build: env!("CARGO_PKG_VERSION"),
        worker,
    };

    let app = Router::new()
        .route("/healthz", get(|| async { axum::http::StatusCode::OK }))
        .route(
            "/version",
            get(|State(state): State<AppState>| async move {
                axum::Json(serde_json::json!({
                    "name": "gateway",
                    "version": state.build,
                }))
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
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    info!(%addr, "gateway listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn post_inputs(
    State(state): State<AppState>,
    Json(body): Json<InputReq>,
) -> Result<&'static str, axum::http::StatusCode> {
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
            Ok("ok")
        }
        Err(e) => {
            error!(error=?e, "push_input failed");
            counter!("gw.inputs.err").increment(1);
            Err(axum::http::StatusCode::BAD_GATEWAY)
        }
    }
}

async fn ws_echo(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> impl axum::response::IntoResponse {
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
