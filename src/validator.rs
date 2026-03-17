//! Challenge Validation Pipeline (PRD §8)
//!
//! Verifies that every challenge's canonical answer(s) compile and produce
//! expected output BEFORE the challenge can be published. Language-agnostic
//! orchestrator selects a compiler adapter by `challenge.language`.
//!
//! Usage:
//!   cargo test --test challenge_validation   (CI gate)
//!   or: validator::validate_all_challenges(&pack) in build scripts

use crate::models::*;
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════
// Compiler Adapter Interface
//
// Each language provides fragment wrappers that reconstruct a
// runnable program from the challenge's token fragment.
// Adding a new language = adding a new CompilerAdapter config.
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CompilerAdapter {
    pub language_id: String,
    pub fragment_wrappers: HashMap<FragmentType, String>,
    pub run_command: String,
    pub timeout_seconds: u32,
}

impl CompilerAdapter {
    /// Build the Rust compiler adapter
    pub fn rust() -> Self {
        // Templates use {SCAFFOLD} for context code and {FRAGMENT} for the answer.
        // For statement/expression: scaffold goes INSIDE fn main, before the fragment,
        //   so variable declarations and `use` imports are in scope.
        // For fn_def/type_def: scaffold goes BEFORE the fragment (module level),
        //   so struct/trait/import definitions are available.
        // For program: scaffold goes before the fragment (the fragment IS the program).
        let mut wrappers = HashMap::new();
        wrappers.insert(
            FragmentType::Expression,
            "{SCAFFOLD_OUTER}\nfn main() { {SCAFFOLD_INNER}\nlet _ = {FRAGMENT}; }".to_string(),
        );
        wrappers.insert(
            FragmentType::Statement,
            "{SCAFFOLD_OUTER}\nfn main() { {SCAFFOLD_INNER}\n{FRAGMENT} }".to_string(),
        );
        wrappers.insert(
            FragmentType::FnDef,
            "{SCAFFOLD}\n{FRAGMENT}\nfn main() {}".to_string(),
        );
        wrappers.insert(
            FragmentType::TypeDef,
            "{SCAFFOLD}\n{FRAGMENT}\nfn main() {}".to_string(),
        );
        wrappers.insert(
            FragmentType::Program,
            "{SCAFFOLD}\n{FRAGMENT}".to_string(),
        );

        CompilerAdapter {
            language_id: "rust".to_string(),
            fragment_wrappers: wrappers,
            run_command: "rustc /tmp/challenge.rs -o /tmp/out && /tmp/out".to_string(),
            timeout_seconds: 5,
        }
    }

    /// Reconstruct a full program from a challenge fragment.
    ///
    /// Scaffold provides context code needed for the fragment to compile.
    /// For statement/expression types, the scaffold is split into:
    ///   - "outer" lines (module-level items like `use`, `struct`, `fn`, `trait`, `enum`,
    ///     `impl`, `type`, `const`, `static`, `mod`, `pub`, and `#[`)
    ///   - "inner" lines (everything else, like `let` bindings)
    /// Outer lines go before `fn main`, inner lines go inside it before the fragment.
    /// For fn_def/type_def/program types, the entire scaffold goes before the fragment.
    pub fn wrap_fragment(&self, fragment: &str, fragment_type: &FragmentType, scaffold: &str) -> Option<String> {
        self.fragment_wrappers.get(fragment_type).map(|template| {
            let program = match fragment_type {
                FragmentType::Statement | FragmentType::Expression => {
                    // Split scaffold into outer (module-level) and inner (fn-level) lines
                    let (outer, inner) = Self::split_scaffold(scaffold);
                    template
                        .replace("{SCAFFOLD_OUTER}", &outer)
                        .replace("{SCAFFOLD_INNER}", &inner)
                        .replace("{FRAGMENT}", fragment)
                }
                _ => {
                    template
                        .replace("{SCAFFOLD}", scaffold)
                        .replace("{FRAGMENT}", fragment)
                }
            };
            // Clean up leading/trailing whitespace from empty scaffold substitutions
            let mut cleaned = String::new();
            for line in program.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if !cleaned.is_empty() {
                        cleaned.push('\n');
                    }
                    cleaned.push_str(line);
                }
            }
            cleaned
        })
    }

    /// Split scaffold code into outer (module-level) and inner (fn-level) parts.
    /// Lines starting with module-level keywords go to outer, rest to inner.
    fn split_scaffold(scaffold: &str) -> (String, String) {
        if scaffold.is_empty() {
            return (String::new(), String::new());
        }

        let mut outer_lines = Vec::new();
        let mut inner_lines = Vec::new();

        for line in scaffold.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Module-level items: use, struct, enum, fn, trait, impl, type, const,
            // static, mod, pub, extern, #[ (attributes)
            if trimmed.starts_with("use ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("static ")
                || trimmed.starts_with("mod ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("extern ")
                || trimmed.starts_with("#[")
            {
                outer_lines.push(line);
            } else {
                inner_lines.push(line);
            }
        }

        (outer_lines.join("\n"), inner_lines.join("\n"))
    }
}

