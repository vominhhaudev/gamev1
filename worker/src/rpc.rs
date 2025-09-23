use std::{collections::HashMap, net::TcpListener, sync::Arc};

use proto::worker::v1::{
    worker_client::WorkerClient,
    worker_server::{Worker, WorkerServer},
    JoinRoomRequest, JoinRoomResponse, LeaveRoomRequest, LeaveRoomResponse,
    PushInputRequest, PushInputResponse, Snapshot,
};
use tokio::sync::RwLock;
use tonic::{transport::{Channel, Endpoint, Server}, Response, Status};
use tracing::{error, info, warn};

use crate::simulation_metrics;

#[derive(Default)]
pub struct WorkerState {
    rooms: RwLock<HashMap<String, RoomInfo>>,
}

#[derive(Debug, Default)]
struct RoomInfo {
    tick: u64,
    players: std::collections::HashSet<String>,
}

#[derive(Clone)]
pub struct WorkerService {
    state: Arc<WorkerState>,
}
impl WorkerService {
    pub fn new(state: Arc<WorkerState>) -> Self { Self { state } }
}

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn join_room(
        &self,
        request: tonic::Request<JoinRoomRequest>,
    ) -> Result<Response<JoinRoomResponse>, Status> {
        let room_id = request.into_inner().room_id;

        // mock player set
        let active_players = {
            let mut rooms = self.state.rooms.write().await;
            let entry = rooms.entry(room_id.clone()).or_default();
            entry.tick = 0;
            entry.players.insert("unknown-player".to_string());
            rooms.values().map(|r| r.players.len()).sum::<usize>()
        };
        simulation_metrics().set_active_players(active_players as i64);

        info!(%room_id, "worker: join");

        let snapshot = Snapshot { tick: 0, payload_json: json::empty_snapshot().to_string() };
        Ok(Response::new(JoinRoomResponse { ok: true, room_id, snapshot: Some(snapshot), error: String::new() }))
    }

    async fn leave_room(
        &self,
        request: tonic::Request<LeaveRoomRequest>,
    ) -> Result<Response<LeaveRoomResponse>, Status> {
        let room_id = request.into_inner().room_id;

        let active_players = {
            let mut rooms = self.state.rooms.write().await;
            if let Some(entry) = rooms.get_mut(&room_id) { entry.players.clear(); }
            rooms.values().map(|r| r.players.len()).sum::<usize>()
        };
        simulation_metrics().set_active_players(active_players as i64);

        info!(%room_id, "worker: leave");
        Ok(Response::new(LeaveRoomResponse { ok: true, room_id, error: String::new() }))
    }

    async fn push_input(
        &self,
        request: tonic::Request<PushInputRequest>,
    ) -> Result<Response<PushInputResponse>, Status> {
        let req = request.into_inner();

        let tick_opt = {
            let mut rooms = self.state.rooms.write().await;
            rooms.get_mut(&req.room_id).map(|info| { info.tick += 1; info.tick })
        };

        let Some(tick) = tick_opt else {
            warn!(room_id = %req.room_id, "worker: input for unknown room");
            return Ok(Response::new(PushInputResponse { ok: false, room_id: req.room_id, snapshot: None, error: "room_not_found".into() }));
        };

        let snapshot_payload = json::input_snapshot(tick, req.sequence, &req.payload_json);
        let snapshot = Snapshot { tick, payload_json: snapshot_payload };

        Ok(Response::new(PushInputResponse { ok: true, room_id: req.room_id, snapshot: Some(snapshot), error: String::new() }))
    }
}

pub async fn serve_rpc(addr: std::net::SocketAddr, svc: WorkerService) {
    info!(%addr, "starting gRPC");
    if let Err(e) = Server::builder()
        .add_service(WorkerServer::new(svc))
        .serve_with_shutdown(addr, async { let _ = tokio::signal::ctrl_c().await; })
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

    let handle = tokio::spawn(async move { serve_rpc(addr, svc).await; });
    (endpoint, handle)
}

mod json {
    use serde_json::{json, Value};
    pub fn empty_snapshot() -> Value { json!({ "entities": [] }) }
    pub fn input_snapshot(tick: u64, sequence: u32, input_json: &str) -> String {
        let parsed_input = serde_json::from_str(input_json).unwrap_or_else(|_| json!(input_json));
        json!({ "tick": tick, "sequence": sequence, "input": parsed_input }).to_string()
    }
}
