//! Authentication utilities for wallet signature verification and JWT handling.

use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH,
    SIGNATURE_LENGTH,
};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Nonce request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceRequest {
    pub wallet_address: String,
}

/// Nonce response payload returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: u64,
    pub request_id: String,
}

/// Signature verification payload sent by clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub wallet_address: String,
    pub signature: String,
    pub message: String,
    pub nonce: String,
    pub request_id: String,
}

/// Successful verification response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub jwt: String,
    pub reconnect_token: String,
    pub expires_at: u64,
    pub user_id: String,
}

/// JWT claims stored in issued tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub reconnect_token: String,
    pub user_id: String,
}

/// Errors emitted while handling authentication flows.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid wallet address format")]
    InvalidWalletAddress,
    #[error("Invalid signature format")]
    InvalidSignature,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Nonce not found or expired")]
    NonceNotFound,
    #[error("JWT encoding error: {0}")]
    JwtEncodingError(String),
    #[error("JWT decoding error: {0}")]
    JwtDecodingError(String),
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// Deterministic test keypair helper used by integration tests.
#[derive(Debug, Clone)]
pub struct TestKeypair {
    signing_key: SigningKey,
    pub wallet_address: String,
}

impl TestKeypair {
    /// Generate a random keypair for local tests.
    pub fn generate() -> Self {
        let mut seed = [0u8; SECRET_KEY_LENGTH];
        thread_rng().fill_bytes(&mut seed);
        Self::from_signing_key(SigningKey::from_bytes(&seed))
    }

    /// Construct a deterministic keypair from the supplied seed bytes.
    pub fn from_seed(seed: &[u8]) -> Self {
        let mut secret = [0u8; SECRET_KEY_LENGTH];
        let copy_len = seed.len().min(SECRET_KEY_LENGTH);
        secret[..copy_len].copy_from_slice(&seed[..copy_len]);
        Self::from_signing_key(SigningKey::from_bytes(&secret))
    }

    /// Sign an arbitrary UTF-8 message and return a base64 signature.
    pub fn sign(&self, message: &str) -> String {
        let signature = self.signing_key.sign(message.as_bytes());
        general_purpose::STANDARD.encode(signature.to_bytes())
    }

    fn from_signing_key(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        let wallet_address = WalletVerifier::verifying_key_to_wallet_address(&verifying_key);
        Self {
            signing_key,
            wallet_address,
        }
    }
}

/// Utilities for validating wallet signatures.
pub struct WalletVerifier;

impl WalletVerifier {
    /// Verify a base64-encoded Ed25519 signature against the provided message.
    pub fn verify_solana_signature(
        message: &str,
        signature: &str,
        wallet_address: &str,
    ) -> Result<bool, AuthError> {
        Self::validate_solana_address(wallet_address)?;

        let signature_bytes = general_purpose::STANDARD
            .decode(signature)
            .map_err(|_| AuthError::InvalidSignature)?;
        if signature_bytes.len() != SIGNATURE_LENGTH {
            return Err(AuthError::InvalidSignature);
        }
        let signature_array: [u8; SIGNATURE_LENGTH] = signature_bytes
            .try_into()
            .map_err(|_| AuthError::InvalidSignature)?;
        let signature =
            Signature::try_from(&signature_array[..]).map_err(|_| AuthError::InvalidSignature)?;

        let verifying_key = Self::wallet_address_to_verifying_key(wallet_address)?;
        verifying_key
            .verify(message.as_bytes(), &signature)
            .map(|_| true)
            .map_err(|_| AuthError::SignatureVerificationFailed)
    }

    /// Ensure a wallet address is a valid base58-encoded Ed25519 public key.
    pub fn validate_solana_address(wallet_address: &str) -> Result<(), AuthError> {
        Self::wallet_address_to_verifying_key(wallet_address).map(|_| ())
    }

    fn wallet_address_to_verifying_key(wallet_address: &str) -> Result<VerifyingKey, AuthError> {
        if wallet_address.len() < 32 || wallet_address.len() > 44 {
            return Err(AuthError::InvalidWalletAddress);
        }

        let raw = bs58::decode(wallet_address)
            .into_vec()
            .map_err(|_| AuthError::InvalidWalletAddress)?;
        if raw.len() != PUBLIC_KEY_LENGTH {
            return Err(AuthError::InvalidWalletAddress);
        }
        let bytes: [u8; PUBLIC_KEY_LENGTH] = raw
            .try_into()
            .map_err(|_| AuthError::InvalidWalletAddress)?;
        VerifyingKey::from_bytes(&bytes).map_err(|_| AuthError::InvalidWalletAddress)
    }

    fn verifying_key_to_wallet_address(verifying_key: &VerifyingKey) -> String {
        bs58::encode(verifying_key.to_bytes()).into_string()
    }