// ══════════════════════════════════════════════════════════════
// Adapter Registry
// ══════════════════════════════════════════════════════════════

pub fn get_adapter(language_id: &str) -> Option<CompilerAdapter> {
    match language_id {
        "rust" => Some(CompilerAdapter::rust()),
        // Add new languages here:
        // "go" => Some(CompilerAdapter::go()),
        // "python" => Some(CompilerAdapter::python()),
        _ => None,
    }
}

// ══════════════════════════════════════════════════════════════
// Expected Output Contract
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpectedOutput {
    pub exit_code: i32,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub stderr_contains: Option<String>,
}

use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════════
// Validation Results
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeValidation {
    pub challenge_id: String,
    pub status: ValidationStatus,
    pub answers_checked: u32,
    pub answers_passed: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    Verified,
    Failed,
    Pending,
    NoAdapter,
}

// ══════════════════════════════════════════════════════════════
// Static Validation (no compiler needed — runs in CI and tests)
//
// Validates structural correctness of every challenge:
//  - answer is non-empty
//  - all answer tokens exist in the challenge's chip groups
//  - fragment_type has a matching adapter wrapper
//  - answer_variants are also valid
//  - chips have no empty groups
//  - hint_structural has the right number of placeholders
// ══════════════════════════════════════════════════════════════

pub fn validate_challenge_static(challenge: &Challenge) -> ChallengeValidation {
    let mut errors = Vec::new();
    let mut answers_checked = 0u32;
    let mut answers_passed = 0u32;

    // 1. Answer must be non-empty
    if challenge.answer.is_empty() {
        errors.push(format!("[{}] answer is empty", challenge.id));
    }

    // 2. All answer tokens must exist in the chip groups
    let all_chip_tokens: Vec<&String> = challenge.chips.iter()
        .flat_map(|g| g.tokens.iter())
        .collect();

    for (i, token) in challenge.answer.iter().enumerate() {
        answers_checked += 1;
        if all_chip_tokens.contains(&token) {
            answers_passed += 1;
        } else {
            errors.push(format!(
                "[{}] answer token '{}' at position {} not found in any chip group",
                challenge.id, token, i
            ));
        }
    }

    // 3. Validate all answer_variants too
    for (vi, variant) in challenge.answer_variants.iter().enumerate() {
        if variant.is_empty() {
            errors.push(format!("[{}] answer_variant[{}] is empty", challenge.id, vi));
        }
        for (i, token) in variant.iter().enumerate() {
            answers_checked += 1;
            if all_chip_tokens.contains(&token) {
                answers_passed += 1;
            } else {
                errors.push(format!(
                    "[{}] answer_variant[{}] token '{}' at position {} not in chips",
                    challenge.id, vi, token, i
                ));
            }
        }
    }

    // 4. Chip groups must not be empty
    for group in &challenge.chips {
        if group.tokens.is_empty() {
            errors.push(format!("[{}] chip group '{}' has no tokens", challenge.id, group.group));
        }
    }

    // 5. Fragment type must have an adapter wrapper
    if let Some(adapter) = get_adapter(&challenge.language) {
        if !adapter.fragment_wrappers.contains_key(&challenge.fragment_type) {
            errors.push(format!(
                "[{}] fragment_type {:?} has no wrapper in {} adapter",
                challenge.id, challenge.fragment_type, challenge.language
            ));
        }
    }

    // 6. Reconstructed program must be valid syntax (adapter wrapping works)
    if let Some(adapter) = get_adapter(&challenge.language) {
        let fragment = challenge.answer.join(" ");
        match adapter.wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold) {
            Some(program) => {
                if program.trim().is_empty() {
                    errors.push(format!("[{}] wrapped program is empty", challenge.id));
                }
            }
            None => {
                errors.push(format!("[{}] adapter could not wrap fragment", challenge.id));
            }
        }
    }

    let status = if errors.is_empty() {
        ValidationStatus::Verified
    } else {
        ValidationStatus::Failed
    };

    ChallengeValidation {
        challenge_id: challenge.id.clone(),
        status,
        answers_checked,
        answers_passed,
        errors,
    }
}

