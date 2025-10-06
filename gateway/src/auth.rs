use axum::{
    extract::State,
    http::{header::AUTHORIZATION, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use hyper::Request;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, warn};

// Re-export AppState for use in auth module
use crate::AppState;

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,  // Subject (user ID)
    pub username: String,
    pub email: String,
    pub role: String,
    pub exp: i64,     // Expiration time
    pub iat: i64,     // Issued at
    pub iss: String,  // Issuer
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

// JWT configuration
const JWT_SECRET_KEY: &str = "JWT_SECRET";
const JWT_ISSUER: &str = "gamev1-gateway";
const ACCESS_TOKEN_EXPIRY: i64 = 15 * 60; // 15 minutes
const REFRESH_TOKEN_EXPIRY: i64 = 7 * 24 * 60 * 60; // 7 days

// Authentication utilities
#[derive(Clone)]
pub struct AuthService {
    secret: String,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let secret = env::var(JWT_SECRET_KEY)
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        let decoding_key = DecodingKey::from_secret(secret.as_ref());

        Ok(Self {
            secret,
            encoding_key,
            decoding_key,
        })
    }

    // Generate JWT token
    pub fn generate_token(&self, user: &User) -> Result<String, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let exp = now + Duration::minutes(ACCESS_TOKEN_EXPIRY);

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            iss: JWT_ISSUER.to_string(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok(token)
    }

    // Generate refresh token
    pub fn generate_refresh_token(&self, user: &User) -> Result<String, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let exp = now + Duration::minutes(REFRESH_TOKEN_EXPIRY);

        let mut refresh_claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            iss: JWT_ISSUER.to_string(),
        };

        // Add refresh token indicator
        refresh_claims.role.push_str(":refresh");

        let token = encode(&Header::default(), &refresh_claims, &self.encoding_key)?;
        Ok(token)
    }

    // Verify JWT token
    pub fn verify_token(&self, token: &str) -> Result<TokenData<Claims>, Box<dyn std::error::Error>> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data)
    }

    // Hash password using bcrypt
    pub fn hash_password(password: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(bcrypt::hash(password, 12)?)
    }

    // Verify password
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(bcrypt::verify(password, hash)?)
    }
}

// Extract user ID from request
pub async fn extract_user_id_from_request(
    request: &Request<hyper::Body>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(auth_header) = request.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..]; // Remove "Bearer " prefix

                // Initialize auth service if not exists
                let auth_service = AuthService::new()?;

                match auth_service.verify_token(token) {
                    Ok(token_data) => {
                        return Ok(Some(token_data.claims.sub));
                    }
                    Err(e) => {
                        warn!("Invalid token: {}", e);
                    }
                }
            }
        }
    }

    Ok(None)
}

// Authentication middleware
pub async fn auth_middleware<B>(
    request: Request<B>,
    next: axum::middleware::Next<B>,
) -> Response {
    // For now, just pass through - we'll implement proper auth later
    next.run(request).await
}

// Login handler
pub async fn login_handler(
    Json(payload): Json<AuthRequest>,
) -> impl IntoResponse {
    // Initialize auth service
    let auth_service = match AuthService::new() {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to initialize auth service: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Authentication service error").into_response();
        }
    };

    // Demo credentials validation
    if payload.username == "demo@example.com" && payload.password == "password123" {
        let user = User {
            id: "demo-user-id".to_string(),
            username: "Demo User".to_string(),
            email: payload.username.clone(),
            role: "user".to_string(),
        };

        match auth_service.generate_token(&user) {
            Ok(access_token) => {
                match auth_service.generate_refresh_token(&user) {
                    Ok(refresh_token) => {
                        let response = AuthResponse {
                            access_token,
                            refresh_token,
                            token_type: "Bearer".to_string(),
                            expires_in: ACCESS_TOKEN_EXPIRY * 60,
                            user: UserInfo {
                                id: user.id,
                                username: user.username,
                                email: user.email,
                                role: user.role,
                            },
                        };

                        return (StatusCode::OK, Json(response)).into_response();
                    }
                    Err(e) => {
                        error!("Failed to generate refresh token: {}", e);
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Token generation error").into_response();
                    }
                }
            }
            Err(e) => {
                error!("Failed to generate access token: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Token generation error").into_response();
            }
        }
    }

    // Invalid credentials
    (StatusCode::UNAUTHORIZED, "Invalid username or password").into_response()
}

