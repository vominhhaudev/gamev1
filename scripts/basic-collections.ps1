# Script cơ bản tạo collections

Write-Host "Tao Collections..." -ForegroundColor Cyan

# Test connection
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get

# Đăng nhập
$authData = @{identity="admin@pocketbase.local"; password="123456789"} | ConvertTo-Json
$authResponse = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/admins/auth-with-password" -Method Post -Body $authData -ContentType "application/json"
$token = $authResponse.token

$headers = @{"Authorization"="Bearer $token"; "Content-Type"="application/json"}

# Games collection
$games = @{name="games"; type="base"; schema=@(@{name="name"; type="text"; required=$true}, @{name="max_players"; type="number"; required=$true})} | ConvertTo-Json -Depth 3
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Post -Body $games -Headers $headers

# Users collection
$users = @{name="users"; type="base"; schema=@(@{name="username"; type="text"; required=$true}, @{name="email"; type="email"; required=$true})} | ConvertTo-Json -Depth 3
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Post -Body $users -Headers $headers

Write-Host "Hoan thanh!" -ForegroundColor Green


