use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::message::{ControlMessage, Frame, StateMessage};
use super::{GameTransport, TransportError, TransportErrorKind, TransportKind};

/// WebRTC DataChannel configuration
#[derive(Debug, Clone)]
pub struct DataChannelConfig {
    /// Channel label
    pub label: String,
    /// Ordered delivery (true for control, false for state)
    pub ordered: bool,
    /// Maximum retransmits (0 for unreliable)
    pub max_retransmits: Option<u16>,
}

impl DataChannelConfig {
    /// Control channel: ordered + reliable
    pub fn control() -> Self {
        Self {
            label: "control".to_string(),
            ordered: true,
            max_retransmits: None, // Reliable
        }
    }

    /// State channel: unordered + partial reliability
    pub fn state() -> Self {
        Self {
            label: "state".to_string(),
            ordered: false,
            max_retransmits: Some(0), // Max 0 retransmits for partial reliability
        }
    }
}

/// WebRTC transport implementation
/// Note: This is a placeholder implementation that uses WebSocket as fallback
/// For production, integrate with actual WebRTC library like webrtc-rs
#[derive(Debug)]
pub struct WebRtcTransport {
    room_id: String,
    peer_id: String,
    is_fallback: bool, // Cache fallback status for sync access

    // DataChannels (simulated with channels)
    control_tx: Option<mpsc::UnboundedSender<Frame>>,
    control_rx: Option<mpsc::UnboundedReceiver<Frame>>,
    state_tx: Option<mpsc::UnboundedSender<Frame>>,
    state_rx: Option<mpsc::UnboundedReceiver<Frame>>,

    // Signaling (placeholder for actual WebRTC signaling)
    signaling_tx: mpsc::UnboundedSender<ControlMessage>,
    _signaling_rx: mpsc::UnboundedReceiver<ControlMessage>,

    // Connection state
    connected: Arc<RwLock<bool>>,
    fallback_to_ws: Arc<RwLock<bool>>,
}

impl WebRtcTransport {
    /// Create new WebRTC transport
    pub fn new(room_id: String, peer_id: String) -> Self {
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (state_tx, state_rx) = mpsc::unbounded_channel();
        let (signaling_tx, signaling_rx) = mpsc::unbounded_channel();

        Self {
            room_id,
            peer_id,
            is_fallback: false,
            control_tx: Some(control_tx),
            control_rx: Some(control_rx),
            state_tx: Some(state_tx),
            state_rx: Some(state_rx),
            signaling_tx,
            _signaling_rx: signaling_rx,
            connected: Arc::new(RwLock::new(false)),
            fallback_to_ws: Arc::new(RwLock::new(false)),
        }
    }

    /// Get signaling sender for external signaling
    pub fn signaling_tx(&self) -> mpsc::UnboundedSender<ControlMessage> {
        self.signaling_tx.clone()
    }

    /// Check if using fallback transport
    pub fn is_fallback(&self) -> bool {
        self.is_fallback
    }

    /// Set connected state
    pub async fn set_connected(&self, connected: bool) {
        *self.connected.write().await = connected;
    }

    /// Handle WebRTC signaling message (placeholder for actual implementation)
    pub async fn handle_signaling(&self, message: ControlMessage) -> Result<(), TransportError> {
        match message {
            ControlMessage::WebRtcOffer { .. } |
            ControlMessage::WebRtcAnswer { .. } |
            ControlMessage::WebRtcIceCandidate { .. } => {
                info!("WebRTC signaling: {:?}", message);
                // In real implementation, this would handle SDP negotiation
                // For now, just mark as connected when we receive offer/answer
                self.set_connected(true).await;
                Ok(())
            }
            _ => Err(TransportError::new(
                TransportErrorKind::Unsupported,
                format!("Unsupported signaling message: {:?}", message)
            ))
        }
    }