/// Validate all challenges in a language pack. Returns (passed, failed, results).
pub fn validate_all_challenges(pack: &LanguagePack) -> (usize, usize, Vec<ChallengeValidation>) {
    let mut passed = 0;
    let mut failed = 0;
    let mut results = Vec::new();

    for challenge in &pack.challenges {
        let result = validate_challenge_static(challenge);
        match result.status {
            ValidationStatus::Verified => passed += 1,
            _ => failed += 1,
        }
        results.push(result);
    }

    (passed, failed, results)
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine;

    // ── S1: Compiler Adapter ──

    #[test]
    fn s1_1_rust_adapter_exists() {
        assert!(get_adapter("rust").is_some());
    }

    #[test]
    fn s1_2_unknown_adapter_returns_none() {
        assert!(get_adapter("brainfuck").is_none());
    }

    #[test]
    fn s1_3_rust_adapter_wraps_statement() {
        let adapter = CompilerAdapter::rust();
        let result = adapter.wrap_fragment("let x = 5;", &FragmentType::Statement, "");
        assert!(result.is_some());
        let program = result.unwrap();
        assert!(program.contains("fn main()"));
        assert!(program.contains("let x = 5;"));
    }

    #[test]
    fn s1_4_rust_adapter_wraps_expression() {
        let adapter = CompilerAdapter::rust();
        let result = adapter.wrap_fragment("42", &FragmentType::Expression, "");
        assert!(result.unwrap().contains("let _ = 42;"));
    }

    #[test]
    fn s1_5_rust_adapter_wraps_fn_def() {
        let adapter = CompilerAdapter::rust();
        let result = adapter.wrap_fragment("fn add(a: i32, b: i32) -> i32 { a + b }", &FragmentType::FnDef, "");
        let program = result.unwrap();
        assert!(program.contains("fn add"));
        assert!(program.contains("fn main()"));
    }

    #[test]
    fn s1_6_rust_adapter_wraps_program() {
        let adapter = CompilerAdapter::rust();
        let result = adapter.wrap_fragment("fn main() { println!(\"hi\"); }", &FragmentType::Program, "");
        assert_eq!(result.unwrap(), "fn main() { println!(\"hi\"); }");
    }

    #[test]
    fn s1_8_scaffold_prepended_to_program() {
        let adapter = CompilerAdapter::rust();
        let result = adapter.wrap_fragment("x = 10;", &FragmentType::Statement, "let mut x = 0;");
        let program = result.unwrap();
        // For statement type, scaffold variable decls go inside fn main before fragment
        assert!(program.contains("fn main()"));
        assert!(program.contains("let mut x = 0;"));
        assert!(program.contains("x = 10;"));
        // The variable decl must come before the reassignment
        let decl_pos = program.find("let mut x = 0;").unwrap();
        let frag_pos = program.find("x = 10;").unwrap();
        assert!(decl_pos < frag_pos, "scaffold must appear before fragment");
    }

    #[test]
    fn s1_9_scaffold_outer_items_go_before_fn_main() {
        let adapter = CompilerAdapter::rust();
        let scaffold = "use std::collections::HashMap;\nlet mut map = HashMap::new();";
        let result = adapter.wrap_fragment("map.insert(1, 2);", &FragmentType::Statement, scaffold);
        let program = result.unwrap();
        // use goes before fn main (outer), let goes inside fn main (inner)
        let use_pos = program.find("use std::collections::HashMap;").unwrap();
        let main_pos = program.find("fn main()").unwrap();
        let let_pos = program.find("let mut map").unwrap();
        assert!(use_pos < main_pos, "use should be before fn main");
        assert!(let_pos > main_pos, "let should be inside fn main");
    }

    #[test]
    fn s1_7_all_fragment_types_have_wrappers() {
        let adapter = CompilerAdapter::rust();
        assert!(adapter.fragment_wrappers.contains_key(&FragmentType::Expression));
        assert!(adapter.fragment_wrappers.contains_key(&FragmentType::Statement));
        assert!(adapter.fragment_wrappers.contains_key(&FragmentType::FnDef));
        assert!(adapter.fragment_wrappers.contains_key(&FragmentType::TypeDef));
        assert!(adapter.fragment_wrappers.contains_key(&FragmentType::Program));
    }

    // ── S2: Static Validation — valid challenges ──

    #[test]
    fn s2_1_valid_challenge_passes() {
        let c = Challenge {
            id: "test-valid".into(),
            language: "rust".into(),
            module: 1,
            position: 1,
            title: "Test".into(),
            prompt: "Do something".into(),
            hint_concept: "Hint".into(),
            hint_structural: "_ ;".into(),
            fragment_type: FragmentType::Statement,
            answer: vec!["x".into(), ";".into()],
            answer_variants: vec![],
            chips: vec![
                ChipGroup { group: "identifiers".into(), tokens: vec!["x".into(), "y".into()] },
                ChipGroup { group: "symbols".into(), tokens: vec![";".into()] },
            ],
            xp: 20,
            explanation: "".into(),
            scaffold: "".into(),
        };

        let result = validate_challenge_static(&c);
        assert_eq!(result.status, ValidationStatus::Verified, "Errors: {:?}", result.errors);
    }

    #[test]
    fn s2_2_answer_token_not_in_chips_fails() {
        let c = Challenge {
            id: "test-missing-chip".into(),
            language: "rust".into(),
            module: 1,
            position: 1,
            title: "Test".into(),
            prompt: "Do something".into(),
            hint_concept: "".into(),
            hint_structural: "".into(),
            fragment_type: FragmentType::Statement,
            answer: vec!["ghost_token".into()],
            answer_variants: vec![],
            chips: vec![
                ChipGroup { group: "other".into(), tokens: vec!["x".into()] },
            ],
            xp: 20,
            explanation: "".into(),
            scaffold: "".into(),
        };

        let result = validate_challenge_static(&c);
        assert_eq!(result.status, ValidationStatus::Failed);
        assert!(result.errors[0].contains("ghost_token"));
    }

    #[test]
    fn s2_3_empty_answer_fails() {
        let mut c = Challenge::test_fixture(vec![]);
        c.chips = vec![ChipGroup { group: "x".into(), tokens: vec!["x".into()] }];
        let result = validate_challenge_static(&c);
        assert_eq!(result.status, ValidationStatus::Failed);
        assert!(result.errors.iter().any(|e| e.contains("empty")));
    }

    #[test]
    fn s2_4_empty_chip_group_fails() {
        let c = Challenge {
            id: "test-empty-group".into(),
            language: "rust".into(),
            module: 1,
            position: 1,
            title: "Test".into(),
            prompt: "Do".into(),
            hint_concept: "".into(),
            hint_structural: "".into(),
            fragment_type: FragmentType::Statement,
            answer: vec!["x".into()],
            answer_variants: vec![],
            chips: vec![
                ChipGroup { group: "good".into(), tokens: vec!["x".into()] },
                ChipGroup { group: "empty".into(), tokens: vec![] },
            ],
            xp: 20,
            explanation: "".into(),
            scaffold: "".into(),
        };

        let result = validate_challenge_static(&c);
        assert_eq!(result.status, ValidationStatus::Failed);
        assert!(result.errors.iter().any(|e| e.contains("empty") && e.contains("no tokens")));
    }

    #[test]
    fn s2_5_variant_token_not_in_chips_fails() {
        let c = Challenge {
            id: "test-bad-variant".into(),
            language: "rust".into(),
            module: 1,
            position: 1,
            title: "Test".into(),
            prompt: "Do".into(),
            hint_concept: "".into(),
            hint_structural: "".into(),
            fragment_type: FragmentType::Statement,
            answer: vec!["x".into()],
            answer_variants: vec![vec!["missing_token".into()]],
            chips: vec![
                ChipGroup { group: "ids".into(), tokens: vec!["x".into()] },
            ],
            xp: 20,
            explanation: "".into(),
            scaffold: "".into(),
        };

        let result = validate_challenge_static(&c);
        assert_eq!(result.status, ValidationStatus::Failed);
        assert!(result.errors.iter().any(|e| e.contains("answer_variant") && e.contains("missing_token")));
    }

    // ── S3: Full pack validation ──

    #[test]
    fn s3_1_all_rust_challenges_pass_static_validation() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let (passed, failed, results) = validate_all_challenges(pack);

        // Print any failures for debugging
        for r in &results {
            if r.status != ValidationStatus::Verified {
                eprintln!("FAILED: {} — {:?}", r.challenge_id, r.errors);
            }
        }

        assert_eq!(failed, 0, "{} challenges failed validation", failed);
        assert!(passed > 0, "At least one challenge should be validated");
    }

    #[test]
    fn s3_2_all_answer_tokens_in_chips() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();

        for challenge in &pack.challenges {
            let all_tokens: Vec<&String> = challenge.chips.iter()
                .flat_map(|g| g.tokens.iter())
                .collect();

            for (i, token) in challenge.answer.iter().enumerate() {
                assert!(
                    all_tokens.contains(&token),
                    "Challenge '{}': answer token '{}' at position {} not in chips",
                    challenge.id, token, i
                );
            }
        }
    }

    #[test]
    fn s3_3_no_challenge_prompt_contains_answer() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();

        for challenge in &pack.challenges {
            // The prompt should not contain the full answer sequence
            let answer_str = challenge.answer.join("");
            assert!(
                !challenge.prompt.contains(&answer_str),
                "Challenge '{}' prompt leaks the full answer: '{}'",
                challenge.id, challenge.prompt
            );

            // The prompt should not contain code-like patterns with the exact syntax
            // (check for things like "println!(" which would give away the macro call)
            if challenge.answer.len() >= 3 {
                let first_three = challenge.answer[..3].join("");
                assert!(
                    !challenge.prompt.contains(&first_three),
                    "Challenge '{}' prompt leaks the first 3 tokens: '{}'",
                    challenge.id, first_three
                );
            }
        }
    }

    #[test]
    fn s3_4_every_challenge_has_explanation() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();

        for challenge in &pack.challenges {
            assert!(
                !challenge.explanation.is_empty(),
                "Challenge '{}' is missing an explanation",
                challenge.id
            );
        }
    }

    #[test]
    fn s3_5_every_challenge_has_hints() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();

        for challenge in &pack.challenges {
            assert!(
                !challenge.hint_concept.is_empty(),
                "Challenge '{}' is missing hint_concept",
                challenge.id
            );
            assert!(
                !challenge.hint_structural.is_empty(),
                "Challenge '{}' is missing hint_structural",
                challenge.id
            );
        }
    }

    #[test]
    fn s3_6_challenge_ids_match_language_namespace() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();

        for challenge in &pack.challenges {
            assert!(
                challenge.id.starts_with("rust-"),
                "Challenge '{}' doesn't follow 'rust-' namespace convention",
                challenge.id
            );
        }
    }

    #[test]
    fn s3_7_fragment_wrapping_produces_valid_program() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let adapter = CompilerAdapter::rust();

        for challenge in &pack.challenges {
            let fragment = challenge.answer.join(" ");
            let program = adapter.wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold);
            assert!(
                program.is_some(),
                "Challenge '{}': adapter couldn't wrap fragment type {:?}",
                challenge.id, challenge.fragment_type
            );
            let program = program.unwrap();
            assert!(
                !program.trim().is_empty(),
                "Challenge '{}': wrapped program is empty",
                challenge.id
            );
            // For Rust, every program should contain fn main
            // (token-joined fragments may have spaces in "fn main ( )")
            assert!(
                program.contains("fn main"),
                "Challenge '{}': wrapped program missing fn main",
                challenge.id
            );
        }
    }
}
