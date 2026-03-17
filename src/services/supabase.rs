// ══════════════════════════════════════════════════════════════
// Supabase Client — Anonymous Auth & Data Sync (Ticket 06)
//
// Communicates via PostgREST REST API + Auth REST endpoints
// using reqwest. Works on both native and WASM targets.
//
// All code below was developed via strict RED/GREEN TDD:
//   1. RED  -- failing test written first
//   2. GREEN -- minimum production code to pass
//   3. REFACTOR -- clean up, keep all tests green
// ══════════════════════════════════════════════════════════════

use crate::services::platform::SecureStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Configuration ──
// Real keys live in src/config.rs (gitignored). See src/config.rs.example.
// dx serve compiles WASM in a subprocess that doesn't inherit shell env vars,
// so we use a source file instead of env!() / option_env!().

pub use crate::config::{SUPABASE_URL, SUPABASE_ANON_KEY};
const JWT_STORAGE_KEY: &str = "supabase_jwt";
const REFRESH_TOKEN_STORAGE_KEY: &str = "supabase_refresh_token";

// ── Error types ──

#[derive(Debug, Clone, PartialEq)]
pub enum SupabaseError {
    /// Network is unreachable or request timed out
    NetworkError(String),
    /// Authentication failed (invalid key, expired token, etc.)
    AuthError(String),
    /// Server returned an error response
    ApiError { status: u16, message: String },
    /// Failed to parse response body
    ParseError(String),
    /// JWT storage read/write failed
    StorageError(String),
    /// No valid JWT available (not authenticated)
    NotAuthenticated,
}

impl std::fmt::Display for SupabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupabaseError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            SupabaseError::AuthError(msg) => write!(f, "Auth error: {}", msg),
            SupabaseError::ApiError { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            SupabaseError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SupabaseError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            SupabaseError::NotAuthenticated => write!(f, "Not authenticated"),
        }
    }
}

// ── Auth response types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: AuthUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub is_anonymous: Option<bool>,
}

