use common_net::{shutdown, telemetry};
use gateway::GatewayConfig;
use room_manager::RoomManagerConfig;
use tokio::sync::oneshot;
use worker::WorkerConfig;

#[tokio::test]
async fn orchestrator_propagates_failure() -> Result<(), server::BoxError> {
    telemetry::init("server-chaos-test");

    let (gateway_ready_tx, gateway_ready_rx) = oneshot::channel();

    let gateway_config = GatewayConfig {
        bind_addr: "127.0.0.1:0"
            .parse()
            .map_err(|err| Box::new(err) as server::BoxError)?,
        worker_endpoint: "http://127.0.0.1:50051".to_string(),
        ready_tx: Some(gateway_ready_tx),
    };

    let worker_config = WorkerConfig {
        rpc_addr: "127.0.0.1:0"
            .parse()
            .map_err(|err| Box::new(err) as server::BoxError)?,
        metrics_addr: "127.0.0.1:0"
            .parse()
            .map_err(|err| Box::new(err) as server::BoxError)?,
        fail_fast: true,
    };

    let room_manager_config = RoomManagerConfig {
        metrics_addr: "127.0.0.1:0"
            .parse()
            .map_err(|err| Box::new(err) as server::BoxError)?,
        ready_tx: None,
    };

    let config = server::ServerConfig {
        gateway: gateway_config,
        worker: worker_config,
        room_manager: room_manager_config,
    };

    let (_shutdown_tx, shutdown_rx) = shutdown::channel();

    let orchestrator = tokio::spawn(server::run_with_shutdown(config, shutdown_rx));

    let _gateway_addr = gateway_ready_rx
        .await
        .map_err(|err| Box::new(err) as server::BoxError)?;

    let orchestrator_result = orchestrator
        .await
        .map_err(|err| Box::new(err) as server::BoxError)?;

    assert!(
        orchestrator_result.is_err(),
        "orchestrator should surface worker failure"
    );

    Ok(())
}
