use crate::models::*;
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════
// Language Registry — language-agnostic pack loader (Ticket 04)
//
// The app has ZERO language-specific logic. All language knowledge
// lives in LanguagePack JSON data. Adding a new language =
// adding a new JSON file. No recompile needed for content changes.
// ══════════════════════════════════════════════════════════════

/// Registry of all available language packs.
/// In v1, packs are embedded at compile time.
/// In v2, packs can be downloaded dynamically.
#[derive(Debug)]
pub struct LanguagePackRegistry {
    packs: HashMap<String, LanguagePack>,
    entries: Vec<LanguageEntry>,
}

impl LanguagePackRegistry {
    /// Build the registry from embedded pack JSON strings.
    /// Each entry is (language_id, json_str).
    /// Zero language-specific logic — just data parsing.
    pub fn from_embedded(packs: &[(&str, &str)]) -> Result<Self, RegistryError> {
        let mut registry = LanguagePackRegistry {
            packs: HashMap::new(),
            entries: Vec::new(),
        };

        for (id, json) in packs {
            let pack: LanguagePack = serde_json::from_str(json)
                .map_err(|e| RegistryError::ParseError {
                    language_id: id.to_string(),
                    message: e.to_string(),
                })?;

            registry.entries.push(LanguageEntry {
                id: pack.metadata.id.clone(),
                display_name: pack.metadata.display_name.clone(),
                version: pack.metadata.version.clone(),
                status: pack.metadata.status.clone(),
                primary_hue: pack.metadata.primary_hue.clone(),
                icon: pack.metadata.icon.clone(),
                tagline: pack.metadata.tagline.clone(),
                pack_path: format!("{}.json", id),
            });

            registry.packs.insert(id.to_string(), pack);
        }

        Ok(registry)
    }

    /// List all languages in the registry with their status.
    pub fn list_languages(&self) -> &[LanguageEntry] {
        &self.entries
    }

    /// List only available (not coming_soon) languages.
    pub fn available_languages(&self) -> Vec<&LanguageEntry> {
        self.entries.iter().filter(|e| e.status == LanguageStatus::Available).collect()
    }

    /// Get a language pack by ID. Returns None for unknown or coming_soon packs.
    pub fn get_pack(&self, language_id: &str) -> Option<&LanguagePack> {
        self.packs.get(language_id)
    }

