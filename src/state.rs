use dioxus::prelude::*;
use crate::models::*;
use crate::engine;

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
}
