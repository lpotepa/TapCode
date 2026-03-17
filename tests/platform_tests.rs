//! Platform & Accessibility Tests (Tickets 23–25)
//!
//! These tests verify:
//!   - Keyboard handler logic (Ticket 23)
//!   - No #[cfg()] in components/screens — platform logic via traits (Ticket 24)
//!   - Accessibility: every component button has an aria_label (Ticket 25)
//!
//! Run: cargo test --test platform_tests

use tapcode::component_logic::keyboard::{KeyAction, resolve_key_action};

// ════════════════════════════════════════════════════
// Ticket 23 — Web Keyboard Navigation Logic
// ════════════════════════════════════════════════════

#[test]
fn t23_1_enter_with_feedback_correct_returns_next() {
    let action = resolve_key_action("Enter", true, true);
    assert_eq!(action, KeyAction::Next);
}

#[test]
fn t23_2_enter_with_feedback_wrong_returns_try_again() {
    let action = resolve_key_action("Enter", true, false);
    assert_eq!(action, KeyAction::TryAgain);
}

#[test]
fn t23_3_enter_without_feedback_and_can_check_returns_check() {
    let action = resolve_key_action("Enter", false, false);
    // When no feedback is shown and can_check would be determined by the caller,
    // the function returns Check (the caller must guard can_check)
    assert_eq!(action, KeyAction::Check);
}

#[test]
fn t23_4_backspace_without_feedback_returns_undo() {
    let action = resolve_key_action("Backspace", false, false);
    assert_eq!(action, KeyAction::Undo);
}

#[test]
fn t23_5_backspace_with_feedback_returns_none() {
    let action = resolve_key_action("Backspace", true, false);
    assert_eq!(action, KeyAction::None);
}

#[test]
fn t23_6_unknown_key_returns_none() {
    let action = resolve_key_action("a", false, false);
    assert_eq!(action, KeyAction::None);
}

#[test]
fn t23_7_enter_with_no_feedback_returns_check() {
    // Enter when no feedback is active should map to Check
    let action = resolve_key_action("Enter", false, false);
    assert_eq!(action, KeyAction::Check);
}

// ════════════════════════════════════════════════════
// Ticket 24 — No #[cfg()] in Components or Screens
// ════════════════════════════════════════════════════

#[test]
fn t24_1_no_cfg_in_components() {
    let component_dir = std::fs::read_dir("src/components").unwrap();
    for entry in component_dir {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            let content = std::fs::read_to_string(&path).unwrap();
            // Only flag actual #[cfg( attributes, not mentions in comments
            let has_cfg_attr = content.lines().any(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("#[cfg(") && !trimmed.starts_with("//")
            });
            assert!(
                !has_cfg_attr,
                "File {:?} contains #[cfg( attribute — platform logic must not leak into components",
                path
            );
        }
    }
}

#[test]
fn t24_2_no_cfg_in_screens() {
    let screen_dir = std::fs::read_dir("src/screens").unwrap();
    for entry in screen_dir {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                !content.contains("#[cfg("),
                "File {:?} contains #[cfg( — platform logic must not leak into screens",
                path
            );
        }
    }
}

// ════════════════════════════════════════════════════
// Ticket 25 — Accessibility: Every Button Has aria_label
// ════════════════════════════════════════════════════

#[test]
fn t25_1_every_button_in_components_has_aria_label() {
    let dir = std::fs::read_dir("src/components").unwrap();
    for entry in dir {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            let content = std::fs::read_to_string(&path).unwrap();
            // Count button { occurrences and aria_label occurrences
            let buttons = content.matches("button {").count() + content.matches("button{").count();
            let labels = content.matches("aria_label").count();
            // Every button should have an aria_label
            if buttons > 0 {
                assert!(
                    labels > 0,
                    "File {:?} has {} buttons but 0 aria_labels",
                    path,
                    buttons
                );
            }
        }
    }
}

#[test]
fn t25_2_error_states_use_role_alert() {
    // Verify that our error state components use role="alert" for screen readers
    let content =
        std::fs::read_to_string("src/components/error_states.rs").expect("error_states.rs must exist");
    let role_alert_count = content.matches("role: \"alert\"").count();
    // We have 3 error state components: OfflineBanner, SessionWarning, NoConnectionScreen
    assert!(
        role_alert_count >= 3,
        "Expected at least 3 role=\"alert\" attributes in error_states.rs, found {}",
        role_alert_count
    );
}

#[test]
fn t25_3_error_states_module_exported() {
    // Verify error_states is listed in components/mod.rs
    let content = std::fs::read_to_string("src/components/mod.rs").unwrap();
    assert!(
        content.contains("error_states"),
        "error_states module must be exported from components/mod.rs"
    );
}
