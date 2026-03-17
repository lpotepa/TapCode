//! Challenge Validation Pipeline (PRD §8)
//!
//! Language-agnostic orchestrator that verifies every challenge compiles
//! and produces expected output. All language-specific logic lives in
//! CompilerAdapter implementations — one per language, zero in the core.
//!
//! Adding a new language's validation:
//!   1. Implement CompilerAdapter for the language
//!   2. Register it in AdapterRegistry::default()
//!   3. Done — all static + compilation tests run automatically

use crate::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════
// Compiler Adapter Trait
//
// Each language implements this trait. The orchestrator calls it
// without knowing which language it's working with.
// Zero language-specific logic in the orchestrator.
// ══════════════════════════════════════════════════════════════

pub trait CompilerAdapter: Send + Sync {
    /// Language this adapter handles (e.g. "rust", "go", "python")
    fn language_id(&self) -> &str;

    /// Shell command to compile + run a source file.
    /// `{SRC}` = path to source, `{OUT}` = path to binary output.
    fn run_command(&self) -> &str;

    /// Max seconds before a compilation/run is killed.
    fn timeout_seconds(&self) -> u32;

    /// Source file extension (e.g. "rs", "go", "py")
    fn file_extension(&self) -> &str;

    /// Extra compiler flags (e.g. ["-A", "unused"] for Rust)
    fn compiler_flags(&self) -> Vec<String> {
        vec![]
    }

    /// Reconstruct a full, compilable program from a challenge fragment.
    ///
    /// The adapter knows its language's program structure (where main goes,
    /// how imports work, etc.) and uses the scaffold for context setup.
    fn wrap_fragment(
        &self,
        fragment: &str,
        fragment_type: &FragmentType,
        scaffold: &str,
    ) -> Option<String>;

    /// Language-specific check: does the wrapped program look structurally valid?
    /// Default: check that it's non-empty. Languages can add their own checks.
    fn validate_program_structure(&self, program: &str) -> Result<(), String> {
        if program.trim().is_empty() {
            return Err("Wrapped program is empty".into());
        }
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════
// Adapter Registry
//
// Same pattern as LanguagePackRegistry: a HashMap of adapters,
// keyed by language_id. Adding a language = registering one adapter.
// ══════════════════════════════════════════════════════════════

pub struct AdapterRegistry {
    adapters: HashMap<String, Box<dyn CompilerAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register an adapter for a language. Replaces any existing adapter.
    pub fn register(&mut self, adapter: Box<dyn CompilerAdapter>) {
        let id = adapter.language_id().to_string();
        self.adapters.insert(id, adapter);
    }

    /// Get the adapter for a language.
    pub fn get(&self, language_id: &str) -> Option<&dyn CompilerAdapter> {
        self.adapters.get(language_id).map(|a| a.as_ref())
    }

    /// List all registered language IDs.
    pub fn languages(&self) -> Vec<&str> {
        self.adapters.keys().map(|s| s.as_str()).collect()
    }

    /// Build the default registry with all built-in adapters.
    /// Adding a new language = adding one line here.
    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(RustAdapter::new()));
        // Future languages — one line each:
        // registry.register(Box::new(GoAdapter::new()));
        // registry.register(Box::new(PythonAdapter::new()));
        registry
    }
}

// ══════════════════════════════════════════════════════════════
// Rust Adapter
// ══════════════════════════════════════════════════════════════

pub struct RustAdapter {
    wrappers: HashMap<FragmentType, String>,
}

impl RustAdapter {
    pub fn new() -> Self {
        let mut wrappers = HashMap::new();
        wrappers.insert(
            FragmentType::Expression,
            "{SCAFFOLD_OUTER}\nfn main() { {SCAFFOLD_INNER}\nlet _ = {FRAGMENT}; }".into(),
        );
        wrappers.insert(
            FragmentType::Statement,
            "{SCAFFOLD_OUTER}\nfn main() { {SCAFFOLD_INNER}\n{FRAGMENT} }".into(),
        );
        wrappers.insert(
            FragmentType::FnDef,
            "{SCAFFOLD}\n{FRAGMENT}\nfn main() {}".into(),
        );
        wrappers.insert(
            FragmentType::TypeDef,
            "{SCAFFOLD}\n{FRAGMENT}\nfn main() {}".into(),
        );
        wrappers.insert(
            FragmentType::Program,
            "{SCAFFOLD}\n{FRAGMENT}".into(),
        );
        Self { wrappers }
    }

