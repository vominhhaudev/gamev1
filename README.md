# gamev1

## 🚀 Khởi động nhanh

### Một lệnh duy nhất:
```bash
# Windows
run-gamev1.bat

# PowerShell - Chạy toàn bộ hệ thống (worker + client)
.\run-game-client-integration.ps1

# Hoặc chạy riêng lẻ:
.\restart-all-services.ps1  # Chỉ backend services
```

### Truy cập:
- **Game Client**: http://localhost:5173
- **Game trực tiếp**: http://localhost:5173/game
- **Đăng nhập**: admin@pocketbase.local / 123456789

### 🎮 Cách chơi game:
1. Mở http://localhost:5173/game trong trình duyệt
2. Nhấn "Join Game" để kết nối với game server
3. Sử dụng **WASD** để di chuyển nhân vật
4. Giữ **Shift** để chạy nhanh (sprint)
5. Thu thập các **hình tròn vàng** để tăng điểm
6. Tránh **hình tròn đỏ** (enemies) kẻo bị tấn công
7. Các **hình tròn xám** là vật cản không thể đi qua

## 📚 Tài liệu chi tiết
- [Quick Start Guide](QUICK-START-GUIDE.md) - Hướng dẫn đầy đủ
- [Client Setup](CLIENT-SETUP-GUIDE.md) - Thiết lập client
- [Node.js Install](NODEJS-INSTALL-GUIDE.md) - Cài đặt Node.js

## 🏗️ Kiến trúc hệ thống
- **Frontend**: SvelteKit + TypeScript
- **Backend**: Rust (Axum) + PocketBase
- **Database**: PocketBase (SQLite)
- **Authentication**: JWT + PocketBase

## 🔧 Phát triển
```bash
# Setup lần đầu
npm install

# Chạy development
npm run dev

# Build production
npm run build
```

## 📝 Ghi chú
- Đăng nhập thành công hiển thị user info và nút Logout
- Không còn lỗi "Failed to fetch" nhờ Vite Proxy
- Tự động xử lý CORS giữa client và gateway

## 🎮 Tính năng Game đã triển khai

### ✅ Hệ thống Core
- **ECS Architecture**: Sử dụng Bevy ECS để quản lý entities
- **Physics Engine**: Rapier 3D physics simulation
- **Network Layer**: gRPC-based communication với input buffering
- **Fixed Timestep**: Game loop 60 FPS với interpolation phù hợp

### ✅ Gameplay Mechanics
- **Player Movement**: Điều khiển nhân vật dựa trên physics với xử lý input
- **Pickup Collection**: Hệ thống điểm với collision detection
- **Dynamic Entities**:
  - Obstacles (tường, gai)
  - Power-ups (tăng tốc, nhảy cao, bất tử)
  - Enemies (cơ bản, nhanh, tank với AI)
- **Combat System**: Enemy AI với pattern tấn công và damage cho player
- **Collision Detection**: Xử lý va chạm thời gian thực cho mọi loại entity

### ✅ Tính năng nâng cao
- **Input Buffer**: Bù trừ độ trễ mạng với sequence numbers
- **Entity Lifecycle**: Spawn, despawn và cleanup phù hợp
- **Comprehensive Testing**: Test tích hợp end-to-end
- **Logging & Metrics**: Tracing chi tiết và monitoring hiệu suất

## 🧪 Chạy Tests

```bash
# Chạy tất cả tests
cargo test

# Chạy test cụ thể
cargo test --package worker                    # Worker tests
cargo test test_game_simulation_basic         # Basic simulation
cargo test test_comprehensive_game_simulation # Full gameplay test

# Chạy integration tests (cần async runtime)
cargo test test_end_to_end_client_worker_integration
cargo test test_input_processing_end_to_end
```

## 🏗️ Kiến trúc Game Engine

### Simulation Engine (`worker/src/simulation.rs`)
- `GameWorld`: Simulation chính với ECS + Physics
- `PlayerInput`: Network input với sequence numbers
- `GameSnapshot`: Serialization trạng thái world
- Collision detection và gameplay logic toàn diện

### Network Layer (`worker/src/rpc.rs`)
- gRPC service cho giao tiếp client
- Xử lý và validate input
- Streaming snapshot thời gian thực

### Hệ thống Entity
- **Players**: Nhân vật có thể di chuyển với physics bodies
- **Pickups**: Items có thể thu thập với giá trị điểm
- **Obstacles**: Object va chạm tĩnh
- **Power-ups**: Tăng cường khả năng tạm thời
- **Enemies**: Entity AI-controlled với combat

## 🎯 Gameplay Features

1. **Movement System**: Điều khiển player dựa trên physics với network input
2. **Scoring**: Thu thập pickups để tăng điểm
3. **Combat**: Enemies tấn công players, power-ups bảo vệ
4. **AI Behavior**: Enemies tìm kiếm và tấn công players thông minh
5. **Collision Handling**: Tất cả entities tương tác thực tế

## 🚀 Trạng thái phát triển

**🎉 HOÀN THÀNH!** Tích hợp Client-Worker hoàn chỉnh:

### ✅ Đã triển khai:
- ✅ **Game Engine hoàn chỉnh** với ECS, Physics, Gameplay
- ✅ **Client Frontend** với Svelte + real-time rendering
- ✅ **Network Integration** với gRPC và input buffering
- ✅ **Interactive Gameplay** với controls và visual feedback
- ✅ **End-to-End Testing** với integration scripts

### 🎮 Client tích hợp bao gồm:
- **Real-time 3D Rendering** với Canvas 2D (có thể nâng cấp lên 3D)
- **Live Input Processing** (WASD movement, Shift sprint)
- **Game State Visualization** với màu sắc phân biệt entities
- **Connection Management** với error handling
- **Game Statistics Display** với tick count và entity counts

### 🚀 Sẵn sàng sử dụng:
```powershell
# Chạy toàn bộ hệ thống
.\run-game-client-integration.ps1

# Sau đó mở: http://localhost:5173/game
```

Hệ thống đã sẵn sàng để mở rộng thêm tính năng multiplayer, nâng cấp graphics, hoặc phát triển gameplay nâng cao hơn!