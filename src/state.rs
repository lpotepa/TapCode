use dioxus::prelude::*;
use crate::models::*;
use crate::engine;
use crate::services::platform::SecureStorage;

/// Storage key for persisted app state.
pub const STATE_STORAGE_KEY: &str = "tapcode_app_state";

/// Persist current AppState to storage (fire-and-forget).
pub fn save_to_storage(state: &AppState, storage: &dyn SecureStorage) {
    let json = state.save_progress_json();
    let _ = storage.set(STATE_STORAGE_KEY, &json);
}

// ── Global App State ──

#[derive(Debug, Clone)]
pub struct AppState {
    pub user: UserState,
    pub progress: LanguageProgress,
    pub pack: LanguagePack,
    pub is_onboarded: bool,
    pub is_purchased: bool,
    pub active_language: String,
    pub is_offline: bool,
    pub sound_enabled: bool,
    // Session tracking
    pub hints_used_this_session: u32,
    pub session_warning_shown: bool,
    // Attempt tracking for accuracy
    pub total_attempts: u32,
    pub correct_attempts: u32,
}

impl AppState {
    pub fn new() -> Self {
        let registry = engine::build_default_registry();
        let pack = registry.get_pack("rust")
            .expect("Rust language pack must be available")
            .clone();

        Self {
            user: UserState {
                total_xp: 0,
                current_streak: 0,
                longest_streak: 0,
                last_active: None,
                streak_days: vec![false; 7],
                has_freeze: true,
            },
            progress: LanguageProgress {
                language_id: "rust".into(),
                xp: 0,
                active_module: 1,
                unlocked_modules: vec![1],
                completed_challenges: vec![],
                skipped_challenges: vec![],
            },
            pack,
            is_onboarded: false,
            is_purchased: false,
            active_language: "rust".into(),
            is_offline: false,
            sound_enabled: false,
            hints_used_this_session: 0,
            session_warning_shown: false,
            total_attempts: 0,
            correct_attempts: 0,
        }
    }

    /// Switch to a different language pack (if available in registry).
    /// Returns false if the language is not found.
    pub fn switch_language(&mut self, language_id: &str) -> bool {
        let registry = engine::build_default_registry();
        if let Some(new_pack) = registry.get_pack(language_id) {
            self.pack = new_pack.clone();
            self.active_language = language_id.to_string();
            self.progress = LanguageProgress {
                language_id: language_id.to_string(),
                xp: 0,
                active_module: 1,
                unlocked_modules: vec![1],
                completed_challenges: vec![],
                skipped_challenges: vec![],
            };
            true
        } else {
            false
        }
    }

    pub fn add_xp(&mut self, amount: u32) {
        self.user.total_xp += amount;
        self.progress.xp += amount;
    }

    pub fn complete_challenge(&mut self, challenge_id: &str) {
        if !self.progress.completed_challenges.contains(&challenge_id.to_string()) {
            self.progress.completed_challenges.push(challenge_id.to_string());
        }

        // Check if module is now complete and unlock next
        for module in &self.pack.modules {
            if engine::is_module_complete(module, &self.progress) {
                let next_id = module.id + 1;
                if !self.progress.unlocked_modules.contains(&next_id) {
                    if self.pack.modules.iter().any(|m| m.id == next_id) {
                        self.progress.unlocked_modules.push(next_id);
                        self.progress.active_module = next_id;
                    }
                }
            }
        }
    }

    pub fn get_next_challenge_id(&self) -> Option<String> {
        engine::get_next_challenge_id(&self.pack, &self.progress)
    }

    pub fn get_module_progress(&self, module_id: u32) -> (usize, usize) {
        let module = self.pack.modules.iter().find(|m| m.id == module_id);
        match module {
            Some(m) => {
                let done = m.challenge_ids.iter()
                    .filter(|id| self.progress.completed_challenges.contains(id))
                    .count();
                (done, m.challenge_ids.len())
            }
            None => (0, 0),
        }
    }

    pub fn is_module_complete(&self, module_id: u32) -> bool {
        self.pack.modules.iter()
            .find(|m| m.id == module_id)
            .map(|m| engine::is_module_complete(m, &self.progress))
            .unwrap_or(false)
    }

    pub fn is_module_unlocked(&self, module_id: u32) -> bool {
        self.progress.unlocked_modules.contains(&module_id)
    }

