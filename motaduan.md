# 🎯 DỰ ÁN GAME SERVER MULTIPLAYER - NATIVE DEPLOYMENT VỚI POCKETBASE

## 📋 TỔNG QUAN
Đây là dự án game server multiplayer sử dụng Rust với architecture microservices, native deployment (không Docker), và PocketBase làm database + auth layer.

---

## WEEK 0 — CHUẨN BỊ & KHỞI ĐỘNG REPO

**Học:** Rust toolchain, cargo workspace, Git, VS Code, native deployment basics.

**Làm:**
- Cài: rustup, wasm32-unknown-unknown, cargo install trunk.
- Tạo mono-repo:
  ```
  /server
    /gateway          # HTTP API & WebSocket
    /room-manager      # Quản lý phòng chơi
    /worker           # Game simulation (ECS + Physics)
    /proto            # gRPC protocol definitions
    /services         # API chậm & background jobs
  /client (SvelteKit + Threlte)
  /scripts (native deployment scripts)
  /pocketbase       # PocketBase binary & data
  ```

- Bật logging/metrics: tracing, tracing-subscriber, prometheus.
- **Xong khi:** Build & chạy được 3 service, có log & `/metrics`.

---

## WEEK 1 — RUST NỀN TẢNG + ECHO NETWORKING

**Học:** Ownership/borrowing, Result/Option, async Rust (Tokio), axum.

**Làm (Server):**
- WS Echo (axum + tokio-tungstenite) tại gateway: `/ws`.
- HTTP: `/healthz`, `/metrics`, `/version`.
- Tạo lib chung: common-net (message framing, serde).

**Làm (Client):**
- SvelteKit route `/net-test` kết nối WS, gửi/nhận echo.

**Xong khi:** WS echo round-trip < 5ms LAN, metrics hiển thị QPS/latency.

---

## WEEK 2 — QUIC/WEBTRANSPORT & FALLBACK WS

**Học:** QUIC/H3 basics, datagram vs stream, backpressure.

**Làm (Server):**
- Thêm WebTransport/QUIC (wtransport):
  - Streams (control, reliable)
  - Datagrams (state, unreliable)
- GET `/negotiate` trả về danh sách transports hỗ trợ.
- Hàng đợi 2 cấp (high/low watermark) + drop oldest cho kênh state.

**Làm (Client):**
- Transport selector: thử QUIC→RTC→WS theo thứ tự; vẽ biểu đồ ping.

**Xong khi:** QUIC chạy ổn định; tắt QUIC thì fallback WS vận hành.

---

## WEEK 3 — AUTHENTICATION + USER MANAGEMENT

**Học:** JWT (HS/RS), session management, rate limiting.

**Làm (Server):**
- `/auth/login` → xác thực user → cấp JWT + session.
- `/auth/refresh` → refresh token.
- Middlewares: rate-limit theo IP & JWT, CORS.

**Làm (Client):**
- Login form, JWT storage, attach token vào requests.

**Xong khi:** User authentication hoạt động, rate-limit chặn flood.

---

## WEEK 4 — WEBRTC DATACHANNEL + SIGNALING

**Học:** SDP/ICE/STUN/TURN, ordered vs unordered, partial-reliability.

**Làm (Server/Gateway):**
- Endpoints signaling: `/rtc/offer`, `/rtc/answer`, `/rtc/ice`.
- DataChannels:
  - `dc-control`: ordered+reliable
  - `dc-state`: unordered + maxRetransmits=0..2
- Abstraction Transport thống nhất API: send_control, send_state, recv_*.

**Làm (Client):**
- Tạo peer, exchange ICE, fallback WS nếu RTC fail; đo loss/jitter.

**Xong khi:** Client chạy được bằng WebRTC và fallback WS/QUIC mượt.

---

## WEEK 5 — SIMULATION WORKER (ECS + PHYSICS) — MVP

**Học:** bevy_ecs (schedules, systems), rapier3d (bodies/colliders), fixed timestep.

**Làm (Worker):**
- Tick cố định 60Hz (accumulator).
- Components: TransformQ, VelocityQ, Player, Pickup, Lifetime.
- Systems: ingest_inputs → validate_inputs → rapier_step → gameplay_logic (pickup/score).
- Giao tiếp nội bộ (gRPC tonic): JoinPlayer, LeavePlayer, PushInput.

**Làm (Gateway):**
- Route input từ client → worker; worker → snapshot → client.

**Xong khi:** 1 phòng chơi mock với bot hoặc 1–2 client thật; điểm tăng khi nhặt.

---

## WEEK 6 — ROOM MANAGER & MATCHMAKING

**Học:** PocketBase collections, real-time subscriptions, TTL/heartbeat.

