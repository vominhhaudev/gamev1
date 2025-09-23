param(
  [ValidateSet('gateway','worker','room-manager')]
  [string]$Service,
  [string]$GatewayBind = "127.0.0.1:8080",
  [string]$WorkerRpc = "127.0.0.1:50051",
  [string]$WorkerMetrics = "127.0.0.1:3100",
  [string]$RoomMgrMetrics = "127.0.0.1:3200"
)

$ErrorActionPreference = 'Stop'

switch ($Service) {
  'gateway' {
    $env:GATEWAY_BIND_ADDR = $GatewayBind
    $env:WORKER_ENDPOINT = "http://$WorkerRpc"
    cargo run -p gateway
  }
  'worker' {
    $env:WORKER_RPC_ADDR = $WorkerRpc
    $env:WORKER_METRICS_ADDR = $WorkerMetrics
    cargo run -p worker
  }
  'room-manager' {
    $env:ROOM_MANAGER_METRICS_ADDR = $RoomMgrMetrics
    cargo run -p room-manager
  }
}

