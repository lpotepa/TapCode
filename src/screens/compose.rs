use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;
use crate::engine;
use crate::components::*;
use crate::components::picker::ChipGroupDisplay;
use crate::models::*;
use crate::validator::AdapterRegistry;

#[derive(Props, Clone, PartialEq)]
pub struct ComposeProps {
    pub module_id: String,
}

#[component]
pub fn ComposeScreen(props: ComposeProps) -> Element {
    let state = use_context::<Signal<AppState>>();
    let nav = navigator();
    let mut assembled: Signal<Vec<(String, String)>> = use_signal(|| vec![]);
    let mut valid_state: Signal<Option<bool>> = use_signal(|| None);

    let module_id: u32 = props.module_id.parse().unwrap_or(1);
    let s = state.read();

    // Access guard: module must be complete to use Free Compose
    if !s.is_module_complete(module_id) {
        return rsx! {
            div {
                class: "app-content flex items-center justify-center",
                div {
                    class: "text-center",
                    div { class: "text-2xl mb-md", "\u{1F512}" }
                    div { class: "text-lg font-semibold mb-sm", "Module {module_id} Locked" }
                    div { class: "text-sm text-secondary mb-lg",
                        "Complete all challenges in this module to unlock Free Compose."
                    }
                    button {
                        class: "btn btn-primary btn-wide",
                        onclick: move |_| { let _ = nav.push(Route::Home {}); },
                        "\u{2190} Back Home"
                    }
                }
            }
        };
    }

    // Gather all chips from this module and earlier
    let mut all_groups: Vec<ChipGroupDisplay> = Vec::new();
    for module in s.pack.modules.iter().filter(|m| m.id <= module_id) {
        for challenge in s.pack.challenges.iter().filter(|c| c.module == module.id) {
            for cg in &challenge.chips {
                let existing = all_groups.iter_mut().find(|g| g.name == cg.group);
                if let Some(existing) = existing {
                    for token in &cg.tokens {
                        if !existing.tokens.contains(token) {
                            existing.tokens.push(token.clone());
                        }
                    }
                } else {
                    let cat = s.pack.categories.iter().find(|c| c.name == cg.group);
                    all_groups.push(ChipGroupDisplay {
                        name: cg.group.clone(),
                        display_name: cat.map(|c| c.display_name.clone()).unwrap_or(cg.group.clone()),
                        css_class: cat.map(|c| c.css_class.clone()).unwrap_or_default(),
                        tokens: cg.tokens.clone(),
                    });
                }
            }
        }
    }

    let tokens_only: Vec<String> = assembled.read().iter().map(|(t, _)| t.clone()).collect();
    let group_names: Vec<String> = all_groups.iter().map(|g| g.name.clone()).collect();
    let group_states = engine::evaluate_context(&tokens_only, &s.pack.context_rules, &group_names);
    let module_title = s.pack.modules.iter().find(|m| m.id == module_id).map(|m| m.title.clone()).unwrap_or_default();
    drop(s);

    let canvas_class = match *valid_state.read() {
        Some(true) => "canvas-valid",
        Some(false) => "canvas-invalid",
        None => "",
    };

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
                div { class: "screen-title", "Free Compose — {module_title}" }
                div {}
            }

            div {
                class: "px-lg pb-3xl flex-col gap-md",

                // Canvas
                div {
                    class: "canvas {canvas_class}",
                    for (i, (token, css_class)) in assembled.read().iter().enumerate() {
                        span {
                            class: "canvas-token {css_class}",
                            key: "compose-{i}",
                            onclick: {
                                let idx = i;
                                move |_| assembled.write().truncate(idx + 1)
                            },
                            "{token}"
                        }
                    }
                    if assembled.read().is_empty() {
                        span { class: "text-muted text-sm font-ui", style: "font-style: italic;",
                            "Build anything you want..."
                        }
                    } else {
                        span { class: "canvas-cursor" }
                    }
                }

                // Action bar
                div {
                    class: "action-bar",

                    button {
                        class: "btn btn-ghost btn-sm",
                        onclick: move |_| {
                            assembled.write().clear();
                            valid_state.set(None);
                        },
                        "🗑 Clear"
                    }

                    button {
                        class: "btn btn-secondary btn-icon",
                        onclick: move |_| { assembled.write().pop(); },
                        "⌫"
                    }

                    div { class: "flex-1" }

                    button {
                        class: "btn btn-secondary",
                        disabled: assembled.read().is_empty(),
                        onclick: move |_| {
                            let adapter_reg = AdapterRegistry::default_registry();
                            if let Some(adapter) = adapter_reg.get(&state.read().active_language) {
                                let tokens = assembled.read();
                                let fragment = tokens.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
                                let valid = adapter.wrap_fragment(&fragment, &FragmentType::Statement, "")
                                    .map(|p| adapter.validate_program_structure(&p).is_ok())
                                    .unwrap_or(false);
                                valid_state.set(Some(valid));
                            }
                        },
                        "Validate"
                    }
                }

                if let Some(valid) = *valid_state.read() {
                    if valid {
                        div { class: "text-sm text-success font-semibold", "✓ Valid syntax" }
                    } else {
                        div { class: "text-sm text-error", "Not valid — check for missing tokens" }
                    }
                }

                // Picker
                TokenPicker {
                    chip_groups: all_groups,
                    group_states: group_states,
                    used_tokens: tokens_only,
                    on_chip_tap: move |token: String| {
                        valid_state.set(None);
                        let css = state.read().pack.categories.iter()
                            .find(|c| c.tokens.contains(&token))
                            .map(|c| c.css_class.replace("chip-", "token-"))
                            .unwrap_or("token-identifier".to_string());
                        assembled.write().push((token, css));
                    },
                }
            }
        }
    }
}