    /// Fallback to WebSocket transport
    pub async fn fallback_to_websocket(&mut self) -> Result<(), TransportError> {
        warn!("WebRTC connection failed, falling back to WebSocket");
        *self.fallback_to_ws.write().await = true;
        self.is_fallback = true; // Update cached status
        // In real implementation, this would establish WebSocket connection
        Ok(())
    }
}

#[async_trait]
impl GameTransport for WebRtcTransport {
    fn kind(&self) -> TransportKind {
        if self.is_fallback {
            TransportKind::WebSocket
        } else {
            TransportKind::WebRtc
        }
    }

    async fn send_frame(&mut self, frame: Frame) -> Result<(), TransportError> {
        if !*self.connected.read().await {
            return Err(TransportError::new(
                TransportErrorKind::ConnectionClosed,
                "WebRTC not connected"
            ));
        }

        // Route to appropriate DataChannel based on channel type
        match frame.channel {
            crate::message::Channel::Control => {
                if let Some(ref mut tx) = self.control_tx {
                    tx.send(frame).map_err(|_| TransportError::new(
                        TransportErrorKind::Backpressure,
                        "Control channel full"
                    ))?;
                } else {
                    return Err(TransportError::new(
                        TransportErrorKind::ConnectionClosed,
                        "Control channel not available"
                    ));
                }
            }
            crate::message::Channel::State => {
                if let Some(ref mut tx) = self.state_tx {
                    tx.send(frame).map_err(|_| TransportError::new(
                        TransportErrorKind::Backpressure,
                        "State channel full"
                    ))?;
                } else {
                    return Err(TransportError::new(
                        TransportErrorKind::ConnectionClosed,
                        "State channel not available"
                    ));
                }
            }
        }

        Ok(())
    }

    async fn recv_frame(&mut self) -> Result<Frame, TransportError> {
        if !*self.connected.read().await {
            return Err(TransportError::new(
                TransportErrorKind::ConnectionClosed,
                "WebRTC not connected"
            ));
        }

        // Try to receive from either control or state channel
        if let Some(ref mut control_rx) = self.control_rx {
            if let Ok(frame) = control_rx.try_recv() {
                return Ok(frame);
            }
        }

        if let Some(ref mut state_rx) = self.state_rx {
            if let Ok(frame) = state_rx.try_recv() {
                return Ok(frame);
            }
        }

        // No frames available
        Err(TransportError::new(
            TransportErrorKind::ConnectionClosed,
            "No frames available"
        ))
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.set_connected(false).await;

        // Close channels
        self.control_tx.take();
        self.control_rx.take();
        self.state_tx.take();
        self.state_rx.take();

        info!("WebRTC transport closed");
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        // For WebRTC, flush is typically handled by the DataChannel implementation
        // In this mock implementation, we just return Ok
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ControlMessage, Frame};

    #[tokio::test]
    async fn webrtc_transport_creation() {
        let transport = WebRtcTransport::new("room123".to_string(), "peer1".to_string());

        assert_eq!(transport.kind(), TransportKind::WebRtc);
        assert_eq!(transport.room_id, "room123");
        assert_eq!(transport.peer_id, "peer1");
        assert!(!transport.is_fallback());
    }

    #[tokio::test]
    async fn webrtc_send_control_frame() {
        let mut transport = WebRtcTransport::new("room123".to_string(), "peer1".to_string());
        transport.set_connected(true).await;

        let frame = Frame::control(1, 12345, ControlMessage::Ping { nonce: 42 });

        let result = transport.send_frame(frame.clone()).await;
        assert!(result.is_ok());

        // In real implementation, frame would be available in the channel
        // For this test, we just verify it doesn't error
    }

    #[tokio::test]
    async fn webrtc_fallback() {
        let mut transport = WebRtcTransport::new("room123".to_string(), "peer1".to_string());

        assert_eq!(transport.kind(), TransportKind::WebRtc);
        assert!(!transport.is_fallback());

        transport.fallback_to_websocket().await.unwrap();

        assert_eq!(transport.kind(), TransportKind::WebSocket);
        assert!(transport.is_fallback());
    }
}
