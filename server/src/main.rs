use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

use common_net::telemetry;
use server::{BoxError, ServerConfig, ServerSettings};

#[derive(Debug, Parser)]
#[command(author, version, about = "Server orchestrator for gamev1")]
struct ServerCli {
    #[arg(long = "config", value_name = "PATH")]
    config_path: Option<PathBuf>,

    #[arg(long, value_name = "ADDR")]
    gateway_bind: Option<SocketAddr>,

    #[arg(long, value_name = "ADDR")]
    worker_metrics_addr: Option<SocketAddr>,

    #[arg(long, value_name = "ADDR")]
    room_manager_metrics_addr: Option<SocketAddr>,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    worker_fail_fast: bool,
}

impl ServerCli {
    fn resolve_config_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.config_path {
            return Some(path.clone());
        }
        std::env::var("SERVER_CONFIG_PATH").ok().map(PathBuf::from)
    }

    fn apply_overrides(&self, settings: &mut ServerSettings) {
        if let Some(addr) = self.gateway_bind {
            settings.gateway.bind_addr = addr;
        }
        if let Some(addr) = self.worker_metrics_addr {
            settings.worker.metrics_addr = addr.to_string();
        }
        if let Some(addr) = self.room_manager_metrics_addr {
            settings.room_manager.metrics_addr = addr;
        }
        if self.worker_fail_fast {
            settings.worker.fail_fast = true;
        }
    }
}

fn build_config(cli: &ServerCli) -> Result<ServerConfig, BoxError> {
    let mut settings = if let Some(path) = cli.resolve_config_path() {
        ServerSettings::from_file(&path)?
    } else {
        ServerSettings::from_env()?
    };

    cli.apply_overrides(&mut settings);

    Ok(settings.into_config())
}

#[tokio::main]
async fn main() {
    telemetry::init("server");

    let cli = ServerCli::parse();

    let config = match build_config(&cli) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(%err, "server: khong the khoi tao cau hinh");
            return;
        }
    };

    if let Err(err) = server::run_with_ctrl_c(config).await {
        tracing::error!(%err, "server ket thuc do loi");
    }
}
