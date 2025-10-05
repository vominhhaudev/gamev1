// Don't declare modules here since we're using lib.rs as the main library module

// WebRTC functionality is integrated directly in main.rs for now

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    middleware,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use hyper::{server::conn::AddrIncoming, Server as HyperServer};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
// Các layer cần thiết cho production
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use tower::ServiceBuilder;
use tracing::{error, info};

use gateway::{auth::{EmailAuthConfig, EmailLoginRequest, RefreshTokenRequest, email_login_handler, email_refresh_handler, email_auth_middleware}, types::InputReq, build_router};

// WebRTC Signaling Types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebRTCOffer {
    pub sdp: String,
    pub offer_type: String, // "offer" or "answer"
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebRTCAnswer {
    pub sdp: String,
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ICECandidate {
    pub candidate: String,
    pub sdp_m_line_index: u32,
    pub sdp_mid: Option<String>,
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebRTCSession {
    pub session_id: String,
    pub user_id: String,
    pub peer_user_id: Option<String>,
    pub status: String, // "negotiating", "connected", "disconnected"
    pub created_at: DateTime<Utc>,
    pub transport_type: String, // "webrtc", "ws", "quic"
}
use proto::worker::v1::worker_client::WorkerClient;

// Nếu không có v1: use proto::worker::PushInputRequest;
use proto::worker::v1::PushInputRequest;

// AppState now comes from gateway::AppState

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common_net::telemetry::init("gateway");

    let metrics_handle = PrometheusBuilder::new().install_recorder().unwrap();

    // Initialize WebRTC metrics
    metrics::describe_counter!("gw.webrtc.offers", "Number of WebRTC offers received");
    metrics::describe_counter!("gw.webrtc.answers", "Number of WebRTC answers received");
    metrics::describe_counter!("gw.webrtc.ice_candidates", "Number of ICE candidates received");
    metrics::describe_counter!("gw.webrtc.sessions_closed", "Number of WebRTC sessions closed");

    // Worker endpoint - có thể config từ env sau
    let worker_endpoint = "http://127.0.0.1:50051".to_string();

    // Build router với worker endpoint - nó sẽ tạo AppState bên trong
    let app = build_router(worker_endpoint).await;

    // Add CORS layer to the main router - allow all origins for development
    // let cors_layer = CorsLayer::new()
    //     .allow_origin(Any)
    //     .allow_methods(Any)
    //     .allow_headers(Any)
    //     .allow_credentials(true);

    let app_with_cors = app.clone();

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
    State(mut state): State<gateway::AppState>,
    Json(body): Json<InputReq>,
) -> impl IntoResponse {
    let t0 = std::time::Instant::now();

    let req = PushInputRequest {
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

async fn ws_echo(
    ws: WebSocketUpgrade,
    State(state): State<gateway::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| echo_session(socket, state.ws_registry, state.transport_registry))
}

async fn echo_session(
    mut socket: WebSocket,
    ws_registry: gateway::WebSocketRegistry,
    transport_registry: gateway::TransportRegistry,
) {
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

// ===== WEBRTC SIGNALING HANDLERS =====

// Handle WebRTC offer
async fn handle_webrtc_offer(
    State(state): State<gateway::AppState>,
    Json(offer): Json<WebRTCOffer>,
) -> impl IntoResponse {
    info!("WebRTC offer received: {:?}", offer);

    // TODO: Extract user_id from JWT token in Authorization header
    let user_id = "temp_user_id"; // Placeholder - cần auth middleware

    let session_id = offer.session_id.unwrap_or_else(|| {
        format!("rtc_{}", chrono::Utc::now().timestamp_millis())
    });

    let session = gateway::types::SignalingSession {
        session_id: session_id.clone(),
        user_id: user_id.to_string(),
        peer_user_id: None,
        status: "negotiating".to_string(),
        created_at: chrono::Utc::now(),
        transport_type: "webrtc".to_string(),
    };

    // Store session
    {
        let mut sessions = state.signaling_sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());
    }

    counter!("gw.webrtc.offers").increment(1);

    Json(json!({
        "status": "offer_received",
        "session_id": session_id,
        "message": "Offer processed successfully"
    }))
}

// Handle WebRTC answer
async fn handle_webrtc_answer(
    State(state): State<gateway::AppState>,
    Json(answer): Json<WebRTCAnswer>,
) -> impl IntoResponse {
    info!("WebRTC answer received for session: {}", answer.session_id);

    // Update session status
    {
        let mut sessions = state.signaling_sessions.write().await;
        if let Some(session) = sessions.get_mut(&answer.session_id) {
            // Update session status if needed
            // Note: SignalingSession doesn't have status field, this might need different logic
            counter!("gw.webrtc.answers").increment(1);
        }
    }

    Json(json!({
        "status": "answer_processed",
        "message": "Answer processed successfully"
    }))
}

// Handle ICE candidate
async fn handle_webrtc_ice(
    State(_state): State<gateway::AppState>,
    Json(candidate): Json<ICECandidate>,
) -> impl IntoResponse {
    info!("WebRTC ICE candidate received for session: {}", candidate.session_id);

    counter!("gw.webrtc.ice_candidates").increment(1);

    Json(json!({
        "status": "ice_candidate_processed",
        "message": "ICE candidate processed successfully"
    }))
}

// List WebRTC sessions for current user
async fn list_webrtc_sessions(
    State(state): State<gateway::AppState>,
) -> impl IntoResponse {
    let user_id = "temp_user_id"; // TODO: Extract from JWT

    let sessions: Vec<_> = {
        let sessions_map = state.signaling_sessions.read().await;
        sessions_map.values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    };

    Json(json!({
        "sessions": sessions,
        "total": sessions.len()
    }))
}

// Close WebRTC session
async fn close_webrtc_session(
    State(state): State<gateway::AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let user_id = "temp_user_id"; // TODO: Extract from JWT

    {
        let mut sessions = state.signaling_sessions.write().await;
        if let Some(session) = sessions.get(&session_id) {
            if session.user_id == user_id {
                sessions.remove(&session_id);
                counter!("gw.webrtc.sessions_closed").increment(1);
                return Json(json!({"status": "session_closed"}));
            }
        }
    }

    Json(json!({"error": "Session not found"}))
}

// ===== AUTHENTICATION HANDLERS =====

async fn auth_login(
    State(state): State<gateway::AppState>,
    Json(login_req): Json<EmailLoginRequest>,
) -> impl IntoResponse {
    match email_login_handler(&state.auth_config, login_req).await {
        Ok(response) => {
            counter!("gw.auth.login.success").increment(1);
            Json::<gateway::auth::EmailAuthResponse>(response).into_response()
        }
        Err(e) => {
            counter!("gw.auth.login.failed").increment(1);
            error!("Login failed: {}", e);
            (axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials").into_response()
        }
    }
}

async fn auth_refresh(
    State(state): State<gateway::AppState>,
    Json(refresh_req): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    match email_refresh_handler(&state.auth_config, refresh_req).await {
        Ok(response) => {
            counter!("gw.auth.refresh.success").increment(1);
            Json::<gateway::auth::EmailAuthResponse>(response).into_response()
        }
        Err(e) => {
            counter!("gw.auth.refresh.failed").increment(1);
            error!("Token refresh failed: {}", e);
            (axum::http::StatusCode::UNAUTHORIZED, "Invalid refresh token").into_response()
        }
    }
}
