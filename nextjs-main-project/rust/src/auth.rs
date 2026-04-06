use axum::http::HeaderMap;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub refresh_secret: String,
    pub access_ttl_seconds: u64,
    pub refresh_ttl_seconds: u64,
    pub secure_cookies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    #[serde(default)]
    role: Option<String>,
    exp: usize,
    iat: usize,
    jti: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Root,
    Admin,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            UserRole::Root => "root",
            UserRole::Admin => "admin",
        }
    }

    fn from_claim(value: Option<&str>) -> Self {
        match value {
            Some(raw) if raw.eq_ignore_ascii_case("admin") => UserRole::Admin,
            _ => UserRole::Root,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessPrincipal {
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
}

pub fn issue_access_token(
    username: &str,
    role: UserRole,
    cfg: &AuthConfig,
    now_unix: u64,
) -> Result<String, String> {
    let claims = AccessClaims {
        sub: username.to_owned(),
        role: Some(role.as_str().to_owned()),
        iat: now_unix as usize,
        exp: now_unix.saturating_add(cfg.access_ttl_seconds) as usize,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )
    .map_err(|err| err.to_string())
}

pub fn validate_access_token(token: &str, cfg: &AuthConfig) -> Result<AccessPrincipal, AuthError> {
    let validation = Validation::new(Algorithm::HS256);
    let decoded = decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AuthError::InvalidToken)?;

    Ok(AccessPrincipal {
        username: decoded.claims.sub,
        role: UserRole::from_claim(decoded.claims.role.as_deref()),
    })
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, AuthError> {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AuthError::MissingToken)?;

    Ok(token)
}

pub fn new_refresh_token() -> String {
    Uuid::new_v4().to_string()
}

pub fn hash_refresh_token(token: &str, refresh_secret: &str) -> String {
    let input = format!("{refresh_secret}:{token}");
    let digest = Sha256::digest(input.as_bytes());
    to_hex(&digest)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for value in bytes {
        output.push_str(&format!("{value:02x}"));
    }
    output
}

pub fn read_cookie(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;

    for part in cookie_header.split(';') {
        let segment = part.trim();
        let (name, value) = segment.split_once('=')?;
        if name == cookie_name {
            return Some(value.to_owned());
        }
    }

    None
}

pub fn build_refresh_cookie(token: &str, max_age_seconds: u64, secure: bool) -> String {
    let mut cookie = format!(
        "refresh_token={token}; Path=/; HttpOnly; Max-Age={max_age_seconds}; SameSite=Strict"
    );

    if secure {
        cookie.push_str("; Secure");
    }

    cookie
}

pub fn build_clear_refresh_cookie(secure: bool) -> String {
    let mut cookie = "refresh_token=; Path=/; HttpOnly; Max-Age=0; SameSite=Strict".to_owned();
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}
