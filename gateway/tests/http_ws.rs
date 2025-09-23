use std::{net::SocketAddr, time::Duration};

use common_net::{
    message::{self, ControlMessage, Frame, FramePayload},
    telemetry,
};
use futures::{SinkExt, StreamExt};
use reqwest::StatusCode;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use worker::rpc;

type BoxError = common_net::metrics::BoxError;

async fn spawn_gateway() -> Result<
    (
        SocketAddr,
        oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
    ),
    BoxError,
> {
    telemetry::init("gateway-test");

    let (worker_endpoint, worker_handle) = rpc::spawn_test_server().await;
    let app = gateway::build_router(&worker_endpoint)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let shutdown = async {
            let _ = shutdown_rx.await;
        };

        if let Err(err) = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown)
        .await
        {
            tracing::error!(%err, "gateway test server gap loi");
        }
    });

    Ok((addr, shutdown_tx, server, worker_handle))
}

#[tokio::test]
async fn http_endpoints_work() -> Result<(), BoxError> {
    let (addr, shutdown_tx, server, worker_handle) = spawn_gateway().await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let base = format!("http://{}", addr);

    let health = client.get(format!("{base}/healthz")).send().await?;
    assert_eq!(StatusCode::OK, health.status());

    let version_resp = client.get(format!("{base}/version")).send().await?;
    assert_eq!(StatusCode::OK, version_resp.status());
    let version_body: serde_json::Value = version_resp.json().await?;
    assert_eq!("gateway", version_body["name"]);

    let metrics_resp = client.get(format!("{base}/metrics")).send().await?;
    assert_eq!(StatusCode::OK, metrics_resp.status());
    let metrics_text = metrics_resp.text().await?;
    assert!(metrics_text.contains("gateway_http_requests_total"));

    let negotiate_resp = client.get(format!("{base}/negotiate")).send().await?;
    assert_eq!(StatusCode::OK, negotiate_resp.status());
    let body: serde_json::Value = negotiate_resp.json().await?;
    assert!(body["transports"].is_array());
    let transports = body["transports"].as_array().unwrap();
    let mut ws_ok = false;
    for t in transports {
        if t["kind"] == "websocket" && t["available"] == true {
            ws_ok = true;
        }
    }
    assert!(ws_ok, "/negotiate must advertise websocket available");

    shutdown_tx.send(()).ok();
    server.await.expect("gateway server task panicked");
    worker_handle.abort();
    Ok(())
}

#[tokio::test]
async fn websocket_echoes_messages() -> Result<(), BoxError> {
    let (addr, shutdown_tx, server, worker_handle) = spawn_gateway().await?;
    let url = format!("ws://{}/ws", addr);

    let (mut ws_stream, _response) = connect_async(&url).await?;
    let frame = Frame::control(1, 123, ControlMessage::Ping { nonce: 9 });
    let payload = message::encode(&frame)?;
    ws_stream.send(Message::Binary(payload)).await?;

    let echoed = ws_stream
        .next()
        .await
        .expect("expected message from gateway")?;

    match echoed {
        Message::Binary(bytes) => {
            let frame = message::decode(&bytes)?;
            assert_eq!(frame.channel, message::Channel::Control);
            match frame.payload {
                FramePayload::Control {
                    message: ControlMessage::Pong { nonce },
                } => assert_eq!(nonce, 9),
                other => panic!("unexpected control payload: {other:?}"),
            }
        }
        other => panic!("unexpected message type: {:?}", other),
    }

    ws_stream.close(None).await.ok();

    shutdown_tx.send(()).ok();
    server.await.expect("gateway server task panicked");
    worker_handle.abort();
    Ok(())
}

#[tokio::test]
async fn websocket_joinroom_emits_joined_event() -> Result<(), BoxError> {
    let (addr, shutdown_tx, server, worker_handle) = spawn_gateway().await?;
    let url = format!("ws://{}/ws", addr);

    let (mut ws_stream, _response) = connect_async(&url).await?;
    let frame = Frame::control(
        1,
        123,
        ControlMessage::JoinRoom { room_id: "alpha".into(), reconnect_token: None },
    );
    let payload = message::encode(&frame)?;
    ws_stream.send(Message::Binary(payload)).await?;

    let echoed = ws_stream
        .next()
        .await
        .expect("expected message from gateway")?;

    match echoed {
        Message::Binary(bytes) => {
            let frame = message::decode(&bytes)?;
            assert_eq!(frame.channel, message::Channel::State);
            match frame.payload {
                FramePayload::State { message } => match message {
                    message::StateMessage::Event { name, data } => {
                        assert_eq!(name, "joined");
                        assert_eq!(data["room_id"], "alpha");
                    }
                    other => panic!("unexpected state message: {other:?}"),
                },
                other => panic!("unexpected payload: {other:?}"),
            }
        }
        other => panic!("unexpected message type: {:?}", other),
    }

    ws_stream.close(None).await.ok();
    shutdown_tx.send(()).ok();
    server.await.expect("gateway server task panicked");
    worker_handle.abort();
    Ok(())
}

#[tokio::test]
async fn websocket_joinroom_receives_tick_sequence() -> Result<(), BoxError> {
    std::env::set_var("GATEWAY_TICK_MS", "50");
    let (addr, shutdown_tx, server, worker_handle) = spawn_gateway().await?;
    let url = format!("ws://{}/ws", addr);

    let (mut ws_stream, _response) = connect_async(&url).await?;
    // join room
    let join = Frame::control(
        1,
        123,
        ControlMessage::JoinRoom { room_id: "beta".into(), reconnect_token: None },
    );
    ws_stream.send(Message::Binary(message::encode(&join)?)).await?;

    // expect first joined event
    let _ = ws_stream.next().await.expect("expect joined");

    // collect 5 ticks
    let mut last_seq: Option<u32> = None;
    let mut received = 0u32;
    while received < 5 {
        let msg = ws_stream.next().await.expect("tick")?;
        if let Message::Binary(bytes) = msg {
            let frame = message::decode(&bytes)?;
            if let FramePayload::State { message: message::StateMessage::Event { name, data: _ } } = frame.payload {
                if name == "tick" {
                    if let Some(prev) = last_seq { assert!(frame.sequence >= prev); }
                    last_seq = Some(frame.sequence);
                    received += 1;
                }
            }
        }
    }

    ws_stream.close(None).await.ok();
    shutdown_tx.send(()).ok();
    server.await.expect("gateway server task panicked");
    worker_handle.abort();
    Ok(())
}
