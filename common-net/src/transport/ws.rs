use std::fmt::Display;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::{Error as WsError, Message},
    MaybeTlsStream, WebSocketStream,
};

use super::{GameTransport, TransportError, TransportErrorKind, TransportKind};
use crate::{message::{self, Frame}, compression::{Compression, CompressionConfig, CompressedData, CompressionAlgorithm}};
use std::clone::Clone;

fn map_ws_error(err: impl Display) -> TransportError {
    TransportError::new(TransportErrorKind::Io, err.to_string())
}

fn map_encode_error(err: serde_json::Error) -> TransportError {
    TransportError::new(TransportErrorKind::EncodingFailure, err.to_string())
}

fn map_decode_error(err: serde_json::Error) -> TransportError {
    TransportError::new(TransportErrorKind::DecodingFailure, err.to_string())
}

pub type WsClientStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WsTransport<S> {
    stream: WebSocketStream<S>,
    compression_config: CompressionConfig,
    negotiated_algorithms: Vec<CompressionAlgorithm>,
    adaptive_compression: bool,
    message_count: usize,
    compression_stats: CompressionStats,
}

#[derive(Debug, Default)]
pub struct CompressionStats {
    pub total_messages: usize,
    pub compressed_messages: usize,
    pub total_original_bytes: usize,
    pub total_compressed_bytes: usize,
    pub compression_time_ms: u64,
}

impl<S> WsTransport<S> {
    pub fn new(stream: WebSocketStream<S>) -> Self {
        Self {
            stream,
            compression_config: CompressionConfig::default(),
            negotiated_algorithms: vec![CompressionAlgorithm::None],
            adaptive_compression: true,
            message_count: 0,
            compression_stats: CompressionStats::default(),
        }
    }

    pub fn with_compression_config(mut self, config: CompressionConfig) -> Self {
        self.compression_config = config.clone();
        self.negotiated_algorithms = vec![config.algorithm];
        self
    }

    pub fn with_adaptive_compression(mut self, enabled: bool) -> Self {
        self.adaptive_compression = enabled;
        self
    }

    pub fn negotiate_compression(&mut self, client_algorithms: &[CompressionAlgorithm]) -> CompressionAlgorithm {
        // Find the best mutually supported algorithm
        let mut best_algorithm = CompressionAlgorithm::None;

        for client_alg in client_algorithms {
            if self.supports_algorithm(*client_alg) {
                match client_alg {
                    #[cfg(feature = "compression")]
                    CompressionAlgorithm::Lz4 => {
                        best_algorithm = *client_alg;
                        break; // LZ4 is fast and good for real-time
                    }
                    #[cfg(feature = "compression")]
                    CompressionAlgorithm::Zstd => {
                        if best_algorithm == CompressionAlgorithm::None {
                            best_algorithm = *client_alg;
                        }
                    }
                    #[cfg(feature = "compression")]
                    CompressionAlgorithm::Snappy => {
                        if best_algorithm == CompressionAlgorithm::None {
                            best_algorithm = *client_alg;
                        }
                    }
                    CompressionAlgorithm::None => {
                        best_algorithm = CompressionAlgorithm::None;
                    }
                }
            }
        }

        // Update compression config
        self.compression_config.algorithm = best_algorithm;
        self.negotiated_algorithms = vec![best_algorithm];

        best_algorithm
    }

    pub fn supports_algorithm(&self, algorithm: CompressionAlgorithm) -> bool {
        match algorithm {
            CompressionAlgorithm::None => true,
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Lz4 => true,
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Zstd => true,
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Snappy => true,
        }
    }

    pub fn get_compression_stats(&self) -> &CompressionStats {
        &self.compression_stats
    }

    pub fn get_compression_ratio(&self) -> f32 {
        if self.compression_stats.total_original_bytes == 0 {
            1.0
        } else {
            self.compression_stats.total_compressed_bytes as f32 /
            self.compression_stats.total_original_bytes as f32
        }
    }

    fn update_compression_stats(&mut self, original_size: usize, compressed_size: usize, compression_time_ms: u64) {
        self.compression_stats.total_messages += 1;
        self.compression_stats.total_original_bytes += original_size;
        self.compression_stats.total_compressed_bytes += compressed_size;
        self.compression_stats.compression_time_ms += compression_time_ms;

        if compressed_size < original_size {
            self.compression_stats.compressed_messages += 1;
        }

        // Adaptive algorithm selection based on performance
        if self.adaptive_compression && self.compression_stats.total_messages % 100 == 0 {
            self.adapt_compression_algorithm();
        }
    }

    fn adapt_compression_algorithm(&mut self) {
        let ratio = self.get_compression_ratio();
        let avg_compression_time = self.compression_stats.compression_time_ms / self.compression_stats.total_messages as u64;

        // Switch algorithms based on performance metrics
        if ratio > 0.8 && avg_compression_time > 5 {
            // Poor compression and slow - try a faster algorithm
            if self.compression_config.algorithm != CompressionAlgorithm::None {
                self.compression_config.algorithm = CompressionAlgorithm::None;
            }
        } else if ratio < 0.6 && avg_compression_time < 10 {
            // Good compression and reasonable speed - try better compression
            #[cfg(feature = "compression")]
            {
                if self.compression_config.algorithm == CompressionAlgorithm::Lz4 {
                    self.compression_config.algorithm = CompressionAlgorithm::Zstd;
                }
            }
        }
    }
}

impl WsTransport<MaybeTlsStream<TcpStream>> {
    pub async fn connect(url: &str) -> Result<Self, TransportError> {
        let (stream, _response) = connect_async(url).await.map_err(map_ws_error)?;
        Ok(Self::new(stream))
    }
}

