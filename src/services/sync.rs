// ======================================================================
// Sync Service — Bridges AppState mutations with Supabase (TAP-30)
//
// Thin layer that fires Supabase CRUD calls after local state changes.
// All methods are fire-and-forget: they log errors but never block the UI.
//
// All code below was developed via strict RED/GREEN TDD:
//   1. RED  -- failing test written first
//   2. GREEN -- minimum production code to pass
//   3. REFACTOR -- clean up, keep all tests green
// ======================================================================

use crate::services::platform::{MemoryStorage, SecureStorage};
use crate::services::supabase::{HttpClient, ReqwestHttpClient, SupabaseClient, SupabaseError};
use crate::state::AppState;
use std::sync::Arc;

/// Concrete SyncService type used in production (reqwest + memory storage).
pub type ProdSyncService = SyncService<ReqwestHttpClient, MemoryStorage>;

/// Sync service that persists AppState changes to Supabase.
/// All methods are fire-and-forget -- they log errors but never block the UI.
///
/// Generic over H (HTTP transport) and S (secure storage) so tests can inject
/// mocks without touching the network.
pub struct SyncService<H: HttpClient, S: SecureStorage> {
    client: Arc<SupabaseClient<H, S>>,
    is_authenticated: bool,
}

impl<H: HttpClient, S: SecureStorage> SyncService<H, S> {
    /// Create a new SyncService wrapping the given SupabaseClient.
    pub fn new(client: Arc<SupabaseClient<H, S>>) -> Self {
        Self {
            client,
            is_authenticated: false,
        }
    }

    /// Initialize: try rehydrate from storage, then anon auth if needed.
    /// Returns Ok(true) if authenticated, Ok(false) if offline/failed.
    pub async fn init(&mut self) -> Result<bool, SupabaseError> {
        // Try rehydrate from stored JWT
        if self.client.rehydrate_from_storage()? {
            self.is_authenticated = true;
            return Ok(true);
        }

        // Try anonymous sign-in
        match self.client.sign_in_anonymous().await {
            Ok(_) => {
                self.is_authenticated = true;
                // Create initial user row
                let today = today_string();
                let _ = self.client.upsert_user_state(0, 0, &today).await;
                Ok(true)
            }
            Err(e) => {
                log::warn!("Auth failed, continuing offline: {}", e);
                Ok(false)
            }
        }
    }

    /// Whether the service has a valid authenticated session.
    pub fn is_authenticated(&self) -> bool {
        self.is_authenticated
    }

    /// Sync after a challenge attempt (correct or wrong).
    /// Inserts the attempt log, upserts language progress, and upserts user state.
    pub async fn sync_challenge_complete(
        &self,
        challenge_id: &str,
        language_id: &str,
        correct: bool,
        attempt_num: u32,
        state: &AppState,
    ) {
        if !self.is_authenticated {
            return;
        }

        // Insert attempt
        if let Err(e) = self
            .client
            .insert_challenge_attempt(challenge_id, language_id, correct, attempt_num as i64)
            .await
        {
            log::warn!("Failed to sync attempt: {}", e);
        }

        // Upsert language progress
        let unlocked: Vec<i64> = state
            .progress
            .unlocked_modules
            .iter()
            .map(|&m| m as i64)
            .collect();
        if let Err(e) = self
            .client
            .upsert_language_progress(
                language_id,
                state.progress.active_module as i64,
                &unlocked,
                state.progress.xp as i64,
            )
            .await
        {
            log::warn!("Failed to sync progress: {}", e);
        }

        // Upsert user state
        let today = today_string();
        if let Err(e) = self
            .client
            .upsert_user_state(
                state.user.total_xp as i64,
                state.user.current_streak as i64,
                &today,
            )
            .await
        {
            log::warn!("Failed to sync user state: {}", e);
        }
    }

    /// Sync streak log entry for today and update user state.
    pub async fn sync_streak(&self, state: &AppState) {
        if !self.is_authenticated {
            return;
        }

        let today = today_string();
        if let Err(e) = self.client.insert_streak_log(&today).await {
            log::warn!("Failed to sync streak: {}", e);
        }
        if let Err(e) = self
            .client
            .upsert_user_state(
                state.user.total_xp as i64,
                state.user.current_streak as i64,
                &today,
            )
            .await
        {
            log::warn!("Failed to sync user state: {}", e);
        }
    }

