param(
  [string]$PocketBaseAddr = "127.0.0.1:8090",
  [string]$WorkerAddr = "127.0.0.1:50051",
  [string]$GatewayAddr = "0.0.0.0:8080"
)

Write-Host "Starting PocketBase (Admin UI at $PocketBaseAddr)"
$PocketBasePath = "pocketbase/pocketbase.exe"
if (Test-Path $PocketBasePath) {
    Start-Process -NoNewWindow $PocketBasePath -ArgumentList "serve" | Out-Null
    Write-Host "  PocketBase started successfully"
} else {
    Write-Host "  WARNING: PocketBase binary not found at $PocketBasePath"
    Write-Host "  Run: pwsh -File scripts/setup-pocketbase.ps1"
}
Start-Sleep -Seconds 2

Write-Host "Starting worker (gRPC at $WorkerAddr)"
$env:WORKER_RPC_ADDR = $WorkerAddr
Start-Process -NoNewWindow powershell -ArgumentList "-NoProfile","-Command","cargo run -p worker" | Out-Null
Start-Sleep -Seconds 1

Write-Host "Starting gateway (HTTP at $GatewayAddr)"
$env:WORKER_GRPC_URI = "http://$WorkerAddr"
Start-Process -NoNewWindow powershell -ArgumentList "-NoProfile","-Command","cargo run -p gateway" | Out-Null

Write-Host "All services launched. Access points:"
Write-Host "  PocketBase Admin: http://$PocketBaseAddr/_/"
Write-Host "  Gateway API: http://$GatewayAddr"
Write-Host "  Health check: http://$GatewayAddr/healthz"
Write-Host "  Metrics: http://$GatewayAddr/metrics"
Write-Host "  WebSocket: ws://$GatewayAddr/ws"

