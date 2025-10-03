use std::{collections::HashMap, net::TcpListener, sync::Arc};

use proto::worker::v1::{
    worker_client::WorkerClient,
    worker_server::{Worker, WorkerServer},
    JoinRoomRequest, JoinRoomResponse, LeaveRoomRequest, LeaveRoomResponse, PushInputRequest,
    PushInputResponse, Snapshot,
};
use tokio::sync::RwLock;
use tonic::{
    transport::{Channel, Endpoint, Server},
    Response, Status,
};
use tracing::{error, info, warn};

use crate::{simulation::{GameWorld, PlayerInput}, simulation_metrics};

pub struct WorkerState {
    game_world: RwLock<GameWorld>,
}

impl WorkerState {
    pub fn new() -> Self {
        Self {
            game_world: RwLock::new(GameWorld::new()),
        }
    }
}

impl Default for WorkerState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct WorkerService {
    state: Arc<WorkerState>,
}
impl WorkerService {
    pub fn new(state: Arc<WorkerState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn join_room(
        &self,
        request: tonic::Request<JoinRoomRequest>,
    ) -> Result<Response<JoinRoomResponse>, Status> {
        let req = request.into_inner();
        let room_id = req.room_id.clone();
        let player_id = req.player_id.clone();

        info!(%room_id, %player_id, "worker: player joining room");

        let mut game_world = self.state.game_world.write().await;

        // Add player vào game world
        let player_entity = game_world.add_player(player_id.clone());

        // Create initial snapshot
        let snapshot = game_world.create_snapshot();

        // Update metrics
        let active_players = 1; // For now, just count this player
        simulation_metrics().set_active_players(active_players);

        info!(%room_id, %player_id, "worker: player joined successfully");

        let snapshot_json = serde_json::to_string(&snapshot)
            .unwrap_or_else(|_| json::empty_snapshot().to_string());

        Ok(Response::new(JoinRoomResponse {
            ok: true,
            room_id,
            snapshot: Some(Snapshot {
                tick: snapshot.tick,
                payload_json: snapshot_json,
            }),
            error: String::new(),
        }))
    }

    async fn leave_room(
        &self,
        request: tonic::Request<LeaveRoomRequest>,
    ) -> Result<Response<LeaveRoomResponse>, Status> {
        let req = request.into_inner();
        let room_id = req.room_id;

        // For now, just update metrics (in real implementation would remove player entity)
        let active_players = 0; // Simplified for MVP
        simulation_metrics().set_active_players(active_players);

        info!(%room_id, "worker: player left room");
        Ok(Response::new(LeaveRoomResponse {
            ok: true,
            room_id,
            error: String::new(),
        }))
    }

    async fn push_input(
        &self,
        request: tonic::Request<PushInputRequest>,
    ) -> Result<Response<PushInputResponse>, Status> {
        let req = request.into_inner();

        info!(room_id = %req.room_id, sequence = %req.sequence, "worker: processing input");

        let mut game_world = self.state.game_world.write().await;

        // Parse input từ JSON
        let input: PlayerInput = match serde_json::from_str(&req.payload_json) {
            Ok(input) => input,
            Err(e) => {
                warn!("Failed to parse player input: {}", e);
                return Ok(Response::new(PushInputResponse {
                    ok: false,
                    room_id: req.room_id,
                    snapshot: None,
                    error: format!("invalid_input: {}", e),
                }));
            }
        };

        // For MVP, assume player_id from input JSON is valid
        // In real implementation, would validate against PlayerEntityMap

        // Add input vào buffer
        game_world.input_buffers
            .entry(input.player_id.clone())
            .or_insert_with(|| crate::simulation::InputBuffer::new())
            .add_input(input);

        // Run game tick để process input
        game_world.tick();

        // Get current snapshot
        let snapshot = game_world.get_snapshot();

        // Serialize snapshot
        let snapshot_json = serde_json::to_string(&snapshot)
            .unwrap_or_else(|_| json::empty_snapshot().to_string());

        info!(room_id = %req.room_id, tick = %snapshot.tick, "worker: input processed, snapshot generated");

        Ok(Response::new(PushInputResponse {
            ok: true,
            room_id: req.room_id,
            snapshot: Some(Snapshot {
                tick: snapshot.tick,
                payload_json: snapshot_json,
            }),
            error: String::new(),
        }))
    }
}

pub async fn serve_rpc(addr: std::net::SocketAddr, svc: WorkerService) {
    info!(%addr, "starting gRPC");
    if let Err(e) = Server::builder()
        .add_service(WorkerServer::new(svc))
        .serve_with_shutdown(addr, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
    {
        error!(?e, "gRPC server error");
    }
}

pub fn channel(endpoint: &str) -> Result<Channel, tonic::transport::Error> {
    Ok(Endpoint::from_shared(endpoint.to_string())?.connect_lazy())
}
pub type Client = WorkerClient<Channel>;
pub fn client(endpoint: &str) -> Result<Client, tonic::transport::Error> {
    Ok(WorkerClient::new(channel(endpoint)?))
}

pub async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind worker test");
    let addr = listener.local_addr().expect("addr");
    drop(listener);

    let endpoint = format!("http://{}", addr);
    let state = Arc::new(WorkerState::default());
    let svc = WorkerService::new(state);

    let handle = tokio::spawn(async move {
        serve_rpc(addr, svc).await;
    });
    (endpoint, handle)
}

mod json {
    use serde_json::{json, Value};
    pub fn empty_snapshot() -> Value {
        json!({ "entities": [] })
    }
    pub fn input_snapshot(tick: u64, sequence: u32, input_json: &str) -> String {
        let parsed_input = serde_json::from_str(input_json).unwrap_or_else(|_| json!(input_json));
        json!({ "tick": tick, "sequence": sequence, "input": parsed_input }).to_string()
    }
}
