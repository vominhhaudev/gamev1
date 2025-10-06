# 🚀 MULTIPLAYER TRANSITION ROADMAP - GameV1

## 🎯 TỔNG QUAN CHUYỂN ĐỔI

**Từ:** Single-Player Endless Runner 3D
**Sang:** Real-time Multiplayer Game Server (theo motaduan.md)

**Thời gian dự kiến:** 8-12 tuần
**Ưu tiên:** Network → Architecture → Features

---

## 📋 HIỆN TRẠNG (ĐÃ CÓ)

### ✅ Backend Foundation
- **Gateway**: HTTP API với Axum + WebSocket cơ bản
- **Worker**: Game logic với Rust (đơn giản)
- **PocketBase**: Database + Auth cơ bản
- **gRPC**: Communication giữa services

### ✅ Frontend Foundation
- **Client**: SvelteKit với Endless Runner 3D hoàn chỉnh
- **WebRTC**: Signaling cơ bản đã có
- **Auth**: Login system đã triển khai

---

## 🎯 KẾ HOẠCH CHUYỂN ĐỔI (8 TUẦN)

### **PHASE 1: NETWORK INFRASTRUCTURE** (Week 1-2)
*Ưu tiên cao nhất - Foundation cho multiplayer*

#### **1.1 QUIC/WebTransport Implementation**
```rust
// Gateway - Thêm WebTransport support
// - Streams (control, reliable)
// - Datagrams (state, unreliable)
// - Backpressure handling
```

**Tasks:**
- [ ] Install `wtransport` crate
- [ ] Implement QUIC server trong gateway
- [ ] Transport negotiation endpoint (`/negotiate`)
- [ ] Fallback chain: QUIC → WebRTC → WebSocket

#### **1.2 Enhanced WebSocket (Fallback)**
- [ ] Improve WebSocket với backpressure
- [ ] Message framing và compression
- [ ] Connection pooling

#### **1.3 Transport Selection Client**
```javascript
// Client - Smart transport selection
const transports = ['quic', 'webrtc', 'websocket'];
const bestTransport = await negotiateBestTransport();
```

---

### **PHASE 2: AUTHENTICATION & SECURITY** (Week 2-3)

#### **2.1 JWT & Session Management**
```rust
// Gateway - Enhanced auth middleware
// - JWT validation + refresh
// - Rate limiting per IP & user
// - Session persistence với Redis
```

**Tasks:**
- [ ] Implement JWT với refresh tokens
- [ ] Redis integration cho session store
- [ ] Rate limiting middleware
- [ ] CORS configuration hoàn chỉnh

#### **2.2 User Profile System**
- [ ] User registration với PocketBase
- [ ] Profile management
- [ ] Avatar và display name

---

### **PHASE 3: ECS ARCHITECTURE** (Week 3-4)
*Chuyển từ single-player sang multiplayer-ready*

#### **3.1 Entity Component System**
```rust
// Worker - ECS implementation
#[derive(Component)]
struct Player {
    id: PlayerId,
    position: Vec3,
    velocity: Vec3,
    input_sequence: u32,
}

#[derive(Component)]
struct NetworkSync {
    last_update: Instant,
    needs_sync: bool,
}
```

**Tasks:**
- [ ] Install `bevy_ecs` hoặc tự implement ECS
- [ ] Define components: Transform, Velocity, Player, NetworkSync
- [ ] Implement systems: Physics, Input, Network

#### **3.2 Fixed Timestep Physics**
- [ ] 60Hz game loop với accumulator
- [ ] Deterministic physics simulation
- [ ] Input prediction và reconciliation

---

### **PHASE 4: MULTIPLAYER CORE** (Week 5-6)

#### **4.1 AOI (Area of Interest)**
```rust
// Worker - Spatial partitioning
struct AOIGrid {
    cell_size: f32,
    grid: HashMap<GridCell, Vec<Entity>>,
}
```

**Tasks:**
- [ ] Implement grid-based AOI
- [ ] Entity visibility culling
- [ ] Interest management per player

#### **4.2 State Synchronization**
- [ ] Keyframe + delta compression
- [ ] Quantization (i16 cho position/rotation)
- [ ] Backpressure với buffer per-client

