G0 — Cố định giao thức & nhịp thời gian

Mục tiêu: Đóng khung cách client ↔ server nói chuyện để tránh “đập” network khi thêm gameplay.

Việc cần làm

Tick & snapshot

Tick server cố định 60 Hz (accumulator).

Gửi snapshot/delta 20–30 Hz (mỗi 2–3 tick).

Versioning/handshake

Handshake có proto_version, server_time_ms.

Chính sách: client version ≠ server → từ chối lịch sự.

Hồ sơ thông điệp

Định nghĩa rõ 3 nhóm: Input (client→server), Snapshot/Delta (server→client), Control (reliable: join/leave/errors).

Quantization: pos i16 (đơn vị cm, phạm vi ±512 m); rot i8 (~1.4°/step).

Framing chung

Dùng chung common-net (Rust & TS): varint length-prefix, checksum nhẹ (tùy chọn), bit-packing helpers.

Deliverables

Tài liệu PROTO.md (+ schema trong /proto).

Test encode/decode round-trip (Rust ↔ TS) + benchmark encode < 2 ms (p95).

DoD

Hot-swap client cũ/mới không crash.

Logger in ra proto_version, snapshot_hz, tick_ms ở startup.

Rủi ro & né

Lỡ thay schema phá vỡ client → versioning + feature flags trong handshake.

G1 — Gameplay Core trong Worker (ECS + Physics)

Mục tiêu: Tách gameplay khỏi transport; đảm bảo loop ổn định.

Việc cần làm

ECS setup

Components: TransformQ, VelocityQ, Player, Pickup, Lifetime.

Systems (theo thứ tự):
ingest_inputs → validate_inputs → rapier_step → gameplay_logic → snapshot_encode.

Anti-cheat tối thiểu

Clamp tốc độ, cooldown actions, reject input quá nhanh (>60 Hz), replay-attack prevention (seq & JWT).

Snapshot/Delta

Keyframe mỗi 10 tick; giữa kỳ gửi delta fields đổi.

Buffer per-client; nếu chậm → drop delta cũ, giữ mới (backpressure).

gRPC nội bộ

JoinPlayer/LeavePlayer/PushInput giữa Gateway ↔ Worker.

Deliverables

/worker chạy được mock room (bot/1–2 client thật), điểm tăng khi nhặt.

Bản đồ log: tick time, encode_ms, queue_depth.

DoD

p95 tick_total < 16 ms; p95 snapshot_encode < 2 ms.

Không panics khi spam input/leave/join.

Rủi ro & né

Physics “nổ” do dt biến thiên → fixed timestep + accumulator.

G2 — Client Web: Lớp mạng & Dự đoán

Mục tiêu: Client mỏng, dự đoán mượt, hòa hợp mọi transport.

Việc cần làm

Abstraction Transport

ITransport { connect, sendControl, sendState, onControl, onState, close }

Implement WebTransport/QUIC, WebRTC DC, WS (fallback).

Prediction + Reconciliation

Input kèm seq & dt_ms; render ngay.

Khi nhận snapshot với last_input_seq_processed → rewind & reapply inputs chưa áp dụng.

Interpolation

Buffer hiển thị ~100–150 ms (tùy RTT), nội suy pos/rot; nhẹ extrapolation khi mất gói.

HUD

Ping, loss, jitter, transport đang dùng, FPS, tick drift.

Deliverables

/client/net (transport + codec), /client/game (render, HUD).
Demo route /play hoạt động trên QUIC/RTC/WS.

DoD

Round-trip input→snapshot hiển thị mượt ở RTT 40–80 ms, loss 1–3%.

Fallback tự động QUIC→RTC→WS khi cắt mạng có chủ đích.

Rủi ro & né

Đọc cả repo tốn băng thông → AOI (giai đoạn G3) + @file scoping trong dev tools.

G3 — AOI & Nén băng thông (Snapshot/Delta/Quant)

Mục tiêu: Giảm mạnh băng thông, ổn định p95.

Việc cần làm

AOI dạng lưới

Cell = tầm nhìn; map entity→cell; per-client gửi chỉ các cell lân cận.

