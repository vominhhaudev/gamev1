use axum::extract::ws::{Message as WsMessage, WebSocket};
use common_net::message::{self, Channel, ControlMessage, Frame, FramePayload, StateMessage};
use futures::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use prometheus::{
    register_int_counter, register_int_gauge, IntCounter, IntGauge,
};
use std::{collections::VecDeque, time::Duration};
use tokio::time;
use tracing::error;
use wtransport::{ClientConfig, Endpoint, ServerConfig};
use rcgen;

/// Trait chung cho các transport (WS, QUIC, RTC).
#[async_trait::async_trait]
pub trait Transport {
    async fn serve(self, socket: WebSocket);

    /// QUIC-specific method (default implementation does nothing).
    async fn serve_quic(&mut self, _connection: wtransport::Connection) {
        panic!("QUIC transport must implement serve_quic");
    }
}

/// Transport state chung cho tất cả implementations.
#[derive(Clone, Default)]
pub struct TransportState {
    next_control_seq: u32,
    next_state_seq: u32,
}

impl TransportState {
    pub fn alloc_sequence(&mut self, channel: Channel) -> u32 {
        match channel {
            Channel::Control => {
                let s = self.next_control_seq;
                self.next_control_seq = self.next_control_seq.wrapping_add(1);
                s
            }
            Channel::State => {
                let s = self.next_state_seq;
                self.next_state_seq = self.next_state_seq.wrapping_add(1);
                s
            }
        }
    }
}


/// QUIC/WebTransport implementation của Transport trait.
pub struct QuicTransport {
    state: TransportState,
    endpoint: Option<Endpoint>,
}

impl QuicTransport {
    pub fn new() -> Self {
        Self {
            state: TransportState::default(),
            endpoint: None,
        }
    }

    /// Start QUIC server listener.
    pub async fn start_server(&mut self, bind_addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Tạo self-signed cert cho development
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
        let cert_der = cert.serialize_der()?;
        let key_der = cert.serialize_private_key_der();

        let config = ServerConfig::builder()
            .with_bind_default(443)  // QUIC dùng port 443
            .with_certificate(&cert_der, &key_der)?
            .keep_alive_interval(Some(Duration::from_secs(3)))?
            .build();

        let endpoint = Endpoint::server(config)?;
        self.endpoint = Some(endpoint);
        Ok(())
    }

    /// Accept QUIC connections và handle từng connection.
    pub async fn accept_connections(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(endpoint) = &self.endpoint {
            while let Some(connection) = endpoint.accept().await {
                let connection = connection.await?;
                let mut transport = QuicTransport::new();
                tokio::spawn(async move {
                    transport.handle_quic_connection(connection).await;
                });
            }
        }
        Ok(())
    }

    /// Handle QUIC connection: stream cho control, datagram cho state.
    async fn handle_quic_connection(&mut self, connection: wtransport::Connection) {
        let mut state = self.state.clone();
        let mut ticker = time::interval(Duration::from_millis(250));
        let mut state_seq = 0u32;

        // 2-level queue với high/low watermark
        const STATE_BUFFER_CAPACITY: usize = 128;
        const STATE_HIGH_WATERMARK: usize = 96;  // 75% capacity
        const STATE_LOW_WATERMARK: usize = 48;   // 37.5% capacity
        let mut state_buffer: VecDeque<Vec<u8>> = VecDeque::with_capacity(STATE_BUFFER_CAPACITY);

        // Spawn state ticker cho datagrams
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let seq = state_seq;
                        state_seq = state_seq.wrapping_add(1);
                        let msg = StateMessage::Event {
                            name: "tick".into(),
                            data: serde_json::json!({"seq": seq}),
                        };
                        if let Ok(bytes) = message::encode(&Frame::state(seq, timestamp_ms(), msg)) {
                            if state_buffer.len() >= STATE_BUFFER_CAPACITY {
                                let _ = state_buffer.pop_front();
                                STATE_BUFFER_DROPPED_TOTAL.inc();
                            }
                            state_buffer.push_back(bytes);
                            STATE_BUFFER_DEPTH.set(state_buffer.len() as i64);
                        }

                        // Active draining khi above high watermark
                        while state_buffer.len() > STATE_HIGH_WATERMARK {
                            if let Some(bytes) = state_buffer.pop_front() {
                                // Send as datagram for unreliable state
                                if connection.send_datagram(&bytes).is_err() {
                                    break;
                                }
                                STATE_BUFFER_DEPTH.set(state_buffer.len() as i64);
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Handle control stream (bidirectional)
        if let Ok(mut stream) = connection.open_bi().await {
            while let Some(msg) = stream.recv().await {
                if let Ok(bytes) = msg {
                    match message::decode(&bytes) {
                        Ok(Frame { payload, .. }) => {
                            match payload {
                                FramePayload::Control {
                                    message: ControlMessage::Ping { nonce },
                                } => {
                                    let seq = state.alloc_sequence(Channel::Control);
                                    let frame = Frame::control(seq, timestamp_ms(), ControlMessage::Pong { nonce });
                                    if let Ok(reply) = message::encode(&frame) {
                                        let _ = stream.send(reply).await;
                                    }
                                }
                                FramePayload::Control {
                                    message: ControlMessage::JoinRoom { room_id, reconnect_token: _ },
                                } => {
                                    let seq = state.alloc_sequence(Channel::State);
                                    let state_msg = StateMessage::Event {
                                        name: "joined".into(),
                                        data: serde_json::json!({"room_id": room_id}),
                                    };
                                    if let Ok(reply) = message::encode(&Frame::state(seq, timestamp_ms(), state_msg)) {
                                        // Send via datagram for state
                                        let _ = connection.send_datagram(&reply);
                                    }
                                }
                                _ => {
                                    let _ = stream.send(bytes).await;
                                }
                            }
                        }
                        Err(_) => {
                            let _ = stream.send(bytes).await;
                        }
                    }
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Transport for QuicTransport {
    async fn serve(self, _socket: WebSocket) {
        // QUIC không dùng WebSocket, dùng connection riêng
        todo!("QUIC transport uses separate listener")
    }

    async fn serve_quic(&mut self, connection: wtransport::Connection) {
        self.handle_quic_connection(connection).await;
    }
}


static STATE_BUFFER_DROPPED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "gateway_state_dropped_total",
        "So goi state bi drop do backpressure"
    )
    .expect("register gateway_state_dropped_total")
});

static STATE_BUFFER_DEPTH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "gateway_state_buffer_depth",
        "Do sau buffer state hien tai"
    )
    .expect("register gateway_state_buffer_depth")
});


pub fn timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}