    /// Convenience helper used in tests to create/sign payloads.
    pub fn create_test_signature(
        message: &str,
        seed: Option<&[u8]>,
    ) -> Result<(String, String), AuthError> {
        let keypair = if let Some(seed) = seed {
            TestKeypair::from_seed(seed)
        } else {
            TestKeypair::generate()
        };
        let signature = keypair.sign(message);
        Ok((signature, keypair.wallet_address.clone()))
    }

    /// Verify a signature generated by [`create_test_signature`].
    pub fn verify_test_signature(
        message: &str,
        signature: &str,
        wallet_address: &str,
    ) -> Result<bool, AuthError> {
        Self::verify_solana_signature(message, signature, wallet_address)
    }
}

/// JWT helper for issuing and verifying tokens.
pub struct JwtUtils {
    secret: [u8; 32],
}

impl JwtUtils {
    pub fn new(secret: [u8; 32]) -> Self {
        Self { secret }
    }

    pub fn generate_token(
        &self,
        wallet_address: &str,
        reconnect_token: &str,
        user_id: &str,
        expires_in_minutes: i64,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let expire = now + Duration::minutes(expires_in_minutes);

        let claims = Claims {
            sub: wallet_address.to_owned(),
            exp: expire.timestamp() as usize,
            iat: now.timestamp() as usize,
            reconnect_token: reconnect_token.to_owned(),
            user_id: user_id.to_owned(),
        };

        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(&self.secret),
        )
        .map_err(|err| AuthError::JwtEncodingError(err.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(&self.secret),
            &jsonwebtoken::Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|err| match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::JwtDecodingError(err.to_string()),
        })
    }
}

/// Simple wrapper to extract/validate bearer tokens from headers.
pub struct JwtAuth {
    jwt_utils: JwtUtils,
}

impl JwtAuth {
    pub fn new(secret: [u8; 32]) -> Self {
        Self {
            jwt_utils: JwtUtils::new(secret),
        }
    }

    pub fn extract_token_from_header(&self, auth_header: &str) -> Result<Claims, AuthError> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken);
        }
        let token = auth_header.trim_start_matches("Bearer ").trim();
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }
        self.jwt_utils.verify_token(token)
    }

    pub fn verify_token_string(&self, token: &str) -> Result<Claims, AuthError> {
        self.jwt_utils.verify_token(token)
    }
}

/// Nonce metadata stored in memory during the authentication flow.
#[derive(Debug, Clone)]
pub struct NonceData {
    pub nonce: String,
    pub wallet_address: String,
    pub expires_at: i64,
}

/// Manages issued nonces and their validity windows.
pub struct NonceManager {
    store: Arc<RwLock<HashMap<String, NonceData>>>,
    ttl_seconds: i64,
}

impl NonceManager {
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
        }
    }

    pub async fn generate_nonce(&self, wallet_address: &str) -> NonceResponse {
        let nonce = Self::generate_random_nonce();
        let expires_at = (Utc::now() + Duration::seconds(self.ttl_seconds)).timestamp() as u64;
        let request_id = Uuid::new_v4().to_string();

        let data = NonceData {
            nonce: nonce.clone(),
            wallet_address: wallet_address.to_owned(),
            expires_at: expires_at as i64,
        };

        self.store.write().await.insert(request_id.clone(), data);

        NonceResponse {
            nonce,
            expires_at,
            request_id,
        }
    }

    pub async fn verify_and_consume_nonce(
        &self,
        request_id: &str,
        nonce: &str,
    ) -> Result<String, AuthError> {
        let mut store = self.store.write().await;
        let data = store.remove(request_id).ok_or(AuthError::NonceNotFound)?;

        if data.nonce != nonce {
            return Err(AuthError::NonceNotFound);
        }
        if Utc::now().timestamp() > data.expires_at {
            return Err(AuthError::NonceNotFound);
        }

        Ok(data.wallet_address)
    }

    fn generate_random_nonce() -> String {
        let mut bytes = [0u8; 32];
        thread_rng().fill_bytes(&mut bytes);
        general_purpose::STANDARD.encode(bytes)
    }

    pub async fn cleanup_expired(&self) {
        let now = Utc::now().timestamp();
        let mut store = self.store.write().await;
        store.retain(|_, data| now <= data.expires_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_generation() {
        let nonce = NonceManager::generate_random_nonce();
        assert_eq!(nonce.len(), 44);
    }

    #[test]
    fn test_wallet_address_validation() {
        let kp = TestKeypair::generate();
        assert!(WalletVerifier::validate_solana_address(&kp.wallet_address).is_ok());
        assert!(WalletVerifier::validate_solana_address("invalid").is_err());
    }
}
