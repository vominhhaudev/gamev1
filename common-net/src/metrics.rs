use std::net::SocketAddr;

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use once_cell::sync::OnceCell;
use prometheus::{
    register_histogram, register_int_counter, register_int_gauge, Encoder, Histogram, IntCounter,
    IntGauge, TextEncoder,
};
use tokio::net::TcpListener;
use tracing::error;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Metric set cho worker mo phong gameplay.
pub struct SimulationMetrics {
    pub ticks_total: IntCounter,
    pub active_players: IntGauge,
}

impl SimulationMetrics {
    pub fn on_startup(&self) {
        self.ticks_total.inc_by(0);
        self.active_players.set(0);
    }

    pub fn inc_ticks(&self, delta: u64) {
        self.ticks_total.inc_by(delta);
    }

    pub fn set_active_players(&self, players: i64) {
        self.active_players.set(players);
    }
}

/// Metric set cho room-manager/matchmaking.
pub struct MatchmakingMetrics {
    pub rooms_created_total: IntCounter,
    pub active_rooms: IntGauge,
    pub matchmaking_queue_depth: IntGauge,
}

impl MatchmakingMetrics {
    pub fn on_startup(&self) {
        self.rooms_created_total.inc_by(0);
        self.active_rooms.set(0);
        self.matchmaking_queue_depth.set(0);
    }

    pub fn inc_rooms_created(&self) {
        self.rooms_created_total.inc();
    }

    pub fn set_active_rooms(&self, rooms: i64) {
        self.active_rooms.set(rooms);
    }

    pub fn set_queue_depth(&self, depth: i64) {
        self.matchmaking_queue_depth.set(depth);
    }
}

/// Metric set cho snapshot/delta pipeline trong tuong lai.
pub struct SnapshotMetrics {
    pub snapshots_broadcast_total: IntCounter,
    pub snapshot_encode_duration_seconds: Histogram,
}

impl SnapshotMetrics {
    pub fn inc_snapshots_broadcast(&self) {
        self.snapshots_broadcast_total.inc();
    }

    pub fn observe_encode_seconds(&self, seconds: f64) {
        self.snapshot_encode_duration_seconds.observe(seconds);
    }
}

static SIMULATION_METRICS: OnceCell<SimulationMetrics> = OnceCell::new();
static MATCHMAKING_METRICS: OnceCell<MatchmakingMetrics> = OnceCell::new();
static SNAPSHOT_METRICS: OnceCell<SnapshotMetrics> = OnceCell::new();

pub fn simulation_metrics() -> &'static SimulationMetrics {
    SIMULATION_METRICS.get_or_init(|| SimulationMetrics {
        ticks_total: register_int_counter!(
            "worker_ticks_total",
            "Tong so tick mo phong worker da thuc hien"
        )
        .expect("register worker_ticks_total"),
        active_players: register_int_gauge!(
            "worker_active_players",
            "So luong player dang duoc mo phong tren worker"
        )
        .expect("register worker_active_players"),
    })
}

pub fn matchmaking_metrics() -> &'static MatchmakingMetrics {
    MATCHMAKING_METRICS.get_or_init(|| MatchmakingMetrics {
        rooms_created_total: register_int_counter!(
            "room_manager_rooms_created_total",
            "Tong so phong duoc tao boi room-manager"
        )
        .expect("register room_manager_rooms_created_total"),
        active_rooms: register_int_gauge!("room_manager_active_rooms", "So phong dang hoat dong")
            .expect("register room_manager_active_rooms"),
        matchmaking_queue_depth: register_int_gauge!(
            "room_manager_matchmaking_queue_depth",
            "So luong yeu cau dang cho trong hang doi matchmaking"
        )
        .expect("register room_manager_matchmaking_queue_depth"),
    })
}

pub fn snapshot_metrics() -> &'static SnapshotMetrics {
    SNAPSHOT_METRICS.get_or_init(|| SnapshotMetrics {
        snapshots_broadcast_total: register_int_counter!(
            "snapshot_broadcast_total",
            "Tong so snapshot duoc gui toi client"
        )
        .expect("register snapshot_broadcast_total"),
        snapshot_encode_duration_seconds: register_histogram!(
            "snapshot_encode_duration_seconds",
            "Thoi gian encode snapshot (giay)",
            vec![0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1]
        )
        .expect("register snapshot_encode_duration_seconds"),
    })
}

pub fn metrics_router(metrics_path: &'static str) -> Router {
    Router::new().route(metrics_path, get(metrics_handler))
}

pub async fn serve_metrics(
    listener: TcpListener,
    metrics_path: &'static str,
) -> Result<(), BoxError> {
    let router = metrics_router(metrics_path);
    axum::serve(listener, router)
        .await
        .map_err(|err| Box::new(err) as BoxError)
}

pub fn spawn_metrics_exporter(
    addr: SocketAddr,
    metrics_path: &'static str,
    service_name: &'static str,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                if let Err(err) = serve_metrics(listener, metrics_path).await {
                    error!(%err, service = service_name, %addr, path = metrics_path, "metrics exporter dung bat thuong");
                }
            }
            Err(err) => {
                error!(%err, service = service_name, %addr, path = metrics_path, "metrics exporter khong the bind");
            }
        }
    })
}

async fn metrics_handler() -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
        error!(%err, "metrics: encode that bai");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let body = match String::from_utf8(buffer) {
        Ok(text) => text,
        Err(err) => {
            error!(%err, "metrics: UTF-8 sai");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, encoder.format_type())
        .body(Body::from(body))
        .unwrap()
}
