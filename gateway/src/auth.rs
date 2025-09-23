//! Authentication system với JWT và wallet signature verification
//!
//! Hỗ trợ:
//! - Solana wallet signature verification (Ed25519)
//! - JWT token generation và validation
//! - Nonce-based authentication flow
//! - Rate limiting cho auth endpoints

use base64::{Engine as _, engine::general_purpose};
use chrono::{Duration, Utc};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, SECRET_KEY_LENGTH};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// JWT Secret key (256-bit)
pub type JwtSecret = [u8; 32];

/// Nonce store để track các nonce đang active
pub type NonceStore = Arc<RwLock<HashMap<String, NonceData>>>;

/// Nonce data với expiry
#[derive(Debug, Clone)]
pub struct NonceData {
    pub nonce: String,
    pub wallet_address: String,
    pub expires_at: i64,  // Unix timestamp
}

/// Auth request/response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceRequest {
    pub wallet_address: String,  // Solana wallet address (base58)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceResponse {
    pub nonce: String,          // Random string to sign
    pub expires_at: u64,        // Unix timestamp
    pub request_id: String,     // Unique request ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub wallet_address: String,  // Base58 encoded wallet address
    pub signature: String,       // Base64 encoded signature
    pub message: String,         // Original message that was signed
    pub nonce: String,           // Nonce from /auth/nonce response
    pub request_id: String,      // Request ID from nonce response
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub jwt: String,            // JWT token for authentication
    pub reconnect_token: String, // Token for reconnection
    pub expires_at: u64,        // Token expiry timestamp
    pub user_id: String,        // User identifier
}

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,            // Subject (wallet_address)
    pub exp: usize,             // Expiry time
    pub iat: usize,             // Issued at time
    pub reconnect_token: String, // Reconnection token
    pub user_id: String,        // Internal user ID
}

/// Authentication errors
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

/// Test keypair cho development và testing
#[derive(Debug, Clone)]
pub struct TestKeypair {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub wallet_address: String,
}

impl TestKeypair {
    /// Generate new test keypair
    pub fn generate() -> Self {
        let mut keypair_bytes = [0u8; 64];
        thread_rng().fill(&mut keypair_bytes);

        let secret_key = SecretKey::from_bytes(&keypair_bytes[..32])
            .expect("Failed to create secret key");
        let public_key = PublicKey::from(&secret_key);
        let keypair = Keypair { secret: secret_key, public: public_key };

        let wallet_address = Self::public_key_to_wallet_address(&keypair.public);

        Self {
            secret_key: keypair.secret,
            public_key: keypair.public,
            wallet_address,
        }
    }

    /// Create test keypair from seed (deterministic)
    pub fn from_seed(seed: &[u8]) -> Self {
        let mut keypair_bytes = [0u8; 64];
        keypair_bytes[..seed.len()].copy_from_slice(seed);

        let secret_key = SecretKey::from_bytes(&keypair_bytes[..32])
            .expect("Failed to create secret key");
        let public_key = PublicKey::from(&secret_key);
        let keypair = Keypair { secret: secret_key, public: public_key };

        let wallet_address = Self::public_key_to_wallet_address(&keypair.public);

        Self {
            secret_key: keypair.secret,
            public_key: keypair.public,
            wallet_address,
        }
    }

    /// Sign message với test keypair
    pub fn sign(&self, message: &str) -> String {
        let signature = self.secret_key.sign(message.as_bytes());
        general_purpose::STANDARD.encode(signature.to_bytes())
    }

    /// Convert public key to Solana wallet address (base58)
    fn public_key_to_wallet_address(public_key: &PublicKey) -> String {
        bs58::encode(public_key.to_bytes()).into_string()
    }
}

/// Wallet signature verifier
pub struct WalletVerifier;