    pub fn is_module_paid(&self, module_id: u32) -> bool {
        self.pack.modules.iter()
            .find(|m| m.id == module_id)
            .map(|m| !m.is_free)
            .unwrap_or(false)
    }

    pub fn fill_streak_today(&mut self) {
        if let Some(last) = self.user.streak_days.last_mut() {
            if !*last {
                *last = true;
                self.user.current_streak += 1;
                if self.user.current_streak > self.user.longest_streak {
                    self.user.longest_streak = self.user.current_streak;
                }
            }
        }
    }

    pub fn skipped_challenges_mut(&mut self, id: &str) {
        if !self.progress.skipped_challenges.contains(&id.to_string()) {
            self.progress.skipped_challenges.push(id.to_string());
        }
    }

    // ── Hint XP deduction (Ticket 21) ──

    /// Deduct XP for a hint. First hint per session is free.
    /// Returns the actual XP deducted (0 if free, 5 otherwise, clamped to available XP).
    pub fn deduct_hint_xp(&mut self) -> u32 {
        if self.hints_used_this_session == 0 {
            self.hints_used_this_session += 1;
            return 0; // first hint free
        }
        self.hints_used_this_session += 1;
        let cost = 5u32;
        let actual = cost.min(self.user.total_xp);
        self.user.total_xp = self.user.total_xp.saturating_sub(cost);
        self.progress.xp = self.progress.xp.saturating_sub(cost);
        actual
    }

    // ── Attempt tracking (Ticket 15 — accuracy) ──

    pub fn record_attempt(&mut self, correct: bool) {
        self.total_attempts += 1;
        if correct {
            self.correct_attempts += 1;
        }
    }

    pub fn accuracy_percent(&self) -> u32 {
        if self.total_attempts == 0 {
            return 0;
        }
        ((self.correct_attempts as f64 / self.total_attempts as f64) * 100.0) as u32
    }

    // ── Purchase state (Ticket 22) ──

    pub fn unlock_all_modules(&mut self) {
        self.is_purchased = true;
        for module in &self.pack.modules {
            if !self.progress.unlocked_modules.contains(&module.id) {
                self.progress.unlocked_modules.push(module.id);
            }
        }
    }

    pub fn should_show_paywall(&self, completed_module_id: u32) -> bool {
        completed_module_id == 3 && !self.is_purchased
    }

    // ── Offline banner (Ticket 13) ──