    /// Sync after purchase (unlocked modules).
    pub async fn sync_purchase(&self, state: &AppState) {
        if !self.is_authenticated {
            return;
        }

        let unlocked: Vec<i64> = state
            .progress
            .unlocked_modules
            .iter()
            .map(|&m| m as i64)
            .collect();
        if let Err(e) = self
            .client
            .upsert_language_progress(
                &state.progress.language_id,
                state.progress.active_module as i64,
                &unlocked,
                state.progress.xp as i64,
            )
            .await
        {
            log::warn!("Failed to sync purchase: {}", e);
        }
    }

    /// Fetch server state and merge into local state (max-value wins).
    pub async fn fetch_and_merge(&self, state: &mut AppState) {
        if !self.is_authenticated {
            return;
        }

        if let Ok(Some(row)) = self.client.fetch_user_state().await {
            if row.total_xp as u32 > state.user.total_xp {
                state.user.total_xp = row.total_xp as u32;
            }
            if row.current_streak as u32 > state.user.current_streak {
                state.user.current_streak = row.current_streak as u32;
            }
            if row.longest_streak as u32 > state.user.longest_streak {
                state.user.longest_streak = row.longest_streak as u32;
            }
        }
    }
}

/// Get today's date as YYYY-MM-DD string.
fn today_string() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

