# Test script để kiểm tra kết nối Client ↔ Worker
Write-Host "🔧 Testing GameV1 Client ↔ Worker Connection" -ForegroundColor Cyan

# 1. Kiểm tra Worker service
Write-Host "1. Checking Worker service..." -ForegroundColor Yellow
try {
    $workerPort = 50051
    $tcpClient = New-Object System.Net.Sockets.TcpClient
    $tcpClient.Connect("127.0.0.1", $workerPort)
    Write-Host "   ✅ Worker is running on port $workerPort" -ForegroundColor Green
    $tcpClient.Close()
} catch {
    Write-Host "   ❌ Worker is NOT running on port $workerPort" -ForegroundColor Red
    Write-Host "   Run: cargo run --bin worker" -ForegroundColor Yellow
}

# 2. Kiểm tra Client service
Write-Host "2. Checking Client service..." -ForegroundColor Yellow
try {
    $clientPort = 5173
    $response = Invoke-WebRequest -Uri "http://localhost:$clientPort" -UseBasicParsing -TimeoutSec 5
    Write-Host "   ✅ Client is running on port $clientPort" -ForegroundColor Green
} catch {
    Write-Host "   ❌ Client is NOT running on port $clientPort" -ForegroundColor Red
    Write-Host "   Run: cd client; npm run dev" -ForegroundColor Yellow
}

# 3. Test gRPC connection từ góc nhìn client
Write-Host "3. Testing gRPC connection..." -ForegroundColor Yellow
Write-Host "   Open browser to: http://localhost:5173" -ForegroundColor Cyan
Write-Host "   Check browser console for connection logs" -ForegroundColor Cyan

# 4. Show current status
Write-Host "4. Current Status:" -ForegroundColor Yellow
Write-Host "   - Worker: $(if (Test-Connection -ComputerName 127.0.0.1 -Port 50051 -Quiet) {'🟢 Running'} else {'🔴 Stopped'})"
Write-Host "   - Client: $(if (Test-Connection -ComputerName 127.0.0.1 -Port 5173 -Quiet) {'🟢 Running'} else {'🔴 Stopped'})"

Write-Host ""
Write-Host "📋 Next Steps:" -ForegroundColor Magenta
Write-Host "1. Ensure both services are running" -ForegroundColor White
Write-Host "2. Open http://localhost:5173 in browser" -ForegroundColor White
Write-Host "3. Check browser console for connection status" -ForegroundColor White
Write-Host "4. Test gameplay interactions" -ForegroundColor White

Write-Host ""
Write-Host "🎮 To start missing services:" -ForegroundColor Magenta
Write-Host "Start Worker: cargo run --bin worker" -ForegroundColor Gray
Write-Host "Start Client: cd client; npm run dev" -ForegroundColor Gray
