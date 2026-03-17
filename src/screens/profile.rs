use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;

#[component]
pub fn ProfileScreen() -> Element {
    let state = use_context::<Signal<AppState>>();
    let nav = navigator();
    let s = state.read();

    let total_challenges: usize = s.pack.modules.iter()
        .map(|m| m.challenge_ids.len())
        .sum();
    let completed = s.progress.completed_challenges.len();
    let accuracy = s.accuracy_percent();

    let completed_modules: Vec<_> = s.pack.modules.iter()
        .filter(|m| s.is_module_complete(m.id))
        .collect();

    rsx! {
        div {
            class: "app-content",

            div {
                class: "screen-header",
                button {
                    class: "btn btn-ghost btn-icon",
                    onclick: move |_| { let _ = nav.push(Route::Home {}); },
                    "←"
                }
                div { class: "screen-title", "Profile" }
                div {}
            }

            div {
                class: "px-lg pb-3xl flex-col gap-xl items-center",

                // Avatar
                div {
                    class: "profile-avatar",
                    "🦀"
                }

                // XP and Level
                div { class: "text-center",
                    div { class: "text-3xl font-extrabold text-accent", "{s.user.total_xp}" }
                    div { class: "text-sm text-secondary", "Total XP" }
                }

                // Stats grid
                div {
                    class: "grid gap-sm w-full",
                    style: "grid-template-columns: repeat(3, 1fr);",

                    div { class: "stat-card",
                        div { class: "stat-value", "🔥 {s.user.current_streak}" }
                        div { class: "stat-label", "Streak" }
                    }
                    div { class: "stat-card",
                        div { class: "stat-value", "{s.user.longest_streak}" }
                        div { class: "stat-label", "Best Streak" }
                    }
                    div { class: "stat-card",
                        div { class: "stat-value", "{accuracy}%" }
                        div { class: "stat-label", "Accuracy" }
                    }
                }

                // Progress
                div {
                    class: "stat-card w-full",
                    div { class: "flex justify-between items-center mb-sm",
                        span { class: "font-semibold", "Rust Progress" }
                        span { class: "text-sm text-secondary", "{completed}/{total_challenges}" }
                    }
                    div {
                        class: "progress-bar",
                        div {
                            class: "progress-fill",
                            style: "width: {(completed as f64 / total_challenges.max(1) as f64 * 100.0) as u32}%",
                        }
                    }
                }

                // Badges
                if !completed_modules.is_empty() {
                    div {
                        class: "w-full",
                        div { class: "text-sm font-semibold text-secondary mb-sm", "BADGES" }
                        div {
                            class: "flex flex-wrap gap-sm",
                            for module in completed_modules.iter() {
                                div {
                                    key: "badge-{module.id}",
                                    class: "stat-card",
                                    style: "min-width: 6rem;",
                                    div { class: "text-2xl mb-xs", "🏆" }
                                    div { class: "text-xs font-semibold", "{module.title}" }
                                }
                            }
                        }
                    }
                }

                // Account section
                div {
                    class: "w-full mt-lg",
                    div { class: "text-sm font-semibold text-secondary mb-sm", "ACCOUNT" }
                    div { class: "stat-card",
                        div { class: "text-sm text-secondary mb-md",
                            "You're using an anonymous session. Create an account to save your progress across devices."
                        }
                        button {
                            class: "btn btn-primary btn-wide btn-sm",
                            "Create Account"
                        }
                    }
                }
            }
        }
    }
}
