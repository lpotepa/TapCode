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
    pub active_language: String,
    pub is_offline: bool,
    pub sound_enabled: bool,
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
            active_language: "rust".into(),
            is_offline: false,
            sound_enabled: false,
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
}
