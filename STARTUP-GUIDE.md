# 🚀 Hướng Dẫn Khởi Động Dự Án Game

## 📋 Mục Lục
- [Khởi Động Nhanh](#khởi-động-nhanh)
- [Cài Đặt Môi Trường](#cài-đặt-môi-trường)
- [Khởi Động PocketBase](#khởi-động-pocketbase)
- [Thêm Dữ Liệu Mới](#thêm-dữ-liệu-mới)
- [Chạy Các Services](#chạy-các-services)
- [Kiểm Tra Hoạt Động](#kiểm-tra-hoạt-động)
- [Troubleshooting](#troubleshooting)

---

## 🚀 Khởi Động Nhanh

### 1. Khởi động tất cả services cùng lúc

```powershell
# Từ thư mục gốc của dự án
powershell -File scripts/run-dev.ps1
```

Script này sẽ:
- ✅ Khởi động PocketBase ở http://127.0.0.1:8090
- ✅ Khởi động Worker service (gRPC)
- ✅ Khởi động Gateway service (HTTP API)

### 2. Kiểm tra hoạt động

```powershell
# Kiểm tra API health
Invoke-RestMethod -Uri "http://127.0.0.1:8080/healthz" -Method Get

# Kiểm tra metrics
Invoke-RestMethod -Uri "http://127.0.0.1:8080/metrics" -Method Get

# Kiểm tra WebSocket
# Mở trình duyệt: http://127.0.0.1:8080
```

---

## 🔧 Cài Đặt Môi Trường

### Yêu cầu hệ thống
- **Rust**: 1.70+ (cài bằng rustup)
- **Node.js**: 18+ (cho client)
- **PocketBase**: Sẽ được tải tự động

### Cài đặt Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Cài đặt Node.js
```bash
# Sử dụng nvm (khuyên dùng)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18
```

### Chuẩn bị dự án
```bash
# Clone dự án (nếu chưa có)
git clone <repository-url>
cd gamev1

# Cài dependencies Rust
cargo build

# Cài dependencies Node.js (client)
cd client && npm install && cd ..
```

---

## 🗄️ Khởi Động PocketBase

### Cách 1: Khởi động thủ công

```powershell
# Khởi động PocketBase server
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090

# Trong terminal mới, kiểm tra hoạt động
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get
```

### Cách 2: Sử dụng script tự động

```powershell
# Script này sẽ tự động tải PocketBase nếu chưa có
powershell -File scripts/setup-pocketbase.ps1

# Khởi động với script development
powershell -File scripts/run-dev.ps1
```

### Cách 3: Khởi động riêng lẻ từng service

```powershell
# 1. Khởi động PocketBase
powershell -File scripts/run-service.ps1 pocketbase

# 2. Trong terminal mới: Khởi động Worker
powershell -File scripts/run-service.ps1 worker

# 3. Trong terminal mới: Khởi động Gateway
powershell -File scripts/run-service.ps1 gateway
```

### Truy cập Admin Dashboard

Sau khi khởi động PocketBase:
- 🌐 **Admin UI**: http://127.0.0.1:8090/_/
- 👤 **Đăng nhập**: `vominhhauviettel@gmail.com` / `pt123456789`

---

## ➕ Thêm Dữ Liệu Mới

### Cách 1: Sử dụng Admin Dashboard

1. Truy cập http://127.0.0.1:8090/_/
2. Đăng nhập với tài khoản admin
3. Vào **Collections** → Chọn collection cần thêm dữ liệu
4. Click **New record** để thêm bản ghi mới

### Cách 2: Sử dụng PowerShell Scripts

#### Thêm Games mới

```powershell
# Thêm game mới qua API
$game = @{
    name = "Battle Royale Advanced"
    max_players = 12
    status = "waiting"
} | ConvertTo-Json

$response = Invoke-RestMethod `
    -Uri "http://127.0.0.1:8090/api/collections/games/records" `
    -Method Post `
    -Body $game `
    -ContentType "application/json"

$response | ConvertTo-Json
```

#### Thêm Players mới

```powershell
# Thêm player mới
$player = @{
    username = "new_player"
    email = "newplayer@game.com"
    score = 0
    is_online = $true
} | ConvertTo-Json

$response = Invoke-RestMethod `
    -Uri "http://127.0.0.1:8090/api/collections/players/records" `
    -Method Post `
    -Body $player `
    -ContentType "application/json"

$response | ConvertTo-Json
```

#### Script hàng loạt

```powershell
# Chạy script thêm dữ liệu mẫu
powershell -File scripts/add-sample-data.ps1
```

### Cách 3: Sử dụng cURL (Command Line)

```bash
# Thêm game mới
curl -X POST http://127.0.0.1:8090/api/collections/games/records \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Team Fortress",
    "max_players": 16,
    "status": "waiting"
  }'

# Thêm player mới
curl -X POST http://127.0.0.1:8090/api/collections/players/records \
  -H "Content-Type: application/json" \
  -d '{
    "username": "curl_player",
    "email": "curl@player.com",
    "score": 1000,
    "is_online": true
  }'
```

### Cách 4: Sử dụng Rust Code

```rust
use pocketbase::PocketBaseClient;

// Trong Rust code
let pb_client = PocketBaseClient::new();
pb_client.authenticate("admin@example.com", "password").await?;

let game_data = serde_json::json!({
    "name": "Rust Game",
    "max_players": 8,
    "status": "waiting"
});

let game_id = pb_client.create_game(&game_data).await?;
println!("Created game with ID: {}", game_id);
```

---

## 🏃 Chạy Các Services

### Development Mode (Khuyên dùng)

```powershell
# Chạy tất cả services cùng lúc
powershell -File scripts/run-dev.ps1

# Các services sẽ khởi động:
# - PocketBase: http://127.0.0.1:8090
# - Gateway API: http://127.0.0.1:8080
# - Worker gRPC: 127.0.0.1:50051
# - Metrics: http://127.0.0.1:8080/metrics
```

### Production Mode

```powershell
# Khởi động từng service riêng biệt
powershell -File scripts/run-service.ps1 pocketbase
powershell -File scripts/run-service.ps1 worker
powershell -File scripts/run-service.ps1 gateway

# Hoặc sử dụng orchestrator
powershell -File scripts/run-orchestrator.ps1 -Config server/server-settings.json
```

### Kiểm tra logs

```powershell
# Xem logs real-time
Get-Content -Path logs/*.log -Wait -Tail 10

# Hoặc trong terminal riêng:
tail -f logs/gateway.log
tail -f logs/worker.log
```

---

## ✅ Kiểm Tra Hoạt Động

### 1. Kiểm tra API Endpoints

```powershell
# Health check
Invoke-RestMethod -Uri "http://127.0.0.1:8080/healthz" -Method Get

# Version info
Invoke-RestMethod -Uri "http://127.0.0.1:8080/version" -Method Get

# Metrics
Invoke-RestMethod -Uri "http://127.0.0.1:8080/metrics" -Method Get
```

### 2. Kiểm tra PocketBase

```powershell
# Health check
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get

# Liệt kê collections
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Get

# Liệt kê games
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get
```

### 3. Kiểm tra kết nối giữa các services

```powershell
# Test WebSocket connection
# Mở trình duyệt và vào: http://127.0.0.1:8080
# Mở Developer Tools → Console để xem WebSocket messages

# Test gRPC connection (internal)
# Worker sẽ tự động kết nối đến PocketBase khi khởi động
```

### 4. Kiểm tra dữ liệu

```powershell
# Xem tất cả games
$games = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get
$games.items | Format-Table id, name, max_players, status

# Xem tất cả players
$players = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/players/records" -Method Get
$players.items | Format-Table id, username, email, score, is_online

# Xem game sessions
$sessions = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/game_sessions/records" -Method Get
$sessions.items | Format-Table id, game_id, player_id, status
```

---

## 🔧 Các Lệnh Hữu Ích

### Quản lý PocketBase

```powershell
# Xóa tất cả dữ liệu (giữ schema)
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get |
  ForEach-Object { $_.items } |
  ForEach-Object { Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records/$($_.id)" -Method Delete }

# Tạo admin user mới
# Truy cập http://127.0.0.1:8090/_/auth/signup
```

### Quản lý Services

```powershell
# Dừng tất cả services
powershell -File scripts/stop-dev.ps1

# Khởi động lại một service cụ thể
powershell -File scripts/run-service.ps1 gateway

# Kiểm tra tiến trình đang chạy
Get-Process -Name "cargo","pocketbase"
```

### Debug và Monitoring

```powershell
# Xem metrics real-time
while ($true) {
    $metrics = Invoke-RestMethod -Uri "http://127.0.0.1:8080/metrics" -Method Get
    $metrics | Select-String "gw_inputs"
    Start-Sleep -Seconds 5
    Clear-Host
}

# Kiểm tra kết nối database
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get
```

---

## 🚨 Troubleshooting

### Lỗi thường gặp

#### 1. "Address already in use"
```powershell
# Dừng các tiến trình đang chạy
powershell -File scripts/stop-dev.ps1

# Hoặc kill thủ công
Get-Process -Name "cargo","pocketbase" | Stop-Process -Force
```

#### 2. PocketBase không khởi động được
```powershell
# Kiểm tra port 8090 có bị chiếm không
netstat -ano | findstr :8090

# Kill tiến trình chiếm port
taskkill /PID <PID> /F

# Khởi động lại
powershell -File scripts/run-dev.ps1
```

#### 3. Không thể kết nối đến database
```powershell
# Kiểm tra PocketBase có chạy không
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get

# Nếu không được, khởi động lại
powershell -File scripts/run-service.ps1 pocketbase
```

#### 4. Rust compile lỗi
```powershell
# Làm sạch và build lại
cargo clean
cargo build

# Nếu vẫn lỗi, cập nhật Rust
rustup update stable
```

#### 5. WebSocket không kết nối được
```powershell
# Kiểm tra Gateway có chạy không
Invoke-RestMethod -Uri "http://127.0.0.1:8080/healthz" -Method Get

# Kiểm tra logs
Get-Content -Path logs/gateway.log -Wait -Tail 10
```

### Logs và Debug

```powershell
# Xem logs của tất cả services
Get-ChildItem logs/ -Name | ForEach-Object {
    Write-Host "=== $_ ==="
    Get-Content "logs/$_" -Tail 5
    Write-Host ""
}

# Theo dõi logs real-time
Get-Content -Path logs/gateway.log -Wait -Tail 10
```

### Performance Monitoring

```powershell
# Xem metrics tổng hợp
$metrics = Invoke-RestMethod -Uri "http://127.0.0.1:8080/metrics" -Method Get
$metrics | Select-String "gw_|worker_|room_"

# Monitor memory usage
Get-Process -Name "cargo","pocketbase" | Format-Table ProcessName, Id, CPU, WorkingSet
```

---

## 📚 Tài Liệu Tham Khảo

- [PocketBase Documentation](https://pocketbase.io/docs/)
- [Axum Web Framework](https://docs.rs/axum/)
- [Tonic gRPC](https://docs.rs/tonic/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)

---

## 🎯 Các Điểm Truy Cập Quan Trọng

| Service | URL | Mô tả |
|---------|-----|--------|
| **PocketBase Admin** | http://127.0.0.1:8090/_/ | Quản lý database |
| **Gateway API** | http://127.0.0.1:8080 | HTTP REST API |
| **Gateway Health** | http://127.0.0.1:8080/healthz | Health check |
| **Gateway Metrics** | http://127.0.0.1:8080/metrics | Prometheus metrics |
| **WebSocket** | ws://127.0.0.1:8080/ws | Real-time communication |
| **Worker gRPC** | 127.0.0.1:50051 | Internal gRPC API |

---

**🎉 Chúc bạn khởi động dự án thành công! Nếu có vấn đề gì, hãy kiểm tra phần Troubleshooting hoặc liên hệ nhóm phát triển.**