impl WalletVerifier {
    /// Verify Solana wallet signature
    ///
    /// # Arguments
    /// * `message` - Original message that was signed
    /// * `signature` - Base64 encoded Ed25519 signature
    /// * `wallet_address` - Base58 encoded wallet address
    ///
    /// # Returns
    /// * `Ok(true)` if signature is valid
    /// * `Err(AuthError)` if verification fails
    pub fn verify_solana_signature(
        message: &str,
        signature: &str,
        wallet_address: &str,
    ) -> Result<bool, AuthError> {
        // Validate wallet address format first
        Self::validate_solana_address(wallet_address)?;

        // Decode signature from base64
        let signature_bytes = general_purpose::STANDARD
            .decode(signature)
            .map_err(|_| AuthError::InvalidSignature)?;

        if signature_bytes.len() != SIGNATURE_LENGTH {
            return Err(AuthError::InvalidSignature);
        }

        let signature = Signature::from_bytes(&signature_bytes);

        // Convert wallet address to public key
        let public_key = Self::wallet_address_to_public_key(wallet_address)?;

        // Verify signature
        match public_key.verify(message.as_bytes(), &signature) {
            Ok(_) => Ok(true),
            Err(_) => Err(AuthError::SignatureVerificationFailed),
        }
    }

    /// Validate Solana wallet address format
    ///
    /// # Arguments
    /// * `wallet_address` - Base58 encoded wallet address
    ///
    /// # Returns
    /// * `Ok(())` if address is valid
    /// * `Err(AuthError)` if address is invalid
    pub fn validate_solana_address(wallet_address: &str) -> Result<(), AuthError> {
        // Check length (32 bytes base58 = 32-44 characters)
        if wallet_address.len() < 32 || wallet_address.len() > 44 {
            return Err(AuthError::InvalidWalletAddress);
        }

        // Try to decode as base58
        let decoded = bs58::decode(wallet_address)
            .into_vec()
            .map_err(|_| AuthError::InvalidWalletAddress)?;

        // Should be 32 bytes (Ed25519 public key)
        if decoded.len() != PUBLIC_KEY_LENGTH {
            return Err(AuthError::InvalidWalletAddress);
        }

        // Try to create public key to verify format
        PublicKey::from_bytes(&decoded).map_err(|_| AuthError::InvalidWalletAddress)?;

        Ok(())
    }

    /// Convert Solana wallet address to Ed25519 public key
    fn wallet_address_to_public_key(wallet_address: &str) -> Result<PublicKey, AuthError> {
        // Base58 decode wallet address to get public key bytes
        let public_key_bytes = bs58::decode(wallet_address)
            .into_vec()
            .map_err(|_| AuthError::InvalidWalletAddress)?;

        if public_key_bytes.len() != PUBLIC_KEY_LENGTH {
            return Err(AuthError::InvalidWalletAddress);
        }

        PublicKey::from_bytes(&public_key_bytes)
            .map_err(|_| AuthError::InvalidWalletAddress)
    }

    /// Create test keypair and sign message cho testing
    ///
    /// # Arguments
    /// * `message` - Message to sign
    /// * `seed` - Optional seed for deterministic keypair
    ///
    /// # Returns
    /// * `Ok((signature, wallet_address))` if successful
    /// * `Err(AuthError)` if signing fails
    pub fn create_test_signature(message: &str, seed: Option<&[u8]>) -> Result<(String, String), AuthError> {
        let keypair = if let Some(s) = seed {
            TestKeypair::from_seed(s)
        } else {
            TestKeypair::generate()
        };

        let signature = keypair.sign(message);
        Ok((signature, keypair.wallet_address))
    }

    /// Verify test signature (cho development/testing)
    ///
    /// # Arguments
    /// * `message` - Original message
    /// * `signature` - Base64 signature
    /// * `wallet_address` - Expected wallet address
    ///
    /// # Returns
    /// * `Ok(true)` if signature is valid
    /// * `Err(AuthError)` if verification fails
    pub fn verify_test_signature(
        message: &str,
        signature: &str,
        wallet_address: &str,
    ) -> Result<bool, AuthError> {
        Self::verify_solana_signature(message, signature, wallet_address)
    }
}

