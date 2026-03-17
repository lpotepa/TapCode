use dioxus::prelude::*;
use crate::models::*;
use crate::component_logic::canvas::compute_indent_styles;

#[derive(Props, Clone, PartialEq)]
pub struct CanvasProps {
    pub tokens: Vec<(String, String)>, // (token, css_class)
    #[props(default)]
    pub diff: Option<Vec<TokenDiff>>,
    #[props(default)]
    pub ghost_hint: Option<Vec<String>>,
    #[props(default = false)]
    pub show_diff: bool,
    pub on_token_tap: EventHandler<usize>,
}

#[component]
pub fn CodeCanvas(props: CanvasProps) -> Element {
    let is_empty = props.tokens.is_empty() && props.ghost_hint.is_none();

    // Compute indentation styles based on brace depth
    let indent_styles = compute_indent_styles(&props.tokens);

    rsx! {
        div {
            class: "canvas",
            role: "region",
            aria_label: "Code canvas — assembled tokens",

            if props.show_diff {
                // Diff mode: read-only display, no token tap handlers
                if let Some(ref diff) = props.diff {
                    for (i, d) in diff.iter().enumerate() {
                        match d {
                            TokenDiff::Match(t) => rsx! {
                                span {
                                    class: "canvas-token token-correct",
                                    key: "diff-{i}",
                                    style: "animation-delay: {i as u32 * 80}ms",
                                    aria_label: "{t} — correct",
                                    "✓ {t}"
                                }
                            },
                            TokenDiff::Wrong { got, expected } => rsx! {
                                span {
                                    class: "canvas-token token-wrong",
                                    key: "diff-{i}",
                                    aria_label: "{got} — wrong, expected {expected}",
                                    title: "Expected: {expected}",
                                    "✗ {got}"
                                }
                            },
                            TokenDiff::Extra(t) => rsx! {
                                span {
                                    class: "canvas-token token-extra",
                                    key: "diff-{i}",
                                    aria_label: "{t} — extra token",
                                    "{t}"
                                }
                            },
                            TokenDiff::Missing(t) => rsx! {
                                span {
                                    class: "canvas-token token-missing",
                                    key: "diff-{i}",
                                    aria_label: "missing token: {t}",
                                    "{t}"
                                }
                            },
                        }
                    }
                }
            } else {
                // Normal token display with indentation
                for (i, (token, css_class)) in props.tokens.iter().enumerate() {
                    {
                        let indent = indent_styles.get(i)
                            .cloned()
                            .unwrap_or_else(|| "margin-left: 0rem".to_string());

                        rsx! {
                            span {
                                class: "canvas-token {css_class}",
                                key: "token-{i}",
                                style: "{indent}",
                                role: "button",
                                aria_label: "{token} at position {i} — tap to backtrack",
                                tabindex: 0,
                                onclick: {
                                    let idx = i;
                                    let show_diff = props.show_diff;
                                    move |_| {
                                        // Guard: do not fire tap in diff mode
                                        if !show_diff {
                                            props.on_token_tap.call(idx);
                                        }
                                    }
                                },
                                "{token}"
                            }
                        }
                    }
                }

                // Ghost hint overlay
                if let Some(ref ghosts) = props.ghost_hint {
                    for (i, ghost) in ghosts.iter().enumerate().skip(props.tokens.len()) {
                        span {
                            class: "canvas-token token-ghost",
                            key: "ghost-{i}",
                            aria_label: "hint placeholder",
                            "{ghost}"
                        }
                    }
                }

                // Blinking cursor
                span {
                    class: "canvas-cursor",
                    aria_hidden: "true",
                }
            }

            // Empty state
            if is_empty && !props.show_diff {
                span {
                    class: "text-muted text-sm font-ui",
                    style: "font-style: italic;",
                    "Tap chips below to build code..."
                }
            }
        }
    }
}