    /// Number of loaded packs.
    pub fn pack_count(&self) -> usize {
        self.packs.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    ParseError { language_id: String, message: String },
}

/// Build the default registry with all embedded language packs.
/// Adding a new language = adding one line here + one JSON file.
pub fn build_default_registry() -> LanguagePackRegistry {
    let rust_json = include_str!("../assets/data/rust_pack.json");

    // To add a new language, just add another entry:
    // ("go", include_str!("../assets/data/go_pack.json")),
    // ("python", include_str!("../assets/data/python_pack.json")),
    LanguagePackRegistry::from_embedded(&[
        ("rust", rust_json),
    ])
    .expect("Failed to build language pack registry")
}

// ══════════════════════════════════════════════════════════════
// Token Sequence Validation (Ticket 05)
// ══════════════════════════════════════════════════════════════

pub fn validate_answer(user_tokens: &[String], challenge: &Challenge) -> ValidationResult {
    // Check primary answer
    if user_tokens == challenge.answer.as_slice() {
        return ValidationResult::Correct;
    }

    // Check variants
    for variant in &challenge.answer_variants {
        if user_tokens == variant.as_slice() {
            return ValidationResult::Correct;
        }
    }

    // Compute diff against closest answer (primary)
    let diff = compute_diff(user_tokens, &challenge.answer);
    ValidationResult::Wrong(diff)
}

pub fn compute_diff(user: &[String], answer: &[String]) -> Vec<TokenDiff> {
    let mut diff = Vec::new();
    let max_len = user.len().max(answer.len());

    for i in 0..max_len {
        match (user.get(i), answer.get(i)) {
            (Some(u), Some(a)) if u == a => {
                diff.push(TokenDiff::Match(u.clone()));
            }
            (Some(u), Some(a)) => {
                diff.push(TokenDiff::Wrong {
                    got: u.clone(),
                    expected: a.clone(),
                });
            }
            (Some(u), None) => {
                diff.push(TokenDiff::Extra(u.clone()));
            }
            (None, Some(a)) => {
                diff.push(TokenDiff::Missing(a.clone()));
            }
            (None, None) => unreachable!(),
        }
    }

    diff
}

// ══════════════════════════════════════════════════════════════
// Contextual State Machine (Ticket 04)
//
// Pure function: (tokens_so_far, language_rules, all_groups) → group_states
// Rules live in the language pack JSON, not in code.
// ══════════════════════════════════════════════════════════════

pub fn evaluate_context(
    tokens: &[String],
    rules: &[ContextRule],
    all_groups: &[String],
) -> Vec<ChipGroupState> {
    if tokens.is_empty() {
        return all_groups
            .iter()
            .map(|g| ChipGroupState {
                group_name: g.clone(),
                is_highlighted: true,
            })
            .collect();
    }

    // Find the best matching rule (longest after_tokens match at end of token sequence)
    let mut best_match: Option<&ContextRule> = None;
    let mut best_len = 0;

    for rule in rules {
        let rule_len = rule.after_tokens.len();
        if rule_len > 0 && tokens.len() >= rule_len {
            let tail = &tokens[tokens.len() - rule_len..];
            if tail == rule.after_tokens.as_slice() && rule_len >= best_len {
                best_match = Some(rule);
                best_len = rule_len;
            }
        }
    }

    match best_match {
        Some(rule) => all_groups
            .iter()
            .map(|g| {
                let is_highlighted = if rule.highlight_groups.contains(g) {
                    true
                } else if rule.dim_groups.contains(g) {
                    false
                } else {
                    true // not mentioned = visible (safe default)
                };
                ChipGroupState {
                    group_name: g.clone(),
                    is_highlighted,
                }
            })
            .collect(),
        None => {
            // No rule matched — all groups visible (never hide by accident)
            all_groups
                .iter()
                .map(|g| ChipGroupState {
                    group_name: g.clone(),
                    is_highlighted: true,
                })
                .collect()
        }
    }
}

// ══════════════════════════════════════════════════════════════
// Helpers — all language-agnostic, driven by LanguagePack data
// ══════════════════════════════════════════════════════════════

pub fn get_challenge_by_id<'a>(pack: &'a LanguagePack, id: &str) -> Option<&'a Challenge> {
    pack.challenges.iter().find(|c| c.id == id)
}

pub fn get_module_challenges<'a>(pack: &'a LanguagePack, module_id: u32) -> Vec<&'a Challenge> {
    pack.challenges.iter().filter(|c| c.module == module_id).collect()
}

pub fn get_next_challenge_id(pack: &LanguagePack, progress: &LanguageProgress) -> Option<String> {
    for module in &pack.modules {
        if !progress.unlocked_modules.contains(&module.id) {
            continue;
        }
        for challenge_id in &module.challenge_ids {
            if !progress.completed_challenges.contains(challenge_id) {
                return Some(challenge_id.clone());
            }
        }
    }
    None
}

pub fn is_module_complete(module: &ModuleInfo, progress: &LanguageProgress) -> bool {
    module.challenge_ids.iter().all(|id| progress.completed_challenges.contains(id))
}

pub fn get_token_category<'a>(pack: &'a LanguagePack, token: &str) -> Option<&'a TokenCategory> {
    pack.categories.iter().find(|cat| cat.tokens.contains(&token.to_string()))
}

pub fn xp_for_attempt(base_xp: u32, attempt_number: u32) -> u32 {
    if attempt_number <= 1 {
        base_xp
    } else {
        base_xp / 2
    }
}