// Register handler
pub async fn register_handler(
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    // Initialize auth service
    let auth_service = match AuthService::new() {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to initialize auth service: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Authentication service error").into_response();
        }
    };

    // Demo registration - accept any valid email format
    if payload.email.contains('@') && payload.email.contains('.') && payload.password.len() >= 6 {
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: payload.username.clone(),
            email: payload.email.clone(),
            role: "user".to_string(),
        };

    match auth_service.generate_token(&user) {
        Ok(access_token) => {
            match auth_service.generate_refresh_token(&user) {
                Ok(refresh_token) => {
                    let response = AuthResponse {
        access_token,
        refresh_token,
                        token_type: "Bearer".to_string(),
                        expires_in: ACCESS_TOKEN_EXPIRY * 60,
                        user: UserInfo {
                            id: user.id,
                            username: user.username,
                            email: user.email,
                            role: user.role,
                        },
                    };

                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(e) => {
                    error!("Failed to generate refresh token: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Token generation error").into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to generate access token: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Token generation error").into_response()
        }
    }
    } else {
        // Invalid registration data
        (StatusCode::BAD_REQUEST, "Invalid email or password (password must be at least 6 characters)").into_response()
    }
}

// Refresh token handler
pub async fn refresh_handler(
    Json(payload): Json<RefreshRequest>,
) -> impl IntoResponse {
    // Initialize auth service
    let auth_service = match AuthService::new() {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to initialize auth service: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Authentication service error").into_response();
        }
    };

    // Verify refresh token
    match auth_service.verify_token(&payload.refresh_token) {
        Ok(token_data) => {
            // Check if it's a refresh token
            if !token_data.claims.role.ends_with(":refresh") {
                return (StatusCode::UNAUTHORIZED, "Invalid refresh token").into_response();
            }

            // TODO: Get user from database using token_data.claims.sub
            // For now, create a dummy user
            let user = User {
                id: token_data.claims.sub.clone(),
                username: token_data.claims.username.clone(),
                email: token_data.claims.email.clone(),
                role: token_data.claims.role.strip_suffix(":refresh").unwrap_or("user").to_string(),
            };

            // Generate new tokens
            match auth_service.generate_token(&user) {
                Ok(access_token) => {
                    match auth_service.generate_refresh_token(&user) {
                        Ok(refresh_token) => {
                            let response = AuthResponse {
        access_token,
        refresh_token,
                                token_type: "Bearer".to_string(),
                                expires_in: ACCESS_TOKEN_EXPIRY * 60,
                                user: UserInfo {
                                    id: user.id,
                                    username: user.username,
                                    email: user.email,
                                    role: user.role,
                                },
                            };

                            (StatusCode::OK, Json(response)).into_response()
                        }
                        Err(e) => {
                            error!("Failed to generate refresh token: {}", e);
                            (StatusCode::INTERNAL_SERVER_ERROR, "Token generation error").into_response()
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to generate access token: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Token generation error").into_response()
                }
            }
        }
        Err(e) => {
            warn!("Invalid refresh token: {}", e);
            (StatusCode::UNAUTHORIZED, "Invalid refresh token").into_response()
        }
    }
}

// Logout handler
pub async fn logout_handler(
    request: Request<hyper::Body>,
) -> impl IntoResponse {
    // TODO: Implement token blacklisting/invalidation
    (StatusCode::OK, "Logged out successfully").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_service_creation() {
        let auth_service = AuthService::new();
        assert!(auth_service.is_ok());
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password";
        let hash = AuthService::hash_password(password);
        assert!(hash.is_ok());

        let is_valid = AuthService::verify_password(password, &hash.unwrap());
        assert!(is_valid.is_ok());
        assert!(is_valid.unwrap());
    }

    #[test]
    fn test_token_generation() {
        let auth_service = AuthService::new().unwrap();

        let user = User {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            role: "user".to_string(),
        };

        let token = auth_service.generate_token(&user);
        assert!(token.is_ok());

        let refresh_token = auth_service.generate_refresh_token(&user);
        assert!(refresh_token.is_ok());
    }
}