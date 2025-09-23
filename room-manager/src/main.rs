use room_manager::RoomManagerConfig;

use common_net::telemetry;

#[tokio::main]
async fn main() {
    telemetry::init("room-manager");

    let config = match RoomManagerConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(%err, "room-manager: cau hinh khong hop le");
            return;
        }
    };

    if let Err(err) = room_manager::run_with_ctrl_c(config).await {
        tracing::error!(%err, "room-manager ket thuc do loi");
    }
}
