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
use crate::message::{self, Frame};

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
}

impl<S> WsTransport<S> {
    pub fn new(stream: WebSocketStream<S>) -> Self {
        Self { stream }
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
        let bytes = message::encode(&frame).map_err(map_encode_error)?;
        self.stream
            .send(Message::Binary(bytes))
            .await
            .map_err(map_ws_error)
    }

    async fn recv_frame(&mut self) -> Result<Frame, TransportError> {
        loop {
            match self.stream.next().await {
                Some(Ok(Message::Binary(bytes))) => {
                    return message::decode(&bytes).map_err(map_decode_error);
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
