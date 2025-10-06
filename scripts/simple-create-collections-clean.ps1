# Script đơn giản tạo collections cho dự án game

Write-Host "Tạo Collections cho dự án Game..." -ForegroundColor Cyan

# Test connection
try {
    $health = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get -TimeoutSec 5
    Write-Host "PocketBase hoạt động tốt" -ForegroundColor Green
} catch {
    Write-Host "Không thể kết nối PocketBase: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

Write-Host "Đăng nhập admin..." -ForegroundColor Yellow

# Đăng nhập
$authData = @{
    identity = "admin@pocketbase.local"
    password = "123456789"
} | ConvertTo-Json

try {
    $authResponse = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/admins/auth-with-password" -Method Post -Body $authData -ContentType "application/json"
    $authToken = $authResponse.token
    Write-Host "Đăng nhập thành công!" -ForegroundColor Green

    $headers = @{
        "Authorization" = "Bearer $authToken"
        "Content-Type" = "application/json"
    }

    # Tạo Games Collection
    Write-Host "Tạo collection: games..." -ForegroundColor Yellow
    $gamesData = @{
        name = "games"
        type = "base"
        schema = @(
            @{ name = "name"; type = "text"; required = $true; options = @{ max = 100 } }
            @{ name = "max_players"; type = "number"; required = $true; options = @{ min = 2; max = 8 } }
            @{ name = "status"; type = "select"; required = $true; options = @{ values = @("waiting", "playing", "finished") } }
        )
    } | ConvertTo-Json -Depth 3

    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Post -Body $gamesData -Headers $headers
    Write-Host "Đã tạo collection games" -ForegroundColor Green

    # Tạo Users Collection
    Write-Host "Tạo collection: users..." -ForegroundColor Yellow
    $usersData = @{
        name = "users"
        type = "base"
        schema = @(
            @{ name = "username"; type = "text"; required = $true; options = @{ max = 50 } }
            @{ name = "email"; type = "email"; required = $true }
            @{ name = "score"; type = "number"; options = @{ min = 0 } }
        )
    } | ConvertTo-Json -Depth 3

    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Post -Body $usersData -Headers $headers
    Write-Host "Đã tạo collection users" -ForegroundColor Green

    Write-Host ""
    Write-Host "Collections đã được tạo thành công!" -ForegroundColor Cyan
    Write-Host "Bạn có thể sử dụng các collections này trong dự án game" -ForegroundColor White

} catch {
    Write-Host "Lỗi: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Giải pháp thay thế:" -ForegroundColor Yellow
    Write-Host "   • Mở trình duyệt: http://127.0.0.1:8090/_/" -ForegroundColor Gray
    Write-Host "   • Đăng nhập với email: admin@pocketbase.local" -ForegroundColor Gray
    Write-Host "   • Tạo collections thủ công qua giao diện web" -ForegroundColor Gray
}

