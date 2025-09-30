use serde::{Deserialize, Serialize};

/// Logical channel for the transport pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Control,
    State,
}

/// Envelope + payload for every frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub channel: Channel,
    pub sequence: u32,
    pub timestamp_ms: u64,
    #[serde(flatten)]
    pub payload: FramePayload,
}

impl Frame {
    pub fn control(sequence: u32, timestamp_ms: u64, message: ControlMessage) -> Self {
        Self {
            channel: Channel::Control,
            sequence,
            timestamp_ms,
            payload: FramePayload::Control { message },
        }
    }

    pub fn state(sequence: u32, timestamp_ms: u64, message: StateMessage) -> Self {
        Self {
            channel: Channel::State,
            sequence,
            timestamp_ms,
            payload: FramePayload::State { message },
        }
    }
}

/// Payload distinguishing control/state channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FramePayload {
    Control { message: ControlMessage },
    State { message: StateMessage },
}

/// Control plane messages (join room, ping, auth...).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    JoinRoom {
        room_id: String,
        reconnect_token: Option<String>,
    },
    LeaveRoom,
    Input {
        seq: u32,
        payload: serde_json::Value,
    },
    AuthRequest {
        wallet: String,
    },
    AuthToken {
        jwt: String,
    },
    // WebRTC signaling messages
    WebRtcOffer {
        room_id: String,
        peer_id: String,
        target_peer_id: Option<String>, // None = broadcast to all
        sdp: String,
    },
    WebRtcAnswer {
        room_id: String,
        peer_id: String,
        target_peer_id: String,
        sdp: String,
    },
    WebRtcIceCandidate {
        room_id: String,
        peer_id: String,
        target_peer_id: Option<String>, // None = broadcast to all
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u32,
    },
}

/// State plane messages (snapshot, delta, event...).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateMessage {
    Snapshot {
        tick: u64,
        entities: Vec<EntitySnapshot>,
    },
    Delta {
        tick: u64,
        changes: Vec<EntityDelta>,
    },
    Event {
        name: String,
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntitySnapshot {
    pub id: String,
    pub components: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityDelta {
    pub id: String,
    pub changes: serde_json::Value,
}

/// Encode a frame into JSON bytes (temporary framing).
pub fn encode(frame: &Frame) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(frame)
}

/// Decode a frame from JSON bytes.
pub fn decode(bytes: &[u8]) -> Result<Frame, serde_json::Error> {
    serde_json::from_slice(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let frame = Frame::control(
            42,
            123_456,
            ControlMessage::JoinRoom {
                room_id: "alpha".into(),
                reconnect_token: Some("token".into()),
            },
        );

        let bytes = encode(&frame).expect("encode");
        let decoded = decode(&bytes).expect("decode");

        assert_eq!(decoded.channel, Channel::Control);
        assert_eq!(decoded.sequence, 42);
    }
}