// ── Database row types (matching Supabase schema) ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserStateRow {
    pub id: String,
    #[serde(default)]
    pub total_xp: i64,
    #[serde(default)]
    pub current_streak: i64,
    #[serde(default)]
    pub longest_streak: i64,
    #[serde(default)]
    pub last_active: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageProgressRow {
    #[serde(default)]
    pub id: Option<String>,
    pub user_id: String,
    pub language_id: String,
    #[serde(default)]
    pub xp: i64,
    #[serde(default)]
    pub active_module: i64,
    #[serde(default)]
    pub unlocked_modules: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChallengeAttemptRow {
    #[serde(default)]
    pub id: Option<String>,
    pub user_id: String,
    pub challenge_id: String,
    pub language_id: String,
    pub correct: bool,
    pub attempt_num: i64,
    #[serde(default)]
    pub attempted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreakLogRow {
    pub user_id: String,
    pub day: String,
}

// ── HTTP adapter trait (for testability) ──

/// Abstraction over HTTP transport. Production uses reqwest.
/// Tests inject a mock that records requests and returns canned responses.
#[allow(async_fn_in_trait)]
pub trait HttpClient: Send + Sync {
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
    ) -> Result<HttpResponse, SupabaseError>;
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

// ── Supabase Client ──

pub struct SupabaseClient<H: HttpClient, S: SecureStorage> {
    pub base_url: String,
    pub anon_key: String,
    jwt: std::sync::Mutex<Option<String>>,
    refresh_token: std::sync::Mutex<Option<String>>,
    user_id: std::sync::Mutex<Option<String>>,
    pub http: H,
    storage: Arc<S>,
}

impl<H: HttpClient, S: SecureStorage> SupabaseClient<H, S> {
    pub fn new(base_url: &str, anon_key: &str, http: H, storage: Arc<S>) -> Self {
        Self {
            base_url: base_url.to_string(),
            anon_key: anon_key.to_string(),
            jwt: std::sync::Mutex::new(None),
            refresh_token: std::sync::Mutex::new(None),
            user_id: std::sync::Mutex::new(None),
            http,
            storage,
        }
    }

    /// Try to rehydrate JWT from secure storage. Returns true if a valid JWT
    /// was found. Does NOT make any network calls.
    pub fn rehydrate_from_storage(&self) -> Result<bool, SupabaseError> {
        let jwt = self
            .storage
            .get(JWT_STORAGE_KEY)
            .map_err(|e| SupabaseError::StorageError(format!("{:?}", e)))?;

        let refresh = self
            .storage
            .get(REFRESH_TOKEN_STORAGE_KEY)
            .map_err(|e| SupabaseError::StorageError(format!("{:?}", e)))?;

        match jwt {
            Some(token) if !token.is_empty() => {
                let uid = Self::extract_user_id_from_jwt(&token);
                *self.jwt.lock().unwrap() = Some(token);
                if let Some(rt) = refresh {
                    *self.refresh_token.lock().unwrap() = Some(rt);
                }
                if let Some(uid) = uid {
                    *self.user_id.lock().unwrap() = Some(uid);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Sign in anonymously. Creates a new anonymous user with Supabase Auth.
    pub async fn sign_in_anonymous(&self) -> Result<AuthResponse, SupabaseError> {
        let url = format!("{}/auth/v1/signup", self.base_url);
        let body = serde_json::json!({}).to_string();

        let headers = vec![
            ("apikey".to_string(), self.anon_key.clone()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];

        let resp = self
            .http
            .request("POST", &url, &headers, Some(body))
            .await?;

        if resp.status == 401 || resp.status == 403 {
            return Err(SupabaseError::AuthError(format!(
                "Authentication failed ({}): {}",
                resp.status, resp.body
            )));
        }

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        let auth: AuthResponse = serde_json::from_str(&resp.body)
            .map_err(|e| SupabaseError::ParseError(e.to_string()))?;

        // Store JWT and refresh token in secure storage
        self.storage
            .set(JWT_STORAGE_KEY, &auth.access_token)
            .map_err(|e| SupabaseError::StorageError(format!("{:?}", e)))?;

        self.storage
            .set(REFRESH_TOKEN_STORAGE_KEY, &auth.refresh_token)
            .map_err(|e| SupabaseError::StorageError(format!("{:?}", e)))?;

        *self.jwt.lock().unwrap() = Some(auth.access_token.clone());
        *self.refresh_token.lock().unwrap() = Some(auth.refresh_token.clone());
        *self.user_id.lock().unwrap() = Some(auth.user.id.clone());

        Ok(auth)
    }

    /// Refresh an expired JWT using the stored refresh token.
    pub async fn refresh_jwt(&self) -> Result<AuthResponse, SupabaseError> {
        let rt = self
            .refresh_token
            .lock()
            .unwrap()
            .clone()
            .ok_or(SupabaseError::NotAuthenticated)?;

        let url = format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            self.base_url
        );
        let body = serde_json::json!({ "refresh_token": rt }).to_string();

        let headers = vec![
            ("apikey".to_string(), self.anon_key.clone()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];

        let resp = self
            .http
            .request("POST", &url, &headers, Some(body))
            .await?;

        if resp.status == 401 || resp.status == 403 {
            return Err(SupabaseError::AuthError(format!(
                "Refresh failed ({}): {}",
                resp.status, resp.body
            )));
        }

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        let auth: AuthResponse = serde_json::from_str(&resp.body)
            .map_err(|e| SupabaseError::ParseError(e.to_string()))?;

        self.storage
            .set(JWT_STORAGE_KEY, &auth.access_token)
            .map_err(|e| SupabaseError::StorageError(format!("{:?}", e)))?;

        self.storage
            .set(REFRESH_TOKEN_STORAGE_KEY, &auth.refresh_token)
            .map_err(|e| SupabaseError::StorageError(format!("{:?}", e)))?;

        *self.jwt.lock().unwrap() = Some(auth.access_token.clone());
        *self.refresh_token.lock().unwrap() = Some(auth.refresh_token.clone());
        *self.user_id.lock().unwrap() = Some(auth.user.id.clone());

        Ok(auth)
    }

    /// Get the current user ID (extracted from JWT).
    pub fn current_user_id(&self) -> Option<String> {
        self.user_id.lock().unwrap().clone()
    }

    /// Get the current JWT. Never log the return value.
    pub fn current_jwt(&self) -> Option<String> {
        self.jwt.lock().unwrap().clone()
    }

    // ── CRUD Operations ──

    /// Upsert user_state row (XP, streak, last_active).
    pub async fn upsert_user_state(
        &self,
        xp: i64,
        streak: i64,
        last_active: &str,
    ) -> Result<(), SupabaseError> {
        let uid = self.require_user_id()?;
        let url = format!("{}/rest/v1/user_state", self.base_url);
        let body = serde_json::json!({
            "id": uid,
            "total_xp": xp,
            "current_streak": streak,
            "last_active": last_active,
        })
        .to_string();

        let headers = self.authed_headers_with(&[
            (
                "Prefer".to_string(),
                "resolution=merge-duplicates".to_string(),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]);

        let resp = self
            .request_with_retry("POST", &url, &headers, Some(body))
            .await?;

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        Ok(())
    }

    /// Fetch user_state for the current user.
    pub async fn fetch_user_state(&self) -> Result<Option<UserStateRow>, SupabaseError> {
        let uid = self.require_user_id()?;
        let url = format!(
            "{}/rest/v1/user_state?id=eq.{}",
            self.base_url, uid
        );

        let headers = self.authed_headers();
        let resp = self
            .request_with_retry("GET", &url, &headers, None)
            .await?;

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        let rows: Vec<UserStateRow> = serde_json::from_str(&resp.body)
            .map_err(|e| SupabaseError::ParseError(e.to_string()))?;

        Ok(rows.into_iter().next())
    }

    /// Upsert language_progress row.
    pub async fn upsert_language_progress(
        &self,
        language_id: &str,
        active_module: i64,
        unlocked_modules: &[i64],
        xp: i64,
    ) -> Result<(), SupabaseError> {
        let uid = self.require_user_id()?;
        let url = format!("{}/rest/v1/language_progress", self.base_url);
        let body = serde_json::json!({
            "user_id": uid,
            "language_id": language_id,
            "active_module": active_module,
            "unlocked_modules": unlocked_modules,
            "xp": xp,
        })
        .to_string();

        let headers = self.authed_headers_with(&[
            (
                "Prefer".to_string(),
                "resolution=merge-duplicates".to_string(),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]);

        let resp = self
            .request_with_retry("POST", &url, &headers, Some(body))
            .await?;

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        Ok(())
    }

    /// Insert a challenge attempt (append-only log).
    pub async fn insert_challenge_attempt(
        &self,
        challenge_id: &str,
        language_id: &str,
        correct: bool,
        attempt_num: i64,
    ) -> Result<(), SupabaseError> {
        let uid = self.require_user_id()?;
        let url = format!("{}/rest/v1/challenge_attempts", self.base_url);
        let body = serde_json::json!({
            "user_id": uid,
            "challenge_id": challenge_id,
            "language_id": language_id,
            "correct": correct,
            "attempt_num": attempt_num,
        })
        .to_string();

        let headers = self.authed_headers_with(&[
            ("Content-Type".to_string(), "application/json".to_string()),
        ]);

        let resp = self
            .request_with_retry("POST", &url, &headers, Some(body))
            .await?;

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        Ok(())
    }

    /// Insert or upsert a streak_log entry for a given day.
    pub async fn insert_streak_log(&self, day: &str) -> Result<(), SupabaseError> {
        let uid = self.require_user_id()?;
        let url = format!("{}/rest/v1/streak_log", self.base_url);
        let body = serde_json::json!({
            "user_id": uid,
            "day": day,
        })
        .to_string();

        let headers = self.authed_headers_with(&[
            (
                "Prefer".to_string(),
                "resolution=merge-duplicates".to_string(),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]);

        let resp = self
            .request_with_retry("POST", &url, &headers, Some(body))
            .await?;

        if resp.status >= 400 {
            return Err(SupabaseError::ApiError {
                status: resp.status,
                message: resp.body,
            });
        }

        Ok(())
    }

    // ── Private helpers ──

    fn require_user_id(&self) -> Result<String, SupabaseError> {
        self.user_id
            .lock()
            .unwrap()
            .clone()
            .ok_or(SupabaseError::NotAuthenticated)
    }

    fn authed_headers(&self) -> Vec<(String, String)> {
        let jwt = self.jwt.lock().unwrap().clone().unwrap_or_default();
        vec![
            ("apikey".to_string(), self.anon_key.clone()),
            (
                "Authorization".to_string(),
                format!("Bearer {}", jwt),
            ),
        ]
    }

    fn authed_headers_with(&self, extra: &[(String, String)]) -> Vec<(String, String)> {
        let mut headers = self.authed_headers();
        headers.extend(extra.iter().cloned());
        headers
    }

    /// Make a request, automatically retrying once on 401 (expired JWT).
    async fn request_with_retry(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
    ) -> Result<HttpResponse, SupabaseError> {
        let resp = self
            .http
            .request(method, url, headers, body.clone())
            .await?;

        if resp.status == 401 {
            // Attempt JWT refresh
            let refresh_result = self.refresh_jwt().await;
            if refresh_result.is_ok() {
                // Rebuild headers with new JWT and retry
                let new_headers = self.rebuild_headers_from(headers);
                return self
                    .http
                    .request(method, url, &new_headers, body)
                    .await;
            }
            // Refresh failed -- return auth error
            return Err(SupabaseError::AuthError(format!(
                "JWT expired and refresh failed: {}",
                resp.body
            )));
        }

        Ok(resp)
    }

    /// Rebuild a header list replacing the Authorization header with the current JWT.
    fn rebuild_headers_from(&self, original: &[(String, String)]) -> Vec<(String, String)> {
        let jwt = self.jwt.lock().unwrap().clone().unwrap_or_default();
        original
            .iter()
            .map(|(k, v)| {
                if k == "Authorization" {
                    (k.clone(), format!("Bearer {}", jwt))
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect()
    }

    /// Extract user ID (sub claim) from a JWT without cryptographic verification.
    /// This is safe because we only use it for local routing; the server validates.
    fn extract_user_id_from_jwt(token: &str) -> Option<String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let payload = parts[1];
        // Add padding if needed for base64
        let padded = match payload.len() % 4 {
            2 => format!("{}==", payload),
            3 => format!("{}=", payload),
            _ => payload.to_string(),
        };
        // base64url -> standard base64
        let standard = padded.replace('-', "+").replace('_', "/");
        let decoded = base64_decode(&standard)?;
        let json_str = String::from_utf8(decoded).ok()?;
        let value: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        value.get("sub").and_then(|v| v.as_str()).map(String::from)
    }
}

/// Minimal base64 decoder (standard alphabet). Avoids adding a base64 crate dependency.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn val(c: u8) -> Option<u8> {
        TABLE.iter().position(|&b| b == c).map(|p| p as u8)
    }

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        let vals: Vec<u8> = chunk.iter().filter_map(|&b| val(b)).collect();
        if vals.len() < 2 {
            return None;
        }
        out.push((vals[0] << 2) | (vals[1] >> 4));
        if vals.len() > 2 {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if vals.len() > 3 {
            out.push((vals[2] << 6) | vals[3]);
        }
    }
    Some(out)
}

// ── Production HttpClient using reqwest ──

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl HttpClient for ReqwestHttpClient {
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
    ) -> Result<HttpResponse, SupabaseError> {
        let mut builder = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "PATCH" => self.client.patch(url),
            "DELETE" => self.client.delete(url),
            _ => {
                return Err(SupabaseError::ApiError {
                    status: 0,
                    message: format!("Unsupported HTTP method: {}", method),
                })
            }
        };

        for (key, value) in headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        if let Some(b) = body {
            builder = builder.body(b);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| SupabaseError::NetworkError(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| SupabaseError::NetworkError(e.to_string()))?;

        Ok(HttpResponse {
            status,
            body: body_text,
        })
    }
}

// ══════════════════════════════════════════════════════════════
// Tests -- RED/GREEN TDD (Ticket 06)
//
// R1: Anonymous auth (7 tests)
// R2: CRUD operations (8 tests)
// R3: Headers & security (4 tests)
// Edge cases (7 tests)
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::platform::MemoryStorage;
    use std::sync::Mutex;

    // ── Mock HTTP Client ──

    #[derive(Debug, Clone)]
    struct RecordedRequest {
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
    }

    struct MockHttpClient {
        responses: Mutex<Vec<HttpResponse>>,
        requests: Mutex<Vec<RecordedRequest>>,
        force_error: Mutex<Option<SupabaseError>>,
    }

    impl MockHttpClient {
        fn new() -> Self {
            Self {
                responses: Mutex::new(Vec::new()),
                requests: Mutex::new(Vec::new()),
                force_error: Mutex::new(None),
            }
        }

        fn enqueue_response(&self, status: u16, body: &str) {
            self.responses.lock().unwrap().push(HttpResponse {
                status,
                body: body.to_string(),
            });
        }

        fn set_network_error(&self, msg: &str) {
            *self.force_error.lock().unwrap() =
                Some(SupabaseError::NetworkError(msg.to_string()));
        }

        fn recorded_requests(&self) -> Vec<RecordedRequest> {
            self.requests.lock().unwrap().clone()
        }

        fn last_request(&self) -> Option<RecordedRequest> {
            self.requests.lock().unwrap().last().cloned()
        }
    }

    impl HttpClient for MockHttpClient {
        async fn request(
            &self,
            method: &str,
            url: &str,
            headers: &[(String, String)],
            body: Option<String>,
        ) -> Result<HttpResponse, SupabaseError> {
            self.requests.lock().unwrap().push(RecordedRequest {
                method: method.to_string(),
                url: url.to_string(),
                headers: headers.to_vec(),
                body: body.clone(),
            });

            if let Some(err) = self.force_error.lock().unwrap().clone() {
                return Err(err);
            }

            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                Ok(HttpResponse {
                    status: 200,
                    body: "[]".to_string(),
                })
            } else {
                Ok(responses.remove(0))
            }
        }
    }

    // ── Test helpers ──

    fn make_auth_response(uid: &str, access: &str, refresh: &str) -> String {
        serde_json::json!({
            "access_token": access,
            "refresh_token": refresh,
            "token_type": "bearer",
            "expires_in": 3600,
            "user": {
                "id": uid,
                "email": null,
                "is_anonymous": true
            }
        })
        .to_string()
    }

    /// Build a fake JWT with the given sub claim (user ID).
    fn fake_jwt(user_id: &str) -> String {
        let header = base64url_encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64url_encode(
            &serde_json::json!({
                "sub": user_id,
                "aud": "authenticated",
                "exp": 9999999999u64
            })
            .to_string(),
        );
        let signature = base64url_encode("fake-signature");
        format!("{}.{}.{}", header, payload, signature)
    }

    fn base64url_encode(input: &str) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let bytes = input.as_bytes();
        let mut result = String::new();

        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;

            result.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
            result.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                result.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
            }
            if chunk.len() > 2 {
                result.push(TABLE[(triple & 0x3F) as usize] as char);
            }
        }

        // base64url: replace + with - and / with _, strip =
        result.replace('+', "-").replace('/', "_")
    }

    fn make_client() -> (
        SupabaseClient<MockHttpClient, MemoryStorage>,
        Arc<MemoryStorage>,
    ) {
        let storage = Arc::new(MemoryStorage::new());
        let http = MockHttpClient::new();
        let client = SupabaseClient::new(
            "https://test.supabase.co",
            "test-anon-key",
            http,
            storage.clone(),
        );
        (client, storage)
    }

    // ════════════════════════════════════════════════
    // R1: Anonymous Auth
    // ════════════════════════════════════════════════

    #[tokio::test]
    async fn r1_1_anon_auth_returns_jwt() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-123");
        client.http.enqueue_response(
            200,
            &make_auth_response("user-123", &jwt, "refresh-abc"),
        );

        let result = client.sign_in_anonymous().await;
        assert!(result.is_ok(), "sign_in_anonymous should return Ok");
        let auth = result.unwrap();
        assert!(!auth.access_token.is_empty(), "JWT should be non-empty");
        assert_eq!(auth.user.id, "user-123");
    }

    #[tokio::test]
    async fn r1_2_anon_auth_stores_jwt() {
        let (client, storage) = make_client();
        let jwt = fake_jwt("user-456");
        client.http.enqueue_response(
            200,
            &make_auth_response("user-456", &jwt, "refresh-def"),
        );

        client.sign_in_anonymous().await.unwrap();

        let stored_jwt = storage.get(JWT_STORAGE_KEY).unwrap();
        assert!(stored_jwt.is_some(), "JWT should be stored after auth");
        assert_eq!(stored_jwt.unwrap(), jwt);

        let stored_refresh = storage.get(REFRESH_TOKEN_STORAGE_KEY).unwrap();
        assert!(stored_refresh.is_some(), "Refresh token should be stored");
        assert_eq!(stored_refresh.unwrap(), "refresh-def");
    }

    #[tokio::test]
    async fn r1_3_rehydrate_jwt_on_launch() {
        let (client, storage) = make_client();
        let jwt = fake_jwt("user-789");

        // Pre-set JWT in storage (simulating app restart)
        storage.set(JWT_STORAGE_KEY, &jwt).unwrap();

        let rehydrated = client.rehydrate_from_storage().unwrap();
        assert!(rehydrated, "Should detect stored JWT");

        // No HTTP request should have been made
        let requests = client.http.recorded_requests();
        assert!(
            requests.is_empty(),
            "No network call when JWT exists in storage"
        );

        assert_eq!(client.current_jwt(), Some(jwt));
        assert_eq!(client.current_user_id(), Some("user-789".to_string()));
    }

    #[tokio::test]
    async fn r1_4_expired_jwt_triggers_refresh() {
        let (client, _) = make_client();

        // Initial auth
        let old_jwt = fake_jwt("user-refresh");
        client.http.enqueue_response(
            200,
            &make_auth_response("user-refresh", &old_jwt, "refresh-old"),
        );
        client.sign_in_anonymous().await.unwrap();

        // CRUD call returns 401, then refresh succeeds, then retry succeeds
        let new_jwt = fake_jwt("user-refresh");
        client
            .http
            .enqueue_response(401, r#"{"message":"JWT expired"}"#);
        client.http.enqueue_response(
            200,
            &make_auth_response("user-refresh", &new_jwt, "refresh-new"),
        );
        client.http.enqueue_response(200, "[]");

        let result = client.fetch_user_state().await;
        assert!(
            result.is_ok(),
            "Should succeed after refresh: {:?}",
            result.err()
        );

        // Verify: auth + fetch(401) + refresh + retry = 4 requests
        let requests = client.http.recorded_requests();
        assert!(
            requests.len() >= 4,
            "Expected >= 4 requests, got {}",
            requests.len()
        );
        assert!(
            requests[2].url.contains("/auth/v1/token"),
            "Third request should be refresh call"
        );
    }

    #[tokio::test]
    async fn r1_5_auth_no_network_returns_error() {
        let (client, _) = make_client();
        client.http.set_network_error("Connection refused");

        let result = client.sign_in_anonymous().await;
        assert!(result.is_err(), "Should return error, not panic");
        match result.unwrap_err() {
            SupabaseError::NetworkError(msg) => {
                assert!(msg.contains("Connection refused"));
            }
            other => panic!("Expected NetworkError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn r1_6_auth_invalid_key_returns_auth_error() {
        let (client, _) = make_client();
        client
            .http
            .enqueue_response(403, r#"{"message":"Invalid API key"}"#);

        let result = client.sign_in_anonymous().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SupabaseError::AuthError(_) => {}
            other => panic!("Expected AuthError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn r1_7_rehydrate_empty_storage_returns_false() {
        let (client, _) = make_client();
        let result = client.rehydrate_from_storage().unwrap();
        assert!(!result, "Should return false when no JWT in storage");
    }

    // ════════════════════════════════════════════════
    // R2: CRUD Operations
    // ════════════════════════════════════════════════

    #[tokio::test]
    async fn r2_1_upsert_user_state() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-1");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-1", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(201, "");
        let result = client.upsert_user_state(20, 1, "2026-03-17").await;
        assert!(result.is_ok(), "Upsert should succeed: {:?}", result.err());

        let req = client.http.last_request().unwrap();
        assert_eq!(req.method, "POST");
        assert!(req.url.contains("/rest/v1/user_state"));
        let body: serde_json::Value = serde_json::from_str(&req.body.unwrap()).unwrap();
        assert_eq!(body["total_xp"], 20);
        assert_eq!(body["current_streak"], 1);
        assert_eq!(body["last_active"], "2026-03-17");
        assert_eq!(body["id"], "user-crud-1");
    }

    #[tokio::test]
    async fn r2_2_fetch_user_state() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-2");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-2", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        let response_body = serde_json::json!([{
            "id": "user-crud-2",
            "total_xp": 100,
            "current_streak": 5,
            "longest_streak": 10,
            "last_active": "2026-03-17"
        }])
        .to_string();
        client.http.enqueue_response(200, &response_body);

        let result = client.fetch_user_state().await;
        assert!(result.is_ok());
        let row = result.unwrap().unwrap();
        assert_eq!(row.total_xp, 100);
        assert_eq!(row.current_streak, 5);
        assert_eq!(row.longest_streak, 10);
        assert_eq!(row.last_active, Some("2026-03-17".to_string()));
    }

    #[tokio::test]
    async fn r2_3_fetch_user_state_empty() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-3");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-3", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(200, "[]");
        let result = client.fetch_user_state().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "Should return None for no rows");
    }

    #[tokio::test]
    async fn r2_4_upsert_language_progress() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-4");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-4", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(201, "");
        let result = client
            .upsert_language_progress("rust", 2, &[1, 2], 120)
            .await;
        assert!(result.is_ok(), "Upsert should succeed: {:?}", result.err());

        let req = client.http.last_request().unwrap();
        assert!(req.url.contains("/rest/v1/language_progress"));
        let body: serde_json::Value = serde_json::from_str(&req.body.unwrap()).unwrap();
        assert_eq!(body["language_id"], "rust");
        assert_eq!(body["active_module"], 2);
        assert_eq!(body["unlocked_modules"], serde_json::json!([1, 2]));
        assert_eq!(body["xp"], 120);
        assert_eq!(body["user_id"], "user-crud-4");
    }

    #[tokio::test]
    async fn r2_5_insert_challenge_attempt() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-5");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-5", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(201, "");
        let result = client
            .insert_challenge_attempt("rust-m1-c1", "rust", true, 1)
            .await;
        assert!(result.is_ok(), "Insert should succeed: {:?}", result.err());

        let req = client.http.last_request().unwrap();
        assert!(req.url.contains("/rest/v1/challenge_attempts"));
        let body: serde_json::Value = serde_json::from_str(&req.body.unwrap()).unwrap();
        assert_eq!(body["challenge_id"], "rust-m1-c1");
        assert_eq!(body["language_id"], "rust");
        assert_eq!(body["correct"], true);
        assert_eq!(body["attempt_num"], 1);
        assert_eq!(body["user_id"], "user-crud-5");
    }

    #[tokio::test]
    async fn r2_6_insert_streak_log_idempotent() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-6");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-6", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        // First insert
        client.http.enqueue_response(201, "");
        let result1 = client.insert_streak_log("2026-03-17").await;
        assert!(result1.is_ok());

        // Second insert (same day) should also succeed
        client.http.enqueue_response(200, "");
        let result2 = client.insert_streak_log("2026-03-17").await;
        assert!(
            result2.is_ok(),
            "Duplicate streak_log insert should not error"
        );

        // Verify merge-duplicates header
        let requests = client.http.recorded_requests();
        for req in requests
            .iter()
            .filter(|r| r.url.contains("/rest/v1/streak_log"))
        {
            let has_prefer = req
                .headers
                .iter()
                .any(|(k, v)| k == "Prefer" && v.contains("resolution=merge-duplicates"));
            assert!(has_prefer, "streak_log should have Prefer: resolution=merge-duplicates");
        }
    }

    #[tokio::test]
    async fn r2_7_crud_without_auth_returns_not_authenticated() {
        let (client, _) = make_client();

        let result = client.upsert_user_state(10, 0, "2026-03-17").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SupabaseError::NotAuthenticated => {}
            other => panic!("Expected NotAuthenticated, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn r2_8_api_error_propagated() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-crud-8");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-crud-8", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client
            .http
            .enqueue_response(500, r#"{"message":"Internal Server Error"}"#);
        let result = client.fetch_user_state().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SupabaseError::ApiError { status, .. } => assert_eq!(status, 500),
            other => panic!("Expected ApiError, got {:?}", other),
        }
    }

    // ════════════════════════════════════════════════
    // R3: Headers & Security
    // ════════════════════════════════════════════════

    #[tokio::test]
    async fn r3_1_every_request_has_apikey_header() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-hdr-1");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-hdr-1", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(200, "[]");
        client.fetch_user_state().await.unwrap();

        for req in &client.http.recorded_requests() {
            let has_apikey = req
                .headers
                .iter()
                .any(|(k, v)| k == "apikey" && v == "test-anon-key");
            assert!(
                has_apikey,
                "Request to {} must have apikey header, got {:?}",
                req.url, req.headers
            );
        }
    }

    #[tokio::test]
    async fn r3_2_every_crud_request_has_bearer_token() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-hdr-2");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-hdr-2", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(200, "[]");
        client.fetch_user_state().await.unwrap();

        client.http.enqueue_response(201, "");
        client.upsert_user_state(10, 0, "2026-03-17").await.unwrap();

        client.http.enqueue_response(201, "");
        client
            .insert_challenge_attempt("rust-m1-c1", "rust", true, 1)
            .await
            .unwrap();

        // Skip auth signup request (index 0)
        for req in client.http.recorded_requests().iter().skip(1) {
            let has_bearer = req
                .headers
                .iter()
                .any(|(k, v)| k == "Authorization" && v.starts_with("Bearer "));
            assert!(
                has_bearer,
                "CRUD request to {} must have Authorization: Bearer, got {:?}",
                req.url, req.headers
            );
        }
    }

    #[tokio::test]
    async fn r3_3_jwt_not_in_display_output() {
        let jwt = fake_jwt("secret-user");
        let error = SupabaseError::AuthError("some auth problem".to_string());
        let display = format!("{}", error);
        assert!(!display.contains(&jwt), "Error display must not contain JWT");

        let error2 = SupabaseError::ApiError {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        let debug = format!("{:?}", error2);
        assert!(!debug.contains(&jwt), "Debug output must not contain JWT");
    }

    #[tokio::test]
    async fn r3_4_auth_request_has_apikey_and_content_type() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-hdr-4");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-hdr-4", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        let req = &client.http.recorded_requests()[0];
        assert!(req.url.contains("/auth/v1/signup"));
        assert!(
            req.headers.iter().any(|(k, _)| k == "apikey"),
            "Auth request must have apikey"
        );
        assert!(
            req.headers
                .iter()
                .any(|(k, v)| k == "Content-Type" && v == "application/json"),
            "Auth request must have Content-Type: application/json"
        );
    }

    // ════════════════════════════════════════════════
    // Edge Cases
    // ════════════════════════════════════════════════

    #[tokio::test]
    async fn edge_1_user_id_extracted_from_jwt() {
        let jwt = fake_jwt("extracted-uid-123");
        let uid =
            SupabaseClient::<MockHttpClient, MemoryStorage>::extract_user_id_from_jwt(&jwt);
        assert_eq!(uid, Some("extracted-uid-123".to_string()));
    }

    #[tokio::test]
    async fn edge_2_invalid_jwt_returns_none_uid() {
        let uid =
            SupabaseClient::<MockHttpClient, MemoryStorage>::extract_user_id_from_jwt(
                "not-a-jwt",
            );
        assert!(uid.is_none());
    }

    #[tokio::test]
    async fn edge_3_rehydrate_sets_user_id() {
        let (client, storage) = make_client();
        let jwt = fake_jwt("rehydrate-uid");
        storage.set(JWT_STORAGE_KEY, &jwt).unwrap();

        client.rehydrate_from_storage().unwrap();
        assert_eq!(
            client.current_user_id(),
            Some("rehydrate-uid".to_string())
        );
    }

    #[tokio::test]
    async fn edge_4_upsert_has_merge_duplicates() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-prefer");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-prefer", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(201, "");
        client.upsert_user_state(10, 1, "2026-03-17").await.unwrap();

        let req = client.http.last_request().unwrap();
        let has_prefer = req
            .headers
            .iter()
            .any(|(k, v)| k == "Prefer" && v.contains("resolution=merge-duplicates"));
        assert!(has_prefer, "upsert should use Prefer: resolution=merge-duplicates");
    }

    #[tokio::test]
    async fn edge_5_language_progress_has_merge_duplicates() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-lp");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-lp", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(201, "");
        client
            .upsert_language_progress("rust", 1, &[1], 0)
            .await
            .unwrap();

        let req = client.http.last_request().unwrap();
        let has_prefer = req
            .headers
            .iter()
            .any(|(k, v)| k == "Prefer" && v.contains("resolution=merge-duplicates"));
        assert!(has_prefer, "language_progress upsert should use merge-duplicates");
    }

    #[tokio::test]
    async fn edge_6_network_error_on_crud_does_not_panic() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-net");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-net", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.set_network_error("timeout");
        let result = client.fetch_user_state().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SupabaseError::NetworkError(msg) => assert!(msg.contains("timeout")),
            other => panic!("Expected NetworkError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn edge_7_fetch_url_includes_user_id_filter() {
        let (client, _) = make_client();
        let jwt = fake_jwt("user-filter");
        client
            .http
            .enqueue_response(200, &make_auth_response("user-filter", &jwt, "rt"));
        client.sign_in_anonymous().await.unwrap();

        client.http.enqueue_response(200, "[]");
        client.fetch_user_state().await.unwrap();

        let req = client.http.last_request().unwrap();
        assert!(
            req.url.contains("id=eq.user-filter"),
            "Fetch URL should filter by user ID: {}",
            req.url
        );
    }
}
