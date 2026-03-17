use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;
use crate::models::*;
use crate::engine::{self, validate_answer, get_challenge_by_id, get_token_category, xp_for_attempt};
use crate::components::*;
use crate::components::picker::ChipGroupDisplay;
use crate::component_logic::keyboard::{KeyAction, resolve_key_action};

#[derive(Props, Clone, PartialEq)]
pub struct LessonProps {
    pub id: String,
}

#[component]
pub fn LessonScreen(props: LessonProps) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let nav = navigator();

    // Lesson-local state
    let mut assembled: Signal<Vec<(String, String)>> = use_signal(|| vec![]); // (token, css_class)
    let mut feedback: Signal<FeedbackKind> = use_signal(|| FeedbackKind::None);
    let mut hint_tier: Signal<HintTier> = use_signal(|| HintTier::None);
    let mut attempt_num: Signal<u32> = use_signal(|| 1);
    let mut show_diff: Signal<bool> = use_signal(|| false);
    let mut diff_data: Signal<Option<Vec<TokenDiff>>> = use_signal(|| None);
    let mut show_confetti: Signal<bool> = use_signal(|| false);
    let mut xp_bouncing: Signal<bool> = use_signal(|| false);
    let mut xp_float: Signal<Option<u32>> = use_signal(|| None);
    let mut ghost_hint: Signal<Option<Vec<String>>> = use_signal(|| None);
    let mut show_module_complete: Signal<bool> = use_signal(|| false);

    // Keyboard action signal (Ticket 23): set in onkeydown, dispatched before render
    let mut pending_key_action: Signal<Option<KeyAction>> = use_signal(|| None);

    let s = state.read();
    let challenge = get_challenge_by_id(&s.pack, &props.id);

    let Some(challenge) = challenge else {
        return rsx! {
            div { class: "app-content flex items-center justify-center",
                div { class: "text-center",
                    div { class: "text-2xl mb-md", "\u{1F50D}" }
                    div { class: "text-lg", "Challenge not found" }
                    button {
                        class: "btn btn-secondary mt-lg",
                        onclick: move |_| { let _ = nav.push(Route::Home {}); },
                        "\u{2190} Back Home"
                    }
                }
            }
        };
    };

    let challenge = challenge.clone();
    let module = s.pack.modules.iter().find(|m| m.id == challenge.module).cloned();
    let module_title = module.as_ref().map(|m| m.title.clone()).unwrap_or_default();
    let (done, total) = s.get_module_progress(challenge.module);

    // Build chip groups for the picker
    let chip_groups: Vec<ChipGroupDisplay> = challenge.chips.iter().map(|cg| {
        let cat = s.pack.categories.iter().find(|c| c.name == cg.group);
        ChipGroupDisplay {
            name: cg.group.clone(),
            display_name: cat.map(|c| c.display_name.clone()).unwrap_or(cg.group.clone()),
            css_class: cat.map(|c| c.css_class.clone()).unwrap_or_default(),
            tokens: cg.tokens.clone(),
        }
    }).collect();

    // Compute context-based chip highlighting
    let tokens_only: Vec<String> = assembled.read().iter().map(|(t, _)| t.clone()).collect();
    let all_groups: Vec<String> = challenge.chips.iter().map(|c| c.group.clone()).collect();
    let group_states = engine::evaluate_context(&tokens_only, &s.pack.context_rules, &all_groups);
    let used_tokens: Vec<String> = tokens_only.clone();
    let total_xp = s.user.total_xp;

    // Drop the borrow before callbacks
    drop(s);

    // ── Callbacks ──

    let _challenge_for_tap = challenge.clone();
    let on_chip_tap = move |token: String| {
        if *show_diff.read() {
            show_diff.set(false);
            diff_data.set(None);
        }
        if *feedback.read() != FeedbackKind::None {
            return;
        }

        // Ticket 21: Clear ghost hint overlay when user starts tapping after viewing structural hint
        if ghost_hint.read().is_some() {
            ghost_hint.set(None);
        }

        let css = get_token_category(&state.read().pack, &token)
            .map(|c| c.css_class.replace("chip-", "token-"))
            .unwrap_or("token-identifier".to_string());

        assembled.write().push((token, css));
    };

    let challenge_for_check = challenge.clone();
    let mut on_check = move |()| {
        let user_tokens: Vec<String> = assembled.read().iter().map(|(t, _)| t.clone()).collect();
        let result = validate_answer(&user_tokens, &challenge_for_check);

        match result {
            ValidationResult::Correct => {
                let xp = xp_for_attempt(challenge_for_check.xp, *attempt_num.read());
                show_confetti.set(true);
                xp_bouncing.set(true);
                xp_float.set(Some(xp));

                state.write().add_xp(xp);
                state.write().complete_challenge(&challenge_for_check.id);
                state.write().fill_streak_today();
                state.write().record_attempt(true);

                feedback.set(FeedbackKind::Correct {
                    xp_awarded: xp,
                    explanation: challenge_for_check.explanation.clone(),
                });
            }
            ValidationResult::Wrong(diff) => {
                state.write().record_attempt(false);
                show_diff.set(true);
                diff_data.set(Some(diff.clone()));
                feedback.set(FeedbackKind::Wrong {
                    diff,
                    explanation: challenge_for_check.hint_concept.clone(),
                });
            }
        }
    };

    let challenge_for_next = challenge.clone();
    let mut on_next = move |()| {
        show_confetti.set(false);
        xp_bouncing.set(false);
        xp_float.set(None);
        feedback.set(FeedbackKind::None);
        assembled.write().clear();
        show_diff.set(false);
        diff_data.set(None);
        hint_tier.set(HintTier::None);
        attempt_num.set(1);
        ghost_hint.set(None);

        // Check if module is complete
        let s = state.read();
        let module = s.pack.modules.iter().find(|m| m.id == challenge_for_next.module);
        if let Some(module) = module {
            if engine::is_module_complete(module, &s.progress) {
                show_module_complete.set(true);
                return;
            }
        }

        // Navigate to next challenge
        if let Some(next_id) = s.get_next_challenge_id() {
            drop(s);
            let _ = nav.push(Route::Lesson { id: next_id });
        } else {
            drop(s);
            let _ = nav.push(Route::Home {});
        }
    };

    let mut on_try_again = move |()| {
        feedback.set(FeedbackKind::None);
        // Ticket 18: Clear confetti on try again (belt-and-suspenders)
        show_confetti.set(false);
        // Ticket 19: Keep show_diff=true and diff_data intact so user sees what's wrong.
        // Keep assembled tokens — user taps wrong token to backtrack from there.
        let current = *attempt_num.read();
        attempt_num.set(current + 1);
    };

    let mut on_undo = move |()| {
        if *feedback.read() != FeedbackKind::None { return; }
        assembled.write().pop();
        if *show_diff.read() {
            show_diff.set(false);
            diff_data.set(None);
        }
    };

    let challenge_for_hint = challenge.clone();
    let on_hint = move |()| {
        let current = hint_tier.read().clone();
        let next = current.next();

        // Ticket 21: Deduct XP for hints (first per session is free, handled inside deduct_hint_xp)
        if next == HintTier::Concept || next == HintTier::Structural {
            state.write().deduct_hint_xp();
        }

        match &next {
            HintTier::Concept => {
                // Just advance tier, hint will show via feedback
            }
            HintTier::Structural => {
                // Show ghost overlay
                let parts: Vec<String> = challenge_for_hint.hint_structural
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                ghost_hint.set(Some(parts));
            }
            HintTier::SkipAvailable => {
                // Skip challenge
                state.write().skipped_challenges_mut(&challenge_for_hint.id);
                let _ = nav.push(Route::Home {});
                return;
            }
            _ => {}
        }

        hint_tier.set(next);
    };

    // Module complete overlay
    if *show_module_complete.read() {
        return rsx! {
            crate::screens::module_complete::ModuleCompleteScreen {
                module_id: challenge.module,
            }
        };
    }

    let can_check = !assembled.read().is_empty() && *feedback.read() == FeedbackKind::None;

    // Dispatch pending keyboard action (Ticket 23)
    // The signal is set in onkeydown, then consumed here on the next render cycle.
    if let Some(action) = pending_key_action.take() {
        match action {
            KeyAction::Check => {
                if can_check {
                    on_check(());
                }
            }
            KeyAction::Undo => on_undo(()),
            KeyAction::Next => on_next(()),
            KeyAction::TryAgain => on_try_again(()),
            KeyAction::None => {}
        }
    }

    rsx! {
        div {
            class: "lesson-layout",
            tabindex: 0,
            onkeydown: move |evt: Event<KeyboardData>| {
                let key = evt.data().key().to_string();
                let has_feedback = *feedback.read() != FeedbackKind::None;
                let feedback_is_correct = matches!(&*feedback.read(), FeedbackKind::Correct { .. });
                let action = resolve_key_action(&key, has_feedback, feedback_is_correct);
                if action != KeyAction::None {
                    pending_key_action.set(Some(action));
                }
            },

            // ── Header ──
            div {
                class: "lesson-header",
                div {
                    class: "screen-header",
                    div {
                        class: "flex items-center gap-sm",
                        button {
                            class: "btn btn-ghost btn-icon",
                            aria_label: "Back to home",
                            onclick: move |_| { let _ = nav.push(Route::Home {}); },
                            "\u{2190}"
                        }
                        div {
                            div { class: "text-sm font-semibold", "{module_title}" }
                            div { class: "text-xs text-muted", "Challenge {challenge.position}/{total}" }
                        }
                    }
                    XpDisplay {
                        xp: total_xp,
                        bouncing: *xp_bouncing.read(),
                        float_amount: *xp_float.read(),
                    }
                }
                div { class: "px-lg",
                    ProgressBar { current: done, total: total }
                }
            }

            // ── Left: Prompt + Canvas ──
            div {
                class: "lesson-left flex-col gap-md",

                // Prompt
                div {
                    class: "prompt",
                    role: "heading",
                    aria_level: "2",
                    "{challenge.prompt}"
                }

                // Canvas
                CodeCanvas {
                    tokens: assembled.read().clone(),
                    diff: diff_data.read().clone(),
                    ghost_hint: ghost_hint.read().clone(),
                    show_diff: *show_diff.read(),
                    on_token_tap: move |idx: usize| {
                        if *feedback.read() != FeedbackKind::None { return; }
                        // Backtrack: remove everything after this index
                        let mut tokens = assembled.write();
                        tokens.truncate(idx + 1);
                        // Ticket 19: Clear diff when user starts editing after a wrong answer
                        if *show_diff.read() {
                            show_diff.set(false);
                            diff_data.set(None);
                        }
                    },
                }

                // Hint card (tier 1)
                if *hint_tier.read() == HintTier::Concept {
                    div {
                        class: "hint-card",
                        role: "complementary",
                        aria_label: "Concept hint",

                        div { class: "hint-cost",
                            if *attempt_num.read() <= 1 { "Free" } else { "-5 XP" }
                        }

                        div { class: "text-sm font-semibold text-warning mb-sm", "\u{1F4A1} Hint" }
                        p { class: "text-sm text-secondary", "{challenge.hint_concept}" }
                        button {
                            class: "btn btn-ghost btn-sm mt-md",
                            onclick: move |_| hint_tier.set(HintTier::None),
                            "Got it"
                        }
                    }
                }
            }

            // ── Right: Picker ──
            div {
                class: "lesson-right",
                TokenPicker {
                    chip_groups: chip_groups,
                    group_states: group_states,
                    used_tokens: used_tokens,
                    on_chip_tap: on_chip_tap,
                }
            }

            // ── Action bar ──
            div {
                class: "lesson-action",
                ActionBar {
                    can_check: can_check,
                    hint_tier: hint_tier.read().clone(),
                    on_check: on_check,
                    on_undo: on_undo,
                    on_hint: on_hint,
                }
            }
        }

        // ── Overlays ──
        Confetti { active: *show_confetti.read() }

        FeedbackPanel {
            feedback: feedback.read().clone(),
            on_next: on_next,
            on_try_again: on_try_again,
        }
    }
}
