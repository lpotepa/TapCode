use dioxus::prelude::*;
use crate::models::HintTier;

// TODO: Keyboard shortcuts (Ticket 11, web-only)
// On web (#[cfg(target_arch = "wasm32")]), attach a document "keydown" listener:
//   - 1-9: select chip at that index within the active/visible group
//   - Enter: call on_check (if can_check is true)
//   - Backspace: call on_undo
//   - Tab: move focus between chip groups
// Only active when feedback == FeedbackKind::None.
// Implementation: use web_sys::EventListener on document, wire to EventHandlers.

#[derive(Props, Clone, PartialEq)]
pub struct ActionBarProps {
    pub can_check: bool,
    pub hint_tier: HintTier,
    pub on_check: EventHandler<()>,
    pub on_undo: EventHandler<()>,
    pub on_hint: EventHandler<()>,
}

#[component]
pub fn ActionBar(props: ActionBarProps) -> Element {
    let hint_label = match props.hint_tier {
        HintTier::None => "💡 Hint",
        HintTier::Concept => "💡 Hint (1/3)",
        HintTier::Structural => "💡 Hint (2/3)",
        HintTier::SkipAvailable => "⏭ Skip",
    };

    rsx! {
        div {
            class: "action-bar",
            role: "toolbar",
            aria_label: "Challenge actions",

            // Hint button
            button {
                class: "btn btn-ghost btn-sm",
                aria_label: "Get hint",
                onclick: move |_| props.on_hint.call(()),
                "{hint_label}"
            }

            // Undo button
            button {
                class: "btn btn-secondary btn-icon",
                aria_label: "Undo last token",
                onclick: move |_| props.on_undo.call(()),
                "⌫"
            }

            // Spacer
            div { class: "flex-1" }

            // Check button
            button {
                class: "btn btn-primary",
                disabled: !props.can_check,
                aria_label: "Check answer",
                onclick: move |_| {
                    if props.can_check {
                        props.on_check.call(());
                    }
                },
                "Check ✓"
            }
        }
    }
}
