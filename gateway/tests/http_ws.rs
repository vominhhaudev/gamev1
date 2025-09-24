use std::{net::SocketAddr, time::Duration};

use common_net::telemetry;
use gateway::auth::{AuthError, JwtUtils, WalletVerifier};
use reqwest::StatusCode;
use tokio::{sync::oneshot, task::JoinHandle};
use worker::rpc;

use gateway::build_router;

type BoxError = common_net::metrics::BoxError;

async fn spawn_gateway() -> Result<
    (
        SocketAddr,
        oneshot::Sender<()>,
        JoinHandle<()>,
        JoinHandle<()>,
    ),
    BoxError,
> {
    telemetry::init("gateway-test");

    let (worker_endpoint, worker_handle) = rpc::spawn_test_server().await;
    let app = build_router(&worker_endpoint)?;

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
            tracing::error!(%err, "gateway test server failed");
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

    shutdown_tx.send(()).ok();
    let _ = server.await;
    worker_handle.abort();
    let _ = worker_handle.await;
    Ok(())
}

#[test]
fn wallet_signature_verification_roundtrip() {
    let message = "deterministic-message";
    let seed = [7u8; 64];
    let (signature, wallet_address) =
        WalletVerifier::create_test_signature(message, Some(&seed)).expect("generate signature");

    let verified = WalletVerifier::verify_test_signature(message, &signature, &wallet_address)
        .expect("verify signature");
    assert!(verified, "signature should validate");

    let err = WalletVerifier::verify_test_signature(message, "AAAA", &wallet_address)
        .expect_err("invalid signature should fail");
    assert!(matches!(
        err,
        AuthError::InvalidSignature | AuthError::SignatureVerificationFailed
    ));
}

#[test]
fn jwt_utils_generates_and_verifies_tokens() {
    let secret = [42u8; 32];
    let jwt = JwtUtils::new(secret);

    let token = jwt
        .generate_token("wallet", "reconnect", "user", 10)
        .expect("token generation");
    let claims = jwt.verify_token(&token).expect("token verification");

    assert_eq!(claims.sub, "wallet");
    assert_eq!(claims.reconnect_token, "reconnect");
    assert_eq!(claims.user_id, "user");

    let invalid = jwt.verify_token("not-a-token");
    assert!(invalid.is_err(), "invalid token should fail");
}