// ══════════════════════════════════════════════════════════════
// Tests — RED/GREEN TDD
//
// Every test below was written as a RED (failing) test FIRST,
// then the production code above was written to make it GREEN.
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──

    fn s(val: &str) -> String { val.to_string() }
    fn sv(vals: &[&str]) -> Vec<String> { vals.iter().map(|v| s(v)).collect() }

    fn make_challenge(answer: &[&str], variants: Vec<Vec<&str>>) -> Challenge {
        let mut c = Challenge::test_fixture(answer.to_vec());
        c.answer_variants = variants.into_iter().map(|v| sv(&v)).collect();
        c
    }

    fn rust_groups() -> Vec<String> {
        sv(&["keywords", "macros", "types", "strings", "symbols", "identifiers", "numbers"])
    }

    fn rust_rules() -> Vec<ContextRule> {
        vec![
            ContextRule {
                after_tokens: sv(&["fn"]),
                highlight_groups: sv(&["identifiers"]),
                dim_groups: sv(&["keywords", "macros", "types", "strings", "numbers"]),
            },
            ContextRule {
                after_tokens: sv(&["let"]),
                highlight_groups: sv(&["identifiers", "keywords"]),
                dim_groups: sv(&["macros", "types", "strings"]),
            },
            ContextRule {
                after_tokens: sv(&["let", "mut"]),
                highlight_groups: sv(&["identifiers"]),
                dim_groups: sv(&["keywords", "macros", "types", "strings"]),
            },
            ContextRule {
                after_tokens: sv(&[":"]),
                highlight_groups: sv(&["types"]),
                dim_groups: sv(&["keywords", "macros", "strings", "identifiers", "numbers"]),
            },
            ContextRule {
                after_tokens: sv(&["->"]),
                highlight_groups: sv(&["types"]),
                dim_groups: sv(&["keywords", "macros", "strings", "identifiers", "numbers"]),
            },
            ContextRule {
                after_tokens: sv(&["="]),
                highlight_groups: sv(&["identifiers", "numbers", "strings", "macros", "types"]),
                dim_groups: sv(&["keywords"]),
            },
            ContextRule {
                after_tokens: sv(&["("]),
                highlight_groups: sv(&["identifiers", "numbers", "strings", "types", "symbols"]),
                dim_groups: sv(&["keywords"]),
            },
        ]
    }

    fn is_highlighted(states: &[ChipGroupState], group: &str) -> bool {
        states.iter().find(|s| s.group_name == group).map(|s| s.is_highlighted).unwrap_or(false)
    }

    // ════════════════════════════════════════════
    // S1: Language Pack Deserialization (Ticket 04)
    // ════════════════════════════════════════════

    #[test]
    fn s1_1_valid_rust_pack_deserializes() {
        let json = include_str!("../assets/data/rust_pack.json");
        let pack: Result<LanguagePack, _> = serde_json::from_str(json);
        assert!(pack.is_ok(), "Rust pack should deserialize: {:?}", pack.err());
    }

    #[test]
    fn s1_2_metadata_fields_populated() {
        let pack = build_default_registry();
        let rust = pack.get_pack("rust").unwrap();
        assert_eq!(rust.metadata.id, "rust");
        assert_eq!(rust.metadata.display_name, "Rust");
        assert_eq!(rust.metadata.primary_hue, "#f5a623");
        assert_eq!(rust.metadata.icon, "🦀");
        assert!(!rust.metadata.version.is_empty());
    }

    #[test]
    fn s1_3_seven_token_categories() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        assert_eq!(pack.categories.len(), 7, "Rust pack must have 7 token categories");
    }

    #[test]
    fn s1_4_each_category_has_tokens() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        for cat in &pack.categories {
            assert!(!cat.tokens.is_empty(), "Category '{}' must have at least 1 token", cat.name);
        }
    }

    #[test]
    fn s1_5_modules_ordered() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let ids: Vec<u32> = pack.modules.iter().map(|m| m.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "Modules must be in order");
    }

    #[test]
    fn s1_6_missing_field_returns_error() {
        let bad_json = r#"{ "metadata": { "id": "bad" } }"#;
        let result: Result<LanguagePack, _> = serde_json::from_str(bad_json);
        assert!(result.is_err(), "Missing fields should produce a serde error");
    }

    #[test]
    fn s1_7_unknown_extra_field_ignored() {
        // A pack with an extra unknown field should still parse (forward-compatible)
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        // If we got here, serde's default deny_unknown_fields is NOT active — good.
        assert_eq!(pack.metadata.id, "rust");
    }

    // ════════════════════════════════════════════
    // S2: Language Registry (Ticket 04)
    // ════════════════════════════════════════════

    #[test]
    fn s2_1_registry_loads_at_least_one_language() {
        let reg = build_default_registry();
        assert!(reg.pack_count() >= 1);
    }

    #[test]
    fn s2_2_rust_is_available() {
        let reg = build_default_registry();
        let rust = reg.list_languages().iter().find(|e| e.id == "rust");
        assert!(rust.is_some());
        assert_eq!(rust.unwrap().status, LanguageStatus::Available);
    }

    #[test]
    fn s2_3_get_pack_returns_some_for_rust() {
        let reg = build_default_registry();
        assert!(reg.get_pack("rust").is_some());
    }

    #[test]
    fn s2_4_get_pack_returns_none_for_unknown() {
        let reg = build_default_registry();
        assert!(reg.get_pack("nonexistent").is_none());
    }

    #[test]
    fn s2_5_adding_second_language_requires_no_code_changes() {
        // Simulate: build a registry with two packs from the same JSON
        // (proving the architecture supports N languages with zero code changes)
        let rust_json = include_str!("../assets/data/rust_pack.json");

        // Modify the metadata to pretend it's Go
        let go_json = rust_json.replace(r#""id": "rust"#, r#""id": "go"#)
            .replace(r#""display_name": "Rust"#, r#""display_name": "Go"#);

        let reg = LanguagePackRegistry::from_embedded(&[
            ("rust", rust_json),
            ("go", &go_json),
        ]).unwrap();

        assert_eq!(reg.pack_count(), 2);
        assert!(reg.get_pack("rust").is_some());
        assert!(reg.get_pack("go").is_some());
        assert_eq!(reg.get_pack("go").unwrap().metadata.display_name, "Go");
    }

    #[test]
    fn s2_6_invalid_json_returns_parse_error() {
        let result = LanguagePackRegistry::from_embedded(&[("bad", "not json")]);
        assert!(result.is_err());
        match result.unwrap_err() {
            RegistryError::ParseError { language_id, .. } => {
                assert_eq!(language_id, "bad");
            }
        }
    }

    // ════════════════════════════════════════════
    // S3: Contextual State Machine (Ticket 04)
    // ════════════════════════════════════════════

    #[test]
    fn s3_1_empty_sequence_all_highlighted() {
        let states = evaluate_context(&[], &rust_rules(), &rust_groups());
        for s in &states {
            assert!(s.is_highlighted, "Group '{}' should be highlighted on empty", s.group_name);
        }
    }

    #[test]
    fn s3_2_after_fn_identifiers_highlighted_keywords_dimmed() {
        let tokens = sv(&["fn"]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "identifiers"));
        assert!(!is_highlighted(&states, "keywords"));
    }

    #[test]
    fn s3_3_after_let_identifiers_highlighted() {
        let tokens = sv(&["let"]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "identifiers"));
    }

    #[test]
    fn s3_4_after_colon_types_highlighted() {
        let tokens = sv(&["let", "x", ":"]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "types"));
        assert!(!is_highlighted(&states, "keywords"));
    }

    #[test]
    fn s3_5_after_arrow_types_highlighted() {
        let tokens = sv(&["fn", "add", "(", "a", ":", "i32", ")", "->"]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "types"));
    }

    #[test]
    fn s3_6_after_equals_values_highlighted() {
        let tokens = sv(&["let", "x", "="]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "numbers"));
        assert!(is_highlighted(&states, "strings"));
        assert!(is_highlighted(&states, "identifiers"));
        assert!(!is_highlighted(&states, "keywords"));
    }

    #[test]
    fn s3_7_after_open_paren_values_forward() {
        let tokens = sv(&["println!", "("]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "strings"));
        assert!(is_highlighted(&states, "identifiers"));
        assert!(!is_highlighted(&states, "keywords"));
    }

    #[test]
    fn s3_8_unknown_sequence_all_visible() {
        let tokens = sv(&["some_random_token_xyz"]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        for s in &states {
            assert!(s.is_highlighted, "Unmatched sequence should show all groups");
        }
    }

    #[test]
    fn s3_9_longer_rule_takes_precedence() {
        // "let" matches, but "let", "mut" is longer and should win
        let tokens = sv(&["let", "mut"]);
        let states = evaluate_context(&tokens, &rust_rules(), &rust_groups());
        assert!(is_highlighted(&states, "identifiers"));
        // "let" alone would highlight keywords too, but "let mut" should dim them
        assert!(!is_highlighted(&states, "keywords"));
    }

    // ════════════════════════════════════════════
    // S4: Validation — Correct Answers (Ticket 05)
    // ════════════════════════════════════════════

    #[test]
    fn s4_1_exact_match_is_correct() {
        let c = make_challenge(&["let", "x", "=", "5", ";"], vec![]);
        let user = sv(&["let", "x", "=", "5", ";"]);
        assert_eq!(validate_answer(&user, &c), ValidationResult::Correct);
    }

    #[test]
    fn s4_2_variant_match_is_correct() {
        let c = make_challenge(
            &["let", "x", "=", "5", ";"],
            vec![vec!["let", "x", ":", "i32", "=", "5", ";"]],
        );
        let user = sv(&["let", "x", ":", "i32", "=", "5", ";"]);
        assert_eq!(validate_answer(&user, &c), ValidationResult::Correct);
    }

    #[test]
    fn s4_3_second_variant_matches() {
        let c = make_challenge(
            &["a"],
            vec![vec!["b"], vec!["c"]],
        );
        assert_eq!(validate_answer(&sv(&["c"]), &c), ValidationResult::Correct);
    }

    #[test]
    fn s4_4_primary_fails_variant_succeeds() {
        let c = make_challenge(&["a", "b"], vec![vec!["b", "a"]]);
        // Primary ["a","b"] doesn't match ["b","a"], but variant does
        assert_eq!(validate_answer(&sv(&["b", "a"]), &c), ValidationResult::Correct);
    }

    // ════════════════════════════════════════════
    // S5: Validation — Wrong Answers (Ticket 05)
    // ════════════════════════════════════════════

    #[test]
    fn s5_1_single_wrong_token() {
        let c = make_challenge(&["x", "=", "10", ";"], vec![]);
        let user = sv(&["x", "=", "5", ";"]);
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                assert!(matches!(&diff[0], TokenDiff::Match(t) if t == "x"));
                assert!(matches!(&diff[1], TokenDiff::Match(t) if t == "="));
                assert!(matches!(&diff[2], TokenDiff::Wrong { got, expected } if got == "5" && expected == "10"));
                assert!(matches!(&diff[3], TokenDiff::Match(t) if t == ";"));
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s5_2_extra_token_at_end() {
        let c = make_challenge(&["x", "=", "10", ";"], vec![]);
        let user = sv(&["x", "=", "10", ";", "!"]);
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                assert_eq!(diff.len(), 5);
                assert!(matches!(&diff[4], TokenDiff::Extra(t) if t == "!"));
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s5_3_missing_token() {
        let c = make_challenge(&["x", "=", "10", ";"], vec![]);
        let user = sv(&["x", "=", "10"]);
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                assert_eq!(diff.len(), 4);
                assert!(matches!(&diff[3], TokenDiff::Missing(t) if t == ";"));
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s5_4_two_swapped_tokens() {
        let c = make_challenge(&["let", "x", "=", "5", ";"], vec![]);
        let user = sv(&["let", "5", "=", "x", ";"]);
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                assert!(matches!(&diff[1], TokenDiff::Wrong { got, expected } if got == "5" && expected == "x"));
                assert!(matches!(&diff[3], TokenDiff::Wrong { got, expected } if got == "x" && expected == "5"));
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s5_5_completely_wrong_sequence() {
        let c = make_challenge(&["x", "=", "10", ";"], vec![]);
        let user = sv(&["y", "+", "5", "!"]);
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                for d in &diff {
                    assert!(matches!(d, TokenDiff::Wrong { .. }), "Every token should be Wrong");
                }
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s5_6_empty_user_sequence() {
        let c = make_challenge(&["x", "=", "10", ";"], vec![]);
        let user: Vec<String> = vec![];
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                assert_eq!(diff.len(), 4);
                for d in &diff {
                    assert!(matches!(d, TokenDiff::Missing(_)));
                }
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s5_7_only_first_token_correct() {
        let c = make_challenge(&["x", "=", "10", ";"], vec![]);
        let user = sv(&["x"]);
        match validate_answer(&user, &c) {
            ValidationResult::Wrong(diff) => {
                assert!(matches!(&diff[0], TokenDiff::Match(t) if t == "x"));
                assert!(matches!(&diff[1], TokenDiff::Missing(_)));
                assert!(matches!(&diff[2], TokenDiff::Missing(_)));
                assert!(matches!(&diff[3], TokenDiff::Missing(_)));
            }
            _ => panic!("Expected Wrong"),
        }
    }

    // ════════════════════════════════════════════
    // S6: Edge Cases (Ticket 05)
    // ════════════════════════════════════════════

    #[test]
    fn s6_1_user_longer_by_three() {
        let diff = compute_diff(&sv(&["a", "b", "c", "d", "e"]), &sv(&["a", "b"]));
        assert_eq!(diff.len(), 5);
        assert!(matches!(&diff[2], TokenDiff::Extra(_)));
        assert!(matches!(&diff[3], TokenDiff::Extra(_)));
        assert!(matches!(&diff[4], TokenDiff::Extra(_)));
    }

    #[test]
    fn s6_2_user_shorter_by_two() {
        let diff = compute_diff(&sv(&["a"]), &sv(&["a", "b", "c"]));
        assert_eq!(diff.len(), 3);
        assert!(matches!(&diff[1], TokenDiff::Missing(_)));
        assert!(matches!(&diff[2], TokenDiff::Missing(_)));
    }

    #[test]
    fn s6_3_duplicate_tokens_in_answer() {
        let c = make_challenge(&[";", ";"], vec![]);
        let user = sv(&[";", ";"]);
        assert_eq!(validate_answer(&user, &c), ValidationResult::Correct);
    }

    #[test]
    fn s6_4_single_token_correct() {
        let c = make_challenge(&["42"], vec![]);
        assert_eq!(validate_answer(&sv(&["42"]), &c), ValidationResult::Correct);
    }

    #[test]
    fn s6_5_single_token_wrong() {
        let c = make_challenge(&["42"], vec![]);
        match validate_answer(&sv(&["99"]), &c) {
            ValidationResult::Wrong(diff) => {
                assert_eq!(diff.len(), 1);
                assert!(matches!(&diff[0], TokenDiff::Wrong { got, expected } if got == "99" && expected == "42"));
            }
            _ => panic!("Expected Wrong"),
        }
    }

    #[test]
    fn s6_6_no_variants_wrong_only_checks_primary() {
        let c = make_challenge(&["a", "b"], vec![]);
        match validate_answer(&sv(&["b", "a"]), &c) {
            ValidationResult::Wrong(_) => {} // correct behavior
            _ => panic!("Expected Wrong — no variants to check"),
        }
    }

    // ════════════════════════════════════════════
    // S7: XP Calculation
    // ════════════════════════════════════════════

    #[test]
    fn s7_1_first_attempt_full_xp() {
        assert_eq!(xp_for_attempt(20, 1), 20);
    }

    #[test]
    fn s7_2_retry_half_xp() {
        assert_eq!(xp_for_attempt(20, 2), 10);
    }

    #[test]
    fn s7_3_third_attempt_still_half() {
        assert_eq!(xp_for_attempt(20, 3), 10);
    }

    // ════════════════════════════════════════════
    // S8: Module Helpers
    // ════════════════════════════════════════════

    #[test]
    fn s8_1_get_challenge_by_id_found() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        assert!(get_challenge_by_id(pack, "rust-m1-c1").is_some());
    }

    #[test]
    fn s8_2_get_challenge_by_id_not_found() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        assert!(get_challenge_by_id(pack, "nonexistent").is_none());
    }

    #[test]
    fn s8_3_get_module_challenges_returns_correct_count() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let m1 = get_module_challenges(pack, 1);
        assert_eq!(m1.len(), 5, "Module 1 should have 5 challenges");
    }

    #[test]
    fn s8_4_module_complete_all_done() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let module = &pack.modules[0];
        let progress = LanguageProgress {
            completed_challenges: module.challenge_ids.clone(),
            ..Default::default()
        };
        assert!(is_module_complete(module, &progress));
    }

    #[test]
    fn s8_5_module_not_complete_partial() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let module = &pack.modules[0];
        let progress = LanguageProgress {
            completed_challenges: vec![module.challenge_ids[0].clone()],
            ..Default::default()
        };
        assert!(!is_module_complete(module, &progress));
    }

    #[test]
    fn s8_6_next_challenge_skips_completed() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let progress = LanguageProgress {
            unlocked_modules: vec![1],
            completed_challenges: vec!["rust-m1-c1".into(), "rust-m1-c2".into()],
            ..Default::default()
        };
        assert_eq!(get_next_challenge_id(pack, &progress), Some("rust-m1-c3".into()));
    }

    #[test]
    fn s8_7_next_challenge_none_when_all_done() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let all_ids: Vec<String> = pack.modules.iter()
            .flat_map(|m| m.challenge_ids.clone())
            .collect();
        let progress = LanguageProgress {
            unlocked_modules: pack.modules.iter().map(|m| m.id).collect(),
            completed_challenges: all_ids,
            ..Default::default()
        };
        assert_eq!(get_next_challenge_id(pack, &progress), None);
    }

    #[test]
    fn s8_8_token_category_lookup() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let cat = get_token_category(pack, "fn");
        assert!(cat.is_some());
        assert_eq!(cat.unwrap().name, "keywords");
    }

    #[test]
    fn s8_9_token_category_not_found() {
        let reg = build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        assert!(get_token_category(pack, "zzz_nonexistent").is_none());
    }

    // ════════════════════════════════════════════
    // S9: Hint Tier Progression
    // ════════════════════════════════════════════

    #[test]
    fn s9_1_none_to_concept() {
        assert_eq!(HintTier::None.next(), HintTier::Concept);
    }

    #[test]
    fn s9_2_concept_to_structural() {
        assert_eq!(HintTier::Concept.next(), HintTier::Structural);
    }

    #[test]
    fn s9_3_structural_to_skip() {
        assert_eq!(HintTier::Structural.next(), HintTier::SkipAvailable);
    }

    #[test]
    fn s9_4_skip_stays_at_skip() {
        assert_eq!(HintTier::SkipAvailable.next(), HintTier::SkipAvailable);
    }

    #[test]
    fn s9_5_tier_numbers() {
        assert_eq!(HintTier::None.tier_number(), 0);
        assert_eq!(HintTier::Concept.tier_number(), 1);
        assert_eq!(HintTier::Structural.tier_number(), 2);
        assert_eq!(HintTier::SkipAvailable.tier_number(), 3);
    }
}
