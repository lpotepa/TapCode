//! Challenge Compilation Verification (PRD §8)
//!
//! This integration test reconstructs every challenge's answer into a full
//! Rust program via the compiler adapter, then invokes `rustc` to verify
//! it actually compiles. This is the CI gate — PRs that break challenges
//! cannot merge.
//!
//! Run: cargo test --test challenge_compilation
//!      (requires rustc on PATH)

use std::io::Write;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tapcode::engine;
use tapcode::models::*;
use tapcode::validator::CompilerAdapter;

/// Monotonic counter to generate unique temp file names, avoiding race conditions
/// when tests run in parallel.
static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Compile a Rust source string with rustc. Returns (success, stderr).
fn try_compile(source: &str) -> (bool, String) {
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join("tapcode_validation");
    std::fs::create_dir_all(&dir).unwrap();

    let src_path = dir.join(format!("challenge_{}.rs", n));
    let out_path = dir.join(format!("challenge_out_{}", n));

    let mut file = std::fs::File::create(&src_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();
    drop(file);

    let output = Command::new("rustc")
        .arg(&src_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--edition")
        .arg("2021")
        // Allow unused variables/imports since fragments are isolated
        .arg("-A")
        .arg("unused")
        .arg("-A")
        .arg("dead_code")
        .output()
        .expect("Failed to invoke rustc — is it on PATH?");

    // Clean up
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&out_path);

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

#[test]
fn every_challenge_answer_compiles_with_rustc() {
    let registry = engine::build_default_registry();
    let pack = registry.get_pack("rust").unwrap();
    let adapter = CompilerAdapter::rust();

    let mut failures: Vec<(String, String, String)> = Vec::new(); // (id, program, error)
    let mut passed = 0;

    for challenge in &pack.challenges {
        let fragment = challenge.answer.join(" ");
        let program = adapter
            .wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold)
            .expect(&format!(
                "Adapter couldn't wrap {} ({:?})",
                challenge.id, challenge.fragment_type
            ));

        let (ok, stderr) = try_compile(&program);

        if ok {
            passed += 1;
        } else {
            failures.push((challenge.id.clone(), program.clone(), stderr));
        }
    }

    if !failures.is_empty() {
        eprintln!("\n╔══════════════════════════════════════════════╗");
        eprintln!("║  CHALLENGE COMPILATION FAILURES               ║");
        eprintln!("╚══════════════════════════════════════════════╝\n");

        for (id, program, stderr) in &failures {
            eprintln!("── {} ──", id);
            eprintln!("Program:\n{}\n", program);
            eprintln!("Error:\n{}\n", stderr);
            eprintln!("─────────────────────────────────────\n");
        }

        panic!(
            "{} of {} challenges failed to compile. See errors above.",
            failures.len(),
            passed + failures.len()
        );
    }

    eprintln!(
        "\n✓ All {} challenges compile successfully with rustc {}",
        passed,
        String::from_utf8_lossy(
            &Command::new("rustc")
                .arg("--version")
                .output()
                .unwrap()
                .stdout
        )
        .trim()
    );
}

#[test]
fn every_answer_variant_also_compiles() {
    let registry = engine::build_default_registry();
    let pack = registry.get_pack("rust").unwrap();
    let adapter = CompilerAdapter::rust();

    let mut failures: Vec<(String, usize, String)> = Vec::new();
    let mut checked = 0;

    for challenge in &pack.challenges {
        for (vi, variant) in challenge.answer_variants.iter().enumerate() {
            let fragment = variant.join(" ");
            let program = adapter
                .wrap_fragment(&fragment, &challenge.fragment_type, &challenge.scaffold)
                .unwrap();

            let (ok, stderr) = try_compile(&program);
            checked += 1;

            if !ok {
                failures.push((challenge.id.clone(), vi, stderr));
            }
        }
    }

    if !failures.is_empty() {
        for (id, vi, stderr) in &failures {
            eprintln!("VARIANT FAIL: {} variant[{}]: {}", id, vi, stderr);
        }
        panic!(
            "{} of {} answer variants failed to compile",
            failures.len(),
            checked
        );
    }

    eprintln!("✓ All {} answer variants compile", checked);
}