// ======================================================================
// Tests -- 11 RED/GREEN TDD scenarios (TAP-30)
//
// T1: Init (3 tests) -- rehydrate, anon auth, offline fallback
// T2: Sync operations (4 tests) -- challenge, streak, purchase, unauthenticated
// T3: Error resilience (2 tests) -- failures don't panic
// T4: Fetch & merge (2 tests) -- server wins / local wins
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::platform::MemoryStorage;
    use crate::services::supabase::{HttpResponse, SupabaseError};
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

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
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

        result.replace('+', "-").replace('/', "_")
    }

    fn make_client_and_storage() -> (
        Arc<SupabaseClient<MockHttpClient, MemoryStorage>>,
        Arc<MemoryStorage>,
    ) {
        let storage = Arc::new(MemoryStorage::new());
        let http = MockHttpClient::new();
        let client = Arc::new(SupabaseClient::new(
            "https://test.supabase.co",
            "test-anon-key",
            http,
            storage.clone(),
        ));
        (client, storage)
    }

    fn make_app_state() -> AppState {
        AppState::new()
    }

    // ════════════════════════════════════════════
    // T1: Init (3 tests)
    // ════════════════════════════════════════════

    #[tokio::test]
    async fn t1_init_no_jwt_calls_anonymous_signup() {
        let (client, _storage) = make_client_and_storage();

        // Enqueue: auth response, then upsert_user_state response
        let jwt = fake_jwt("new-user");
        client
            .http
            .enqueue_response(200, &make_auth_response("new-user", &jwt, "refresh-1"));
        client.http.enqueue_response(201, ""); // upsert_user_state

        let mut sync = SyncService::new(client.clone());
        let result = sync.init().await;

        assert!(result.is_ok());
        assert!(result.unwrap(), "Should return true (authenticated)");
        assert!(sync.is_authenticated());

        // Verify: sign_in_anonymous was called (POST to /auth/v1/signup)
        let requests = client.http.recorded_requests();
        assert!(
            requests.iter().any(|r| r.url.contains("/auth/v1/signup")),
            "Should call sign_in_anonymous when no JWT in storage"
        );
    }

    #[tokio::test]
    async fn t2_init_with_jwt_skips_network() {
        let (client, storage) = make_client_and_storage();

        // Pre-set JWT in storage (simulating app restart with stored session)
        let jwt = fake_jwt("existing-user");
        storage.set("supabase_jwt", &jwt).unwrap();

        let mut sync = SyncService::new(client.clone());
        let result = sync.init().await;

        assert!(result.is_ok());
        assert!(result.unwrap(), "Should return true (rehydrated)");
        assert!(sync.is_authenticated());

        // No HTTP requests should have been made
        assert_eq!(
            client.http.request_count(),
            0,
            "No network calls when JWT exists in storage"
        );
    }

    #[tokio::test]
    async fn t3_init_network_failure_returns_false() {
        let (client, _storage) = make_client_and_storage();

        // Force network error on auth attempt
        client.http.set_network_error("Connection refused");

        let mut sync = SyncService::new(client.clone());
        let result = sync.init().await;

        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "Should return false when network fails"
        );
        assert!(
            !sync.is_authenticated(),
            "Should not be authenticated after network failure"
        );
    }

    // ════════════════════════════════════════════
    // T2: Sync operations (4 tests)
    // ════════════════════════════════════════════

    #[tokio::test]
    async fn t4_sync_challenge_inserts_attempt_and_upserts() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate first
        let jwt = fake_jwt("sync-user-1");
        client
            .http
            .enqueue_response(200, &make_auth_response("sync-user-1", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert_user_state

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        // Clear request log so we only see sync calls
        client.http.requests.lock().unwrap().clear();

        // Enqueue responses for 3 CRUD calls
        client.http.enqueue_response(201, ""); // insert_challenge_attempt
        client.http.enqueue_response(201, ""); // upsert_language_progress
        client.http.enqueue_response(201, ""); // upsert_user_state

        let mut state = make_app_state();
        state.user.total_xp = 40;
        state.progress.xp = 40;
        state.user.current_streak = 1;

        sync.sync_challenge_complete("rust-m1-c1", "rust", true, 1, &state)
            .await;

        let requests = client.http.recorded_requests();
        assert_eq!(requests.len(), 3, "Should make 3 API calls");

        // 1. insert_challenge_attempt
        assert!(
            requests[0].url.contains("/rest/v1/challenge_attempts"),
            "First call should be challenge_attempts insert"
        );
        let body0: serde_json::Value =
            serde_json::from_str(&requests[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body0["challenge_id"], "rust-m1-c1");
        assert_eq!(body0["language_id"], "rust");
        assert_eq!(body0["correct"], true);
        assert_eq!(body0["attempt_num"], 1);

        // 2. upsert_language_progress
        assert!(
            requests[1].url.contains("/rest/v1/language_progress"),
            "Second call should be language_progress upsert"
        );
        let body1: serde_json::Value =
            serde_json::from_str(&requests[1].body.as_ref().unwrap()).unwrap();
        assert_eq!(body1["language_id"], "rust");
        assert_eq!(body1["xp"], 40);

        // 3. upsert_user_state
        assert!(
            requests[2].url.contains("/rest/v1/user_state"),
            "Third call should be user_state upsert"
        );
        let body2: serde_json::Value =
            serde_json::from_str(&requests[2].body.as_ref().unwrap()).unwrap();
        assert_eq!(body2["total_xp"], 40);
        assert_eq!(body2["current_streak"], 1);
    }

    #[tokio::test]
    async fn t5_sync_streak_inserts_log_and_upserts_user() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate
        let jwt = fake_jwt("streak-user");
        client
            .http
            .enqueue_response(200, &make_auth_response("streak-user", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        client.http.requests.lock().unwrap().clear();

        // Enqueue responses for streak sync
        client.http.enqueue_response(201, ""); // insert_streak_log
        client.http.enqueue_response(201, ""); // upsert_user_state

        let mut state = make_app_state();
        state.user.total_xp = 60;
        state.user.current_streak = 3;

        sync.sync_streak(&state).await;

        let requests = client.http.recorded_requests();
        assert_eq!(requests.len(), 2, "Should make 2 API calls for streak sync");

        assert!(
            requests[0].url.contains("/rest/v1/streak_log"),
            "First call should be streak_log insert"
        );
        assert!(
            requests[1].url.contains("/rest/v1/user_state"),
            "Second call should be user_state upsert"
        );
    }

    #[tokio::test]
    async fn t6_sync_purchase_upserts_progress() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate
        let jwt = fake_jwt("purchase-user");
        client
            .http
            .enqueue_response(200, &make_auth_response("purchase-user", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        client.http.requests.lock().unwrap().clear();

        // Enqueue response for purchase sync
        client.http.enqueue_response(201, ""); // upsert_language_progress

        let mut state = make_app_state();
        state.unlock_all_modules();

        sync.sync_purchase(&state).await;

        let requests = client.http.recorded_requests();
        assert_eq!(requests.len(), 1, "Should make 1 API call for purchase sync");
        assert!(
            requests[0].url.contains("/rest/v1/language_progress"),
            "Should upsert language_progress after purchase"
        );

        let body: serde_json::Value =
            serde_json::from_str(&requests[0].body.as_ref().unwrap()).unwrap();
        let unlocked = body["unlocked_modules"].as_array().unwrap();
        assert!(
            unlocked.len() > 3,
            "Should have all modules unlocked, got {:?}",
            unlocked
        );
    }

    #[tokio::test]
    async fn t7_sync_when_not_authenticated_does_nothing() {
        let (client, _storage) = make_client_and_storage();

        // Do NOT call init -- service is unauthenticated
        let sync = SyncService::new(client.clone());
        assert!(!sync.is_authenticated());

        let state = make_app_state();
        sync.sync_challenge_complete("rust-m1-c1", "rust", true, 1, &state)
            .await;
        sync.sync_streak(&state).await;
        sync.sync_purchase(&state).await;

        assert_eq!(
            client.http.request_count(),
            0,
            "No HTTP calls when not authenticated"
        );
    }

    // ════════════════════════════════════════════
    // T3: Error resilience (2 tests)
    // ════════════════════════════════════════════

    #[tokio::test]
    async fn t8_sync_failure_logs_but_does_not_panic() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate
        let jwt = fake_jwt("error-user");
        client
            .http
            .enqueue_response(200, &make_auth_response("error-user", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        // Now force all subsequent calls to fail
        client.http.set_network_error("server down");

        let state = make_app_state();

        // None of these should panic
        sync.sync_challenge_complete("rust-m1-c1", "rust", true, 1, &state)
            .await;
        sync.sync_streak(&state).await;
        sync.sync_purchase(&state).await;

        // If we got here without panicking, the test passes
        assert!(sync.is_authenticated(), "Still authenticated despite errors");
    }

    #[tokio::test]
    async fn t9_sync_partial_failure_continues() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate
        let jwt = fake_jwt("partial-user");
        client
            .http
            .enqueue_response(200, &make_auth_response("partial-user", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        client.http.requests.lock().unwrap().clear();

        // First call (challenge attempt) returns 500, rest succeed
        client
            .http
            .enqueue_response(500, r#"{"message":"Internal Server Error"}"#);
        client.http.enqueue_response(201, ""); // language_progress
        client.http.enqueue_response(201, ""); // user_state

        let state = make_app_state();
        sync.sync_challenge_complete("rust-m1-c1", "rust", true, 1, &state)
            .await;

        // All 3 calls should have been attempted despite first failure
        let requests = client.http.recorded_requests();
        assert_eq!(
            requests.len(),
            3,
            "All 3 calls should be attempted even if first fails"
        );
    }

    // ════════════════════════════════════════════
    // T4: Fetch & merge (2 tests)
    // ════════════════════════════════════════════

    #[tokio::test]
    async fn t10_fetch_merge_server_higher_wins() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate
        let jwt = fake_jwt("merge-user-1");
        client
            .http
            .enqueue_response(200, &make_auth_response("merge-user-1", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        // Enqueue server state response (higher values)
        let server_state = serde_json::json!([{
            "id": "merge-user-1",
            "total_xp": 200,
            "current_streak": 10,
            "longest_streak": 15,
            "last_active": "2026-03-17"
        }])
        .to_string();
        client.http.enqueue_response(200, &server_state);

        let mut state = make_app_state();
        state.user.total_xp = 100;
        state.user.current_streak = 5;
        state.user.longest_streak = 8;

        sync.fetch_and_merge(&mut state).await;

        assert_eq!(state.user.total_xp, 200, "Server XP (200) should win over local (100)");
        assert_eq!(
            state.user.current_streak, 10,
            "Server streak (10) should win over local (5)"
        );
        assert_eq!(
            state.user.longest_streak, 15,
            "Server longest (15) should win over local (8)"
        );
    }

    #[tokio::test]
    async fn t11_fetch_merge_local_higher_keeps() {
        let (client, _storage) = make_client_and_storage();

        // Authenticate
        let jwt = fake_jwt("merge-user-2");
        client
            .http
            .enqueue_response(200, &make_auth_response("merge-user-2", &jwt, "rt"));
        client.http.enqueue_response(201, ""); // init upsert

        let mut sync = SyncService::new(client.clone());
        sync.init().await.unwrap();

        // Enqueue server state response (lower values)
        let server_state = serde_json::json!([{
            "id": "merge-user-2",
            "total_xp": 50,
            "current_streak": 2,
            "longest_streak": 3,
            "last_active": "2026-03-16"
        }])
        .to_string();
        client.http.enqueue_response(200, &server_state);

        let mut state = make_app_state();
        state.user.total_xp = 200;
        state.user.current_streak = 7;
        state.user.longest_streak = 12;

        sync.fetch_and_merge(&mut state).await;

        assert_eq!(state.user.total_xp, 200, "Local XP (200) should be kept over server (50)");
        assert_eq!(
            state.user.current_streak, 7,
            "Local streak (7) should be kept over server (2)"
        );
        assert_eq!(
            state.user.longest_streak, 12,
            "Local longest (12) should be kept over server (3)"
        );
    }
}