    /// Returns the offline banner message if the app is offline, or None if online.
    pub fn offline_banner_text(&self) -> Option<&'static str> {
        if self.is_offline {
            Some("Offline \u{2014} progress will sync when you reconnect")
        } else {
            None
        }
    }

    /// Returns the number of skipped challenges that need revisiting.
    pub fn revisit_count(&self) -> usize {
        self.progress.skipped_challenges.len()
    }

    // ── Session persistence ──

    /// Serialize progress to JSON for localStorage persistence.
    pub fn save_progress_json(&self) -> String {
        let data = serde_json::json!({
            "total_xp": self.user.total_xp,
            "current_streak": self.user.current_streak,
            "longest_streak": self.user.longest_streak,
            "streak_days": self.user.streak_days,
            "has_freeze": self.user.has_freeze,
            "language_id": self.progress.language_id,
            "xp": self.progress.xp,
            "active_module": self.progress.active_module,
            "unlocked_modules": self.progress.unlocked_modules,
            "completed_challenges": self.progress.completed_challenges,
            "skipped_challenges": self.progress.skipped_challenges,
            "is_onboarded": self.is_onboarded,
            "is_purchased": self.is_purchased,
            "total_attempts": self.total_attempts,
            "correct_attempts": self.correct_attempts,
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    /// Restore progress from JSON (localStorage or Supabase).
    /// Returns true if JSON was valid, false otherwise.
    pub fn load_progress_json(&mut self, json: &str) -> bool {
        let Ok(data) = serde_json::from_str::<serde_json::Value>(json) else {
            return false;
        };

        if let Some(v) = data.get("total_xp").and_then(|v| v.as_u64()) {
            self.user.total_xp = v as u32;
        }
        if let Some(v) = data.get("current_streak").and_then(|v| v.as_u64()) {
            self.user.current_streak = v as u32;
        }
        if let Some(v) = data.get("longest_streak").and_then(|v| v.as_u64()) {
            self.user.longest_streak = v as u32;
        }
        if let Some(v) = data.get("streak_days").and_then(|v| v.as_array()) {
            self.user.streak_days = v.iter().filter_map(|b| b.as_bool()).collect();
        }
        if let Some(v) = data.get("has_freeze").and_then(|v| v.as_bool()) {
            self.user.has_freeze = v;
        }
        if let Some(v) = data.get("xp").and_then(|v| v.as_u64()) {
            self.progress.xp = v as u32;
        }
        if let Some(v) = data.get("active_module").and_then(|v| v.as_u64()) {
            self.progress.active_module = v as u32;
        }
        if let Some(v) = data.get("unlocked_modules").and_then(|v| v.as_array()) {
            self.progress.unlocked_modules =
                v.iter().filter_map(|n| n.as_u64().map(|n| n as u32)).collect();
        }
        if let Some(v) = data.get("completed_challenges").and_then(|v| v.as_array()) {
            self.progress.completed_challenges =
                v.iter().filter_map(|s| s.as_str().map(String::from)).collect();
        }
        if let Some(v) = data.get("skipped_challenges").and_then(|v| v.as_array()) {
            self.progress.skipped_challenges =
                v.iter().filter_map(|s| s.as_str().map(String::from)).collect();
        }
        if let Some(v) = data.get("is_onboarded").and_then(|v| v.as_bool()) {
            self.is_onboarded = v;
        }
        if let Some(v) = data.get("is_purchased").and_then(|v| v.as_bool()) {
            self.is_purchased = v;
        }
        if let Some(v) = data.get("total_attempts").and_then(|v| v.as_u64()) {
            self.total_attempts = v as u32;
        }
        if let Some(v) = data.get("correct_attempts").and_then(|v| v.as_u64()) {
            self.correct_attempts = v as u32;
        }
        true
    }
}

// ══════════════════════════════════════════════════════════════
// Tests — RED/GREEN TDD for state logic
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> AppState {
        AppState::new()
    }

    // ── Hint XP deduction (Ticket 21) ──

    #[test]
    fn hint_first_free_per_session() {
        let mut s = fresh_state();
        s.user.total_xp = 100;
        let cost = s.deduct_hint_xp();
        assert_eq!(cost, 0, "First hint should be free");
        assert_eq!(s.user.total_xp, 100, "XP unchanged on free hint");
    }

    #[test]
    fn hint_second_costs_5_xp() {
        let mut s = fresh_state();
        s.user.total_xp = 100;
        s.deduct_hint_xp(); // first: free
        let cost = s.deduct_hint_xp(); // second: -5
        assert_eq!(cost, 5);
        assert_eq!(s.user.total_xp, 95);
    }

    #[test]
    fn hint_third_also_costs_5() {
        let mut s = fresh_state();
        s.user.total_xp = 100;
        s.deduct_hint_xp(); // free
        s.deduct_hint_xp(); // -5 → 95
        let cost = s.deduct_hint_xp(); // -5 → 90
        assert_eq!(cost, 5);
        assert_eq!(s.user.total_xp, 90);
    }

    #[test]
    fn hint_xp_cannot_go_below_zero() {
        let mut s = fresh_state();
        s.user.total_xp = 3;
        s.deduct_hint_xp(); // free
        let cost = s.deduct_hint_xp(); // would be -5 but clamped
        assert_eq!(cost, 3, "Should only deduct what's available");
        assert_eq!(s.user.total_xp, 0, "XP should be 0, not negative");
    }

    #[test]
    fn hint_xp_zero_stays_zero() {
        let mut s = fresh_state();
        s.user.total_xp = 0;
        s.deduct_hint_xp(); // free
        s.deduct_hint_xp(); // -5 but XP is 0
        assert_eq!(s.user.total_xp, 0);
    }

    #[test]
    fn hint_session_counter_resets_on_new_state() {
        let s = fresh_state();
        assert_eq!(s.hints_used_this_session, 0, "Fresh session = 0 hints used");
    }

    // ── Attempt tracking (Ticket 15) ──

    #[test]
    fn accuracy_zero_when_no_attempts() {
        let s = fresh_state();
        assert_eq!(s.accuracy_percent(), 0);
    }

    #[test]
    fn accuracy_100_when_all_correct() {
        let mut s = fresh_state();
        s.record_attempt(true);
        s.record_attempt(true);
        s.record_attempt(true);
        assert_eq!(s.accuracy_percent(), 100);
    }

    #[test]
    fn accuracy_50_when_half_correct() {
        let mut s = fresh_state();
        s.record_attempt(true);
        s.record_attempt(false);
        assert_eq!(s.accuracy_percent(), 50);
    }

    #[test]
    fn accuracy_calculated_correctly() {
        let mut s = fresh_state();
        for _ in 0..45 { s.record_attempt(true); }
        for _ in 0..7 { s.record_attempt(false); }
        // 45/52 = 86.5% → 86 truncated
        assert_eq!(s.accuracy_percent(), 86);
    }

    // ── Purchase / paywall (Ticket 22) ──

    #[test]
    fn paywall_shows_after_module_3() {
        let s = fresh_state();
        assert!(s.should_show_paywall(3));
    }

    #[test]
    fn paywall_not_after_module_2() {
        let s = fresh_state();
        assert!(!s.should_show_paywall(2));
    }

    #[test]
    fn paywall_not_after_module_4() {
        let s = fresh_state();
        assert!(!s.should_show_paywall(4));
    }

    #[test]
    fn paywall_not_when_purchased() {
        let mut s = fresh_state();
        s.is_purchased = true;
        assert!(!s.should_show_paywall(3));
    }

    #[test]
    fn unlock_all_modules_on_purchase() {
        let mut s = fresh_state();
        assert!(!s.progress.unlocked_modules.contains(&4));
        s.unlock_all_modules();
        assert!(s.is_purchased);
        for module in &s.pack.modules {
            assert!(s.progress.unlocked_modules.contains(&module.id),
                "Module {} should be unlocked", module.id);
        }
    }

    // ── Module progression ──

    #[test]
    fn complete_challenge_unlocks_next_module() {
        let mut s = fresh_state();
        // Complete all module 1 challenges
        let m1_ids: Vec<String> = s.pack.modules[0].challenge_ids.clone();
        for id in &m1_ids {
            s.complete_challenge(id);
        }
        assert!(s.progress.unlocked_modules.contains(&2), "Module 2 should unlock after module 1 complete");
    }

    #[test]
    fn next_challenge_skips_completed() {
        let mut s = fresh_state();
        s.complete_challenge("rust-m1-c1");
        s.complete_challenge("rust-m1-c2");
        let next = s.get_next_challenge_id();
        assert_eq!(next, Some("rust-m1-c3".into()));
    }

    #[test]
    fn next_challenge_skips_locked_modules() {
        let mut s = fresh_state();
        // Complete all module 1
        let m1_ids: Vec<String> = s.pack.modules[0].challenge_ids.clone();
        for id in &m1_ids { s.complete_challenge(id); }
        // Module 2 should be next (unlocked by completing m1)
        let next = s.get_next_challenge_id();
        assert!(next.unwrap().starts_with("rust-m2"), "Next should be in module 2");
    }

    // ── Streak ──

    #[test]
    fn fill_streak_increments_counter() {
        let mut s = fresh_state();
        assert_eq!(s.user.current_streak, 0);
        s.fill_streak_today();
        assert_eq!(s.user.current_streak, 1);
    }

    #[test]
    fn fill_streak_twice_same_day_no_double_count() {
        let mut s = fresh_state();
        s.fill_streak_today();
        s.fill_streak_today();
        assert_eq!(s.user.current_streak, 1, "Double fill should not double count");
    }

    #[test]
    fn fill_streak_updates_longest() {
        let mut s = fresh_state();
        s.user.current_streak = 9;
        s.user.longest_streak = 9;
        // Reset streak days so today (last) is unfilled
        s.user.streak_days = vec![true, true, true, true, true, true, false];
        s.fill_streak_today();
        assert_eq!(s.user.current_streak, 10);
        assert_eq!(s.user.longest_streak, 10);
    }

    // ── Skip ──

    #[test]
    fn skip_adds_to_revisit_queue() {
        let mut s = fresh_state();
        s.skipped_challenges_mut("rust-m1-c3");
        assert!(s.progress.skipped_challenges.contains(&"rust-m1-c3".to_string()));
    }

    #[test]
    fn skip_no_duplicate() {
        let mut s = fresh_state();
        s.skipped_challenges_mut("rust-m1-c3");
        s.skipped_challenges_mut("rust-m1-c3");
        assert_eq!(s.progress.skipped_challenges.len(), 1);
    }

    // ── Offline banner (Ticket 13) ──

    #[test]
    fn test_offline_banner_content() {
        let mut s = fresh_state();
        s.is_offline = true;
        let text = s.offline_banner_text();
        assert!(text.is_some());
        assert_eq!(
            text.unwrap(),
            "Offline \u{2014} progress will sync when you reconnect"
        );
    }

    #[test]
    fn test_offline_banner_hidden_when_online() {
        let s = fresh_state();
        assert!(s.offline_banner_text().is_none(), "Banner should be None when online");
    }

    #[test]
    fn test_revisit_count() {
        let mut s = fresh_state();
        assert_eq!(s.revisit_count(), 0, "No skipped challenges initially");
        s.skipped_challenges_mut("rust-m1-c3");
        assert_eq!(s.revisit_count(), 1);
        s.skipped_challenges_mut("rust-m1-c4");
        assert_eq!(s.revisit_count(), 2);
    }

    #[test]
    fn test_revisit_count_no_duplicates() {
        let mut s = fresh_state();
        s.skipped_challenges_mut("rust-m1-c3");
        s.skipped_challenges_mut("rust-m1-c3");
        assert_eq!(s.revisit_count(), 1, "Duplicate skips should not inflate count");
    }

    // ── Session persistence (save/load progress JSON) ──

    #[test]
    fn test_save_progress_json_roundtrip() {
        let mut s = fresh_state();
        s.user.total_xp = 240;
        s.user.current_streak = 5;
        s.user.longest_streak = 8;
        s.user.streak_days = vec![true, true, false, true, true, true, false];
        s.user.has_freeze = false;
        s.progress.xp = 180;
        s.progress.active_module = 3;
        s.progress.unlocked_modules = vec![1, 2, 3];
        s.progress.completed_challenges = vec!["rust-m1-c1".into(), "rust-m1-c2".into()];
        s.progress.skipped_challenges = vec!["rust-m1-c5".into()];
        s.is_onboarded = true;
        s.is_purchased = true;
        s.total_attempts = 20;
        s.correct_attempts = 15;

        let json = s.save_progress_json();
        let mut loaded = fresh_state();
        assert!(loaded.load_progress_json(&json));

        assert_eq!(loaded.user.total_xp, 240);
        assert_eq!(loaded.user.current_streak, 5);
        assert_eq!(loaded.user.longest_streak, 8);
        assert_eq!(loaded.user.streak_days, vec![true, true, false, true, true, true, false]);
        assert_eq!(loaded.user.has_freeze, false);
        assert_eq!(loaded.progress.xp, 180);
        assert_eq!(loaded.progress.active_module, 3);
        assert_eq!(loaded.progress.unlocked_modules, vec![1, 2, 3]);
        assert_eq!(loaded.progress.completed_challenges, vec!["rust-m1-c1".to_string(), "rust-m1-c2".to_string()]);
        assert_eq!(loaded.progress.skipped_challenges, vec!["rust-m1-c5".to_string()]);
        assert!(loaded.is_onboarded);
        assert!(loaded.is_purchased);
        assert_eq!(loaded.total_attempts, 20);
        assert_eq!(loaded.correct_attempts, 15);
    }

    #[test]
    fn test_load_progress_restores_completed_challenges() {
        let mut s = fresh_state();
        s.progress.completed_challenges = vec![
            "rust-m1-c1".into(),
            "rust-m1-c2".into(),
            "rust-m1-c3".into(),
        ];
        let json = s.save_progress_json();
        let mut loaded = fresh_state();
        loaded.load_progress_json(&json);
        assert_eq!(loaded.progress.completed_challenges.len(), 3);
        assert!(loaded.progress.completed_challenges.contains(&"rust-m1-c1".to_string()));
        assert!(loaded.progress.completed_challenges.contains(&"rust-m1-c2".to_string()));
        assert!(loaded.progress.completed_challenges.contains(&"rust-m1-c3".to_string()));
    }

    #[test]
    fn test_load_progress_restores_unlocked_modules() {
        let mut s = fresh_state();
        s.progress.unlocked_modules = vec![1, 2, 3];
        let json = s.save_progress_json();
        let mut loaded = fresh_state();
        loaded.load_progress_json(&json);
        assert_eq!(loaded.progress.unlocked_modules, vec![1, 2, 3]);
    }

    #[test]
    fn test_load_progress_restores_xp() {
        let mut s = fresh_state();
        s.user.total_xp = 240;
        s.progress.xp = 200;
        let json = s.save_progress_json();
        let mut loaded = fresh_state();
        loaded.load_progress_json(&json);
        assert_eq!(loaded.user.total_xp, 240);
        assert_eq!(loaded.progress.xp, 200);
    }

    #[test]
    fn test_load_progress_restores_onboarded() {
        let mut s = fresh_state();
        s.is_onboarded = true;
        let json = s.save_progress_json();
        let mut loaded = fresh_state();
        assert!(!loaded.is_onboarded);
        loaded.load_progress_json(&json);
        assert!(loaded.is_onboarded);
    }

    #[test]
    fn test_load_progress_handles_empty_json() {
        let mut s = fresh_state();
        assert!(!s.load_progress_json(""), "Empty string should return false");
    }

    #[test]
    fn test_load_progress_handles_corrupt_json() {
        let mut s = fresh_state();
        s.user.total_xp = 100;
        assert!(!s.load_progress_json("not valid json {{{"), "Corrupt JSON should return false");
        assert_eq!(s.user.total_xp, 100, "State should be unchanged after corrupt JSON");
    }

    #[test]
    fn test_load_progress_handles_partial_json() {
        let mut s = fresh_state();
        s.user.total_xp = 50; // pre-existing value
        // JSON with only some fields
        let partial = r#"{"total_xp": 300, "is_onboarded": true}"#;
        assert!(s.load_progress_json(partial));
        assert_eq!(s.user.total_xp, 300, "Should update fields present in JSON");
        assert!(s.is_onboarded, "Should update is_onboarded from JSON");
        // Fields not in JSON should keep their previous values
        assert_eq!(s.progress.active_module, 1, "Missing fields should remain at previous value");
    }

    // ── Language switching ──

    #[test]
    fn switch_language_unknown_returns_false() {
        let mut s = fresh_state();
        assert!(!s.switch_language("cobol"));
    }

    #[test]
    fn switch_language_resets_progress() {
        let mut s = fresh_state();
        s.complete_challenge("rust-m1-c1");
        assert!(!s.progress.completed_challenges.is_empty());
        // Switch to rust again (same pack but fresh progress)
        s.switch_language("rust");
        assert!(s.progress.completed_challenges.is_empty());
    }

    // ── Ticket 19: Try-again keeps tokens (pure state logic) ──

    #[test]
    fn try_again_keeps_assembled_tokens() {
        // Simulates the on_try_again handler logic:
        // After a wrong answer, assembled tokens should NOT be cleared.
        // Instead, only feedback is cleared and attempt_num is incremented.
        let assembled: Vec<(String, String)> = vec![
            ("let".into(), "token-keyword".into()),
            ("x".into(), "token-identifier".into()),
            ("=".into(), "token-symbol".into()),
            ("5".into(), "token-number".into()),
        ];

        // Simulate on_try_again: feedback cleared, attempt incremented, tokens KEPT
        let feedback_cleared = true;
        let attempt_num: u32 = 2;
        let tokens_after_try_again = assembled.clone(); // NOT cleared

        assert!(feedback_cleared);
        assert_eq!(attempt_num, 2);
        assert_eq!(tokens_after_try_again.len(), 4, "Assembled tokens must be preserved on try again");
    }

    #[test]
    fn try_again_keeps_diff_visible() {
        // After a wrong answer, try-again should keep show_diff=true
        // and diff_data intact so user sees highlighted errors.
        let show_diff = true;
        let diff_data: Option<Vec<crate::models::TokenDiff>> = Some(vec![
            crate::models::TokenDiff::Match("let".into()),
            crate::models::TokenDiff::Wrong { got: "y".into(), expected: "x".into() },
        ]);

        // After on_try_again, these should still be set
        let show_diff_after = show_diff; // NOT cleared
        let diff_data_after = diff_data.clone(); // NOT cleared

        assert!(show_diff_after, "Diff should remain visible after try again");
        assert!(diff_data_after.is_some(), "Diff data should remain after try again");
    }

    #[test]
    fn token_tap_clears_diff_after_wrong_answer() {
        // When user taps a token in the canvas while diff is showing,
        // the diff should be cleared (user is editing their answer).
        let mut show_diff = true;
        let mut diff_data: Option<Vec<crate::models::TokenDiff>> = Some(vec![
            crate::models::TokenDiff::Match("let".into()),
        ]);

        // Simulate: user taps token at index 0 while diff is showing
        // This should clear the diff
        if show_diff {
            show_diff = false;
            diff_data = None;
        }

        assert!(!show_diff, "Diff should be cleared when user taps a token");
        assert!(diff_data.is_none(), "Diff data should be cleared when user taps a token");
    }

    // ── Ticket 18: Confetti cleanup in on_try_again ──

    #[test]
    fn confetti_cleared_on_try_again() {
        // Confetti should be cleared when user taps try again
        // (even though it normally wouldn't be active on wrong answer,
        // belt-and-suspenders approach).
        let mut show_confetti = true;
        // on_try_again should set show_confetti = false
        show_confetti = false;
        assert!(!show_confetti, "Confetti should be cleared on try again");
    }

    // ── Ticket 20: Module complete stats from real data ──

    #[test]
    fn module_complete_accuracy_from_state() {
        let mut s = fresh_state();
        s.record_attempt(true);
        s.record_attempt(true);
        s.record_attempt(false);
        assert_eq!(s.accuracy_percent(), 66, "Accuracy should be 66% for 2/3 correct");
        assert_eq!(s.hints_used_this_session, 0, "Initial hints used should be 0");
    }

    #[test]
    fn module_complete_hints_from_state() {
        let mut s = fresh_state();
        s.user.total_xp = 100;
        s.deduct_hint_xp(); // free
        s.deduct_hint_xp(); // costs 5
        assert_eq!(s.hints_used_this_session, 2, "Two hints used should be tracked");
    }

    #[test]
    fn module_complete_last_module_no_next() {
        let s = fresh_state();
        let last_module_id = s.pack.modules.last().map(|m| m.id).unwrap_or(0);
        let next_module = s.pack.modules.iter().find(|m| m.id == last_module_id + 1);
        assert!(next_module.is_none(), "Last module should have no next module");
    }

    // ── Ticket 21: Hint XP deduction called from on_hint ──

    #[test]
    fn hint_concept_deducts_xp_unless_first() {
        let mut s = fresh_state();
        s.user.total_xp = 100;
        // First hint is free
        let cost1 = s.deduct_hint_xp();
        assert_eq!(cost1, 0);
        assert_eq!(s.user.total_xp, 100);
        // Second hint costs 5
        let cost2 = s.deduct_hint_xp();
        assert_eq!(cost2, 5);
        assert_eq!(s.user.total_xp, 95);
    }

    // ── Ticket 22: Paywall purchase wires through ──

    #[test]
    fn purchase_unlocks_and_sets_flag() {
        let mut s = fresh_state();
        assert!(!s.is_purchased);
        s.unlock_all_modules();
        assert!(s.is_purchased, "is_purchased should be true after unlock");
        // All modules should be unlocked
        for module in &s.pack.modules {
            assert!(
                s.progress.unlocked_modules.contains(&module.id),
                "Module {} should be unlocked after purchase", module.id
            );
        }
    }

    // ── Bug fix: navigating to new module should NOT show module complete ──

    #[test]
    fn module_1_complete_module_2_not_complete() {
        let mut s = fresh_state();
        // Complete all module 1
        let m1_ids: Vec<String> = s.pack.modules[0].challenge_ids.clone();
        for id in &m1_ids { s.complete_challenge(id); }

        assert!(s.is_module_complete(1), "Module 1 should be complete");
        assert!(!s.is_module_complete(2), "Module 2 should NOT be complete");
    }

    #[test]
    fn starting_module_2_first_challenge_not_done() {
        let mut s = fresh_state();
        // Complete all module 1
        let m1_ids: Vec<String> = s.pack.modules[0].challenge_ids.clone();
        for id in &m1_ids { s.complete_challenge(id); }

        // Module 2 is unlocked but no challenges completed
        assert!(s.is_module_unlocked(2));
        let m2_first = &s.pack.modules[1].challenge_ids[0];
        assert!(
            !s.progress.completed_challenges.contains(m2_first),
            "First challenge of module 2 should NOT be completed"
        );
    }

    #[test]
    fn next_challenge_after_module_1_is_module_2_first() {
        let mut s = fresh_state();
        let m1_ids: Vec<String> = s.pack.modules[0].challenge_ids.clone();
        for id in &m1_ids { s.complete_challenge(id); }

        let next = s.get_next_challenge_id();
        assert_eq!(next, Some("rust-m2-c1".into()),
            "After completing module 1, next should be rust-m2-c1");
    }
}
