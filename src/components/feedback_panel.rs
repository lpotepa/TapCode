use dioxus::prelude::*;
use crate::models::FeedbackKind;

#[derive(Props, Clone, PartialEq)]
pub struct FeedbackPanelProps {
    pub feedback: FeedbackKind,
    pub on_next: EventHandler<()>,
    pub on_try_again: EventHandler<()>,
}

#[component]
pub fn FeedbackPanel(props: FeedbackPanelProps) -> Element {
    match &props.feedback {
        FeedbackKind::None => rsx! {},

        FeedbackKind::Correct { xp_awarded, explanation } => {
            let xp = *xp_awarded;
            let explanation = explanation.clone();

            rsx! {
                div {
                    class: "feedback-overlay",
                    div {
                        class: "feedback-panel feedback-correct",
                        role: "alert",
                        aria_live: "assertive",

                        div {
                            class: "feedback-heading feedback-heading-correct",
                            span { "✓" }
                            " Correct!"
                        }

                        div { class: "feedback-xp",
                            "⚡ +{xp} XP"
                        }

                        p { class: "feedback-explanation",
                            "{explanation}"
                        }

                        button {
                            class: "btn btn-primary btn-wide",
                            aria_label: "Next challenge",
                            onclick: move |_| props.on_next.call(()),
                            "Next →"
                        }
                    }
                }
            }
        }

        FeedbackKind::Wrong { explanation, .. } => {
            let explanation = explanation.clone();

            rsx! {
                div {
                    class: "feedback-overlay",
                    div {
                        class: "feedback-panel feedback-wrong",
                        role: "alert",
                        aria_live: "assertive",

                        div {
                            class: "feedback-heading feedback-heading-wrong",
                            "Not quite"
                        }

                        p { class: "feedback-explanation",
                            "{explanation}"
                        }

                        button {
                            class: "btn btn-secondary btn-wide",
                            aria_label: "Try again",
                            onclick: move |_| props.on_try_again.call(()),
                            "Try Again"
                        }
                    }
                }
            }
        }
    }
}
