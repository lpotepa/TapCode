//! Challenge Compilation Verification (PRD §8)
//!
//! Language-agnostic integration test. For every language pack in the
//! registry, finds the matching compiler adapter and invokes the real
//! compiler on every challenge answer. Supports any language — adding
//! a new one requires zero changes to this test.
//!
//! Run: cargo test --test challenge_compilation

use std::io::Write;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tapcode::engine;
use tapcode::validator::{AdapterRegistry, CompilerAdapter};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Compile a source file using the language's compiler.
/// Returns (success, stderr).
fn try_compile(source: &str, adapter: &dyn CompilerAdapter) -> (bool, String) {
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("tapcode_validation");
    std::fs::create_dir_all(&dir).unwrap();

    let ext = adapter.file_extension();
    let src_path = dir.join(format!("challenge_{}.{}", n, ext));
    let out_path = dir.join(format!("challenge_out_{}", n));

    let mut file = std::fs::File::create(&src_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();
    drop(file);

    // Build compiler command from adapter metadata
    let mut cmd = Command::new(compiler_binary(adapter));
    cmd.arg(&src_path);
    cmd.arg("-o").arg(&out_path);

    // Add language-specific flags
    for flag in adapter.compiler_flags() {
        cmd.arg(flag);
    }

    // Rust-specific: add edition flag
    if adapter.language_id() == "rust" {
        cmd.arg("--edition").arg("2021");
    }

    let output = cmd.output()
        .unwrap_or_else(|e| panic!(
            "Failed to invoke compiler for '{}': {}. Is it on PATH?",
            adapter.language_id(), e
        ));

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&out_path);

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

/// Extract the compiler binary name from the adapter's run_command.
fn compiler_binary(adapter: &dyn CompilerAdapter) -> String {
    // run_command format: "rustc {SRC} -o {OUT} ..." or "python3 {SRC}"
    adapter.run_command()
        .split_whitespace()
        .next()
        .unwrap_or("rustc")
        .to_string()
}

/// Test every challenge across ALL registered languages.
/// Language-agnostic: iterates the pack registry × adapter registry.
#[test]
fn every_challenge_answer_compiles() {
    let pack_registry = engine::build_default_registry();
    let adapter_registry = AdapterRegistry::default_registry();

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut all_failures: Vec<(String, String, String, String)> = Vec::new(); // (lang, id, program, error)

    // For each language with both a pack AND an adapter
    for lang_id in adapter_registry.languages() {
        let Some(pack) = pack_registry.get_pack(lang_id) else { continue };
        let adapter = adapter_registry.get(lang_id).unwrap();

        eprintln!("\n── Validating {} ({} challenges) ──", lang_id, pack.challenges.len());

        for challenge in &pack.challenges {
            let fragment = challenge.answer.join(" ");
            let program = adapter
                .wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold)
                .unwrap_or_else(|| panic!(
                    "Adapter '{}' couldn't wrap {} ({:?})",
                    lang_id, challenge.id, challenge.fragment_type
                ));

            let (ok, stderr) = try_compile(&program, adapter);

            if ok {
                total_passed += 1;
            } else {
                total_failed += 1;
                all_failures.push((
                    lang_id.to_string(),
                    challenge.id.clone(),
                    program.clone(),
                    stderr,
                ));
            }
        }
    }

    if !all_failures.is_empty() {
        eprintln!("\n╔══════════════════════════════════════════════╗");
        eprintln!("║  CHALLENGE COMPILATION FAILURES               ║");
        eprintln!("╚══════════════════════════════════════════════╝\n");

        for (lang, id, program, stderr) in &all_failures {
            eprintln!("── [{}] {} ──", lang, id);
            eprintln!("Program:\n{}\n", program);
            eprintln!("Error:\n{}\n", stderr);
        }

        panic!(
            "{} of {} challenges failed to compile across {} language(s)",
            total_failed,
            total_passed + total_failed,
            adapter_registry.languages().len()
        );
    }

    eprintln!(
        "\n✓ All {} challenges compile across {} language(s)",
        total_passed,
        adapter_registry.languages().len()
    );
}

/// Test every answer variant across all languages.
#[test]
fn every_answer_variant_also_compiles() {
    let pack_registry = engine::build_default_registry();
    let adapter_registry = AdapterRegistry::default_registry();

    let mut failures: Vec<(String, String, usize, String)> = Vec::new();
    let mut checked = 0;

    for lang_id in adapter_registry.languages() {
        let Some(pack) = pack_registry.get_pack(lang_id) else { continue };
        let adapter = adapter_registry.get(lang_id).unwrap();

        for challenge in &pack.challenges {
            for (vi, variant) in challenge.answer_variants.iter().enumerate() {
                let fragment = variant.join(" ");
                let program = adapter
                    .wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold)
                    .unwrap();

                let (ok, stderr) = try_compile(&program, adapter);
                checked += 1;

                if !ok {
                    failures.push((lang_id.to_string(), challenge.id.clone(), vi, stderr));
                }
            }
        }
    }

    if !failures.is_empty() {
        for (lang, id, vi, stderr) in &failures {
            eprintln!("VARIANT FAIL: [{}] {} variant[{}]: {}", lang, id, vi, stderr);
        }
        panic!("{} of {} variants failed", failures.len(), checked);
    }

    eprintln!("✓ All {} answer variants compile", checked);
}