Keyframe/Delta

Keyframe mỗi 10 tick; delta chỉ field đổi (SoA friendly).

Quantization

Pos i16 (cm), rot i8; bit-packing (SoA) để ghép bytes liền nhau.

Backpressure thông minh

Buffer per-client; drop oldest delta nếu nghẽn; không drop keyframe sắp tới.

Deliverables

Biểu đồ băng thông: idle, di chuyển, combat.

Flag bật/tắt AOI trong config để A/B.

DoD

Băng thông mục tiêu: < 20–40 KB/s/client (cảnh tĩnh thấp hơn).

p95 snapshot_encode vẫn < 2 ms, CPU encode < 5% worker.

Rủi ro & né

“Rubber-band” do buffer quá ngắn → tinh chỉnh buffer_ms theo RTT p95.

G4 — Luật chơi & Vòng đời trận

Mục tiêu: Từ sandbox thành “game”: có luật thắng thua, vòng đời match đầy đủ.

Việc cần làm

Rules

Mode MVP: pickup/score (hoặc goal zone).

Vòng đời: pre-match → in-match (3–5′) → post-match.

Persistence & API chậm (đã nền tảng ở tuần 8)

Khi room đóng: ghi matches, participants, cập nhật leaderboard (có cache Redis).

Reconnect & Sticky routing

Dùng reconnect_token + sticky_token → trở lại đúng room/worker.

Chống gian lận + Lag Compensation (cơ bản)

Server-authoritative hit/pick.

Lưu ring-buffer positions 200–300 ms để kiểm tra va chạm theo tick lịch sử (nếu có bắn súng/đụng chạm nhanh).

Deliverables

/services/leaderboard trả lời < 150 ms.

Flow end-match → ghi DB → cập nhật UI leaderboard.

DoD

Match loop end-to-end không lỗi; reconnect > 99% thành công.

Leaderboard nhất quán, không trùng bản ghi (idempotency).

Rủi ro & né

API chậm kéo tick → job queue riêng cho việc nặng, không chặn tick.

G5 — Quan sát, kiểm thử tải & hardening

Mục tiêu: SLO rõ ràng, biết giới hạn hệ thống.

Việc cần làm

Metrics

p50/95/99 end-to-end latency, tick drift, encode_ms, queue_depth, drop%, CCU/rooms.

Headless clients

Sinh tải qua WS/RTC/QUIC (100–1000 CCU tùy máy).

Net-em & Chaos

Emulate loss 1–5%, RTT 40–120 ms; kill worker; drop network; đảm bảo fallback hoạt động.

Tối ưu khi quá tải

Giảm tick về 30 Hz, thắt chặt AOI, giảm tần suất keyframe, ưu tiên delta quan trọng (players>props).

Deliverables

Dashboard Grafana: SLO realtime.

Báo cáo tải: giới hạn trước khi vi phạm SLO.

DoD
Ví dụ SLO: p95 end-to-end < 120 ms, drop < 3%, reconnect > 99%.

Tài liệu runbook: scale, incident, hot-swap worker.

Rủi ro & né

TURN/QUIC trên một số mạng “kén” → sẵn sàng TURN IP tĩnh; ingress H3 cho WebTransport.

Cách quản lý công việc (thực dụng)

Nhóm PR theo giai đoạn (G0→G5). Mỗi PR có checklist DoD ở trên.

Feature flags: aoi_enabled, delta_enabled, quant_enabled, lagcomp_enabled.

Bench & test tự động:

Encode/decode round-trip;

So sánh snapshot bandwidth với/không AOI/quant;

Test reconnect & fallback (ngắt QUIC/RTC có chủ đích).

Ràng buộc kiến trúc cần giữ xuyên suốt

Server-authoritative: client chỉ gửi ý định (inputs).

Control vs State: control đi reliable/ordered; state đi unreliable/partial-reliability.

Transport-agnostic: mọi logic gameplay “không biết” QUIC/RTC/WS; chỉ biết ITransport.

Schema-first: mọi thay đổi gameplay → đổi /proto + test tương thích.