use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;
use crate::components::*;
use crate::engine;

#[component]
pub fn HomeScreen() -> Element {
    let state = use_context::<Signal<AppState>>();
    let nav = navigator();
    let s = state.read();

    let next_id = s.get_next_challenge_id();
    let all_complete = next_id.is_none();

    rsx! {
        div {
            class: "app-content",

            // Header
            div {
                class: "screen-header",
                div {
                    class: "flex items-center gap-sm",
                    span { class: "text-2xl font-extrabold text-accent", "TapCode" }
                    span { class: "text-sm", "🦀" }
                }
                XpDisplay { xp: s.user.total_xp }
            }

            div {
                class: "flex-col gap-xl px-lg pb-3xl",

                // Streak section — dominant visual
                div {
                    class: "py-xl",
                    StreakDisplay {
                        count: s.user.current_streak,
                        days: s.user.streak_days.clone(),
                        today_filled: s.user.streak_days.last().copied().unwrap_or(false),
                        has_freeze: s.user.has_freeze,
                        previous_streak: if s.user.current_streak == 0 && s.user.longest_streak > 0 {
                            Some(s.user.longest_streak)
                        } else {
                            None
                        },
                    }
                }

                // Continue CTA
                if !all_complete {
                    if let Some(ref challenge_id) = next_id {
                        button {
                            class: "btn btn-primary btn-wide btn-lg",
                            aria_label: "Continue to next challenge",
                            onclick: {
                                let id = challenge_id.clone();
                                move |_| {
                                    let _ = nav.push(Route::Lesson { id: id.clone() });
                                }
                            },
                            "Continue →"
                        }
                    }
                } else {
                    div {
                        class: "stat-card text-center p-xl",
                        div { class: "text-2xl mb-sm", "🎉" }
                        div { class: "text-lg font-bold", "You've completed all available Rust modules!" }
                        div { class: "text-sm text-secondary mt-sm",
                            "Total XP: {s.user.total_xp} · Challenges solved: {s.progress.completed_challenges.len()}"
                        }
                        button {
                            class: "btn btn-secondary btn-sm mt-lg mx-auto",
                            "Notify me when new content drops"
                        }
                    }
                }

                // Module list
                div {
                    class: "flex-col gap-sm mt-lg",

                    div { class: "text-sm font-semibold text-secondary mb-sm",
                        "MODULES"
                    }

                    for module in s.pack.modules.iter() {
                        {
                            let m_id = module.id;
                            let is_unlocked = s.is_module_unlocked(m_id);
                            let is_complete = s.is_module_complete(m_id);
                            let is_paid = s.is_module_paid(m_id);
                            let (done, total) = s.get_module_progress(m_id);

                            let card_class = if !is_unlocked {
                                "module-card module-card-locked"
                            } else if is_complete {
                                "module-card module-card-completed"
                            } else {
                                "module-card"
                            };

                            let num_class = if is_complete {
                                "module-number module-number-complete"
                            } else if is_unlocked {
                                "module-number module-number-active"
                            } else {
                                "module-number"
                            };

                            rsx! {
                                button {
                                    key: "module-{m_id}",
                                    class: "{card_class}",
                                    aria_label: "Module {m_id}: {module.title} — {done} of {total} complete",
                                    disabled: !is_unlocked,
                                    onclick: move |_| {
                                        if is_unlocked {
                                            let _ = nav.push(Route::ModuleMap { id: m_id.to_string() });
                                        }
                                    },

                                    div { class: "{num_class}",
                                        if is_complete {
                                            "✓"
                                        } else {
                                            "{m_id}"
                                        }
                                    }

                                    div { class: "module-info",
                                        div { class: "module-title", "{module.title}" }
                                        div { class: "module-meta",
                                            "{done}/{total} challenges"
                                            if !module.is_free { " · Premium" }
                                        }
                                    }

                                    if !is_unlocked {
                                        span { class: "module-lock-icon",
                                            if is_paid { "💎" } else { "🔒" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Language selector
                div {
                    class: "mt-xl",

                    div { class: "text-sm font-semibold text-secondary mb-sm",
                        "LANGUAGE"
                    }

                    div { class: "flex gap-sm",
                        div {
                            class: "language-card",
                            style: "max-width: none; animation: none; padding: 1rem;",
                            div { class: "flex items-center gap-sm",
                                span { class: "text-2xl", "🦀" }
                                div {
                                    div { class: "font-bold", "Rust" }
                                    div { class: "text-xs text-secondary", "10 modules · 50 challenges" }
                                }
                            }
                        }

                        div {
                            class: "language-card language-card-ghost",
                            style: "max-width: none; animation: none; padding: 1rem; flex: 1;",
                            div { class: "text-sm text-muted", "More languages coming soon" }
                        }
                    }
                }
            }
        }
    }
}