**Làm:**
- Room Manager: CreateRoom, Assign, CloseRoom, Heartbeat.
- PocketBase: collections cho rooms, players, matches.
- Gateway gọi Assign để route; lưu sticky_token cho reconnect.

**Xong khi:** Tạo được 50–100 phòng nhỏ, vào/ra ổn, worker crash không kéo sập cụm.

---

## WEEK 7 — AOI + SNAPSHOT/DELTA + QUANTIZATION

**Học:** AOI grid/quadtree, keyframe/delta, SoA & bit-packing.

**Làm (Worker):**
- AOI: lưới đều (cell≈tầm nhìn).
- Keyframe mỗi 10 tick; delta chỉ field đổi; nén i16/i8 cho pos/rot.
- Backpressure: buffer per-client, drop delta cũ.

**Làm (Client):**
- Interpolation + Reconciliation: xài last_input_seq_processed.
- HUD hiển thị ping, loss, transport đang dùng.

**Xong khi:** Băng thông giảm mạnh; p95 snapshot_encode < 2ms; gameplay mượt với packet-loss 1–3%.

---

## WEEK 8 — PERSISTENCE + API CHẬM

**Học:** PocketBase schema design, real-time subscriptions, transaction patterns.

**Làm (Services):**
- Collections: users, matches, participants, leaderboard, inventory.
- Ghi kết quả trận khi room đóng; API leaderboard + real-time updates.
- Background jobs cho cleanup, maintenance.

**Xong khi:** Xem được bảng xếp hạng; restart server không mất dữ liệu trận.

---

## WEEK 9 — BACKGROUND JOBS & CACHING

**Học:** Job queues, caching strategies, background processing.

**Làm (Services):**
- Job worker: cleanup old data, generate reports, maintenance tasks.
- Caching layer: in-memory + PocketBase for hot data.
- Background sync: real-time updates không chặn main tick.

**Xong khi:** 100+ background jobs/phút không ảnh hưởng p95 tick/latency.

---

## WEEK 10 — NATIVE DEPLOYMENT & OBSERVABILITY

**Học:** Systemd/Supervisor, native monitoring, load balancing.

**Làm:**
- Process Manager: systemd services cho tất cả components.
- Load Balancing: nginx với health checks.
- Metrics: p50/95/99 latency, tick drift, encode time, drop %, CCU, rooms.
- Native monitoring: Prometheus + Grafana + structured logging.

**Xong khi:** Dashboard live SLO; auto-restart services; health monitoring.

---

## WEEK 11 — LOAD TEST & HARDENING

**Học:** Load testing tools, performance profiling, chaos engineering.

**Làm:**
- Load testing: headless clients (100–1000 CCU).
- Network simulation: loss 1–5%, RTT 40–120ms.
- Chaos testing: kill processes, network partitions.
- Performance optimization: tick rate scaling, memory pooling.

**Xong khi:** SLO đề ra đạt (p95 end-to-end < 120ms, drop < 3%, reconnect > 99%).

---

## WEEK 12 — ALPHA RELEASE & DOCUMENTATION

**Làm:**
- Đóng băng scope Alpha (bug critical 0, documentation đầy đủ).
- Viết runbook: deployment, scaling, monitoring, incident response.
- Performance benchmarks và capacity planning.
- Tag v0.1.0-alpha, demo video, internal testing.

**Xong khi:** Checklist Alpha pass, có demo video và deployment guide.

---

## HỌC GÌ SONG HÀNH MỖI TUẦN

**Rust:** Tuần 1–2 (Rust Book: ownership/borrowing, error, traits; async).

**Networking:** Tuần 2–4 (QUIC/H3/WebTransport, WebRTC, STUN/TURN, WS).

**ECS/Physics:** Tuần 5–7 (bevy_ecs/rapier, fixed timestep, AOI, snapshot).

**Data:** Tuần 8–9 (PocketBase, job queues, caching, real-time subscriptions).

**Ops:** Tuần 10–11 (native deployment, load testing, chaos engineering).

---

## TIÊU CHÍ ALPHA (SERVER)

✅ Kết nối WebTransport/QUIC, WebRTC, WS fallback hoạt động.

✅ Authentication + session management, rate-limit, reconnect token.

✅ Matchmaking/Rooms tự động; sticky routing; heartbeat/TTL.

✅ Worker 60Hz với AOI + snapshot/delta + quantization; backpressure tốt.

✅ Anti-cheat tối thiểu (clamp tốc độ, cooldown, server-side validation).

✅ Persistence kết quả trận; Leaderboard API < 150ms.

✅ Background jobs không chặn tick.

✅ Native deployment với monitoring; SLO dashboard live.

---

## ARCHITECTURE OVERVIEW