impl WsTransport<TcpStream> {
    pub async fn accept(stream: TcpStream) -> Result<Self, TransportError> {
        let ws_stream = accept_async(stream).await.map_err(map_ws_error)?;
        Ok(Self::new(ws_stream))
    }
}

#[async_trait]
impl<S> GameTransport for WsTransport<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    fn kind(&self) -> TransportKind {
        TransportKind::WebSocket
    }

    async fn send_frame(&mut self, frame: Frame) -> Result<(), TransportError> {
        let start_time = std::time::Instant::now();
        let bytes = message::encode(&frame).map_err(map_encode_error)?;

        // Apply compression with current config
        let compressed = Compression::compress(&bytes, &self.compression_config);

        let message_data = if compressed.is_effective() {
            // Prepend compression header
            let mut data = Vec::new();
            data.push(compressed.algorithm as u8); // 1 byte for algorithm
            data.extend_from_slice(&compressed.data);
            data
        } else {
            bytes.clone() // Use original if compression not effective
        };

        // Update compression statistics
        let compression_time = start_time.elapsed().as_millis() as u64;
        self.update_compression_stats(bytes.len(), message_data.len(), compression_time);

        self.stream
            .send(Message::Binary(message_data))
            .await
            .map_err(map_ws_error)
    }

    async fn recv_frame(&mut self) -> Result<Frame, TransportError> {
        loop {
            match self.stream.next().await {
                Some(Ok(Message::Binary(bytes))) => {
                    // Check if data is compressed (first byte indicates compression algorithm)
                    let data = if bytes.len() > 1 {
                        match bytes[0] {
                            0 => bytes[1..].to_vec(), // No compression
                            #[cfg(feature = "compression")]
                            1 => {
                                // LZ4 compressed
                                let compressed = CompressedData {
                                    algorithm: crate::compression::CompressionAlgorithm::Lz4,
                                    original_size: 0, // Unknown
                                    compressed_size: bytes.len() - 1,
                                    data: bytes[1..].to_vec(),
                                };
                                Compression::decompress(&compressed)
                                    .map_err(|e| TransportError::new(TransportErrorKind::DecodingFailure, e.to_string()))?
                            }
                            #[cfg(feature = "compression")]
                            2 => {
                                // Zstd compressed
                                let compressed = CompressedData {
                                    algorithm: crate::compression::CompressionAlgorithm::Zstd,
                                    original_size: 0, // Unknown
                                    compressed_size: bytes.len() - 1,
                                    data: bytes[1..].to_vec(),
                                };
                                Compression::decompress(&compressed)
                                    .map_err(|e| TransportError::new(TransportErrorKind::DecodingFailure, e.to_string()))?
                            }
                            #[cfg(feature = "compression")]
                            3 => {
                                // Snappy compressed
                                let compressed = CompressedData {
                                    algorithm: crate::compression::CompressionAlgorithm::Snappy,
                                    original_size: 0, // Unknown
                                    compressed_size: bytes.len() - 1,
                                    data: bytes[1..].to_vec(),
                                };
                                Compression::decompress(&compressed)
                                    .map_err(|e| TransportError::new(TransportErrorKind::DecodingFailure, e.to_string()))?
                            }
                            _ => {
                                // Unknown compression algorithm, treat as uncompressed
                                bytes.to_vec()
                            }
                        }
                    } else {
                        bytes
                    };

                    return message::decode(&data).map_err(map_decode_error);
                }
                Some(Ok(Message::Text(text))) => {
                    return message::decode(text.as_bytes()).map_err(map_decode_error);
                }
                Some(Ok(Message::Ping(payload))) => {
                    self.stream
                        .send(Message::Pong(payload))
                        .await
                        .map_err(map_ws_error)?;
                }
                Some(Ok(Message::Pong(_))) => {}
                Some(Ok(Message::Close(_))) => {
                    return Err(TransportError::new(
                        TransportErrorKind::ConnectionClosed,
                        "websocket closed",
                    ));
                }
                Some(Ok(other)) => {
                    return Err(TransportError::new(
                        TransportErrorKind::Unsupported,
                        format!("unsupported message: {other:?}"),
                    ));
                }
                Some(Err(err)) => return Err(map_ws_error(err)),
                None => {
                    return Err(TransportError::new(
                        TransportErrorKind::ConnectionClosed,
                        "websocket closed",
                    ));
                }
            }
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.stream
            .close(None)
            .await
            .map_err(|err: WsError| map_ws_error(err))
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        self.stream.flush().await.map_err(map_ws_error)
    }

    fn set_compression_config(&mut self, config: CompressionConfig) {
        self.compression_config = config;
    }

    fn get_compression_config(&self) -> &CompressionConfig {
        &self.compression_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Channel, ControlMessage};

    #[tokio::test]
    async fn ws_transport_roundtrip() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("accept");
            let mut transport = WsTransport::accept(tcp).await.expect("accept ws");
            transport.recv_frame().await.expect("recv")
        });

        let url = format!("ws://{addr}");
        let mut client = WsTransport::connect(&url).await.expect("connect");
        let start = tokio::time::Instant::now();
        let frame = Frame::control(1, 123, ControlMessage::Ping { nonce: 9 });
        client.send_frame(frame).await.expect("send");

        let result = server.await.expect("join");
        let rtt = start.elapsed();
        assert_eq!(result.channel, Channel::Control);
        assert!(rtt < std::time::Duration::from_secs(1));
    }
}
