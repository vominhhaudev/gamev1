use std::{net::SocketAddr, sync::Arc};
use common_net::metrics::{self, SimulationMetrics};
use tracing::info;

pub type BoxError = metrics::BoxError;

const DEFAULT_METRICS_ADDR: &str = "127.0.0.1:3100";
const DEFAULT_RPC_ADDR: &str = "127.0.0.1:50051";
pub const METRICS_PATH: &str = "/metrics";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WorkerSettings {
    pub rpc_addr: String,
    pub metrics_addr: String,
    pub fail_fast: bool,
}
impl Default for WorkerSettings {
    fn default() -> Self {
        Self { rpc_addr: DEFAULT_RPC_ADDR.into(), metrics_addr: DEFAULT_METRICS_ADDR.into(), fail_fast: false }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub rpc_addr: SocketAddr,
    pub metrics_addr: SocketAddr,
    pub fail_fast: bool,
}
impl WorkerConfig {
    pub fn from_env() -> Result<Self, BoxError> {
        Ok(Self {
            rpc_addr: env_socket("WORKER_RPC_ADDR", DEFAULT_RPC_ADDR)?,
            metrics_addr: env_socket("WORKER_METRICS_ADDR", DEFAULT_METRICS_ADDR)?,
            fail_fast: std::env::var("WORKER_FAIL_FAST").ok().as_deref() == Some("1"),
        })
    }
    pub fn from_settings(s: WorkerSettings) -> Result<Self, BoxError> {
        Ok(Self {
            rpc_addr: s.rpc_addr.parse().map_err(|e| Box::new(e) as BoxError)?,
            metrics_addr: s.metrics_addr.parse().map_err(|e| Box::new(e) as BoxError)?,
            fail_fast: s.fail_fast,
        })
    }
}

impl WorkerSettings {
    pub fn from_env() -> Result<Self, BoxError> {
        Ok(Self {
            rpc_addr: std::env::var("WORKER_RPC_ADDR").unwrap_or_else(|_| DEFAULT_RPC_ADDR.to_string()),
            metrics_addr: std::env::var("WORKER_METRICS_ADDR").unwrap_or_else(|_| DEFAULT_METRICS_ADDR.to_string()),
            fail_fast: std::env::var("WORKER_FAIL_FAST").ok().as_deref() == Some("1"),
        })
    }
}
pub fn simulation_metrics() -> &'static SimulationMetrics { metrics::simulation_metrics() }

pub async fn run_with_ctrl_c(config: WorkerConfig) -> Result<(), BoxError> {
    let (tx, rx) = common_net::shutdown::channel();
    let j = tokio::spawn(async move { let _ = run(config, rx).await; });

    tokio::signal::ctrl_c().await.ok();
    info!("worker: ctrl_c received, shutting down");
    common_net::shutdown::trigger(&tx);
    let _ = j.await;
    Ok(())
}

pub async fn run(
    config: WorkerConfig,
    shutdown_rx: common_net::shutdown::ShutdownReceiver,
) -> Result<(), BoxError> {
    simulation_metrics().on_startup();

    let _metrics_task = metrics::spawn_metrics_exporter(config.metrics_addr, METRICS_PATH, "worker");

    let state = Arc::new(crate::rpc::WorkerState::default());
    let svc = crate::rpc::WorkerService::new(state.clone());

    info!(addr = %config.rpc_addr, "worker: starting gRPC");
    let task = tokio::spawn(async move { crate::rpc::serve_rpc(config.rpc_addr, svc).await; });

    common_net::shutdown::wait(shutdown_rx).await;
    task.abort();
    Ok(())
}

fn env_socket(key: &str, default: &str) -> Result<SocketAddr, BoxError> {
    let value = std::env::var(key).unwrap_or_else(|_| default.to_string());
    Ok(value.parse().map_err(|err| Box::new(err) as BoxError)?)
}

pub mod rpc;
pub mod snapshot;
