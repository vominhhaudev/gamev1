param(
  [string]$WorkerAddr = "127.0.0.1:50051",
  [string]$GatewayAddr = "0.0.0.0:8080"
)

Write-Host "Starting worker (gRPC at $WorkerAddr)"
$env:WORKER_RPC_ADDR = $WorkerAddr
Start-Process -NoNewWindow powershell -ArgumentList "-NoProfile","-Command","cargo run -p worker" | Out-Null
Start-Sleep -Seconds 1

Write-Host "Starting gateway (HTTP at $GatewayAddr)"
$env:WORKER_GRPC_URI = "http://$WorkerAddr"
Start-Process -NoNewWindow powershell -ArgumentList "-NoProfile","-Command","cargo run -p gateway" | Out-Null

Write-Host "Both services launched. Health checks:" 
Write-Host "  GET http://$GatewayAddr/healthz"
Write-Host "  GET http://$GatewayAddr/version"
Write-Host "  GET http://$GatewayAddr/metrics"
Write-Host "WebSocket echo: ws://$GatewayAddr/ws"

