use worker::WorkerConfig;

use common_net::telemetry;

#[tokio::main]
async fn main() {
    telemetry::init("worker");

    let config = match WorkerConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(%err, "worker: cau hinh khong hop le");
            return;
        }
    };

    if let Err(err) = worker::run_with_ctrl_c(config).await {
        tracing::error!(%err, "worker ket thuc do loi");
    }
}
