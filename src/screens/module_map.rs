use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;
use crate::engine;

#[derive(Props, Clone, PartialEq)]
pub struct ModuleMapProps {
    pub id: String,
}

#[component]
pub fn ModuleMapScreen(props: ModuleMapProps) -> Element {
    let state = use_context::<Signal<AppState>>();
    let nav = navigator();
    let s = state.read();

    let module_id: u32 = props.id.parse().unwrap_or(1);
    let module = s.pack.modules.iter().find(|m| m.id == module_id);

    let Some(module) = module else {
        return rsx! {
            div { class: "app-content flex items-center justify-center",
                "Module not found"
            }
        };
    };

    let is_complete = s.is_module_complete(module_id);
    let challenges = engine::get_module_challenges(&s.pack, module_id);

    rsx! {
        div {
            class: "app-content",

            div {
                class: "screen-header",
                button {
                    class: "btn btn-ghost btn-icon",
                    aria_label: "Back",
                    onclick: move |_| { let _ = nav.push(Route::Home {}); },
                    "←"
                }
                div { class: "screen-title", "Module {module_id}" }
                div {}
            }

            div {
                class: "px-lg pb-3xl flex-col gap-lg",

                // Module header
                div {
                    class: "text-center py-lg",
                    if is_complete {
                        div { class: "text-4xl mb-sm", "🏆" }
                    }
                    div { class: "text-2xl font-bold", "{module.title}" }
                    div { class: "text-sm text-secondary mt-xs", "{module.description}" }
                }

                // Free Compose button (if complete)
                if is_complete {
                    button {
                        class: "btn btn-secondary btn-wide",
                        aria_label: "Open free compose mode",
                        onclick: move |_| { let _ = nav.push(Route::FreeCompose { module_id: module_id.to_string() }); },
                        "🎨 Free Compose"
                    }
                }

                // Challenge list
                div {
                    class: "flex-col gap-sm",
                    div { class: "text-sm font-semibold text-secondary mb-sm", "CHALLENGES" }

                    for challenge in challenges.iter() {
                        {
                            let is_done = s.progress.completed_challenges.contains(&challenge.id);
                            let is_skipped = s.progress.skipped_challenges.contains(&challenge.id);
                            let c_id = challenge.id.clone();

                            let status_icon = if is_done {
                                "✓"
                            } else if is_skipped {
                                "⏭"
                            } else {
                                "○"
                            };

                            let status_color = if is_done {
                                "text-success"
                            } else if is_skipped {
                                "text-warning"
                            } else {
                                "text-muted"
                            };

                            rsx! {
                                button {
                                    key: "{c_id}",
                                    class: "module-card",
                                    onclick: {
                                        let id = c_id.clone();
                                        move |_| { let _ = nav.push(Route::Lesson { id: id.clone() }); }
                                    },

                                    div { class: "module-number {status_color}",
                                        "{status_icon}"
                                    }

                                    div { class: "module-info",
                                        div { class: "module-title", "{challenge.title}" }
                                        div { class: "module-meta",
                                            "{challenge.prompt}"
                                        }
                                    }

                                    if is_done {
                                        span { class: "text-xs text-success font-semibold",
                                            "+{challenge.xp} XP"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Next module preview
                {
                    let next = s.pack.modules.iter().find(|m| m.id == module_id + 1);
                    rsx! {
                        if let Some(next_module) = next {
                            div {
                                class: "stat-card mt-lg",
                                div { class: "text-xs text-muted mb-xs", "NEXT UP" }
                                div { class: "text-lg font-bold", "Module {next_module.id} — {next_module.title}" }
                                div { class: "text-sm text-secondary",
                                    "{next_module.challenge_ids.len()} challenges"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