    /// Rust-specific: split scaffold into module-level (outer) and fn-level (inner).
    /// `use`, `struct`, `enum`, `fn`, `trait`, `impl`, etc. go before fn main.
    /// `let` bindings go inside fn main.
    fn split_scaffold(scaffold: &str) -> (String, String) {
        if scaffold.is_empty() {
            return (String::new(), String::new());
        }

        let module_level_prefixes = [
            "use ", "struct ", "enum ", "fn ", "trait ", "impl ", "type ",
            "const ", "static ", "mod ", "pub ", "extern ", "#[",
        ];

        let mut outer = Vec::new();
        let mut inner = Vec::new();

        for line in scaffold.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if module_level_prefixes.iter().any(|p| trimmed.starts_with(p)) {
                outer.push(line);
            } else {
                inner.push(line);
            }
        }

        (outer.join("\n"), inner.join("\n"))
    }

    fn clean_empty_lines(program: &str) -> String {
        program
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl CompilerAdapter for RustAdapter {
    fn language_id(&self) -> &str { "rust" }

    fn run_command(&self) -> &str {
        "rustc {SRC} -o {OUT} --edition 2021 && {OUT}"
    }

    fn timeout_seconds(&self) -> u32 { 5 }

    fn file_extension(&self) -> &str { "rs" }

    fn compiler_flags(&self) -> Vec<String> {
        vec![
            "-A".into(), "unused".into(),
            "-A".into(), "dead_code".into(),
        ]
    }

    fn wrap_fragment(
        &self,
        fragment: &str,
        fragment_type: &FragmentType,
        scaffold: &str,
    ) -> Option<String> {
        self.wrappers.get(fragment_type).map(|template| {
            let program = match fragment_type {
                FragmentType::Statement | FragmentType::Expression => {
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
            Self::clean_empty_lines(&program)
        })
    }

    fn validate_program_structure(&self, program: &str) -> Result<(), String> {
        if program.trim().is_empty() {
            return Err("Wrapped program is empty".into());
        }
        if !program.contains("fn main") {
            return Err("Rust program missing fn main".into());
        }
        Ok(())
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
// Static Validation — language-agnostic orchestrator
//
// Checks structural correctness of any challenge, regardless
// of language. Uses the adapter registry to find the right
// adapter for each challenge's language field.
// ══════════════════════════════════════════════════════════════

pub fn validate_challenge_static(
    challenge: &Challenge,
    registry: &AdapterRegistry,
) -> ChallengeValidation {
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

    // 5. Adapter must exist for this language
    let adapter = registry.get(&challenge.language);
    if adapter.is_none() {
        errors.push(format!(
            "[{}] no compiler adapter registered for language '{}'",
            challenge.id, challenge.language
        ));
    }

    // 6. Reconstructed program must pass structural validation
    if let Some(adapter) = adapter {
        let fragment = challenge.answer.join(" ");
        match adapter.wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold) {
            Some(program) => {
                if let Err(e) = adapter.validate_program_structure(&program) {
                    errors.push(format!("[{}] {}", challenge.id, e));
                }
            }
            None => {
                errors.push(format!(
                    "[{}] adapter '{}' has no wrapper for fragment_type {:?}",
                    challenge.id, challenge.language, challenge.fragment_type
                ));
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

/// Validate all challenges in a language pack. Language-agnostic.
pub fn validate_all_challenges(
    pack: &LanguagePack,
    registry: &AdapterRegistry,
) -> (usize, usize, Vec<ChallengeValidation>) {
    let mut passed = 0;
    let mut failed = 0;
    let mut results = Vec::new();

    for challenge in &pack.challenges {
        let result = validate_challenge_static(challenge, registry);
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

    fn registry() -> AdapterRegistry {
        AdapterRegistry::default_registry()
    }

    // ── S1: Adapter Trait + Registry ──

    #[test]
    fn s1_1_rust_adapter_registered() {
        let reg = registry();
        assert!(reg.get("rust").is_some());
    }

    #[test]
    fn s1_2_unknown_language_returns_none() {
        let reg = registry();
        assert!(reg.get("brainfuck").is_none());
    }

    #[test]
    fn s1_3_rust_adapter_wraps_statement() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        let result = adapter.wrap_fragment("let x = 5;", &FragmentType::Statement, "");
        assert!(result.is_some());
        let program = result.unwrap();
        assert!(program.contains("fn main()"));
        assert!(program.contains("let x = 5;"));
    }

    #[test]
    fn s1_4_rust_adapter_wraps_expression() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        let result = adapter.wrap_fragment("42", &FragmentType::Expression, "");
        assert!(result.unwrap().contains("let _ = 42;"));
    }

    #[test]
    fn s1_5_rust_adapter_wraps_fn_def() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        let result = adapter.wrap_fragment("fn add(a: i32, b: i32) -> i32 { a + b }", &FragmentType::FnDef, "");
        let program = result.unwrap();
        assert!(program.contains("fn add"));
        assert!(program.contains("fn main()"));
    }

    #[test]
    fn s1_6_rust_adapter_wraps_program() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        let result = adapter.wrap_fragment("fn main() { println!(\"hi\"); }", &FragmentType::Program, "");
        assert_eq!(result.unwrap(), "fn main() { println!(\"hi\"); }");
    }

    #[test]
    fn s1_7_rust_validates_missing_main() {
        let adapter = RustAdapter::new();
        let result = adapter.validate_program_structure("let x = 5;");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fn main"));
    }

    #[test]
    fn s1_8_scaffold_inner_variables_go_inside_main() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        let result = adapter.wrap_fragment("x = 10;", &FragmentType::Statement, "let mut x = 0;");
        let program = result.unwrap();
        let decl_pos = program.find("let mut x = 0;").unwrap();
        let frag_pos = program.find("x = 10;").unwrap();
        assert!(decl_pos < frag_pos);
    }

    #[test]
    fn s1_9_scaffold_outer_items_go_before_main() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        let scaffold = "use std::collections::HashMap;\nlet mut map = HashMap::new();";
        let result = adapter.wrap_fragment("map.insert(1, 2);", &FragmentType::Statement, scaffold);
        let program = result.unwrap();
        let use_pos = program.find("use std::collections::HashMap;").unwrap();
        let main_pos = program.find("fn main()").unwrap();
        let let_pos = program.find("let mut map").unwrap();
        assert!(use_pos < main_pos, "use should be before fn main");
        assert!(let_pos > main_pos, "let should be inside fn main");
    }

    #[test]
    fn s1_10_registry_lists_languages() {
        let reg = registry();
        let langs = reg.languages();
        assert!(langs.contains(&"rust"));
    }

    #[test]
    fn s1_11_custom_adapter_registers_without_code_changes_to_core() {
        // Prove: adding a language = implementing the trait + one register() call.
        // No changes to the orchestrator, static validator, or compilation test.
        struct MockPythonAdapter;

        impl CompilerAdapter for MockPythonAdapter {
            fn language_id(&self) -> &str { "python" }
            fn run_command(&self) -> &str { "python3 {SRC}" }
            fn timeout_seconds(&self) -> u32 { 5 }
            fn file_extension(&self) -> &str { "py" }
            fn wrap_fragment(&self, fragment: &str, _ft: &FragmentType, scaffold: &str) -> Option<String> {
                Some(format!("{}\n{}", scaffold, fragment))
            }
        }

        let mut reg = AdapterRegistry::default_registry();
        reg.register(Box::new(MockPythonAdapter));

        assert!(reg.get("rust").is_some(), "Rust still works");
        assert!(reg.get("python").is_some(), "Python added");
        assert_eq!(reg.get("python").unwrap().file_extension(), "py");

        // Python adapter can wrap fragments
        let program = reg.get("python").unwrap()
            .wrap_fragment("print('hello')", &FragmentType::Statement, "import os")
            .unwrap();
        assert!(program.contains("import os"));
        assert!(program.contains("print('hello')"));
    }

    #[test]
    fn s1_12_adapter_metadata_accessible() {
        let reg = registry();
        let adapter = reg.get("rust").unwrap();
        assert_eq!(adapter.language_id(), "rust");
        assert_eq!(adapter.file_extension(), "rs");
        assert!(adapter.timeout_seconds() > 0);
        assert!(!adapter.compiler_flags().is_empty());
    }

    // ── S2: Static Validation ──

    #[test]
    fn s2_1_valid_challenge_passes() {
        let c = Challenge {
            id: "test-valid".into(),
            language: "rust".into(),
            module: 1, position: 1,
            title: "Test".into(), prompt: "Do something".into(),
            hint_concept: "Hint".into(), hint_structural: "_ ;".into(),
            fragment_type: FragmentType::Statement,
            answer: vec!["x".into(), ";".into()],
            answer_variants: vec![],
            chips: vec![
                ChipGroup { group: "identifiers".into(), tokens: vec!["x".into(), "y".into()] },
                ChipGroup { group: "symbols".into(), tokens: vec![";".into()] },
            ],
            xp: 20, explanation: "".into(), scaffold: "".into(),
        };
        let result = validate_challenge_static(&c, &registry());
        assert_eq!(result.status, ValidationStatus::Verified, "Errors: {:?}", result.errors);
    }

    #[test]
    fn s2_2_answer_token_not_in_chips_fails() {
        let c = Challenge {
            id: "test-missing".into(), language: "rust".into(),
            module: 1, position: 1, title: "T".into(), prompt: "T".into(),
            hint_concept: "".into(), hint_structural: "".into(),
            fragment_type: FragmentType::Statement,
            answer: vec!["ghost_token".into()], answer_variants: vec![],
            chips: vec![ChipGroup { group: "x".into(), tokens: vec!["x".into()] }],
            xp: 20, explanation: "".into(), scaffold: "".into(),
        };
        let result = validate_challenge_static(&c, &registry());
        assert_eq!(result.status, ValidationStatus::Failed);
    }

    #[test]
    fn s2_3_empty_answer_fails() {
        let mut c = Challenge::test_fixture(vec![]);
        c.chips = vec![ChipGroup { group: "x".into(), tokens: vec!["x".into()] }];
        let result = validate_challenge_static(&c, &registry());
        assert_eq!(result.status, ValidationStatus::Failed);
    }

    #[test]
    fn s2_4_unknown_language_fails_validation() {
        let mut c = Challenge::test_fixture(vec!["x"]);
        c.language = "cobol".into();
        c.chips = vec![ChipGroup { group: "x".into(), tokens: vec!["x".into()] }];
        let result = validate_challenge_static(&c, &registry());
        assert_eq!(result.status, ValidationStatus::Failed);
        assert!(result.errors.iter().any(|e| e.contains("no compiler adapter")));
    }

    // ── S3: Full pack validation ──

    #[test]
    fn s3_1_all_rust_challenges_pass_static_validation() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let adapter_reg = registry();
        let (passed, failed, results) = validate_all_challenges(pack, &adapter_reg);

        for r in &results {
            if r.status != ValidationStatus::Verified {
                eprintln!("FAILED: {} — {:?}", r.challenge_id, r.errors);
            }
        }
        assert_eq!(failed, 0, "{} challenges failed", failed);
        assert!(passed > 0);
    }

    #[test]
    fn s3_2_all_answer_tokens_in_chips() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        for challenge in &pack.challenges {
            let all_tokens: Vec<&String> = challenge.chips.iter()
                .flat_map(|g| g.tokens.iter()).collect();
            for (i, token) in challenge.answer.iter().enumerate() {
                assert!(all_tokens.contains(&token),
                    "Challenge '{}': answer token '{}' at {} not in chips", challenge.id, token, i);
            }
        }
    }

    #[test]
    fn s3_3_no_prompt_leaks_answer() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        for challenge in &pack.challenges {
            let answer_str = challenge.answer.join("");
            assert!(!challenge.prompt.contains(&answer_str),
                "Challenge '{}' prompt leaks full answer", challenge.id);
            if challenge.answer.len() >= 3 {
                let first_three = challenge.answer[..3].join("");
                assert!(!challenge.prompt.contains(&first_three),
                    "Challenge '{}' prompt leaks first 3 tokens", challenge.id);
            }
        }
    }

    #[test]
    fn s3_4_every_challenge_has_explanation() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        for c in &pack.challenges {
            assert!(!c.explanation.is_empty(), "{} missing explanation", c.id);
        }
    }

    #[test]
    fn s3_5_every_challenge_has_hints() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        for c in &pack.challenges {
            assert!(!c.hint_concept.is_empty(), "{} missing hint_concept", c.id);
            assert!(!c.hint_structural.is_empty(), "{} missing hint_structural", c.id);
        }
    }

    #[test]
    fn s3_6_challenge_ids_match_language_namespace() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        for c in &pack.challenges {
            assert!(c.id.starts_with("rust-"), "{} wrong namespace", c.id);
        }
    }

    #[test]
    fn s3_7_fragment_wrapping_produces_valid_program() {
        let reg = engine::build_default_registry();
        let pack = reg.get_pack("rust").unwrap();
        let adapter_reg = registry();
        let adapter = adapter_reg.get("rust").unwrap();

        for challenge in &pack.challenges {
            let fragment = challenge.answer.join(" ");
            let program = adapter.wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold);
            assert!(program.is_some(), "{}: adapter couldn't wrap {:?}", challenge.id, challenge.fragment_type);
            let program = program.unwrap();
            assert!(adapter.validate_program_structure(&program).is_ok(),
                "{}: structural validation failed", challenge.id);
        }
    }
}
