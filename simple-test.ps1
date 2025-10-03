# Simple connection test
Write-Host "Testing GameV1 Services..." -ForegroundColor Cyan

# Test Worker (port 50051)
try {
    $tcpClient = New-Object System.Net.Sockets.TcpClient
    $tcpClient.Connect("127.0.0.1", 50051)
    Write-Host "Worker: RUNNING" -ForegroundColor Green
    $tcpClient.Close()
} catch {
    Write-Host "Worker: STOPPED" -ForegroundColor Red
}

# Test Client (port 5173)
try {
    $response = Invoke-WebRequest -Uri "http://localhost:5173" -UseBasicParsing -TimeoutSec 3
    Write-Host "Client: RUNNING" -ForegroundColor Green
} catch {
    Write-Host "Client: STOPPED" -ForegroundColor Red
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Start Worker: cargo run --bin worker"
Write-Host "2. Start Client: cd client; npm run dev"
Write-Host "3. Open: http://localhost:5173"