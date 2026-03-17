use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Language Registry ──
// The registry is the top-level container that holds all available language packs.
// Adding a new language = adding a new JSON file + one entry in registry.json.
// Zero code changes in the app.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageRegistry {
    pub languages: Vec<LanguageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageEntry {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub status: LanguageStatus,
    pub primary_hue: String,
    pub icon: String,
    pub tagline: String,
    /// Relative path to the language pack JSON (resolved at build time)
    pub pack_path: String,
}

// ── Language Pack ──
// Self-contained bundle: everything the app needs to teach one language.
// The app has ZERO language-specific logic — this struct IS the language.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagePack {
    pub metadata: LanguageMetadata,
    pub categories: Vec<TokenCategory>,
    pub context_rules: Vec<ContextRule>,
    pub syntax_colors: HashMap<String, String>,
    pub modules: Vec<ModuleInfo>,
    pub challenges: Vec<Challenge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetadata {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub status: LanguageStatus,
    pub primary_hue: String,
    pub icon: String,
    pub tagline: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LanguageStatus {
    Available,
    ComingSoon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenCategory {
    pub name: String,
    pub display_name: String,
    pub css_class: String,
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextRule {
    pub after_tokens: Vec<String>,
    pub highlight_groups: Vec<String>,
    pub dim_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleInfo {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub challenge_ids: Vec<String>,
    pub is_free: bool,
}

// ── Challenge ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Challenge {
    pub id: String,
    pub language: String,
    pub module: u32,
    pub position: u32,
    pub title: String,
    pub prompt: String,
    pub hint_concept: String,
    pub hint_structural: String,
    pub fragment_type: FragmentType,
    pub answer: Vec<String>,
    #[serde(default)]
    pub answer_variants: Vec<Vec<String>>,
    pub chips: Vec<ChipGroup>,
    pub xp: u32,
    #[serde(default)]
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChipGroup {
    pub group: String,
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FragmentType {
    Expression,
    Statement,
    FnDef,
    TypeDef,
    Program,
}

// ── Validation ──

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Correct,
    Wrong(Vec<TokenDiff>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenDiff {
    Match(String),
    Wrong { got: String, expected: String },
    Extra(String),
    Missing(String),
}

// ── User State ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserState {
    pub total_xp: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub last_active: Option<String>,
    pub streak_days: Vec<bool>, // last 7 days
    pub has_freeze: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageProgress {
    pub language_id: String,
    pub xp: u32,
    pub active_module: u32,
    pub unlocked_modules: Vec<u32>,
    pub completed_challenges: Vec<String>,
    pub skipped_challenges: Vec<String>,
}

// ── UI State Enums ──

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackKind {
    None,
    Correct {
        xp_awarded: u32,
        explanation: String,
    },
    Wrong {
        diff: Vec<TokenDiff>,
        explanation: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HintTier {
    None,
    Concept,
    Structural,
    SkipAvailable,
}

impl HintTier {
    pub fn next(&self) -> HintTier {
        match self {
            HintTier::None => HintTier::Concept,
            HintTier::Concept => HintTier::Structural,
            HintTier::Structural => HintTier::SkipAvailable,
            HintTier::SkipAvailable => HintTier::SkipAvailable,
        }
    }

    pub fn tier_number(&self) -> u8 {
        match self {
            HintTier::None => 0,
            HintTier::Concept => 1,
            HintTier::Structural => 2,
            HintTier::SkipAvailable => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Splash,
    LanguageSelect,
    SkillCheck,
    IntroCards(u8),
    Complete,
}

// ── Chip Group State (for contextual highlighting) ──

#[derive(Debug, Clone, PartialEq)]
pub struct ChipGroupState {
    pub group_name: String,
    pub is_highlighted: bool,
}

impl Default for FeedbackKind {
    fn default() -> Self {
        FeedbackKind::None
    }
}

// ── Test helpers ──

impl Challenge {
    /// Create a minimal challenge for testing — avoids boilerplate in tests
    #[cfg(test)]
    pub fn test_fixture(answer: Vec<&str>) -> Self {
        Challenge {
            id: "test-c1".into(),
            language: "rust".into(),
            module: 1,
            position: 1,
            title: "Test".into(),
            prompt: "Test prompt".into(),
            hint_concept: "Test hint".into(),
            hint_structural: "_ = _;".into(),
            fragment_type: FragmentType::Statement,
            answer: answer.into_iter().map(String::from).collect(),
            answer_variants: vec![],
            chips: vec![],
            xp: 20,
            explanation: "Test explanation".into(),
        }
    }
}
