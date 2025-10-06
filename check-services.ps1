# Simple service status checker
Write-Host "=== GameV1 Service Status ===" -ForegroundColor Cyan

# Check Gateway
$gateway = Get-Process -Name "*gateway*" -ErrorAction SilentlyContinue
if ($gateway) {
    $port8080 = netstat -ano | findstr ":8080.*LISTENING"
    $gatewayStatus = if ($port8080) { "OK" } else { "ERROR" }
    Write-Host "Gateway: " -NoNewline
    if ($gatewayStatus -eq "OK") {
        Write-Host $gatewayStatus -ForegroundColor Green
    } else {
        Write-Host $gatewayStatus -ForegroundColor Red
    }
} else {
    Write-Host "Gateway: " -NoNewline
    Write-Host "STOPPED" -ForegroundColor Yellow
}

# Check Worker
$worker = Get-Process -Name "*worker*" -ErrorAction SilentlyContinue
if ($worker) {
    Write-Host "Worker: " -NoNewline
    Write-Host "OK" -ForegroundColor Green
} else {
    Write-Host "Worker: " -NoNewline
    Write-Host "STOPPED" -ForegroundColor Yellow
}

# Check PocketBase
$pocketbase = Get-Process -Name "*pocketbase*" -ErrorAction SilentlyContinue
if ($pocketbase) {
    Write-Host "PocketBase: " -NoNewline
    Write-Host "OK" -ForegroundColor Green
} else {
    Write-Host "PocketBase: " -NoNewline
    Write-Host "STOPPED" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== Next Priorities ===" -ForegroundColor Cyan
Write-Host "1. Load Testing with 100+ concurrent clients" -ForegroundColor Yellow
Write-Host "2. Performance optimization and memory pooling" -ForegroundColor Yellow
Write-Host "3. Advanced matchmaking features" -ForegroundColor Yellow
Write-Host "4. Alpha release preparation" -ForegroundColor Yellow
