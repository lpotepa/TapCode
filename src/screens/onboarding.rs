use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;
use crate::models::OnboardingStep;

#[component]
pub fn OnboardingScreen() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let nav = navigator();
    let mut step: Signal<OnboardingStep> = use_signal(|| OnboardingStep::Splash);

    let current_step = step.read().clone();
    match current_step {
        OnboardingStep::Splash => {
            // Auto-advance after splash
            rsx! {
                div {
                    class: "onboarding-screen",
                    onclick: move |_| step.set(OnboardingStep::LanguageSelect),

                    div { class: "onboarding-logo", "TapCode" }
                    div { class: "onboarding-tagline", "Build code by tapping" }
                }
            }
        }

        OnboardingStep::LanguageSelect => {
            rsx! {
                div {
                    class: "onboarding-screen",

                    div { class: "text-2xl font-bold mb-sm", style: "animation: fadeInUp 0.4s ease-out both;",
                        "What do you want to learn?"
                    }
                    div { class: "text-sm text-secondary mb-xl", style: "animation: fadeInUp 0.4s ease-out 0.1s both;",
                        "Pick a language to get started"
                    }

                    div { class: "flex-col gap-md w-full items-center",
                        // Rust card
                        button {
                            class: "language-card",
                            onclick: move |_| step.set(OnboardingStep::SkillCheck),

                            div { class: "flex items-center gap-md",
                                span { class: "text-4xl", "🦀" }
                                div { class: "text-left",
                                    div { class: "text-xl font-bold text-accent", "Rust" }
                                    div { class: "text-sm text-secondary", "Systems programming. Memory safe." }
                                }
                            }
                        }

                        // Ghost card
                        div {
                            class: "language-card language-card-ghost",
                            div { class: "flex items-center gap-md",
                                span { class: "text-4xl", "🔮" }
                                div { class: "text-left",
                                    div { class: "text-lg font-bold text-muted", "More languages" }
                                    div { class: "text-sm text-muted", "Coming soon" }
                                }
                            }
                        }
                    }
                }
            }
        }

        OnboardingStep::SkillCheck => {
            rsx! {
                div {
                    class: "onboarding-screen",

                    div { class: "text-2xl font-bold mb-lg", style: "animation: fadeInUp 0.4s ease-out both;",
                        "Have you written any Rust before?"
                    }

                    div { class: "flex-col gap-sm w-full items-center",
                        button {
                            class: "skill-option",
                            style: "animation: fadeInUp 0.3s ease-out 0.1s both;",
                            onclick: move |_| {
                                state.write().is_onboarded = true;
                                let _ = nav.replace(Route::Lesson { id: "rust-m1-c1".into() });
                            },
                            "Never — start from scratch"
                        }
                        button {
                            class: "skill-option",
                            style: "animation: fadeInUp 0.3s ease-out 0.2s both;",
                            onclick: move |_| {
                                state.write().is_onboarded = true;
                                let _ = nav.replace(Route::Lesson { id: "rust-m1-c3".into() });
                            },
                            "A little — I've seen some Rust"
                        }
                        button {
                            class: "skill-option",
                            style: "animation: fadeInUp 0.3s ease-out 0.3s both;",
                            onclick: move |_| {
                                state.write().is_onboarded = true;
                                let _ = nav.replace(Route::Lesson { id: "rust-m2-c1".into() });
                            },
                            "Yes — I know the basics"
                        }

                        button {
                            class: "btn btn-ghost btn-sm mt-lg",
                            style: "animation: fadeIn 0.3s ease-out 0.5s both;",
                            onclick: move |_| {
                                state.write().is_onboarded = true;
                                let _ = nav.replace(Route::Lesson { id: "rust-m1-c1".into() });
                            },
                            "Skip →"
                        }
                    }
                }
            }
        }

        _ => {
            rsx! {}
        }
    }
}
