use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;

#[derive(Props, Clone, PartialEq)]
pub struct ModuleCompleteProps {
    pub module_id: u32,
}

#[component]
pub fn ModuleCompleteScreen(props: ModuleCompleteProps) -> Element {
    let state = use_context::<Signal<AppState>>();
    let nav = navigator();
    let s = state.read();

    let module = s.pack.modules.iter().find(|m| m.id == props.module_id);
    let module_title = module.map(|m| m.title.clone()).unwrap_or_default();
    let (done, _total) = s.get_module_progress(props.module_id);

    let next_module = s.pack.modules.iter().find(|m| m.id == props.module_id + 1);
    let module_xp = done as u32 * 20; // Approximate

    let should_show_paywall = props.module_id == 3; // Paywall after module 3

    rsx! {
        div {
            class: "celebration-screen",

            // Badge
            div { class: "badge-container",
                div { class: "badge-burst" }
                div { class: "badge-icon", "🏆" }
            }

            // Title
            div {
                class: "text-center",
                style: "animation: fadeInUp 0.5s ease-out 0.8s both;",
                div { class: "text-2xl font-bold", "Module {props.module_id} Complete!" }
                div { class: "text-lg text-secondary mt-xs", "{module_title}" }
            }

            // XP total
            div {
                class: "feedback-xp mt-lg",
                style: "animation: fadeInUp 0.5s ease-out 1s both; font-size: 1.25rem;",
                "⚡ +{module_xp} XP"
            }

            // Stats
            div {
                class: "grid gap-sm mt-lg w-full",
                style: "max-width: 20rem; grid-template-columns: repeat(3, 1fr); animation: fadeInUp 0.5s ease-out 1.2s both;",

                div { class: "stat-card",
                    div { class: "stat-value", "87%" }
                    div { class: "stat-label", "Accuracy" }
                }
                div { class: "stat-card",
                    div { class: "stat-value", "{done}" }
                    div { class: "stat-label", "Challenges" }
                }
                div { class: "stat-card",
                    div { class: "stat-value", "1" }
                    div { class: "stat-label", "Hints" }
                }
            }

            // Unlock
            div {
                class: "stat-card mt-lg w-full text-center",
                style: "max-width: 20rem; animation: fadeInUp 0.5s ease-out 1.4s both;",
                div { class: "text-xs text-secondary mb-xs", "UNLOCKED" }
                div { class: "font-semibold", "🎨 Free Compose — {module_title}" }
            }

            // Next module preview
            div {
                class: "mt-xl w-full flex-col gap-sm",
                style: "max-width: 20rem; animation: fadeInUp 0.5s ease-out 1.6s both;",

                if let Some(next) = next_module {
                    div { class: "text-center text-sm text-secondary mb-sm",
                        "Next up: Module {next.id}"
                    }
                    div { class: "text-center text-lg font-bold mb-xs",
                        "{next.title}"
                    }
                    div { class: "text-center text-xs text-muted mb-lg",
                        "{next.challenge_ids.len()} challenges"
                    }

                    if should_show_paywall {
                        button {
                            class: "btn btn-primary btn-wide btn-lg",
                            onclick: move |_| { let _ = nav.replace(Route::Paywall {}); },
                            "Unlock Full Curriculum"
                        }
                    } else {
                        {
                            let next_challenge = next.challenge_ids.first().cloned().unwrap_or_default();
                            rsx! {
                                button {
                                    class: "btn btn-primary btn-wide btn-lg",
                                    onclick: move |_| { let _ = nav.replace(Route::Lesson { id: next_challenge.clone() }); },
                                    "Start Module {next.id} →"
                                }
                            }
                        }
                    }
                }

                button {
                    class: "btn btn-ghost btn-wide",
                    onclick: move |_| { let _ = nav.replace(Route::Home {}); },
                    "Go Home"
                }
            }
        }
    }
}