/// JWT utilities
pub struct JwtUtils {
    secret: JwtSecret,
}

impl JwtUtils {
    /// Create new JWT utils with secret key
    pub fn new(secret: JwtSecret) -> Self {
        Self { secret }
    }

    /// Generate JWT token
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

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| AuthError::JwtEncodingError(e.to_string()))
    }

    /// Verify and decode JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::JwtDecodingError(e.to_string()),
        })?;

        Ok(token_data.claims)
    }
}

/// JWT Authentication Middleware
pub struct JwtAuth {
    jwt_utils: JwtUtils,
}

impl JwtAuth {
    /// Create new JWT auth middleware
    pub fn new(secret: JwtSecret) -> Self {
        Self {
            jwt_utils: JwtUtils::new(secret),
        }
    }

    /// Extract and validate JWT token from Authorization header
    pub fn extract_token_from_header(&self, auth_header: &str) -> Result<Claims, AuthError> {
        // Expected format: "Bearer <token>"
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken);
        }

        let token = auth_header.trim_start_matches("Bearer ").trim();
        if token.is_empty() {
            return Err(AuthError::InvalidToken);
        }

        self.jwt_utils.verify_token(token)
    }

    /// Verify JWT token string
    pub fn verify_token_string(&self, token: &str) -> Result<Claims, AuthError> {
        self.jwt_utils.verify_token(token)
    }
}

/// Nonce manager để quản lý các nonce đang active
pub struct NonceManager {
    store: NonceStore,
    ttl_seconds: i64, // Time-to-live cho nonce
}

impl NonceManager {
    /// Create new nonce manager
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl_seconds,
        }
    }

    /// Generate and store new nonce
    pub async fn generate_nonce(&self, wallet_address: &str) -> NonceResponse {
        let nonce = Self::generate_random_nonce();
        let expires_at = (Utc::now() + Duration::seconds(self.ttl_seconds)).timestamp() as u64;
        let request_id = Uuid::new_v4().to_string();

        let nonce_data = NonceData {
            nonce: nonce.clone(),
            wallet_address: wallet_address.to_owned(),
            expires_at: expires_at as i64,
        };

        self.store.write().await.insert(request_id.clone(), nonce_data);

        NonceResponse {
            nonce,
            expires_at,
            request_id,
        }
    }

    /// Verify nonce và remove khỏi store
    pub async fn verify_and_consume_nonce(
        &self,
        request_id: &str,
        nonce: &str,
    ) -> Result<String, AuthError> {
        let mut store = self.store.write().await;
        let nonce_data = store.remove(request_id)
            .ok_or(AuthError::NonceNotFound)?;

        // Check if nonce matches
        if nonce_data.nonce != nonce {
            return Err(AuthError::NonceNotFound);
        }

        // Check if nonce expired
        let now = Utc::now().timestamp();
        if now > nonce_data.expires_at {
            return Err(AuthError::NonceNotFound);
        }

        Ok(nonce_data.wallet_address)
    }

    /// Generate random nonce string
    fn generate_random_nonce() -> String {
        let mut rng = thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        general_purpose::STANDARD.encode(bytes)
    }

    /// Cleanup expired nonces (call periodically)
    pub async fn cleanup_expired(&self) {
        let now = Utc::now().timestamp();
        let mut store = self.store.write().await;
        store.retain(|_, nonce_data| now <= nonce_data.expires_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_generation() {
        let nonce = NonceManager::generate_random_nonce();
        assert_eq!(nonce.len(), 44); // Base64 encoded 32 bytes
    }

    #[test]
    fn test_wallet_address_validation() {
        // Test with valid Solana wallet address
        let valid_address = "11111111111111111111111111111112";
        let result = WalletVerifier::wallet_address_to_public_key(valid_address);
        assert!(result.is_ok());

        // Test with invalid address
        let invalid_address = "invalid";
        let result = WalletVerifier::wallet_address_to_public_key(invalid_address);
        assert!(result.is_err());
    }
}
