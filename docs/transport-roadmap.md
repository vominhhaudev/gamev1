# Week 2 Transport Roadmap

## Muc tieu
- Bo sung QUIC/WebTransport ket hop fallback WebRTC -> WebSocket.
- Duy tri chung API GameTransport (xem common-net/src/transport/mod.rs).
- Do do tre, backpressure va dam bao control/state channel doc lap.

## Danh gia thu vien
- **wtransport**: ho tro WebTransport/HTTP3, tuong thich trinh duyet; can kiem tra license va trang thai Windows.
- **quinn**: thu an QUIC; huu ich cho client native/headless.
- **webrtc-rs**: dung cho fallback DataChannel.

## Ke hoach trien khai
1. **Adapter WebSocket**: trien khai WsTransport dau tien dung 	okio-tungstenite de hien thuc trait, de test.
2. **Adapter WebTransport**: dung wtransport voi control/state mapping, fallback sang WebSocket khi handshake fail.
3. **Adapter WebRTC**: tai su dung signaling gateway (Week 4) de tao RtcTransport.
4. **Negotiation API**: them endpoint GET /negotiate tra ve danh sach transport ho tro + token auth.
5. **Kiem thu**:
   - Integration test handshake/toggle fallback voi mock server.
   - Benchmark RTT qua tung layer, log ra Prometheus (	ransport_kind).

## Hanh dong song song
- Chuan bi schema message cho negotiate/upgrade trong common-net::message (vd ControlMessage::TransportOffer).
- Them metric gauge active connections theo TransportKind.
- Nang cap client /net-test de lua chon transport (dau tien WebSocket, sau do them QUIC/RTC).
