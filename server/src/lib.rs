use std::{fs, future::Future, path::Path, pin::Pin};

use common_net::shutdown;
use gateway::{GatewayConfig, GatewaySettings};
use room_manager::{RoomManagerConfig, RoomManagerSettings};
use tokio::task::JoinSet;
use tracing::{error, info};
use worker::{WorkerConfig, WorkerSettings};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServerSettings {
    pub gateway: GatewaySettings,
    pub worker: WorkerSettings,
    pub room_manager: RoomManagerSettings,
}

impl ServerSettings {
    pub fn from_env() -> Result<Self, BoxError> {
        Ok(Self {
            gateway: GatewaySettings::from_env()?,
            worker: WorkerSettings::from_env()?,
            room_manager: RoomManagerSettings::from_env()?,
        })
    }

    pub fn from_file(path: &Path) -> Result<Self, BoxError> {
        let raw = fs::read_to_string(path).map_err(|err| Box::new(err) as BoxError)?;
        let settings = serde_json::from_str(&raw).map_err(|err| Box::new(err) as BoxError)?;
        Ok(settings)
    }

    pub fn into_config(self) -> ServerConfig {
        ServerConfig::from_settings(self)
    }
}

#[derive(Debug)]
pub struct ServerConfig {
    pub gateway: GatewayConfig,
    pub worker: WorkerConfig,
    pub room_manager: RoomManagerConfig,
}

impl ServerConfig {
    pub fn from_settings(settings: ServerSettings) -> Self {
        Self {
            gateway: GatewayConfig::from_settings(settings.gateway),
            worker: WorkerConfig::from_settings(settings.worker).expect("valid worker settings"),
            room_manager: RoomManagerConfig::from_settings(settings.room_manager),
        }
    }

    pub fn from_env() -> Result<Self, BoxError> {
        ServerSettings::from_env().map(Self::from_settings)
    }
}

pub async fn run() -> Result<(), BoxError> {
    let config = ServerConfig::from_env()?;
    run_with_ctrl_c(config).await
}

pub async fn run_with_ctrl_c(config: ServerConfig) -> Result<(), BoxError> {
    let (shutdown_tx, shutdown_rx) = shutdown::channel();

    let ctrl_c = tokio::spawn(async move {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!(%err, "server: khong the lang nghe ctrl_c");
        }
        shutdown::trigger(&shutdown_tx);
    });

    let result = run_with_shutdown(config, shutdown_rx).await;

    ctrl_c.abort();
    result
}

pub async fn run_with_shutdown(
    config: ServerConfig,
    shutdown_rx: shutdown::ShutdownReceiver,
) -> Result<(), BoxError> {
    let (service_shutdown_tx, service_shutdown_rx) = shutdown::channel();

    let ServerConfig {
        gateway,
        worker,
        room_manager,
    } = config;

    let mut join_set: JoinSet<Result<(), BoxError>> = JoinSet::new();

    let gateway_shutdown = service_shutdown_rx.clone();
    join_set.spawn(async move { gateway::run(gateway, gateway_shutdown).await });

    let worker_shutdown = service_shutdown_rx.clone();
    join_set.spawn(async move { worker::run(worker, worker_shutdown).await });

    let room_manager_shutdown = service_shutdown_rx;
    join_set.spawn(async move { room_manager::run(room_manager, room_manager_shutdown).await });

    let mut shutdown_future: Pin<Box<dyn Future<Output = ()> + Send>> =
        Box::pin(shutdown::wait(shutdown_rx));
    let mut service_error: Option<BoxError> = None;

    loop {
        tokio::select! {
            _ = &mut shutdown_future => {
                info!("server: nhan tin hieu shutdown tu ben ngoai");
                shutdown::trigger(&service_shutdown_tx);
                break;
            }
            maybe_task = join_set.join_next() => {
                match maybe_task {
                    Some(Ok(Ok(()))) => continue,
                    Some(Ok(Err(err))) => {
                        error!(%err, "server: mot service ket thuc voi loi");
                        service_error = Some(err);
                        shutdown::trigger(&service_shutdown_tx);
                        break;
                    }
                    Some(Err(join_err)) => {
                        let err: BoxError = Box::new(join_err);
                        error!(%err, "server: join handle gap loi");
                        service_error = Some(err);
                        shutdown::trigger(&service_shutdown_tx);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    shutdown::trigger(&service_shutdown_tx);

    let drain_result = drain_join_set(&mut join_set).await;

    if let Some(err) = service_error {
        return Err(err);
    }

    drain_result
}

async fn drain_join_set(join_set: &mut JoinSet<Result<(), BoxError>>) -> Result<(), BoxError> {
    let mut first_err: Option<BoxError> = None;

    while let Some(task) = join_set.join_next().await {
        match task {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if first_err.is_none() {
                    first_err = Some(err);
                }
            }
            Err(join_err) => {
                if first_err.is_none() {
                    first_err = Some(Box::new(join_err) as BoxError);
                }
            }
        }
    }

    if let Some(err) = first_err {
        return Err(err);
    }

    Ok(())
}
