use std::env;

use common_net::{
    metrics::{self, MatchmakingMetrics},
    shutdown,
};
use tokio::sync::oneshot;
use tracing::{error, info};

pub type BoxError = metrics::BoxError;

const DEFAULT_METRICS_ADDR: &str = "127.0.0.1:3200";

pub const METRICS_PATH: &str = "/metrics";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RoomManagerSettings {
    pub metrics_addr: std::net::SocketAddr,
}

impl RoomManagerSettings {
    pub fn from_env() -> Result<Self, BoxError> {
        let metrics_addr = env::var("ROOM_MANAGER_METRICS_ADDR")
            .unwrap_or_else(|_| DEFAULT_METRICS_ADDR.to_string());
        let metrics_addr = metrics_addr
            .parse()
            .map_err(|err| Box::new(err) as BoxError)?;
        Ok(Self { metrics_addr })
    }
}

impl Default for RoomManagerSettings {
    fn default() -> Self {
        Self {
            metrics_addr: DEFAULT_METRICS_ADDR
                .parse()
                .expect("default room-manager metrics addr"),
        }
    }
}

#[derive(Debug)]
pub struct RoomManagerConfig {
    pub metrics_addr: std::net::SocketAddr,
    pub ready_tx: Option<oneshot::Sender<std::net::SocketAddr>>,
}

impl RoomManagerConfig {
    pub fn from_settings(settings: RoomManagerSettings) -> Self {
        Self {
            metrics_addr: settings.metrics_addr,
            ready_tx: None,
        }
    }

    pub fn from_env() -> Result<Self, BoxError> {
        RoomManagerSettings::from_env().map(Self::from_settings)
    }
}

pub fn matchmaking_metrics() -> &'static MatchmakingMetrics {
    metrics::matchmaking_metrics()
}

pub async fn run_with_ctrl_c(config: RoomManagerConfig) -> Result<(), BoxError> {
    let (shutdown_tx, shutdown_rx) = shutdown::channel();

    let ctrl_c = tokio::spawn(async move {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!(%err, "room-manager: khong the lang nghe ctrl_c");
        }
        shutdown::trigger(&shutdown_tx);
    });

    let result = run(config, shutdown_rx).await;

    ctrl_c.abort();
    result
}

pub async fn run(
    config: RoomManagerConfig,
    shutdown_rx: shutdown::ShutdownReceiver,
) -> Result<(), BoxError> {
    matchmaking_metrics().on_startup();

    let listener = tokio::net::TcpListener::bind(config.metrics_addr)
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    let local_addr = listener
        .local_addr()
        .map_err(|err| Box::new(err) as BoxError)?;

    if let Some(tx) = config.ready_tx {
        let _ = tx.send(local_addr);
    }

    info!(%local_addr, path = METRICS_PATH, "room-manager metrics exporter dang lang nghe");

    let server = tokio::spawn(async move {
        if let Err(err) = metrics::serve_metrics(listener, METRICS_PATH).await {
            error!(%err, "room-manager metrics exporter dung bat thuong");
        }
    });

    shutdown::wait(shutdown_rx).await;

    server.abort();
    Ok(())
}