---

### **PHASE 5: ROOM SYSTEM** (Week 6-7)

#### **5.1 Room Manager Service**
```rust
// Room Manager - Microservice mới
struct Room {
    id: RoomId,
    players: Vec<PlayerId>,
    max_players: usize,
    game_state: GameState,
}
```

**Tasks:**
- [ ] Tạo room-manager service mới
- [ ] Room creation/joining logic
- [ ] Player assignment và routing

#### **5.2 Matchmaking**
- [ ] Simple matchmaking (first-available)
- [ ] Skill-based matching (sau này)
- [ ] Queue management

---

### **PHASE 6: PERSISTENCE & SCALING** (Week 7-8)

#### **6.1 Database Schema**
```sql
-- PocketBase collections mới
users (id, username, email, avatar)
matches (id, players, winner, duration, timestamp)
leaderboards (user_id, score, rank)
```

#### **6.2 Background Jobs**
- [ ] Match result processing
- [ ] Leaderboard updates
- [ ] Cleanup old data

---

## 🔧 CÁC BƯỚC TIẾP THEO CỤ THỂ

### **WEEK 1: BẮT ĐẦU TRANSITION**

**Ngày 1-2: Setup Infrastructure**
```bash
# 1. Cài đặt dependencies mới
cargo add wtransport tokio-tungstenite redis

# 2. Tạo branch mới cho multiplayer
git checkout -b feature/multiplayer-transition

# 3. Update Cargo.toml với dependencies mới
```

**Ngày 3-4: QUIC Implementation**
```rust
// gateway/src/transport.rs
pub async fn start_quic_server(port: u16) -> Result<()> {
    let config = ServerConfig::builder_with_single_cert(...)?;
    let server = wtransport::Server::with_config(config)?;

    // Handle connections...
}
```

**Ngày 5-7: Transport Negotiation**
```javascript
// client/src/transport/negotiator.ts
export async function negotiateTransport() {
    const transports = ['quic', 'webrtc', 'websocket'];

    for (const transport of transports) {
        if (await testTransport(transport)) {
            return transport;
        }
    }

    throw new Error('No transport available');
}
```

### **WEEK 2: AUTHENTICATION OVERHAUL**

**Cập nhật auth middleware:**
```rust
// gateway/src/auth.rs
pub struct AuthMiddleware<S> {
    redis: RedisClient,
}

impl<S> AuthMiddleware<S> {
    pub async fn validate_jwt(&self, token: &str) -> Result<User> {
        // JWT validation + Redis session check
    }
}
```

---

## 📊 METRICS & MILESTONES

### **Milestone 1: Basic Multiplayer** (End Week 2)
- ✅ QUIC/WebTransport hoạt động
- ✅ WebSocket fallback mượt
- ✅ Transport negotiation tự động
- ✅ Enhanced authentication

### **Milestone 2: ECS Ready** (End Week 4)
- ✅ ECS architecture implemented
- ✅ Fixed timestep physics
- ✅ Basic state synchronization

### **Milestone 3: Multiplayer Alpha** (End Week 6)
- ✅ 2-4 players có thể chơi cùng
- ✅ Room system cơ bản
- ✅ Real-time synchronization

---

## 🎯 LỢI ÍCH KHI CHUYỂN ĐỔI

1. **Technical Growth**: Học ECS, advanced networking, real-time systems
2. **Market Value**: Multiplayer games có engagement cao hơn
3. **Scalability**: Architecture sẵn sàng mở rộng
4. **Future-Proof**: Foundation cho các tính năng nâng cao

---

## ⚠️ CHALLENGES TO EXPECT

1. **Network Latency**: Phải implement prediction/reconciliation
2. **State Synchronization**: Complex state management
3. **Cheating Prevention**: Server-side validation cần thiết
4. **Performance**: 60fps với multiple players challenging
5. **Debugging**: Network issues khó debug hơn

Bạn có muốn bắt đầu với Phase 1 (Network Infrastructure) không? Tôi sẽ giúp implement QUIC/WebTransport ngay bây giờ! 🚀
