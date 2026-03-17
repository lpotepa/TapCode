use dioxus::prelude::*;
use crate::route::Route;

#[component]
pub fn PaywallScreen() -> Element {
    let nav = navigator();

    rsx! {
        div {
            class: "paywall-screen",

            // Back button — always works, no trapping
            button {
                class: "paywall-back btn btn-ghost btn-sm",
                aria_label: "Go back",
                onclick: move |_| { let _ = nav.push(Route::Home {}); },
                "← Back"
            }

            div { class: "flex-col gap-xl items-center flex-1",

                // Headline
                div { class: "text-center",
                    div { class: "text-3xl font-extrabold mb-sm",
                        style: "animation: fadeInUp 0.5s ease-out both;",
                        "Keep going with Rust"
                    }
                    div { class: "text-secondary",
                        style: "animation: fadeInUp 0.5s ease-out 0.1s both;",
                        "Unlock the full curriculum"
                    }
                }

                // What you've learned
                div { class: "w-full",
                    style: "max-width: 24rem; animation: fadeInUp 0.5s ease-out 0.2s both;",

                    div { class: "text-sm font-semibold text-secondary mb-sm", "YOU'VE LEARNED" }

                    div { class: "flex-col gap-xs",
                        div { class: "flex items-center gap-sm text-success",
                            span { "✓" } span { "First Output" }
                        }
                        div { class: "flex items-center gap-sm text-success",
                            span { "✓" } span { "Variables & Bindings" }
                        }
                        div { class: "flex items-center gap-sm text-success",
                            span { "✓" } span { "Functions" }
                        }
                    }
                }

                // What you'll unlock
                div { class: "w-full",
                    style: "max-width: 24rem; animation: fadeInUp 0.5s ease-out 0.3s both;",

                    div { class: "text-sm font-semibold text-secondary mb-sm", "UNLOCKS" }

                    div { class: "flex-col gap-xs",
                        div { class: "flex items-center gap-sm text-primary",
                            span { "→" } span { "Control Flow" }
                        }
                        div { class: "flex items-center gap-sm text-primary",
                            span { "→" } span { "Ownership (the hard part)" }
                        }
                        div { class: "flex items-center gap-sm text-primary",
                            span { "→" } span { "Structs" }
                        }
                        div { class: "flex items-center gap-sm text-primary",
                            span { "→" } span { "Enums & Error Handling" }
                        }
                        div { class: "flex items-center gap-sm text-primary",
                            span { "→" } span { "Traits & Collections" }
                        }
                        div { class: "flex items-center gap-sm text-accent font-semibold",
                            span { "+" } span { "All future languages" }
                        }
                    }
                }

                // Pricing cards
                div {
                    class: "flex-col gap-sm w-full",
                    style: "max-width: 24rem; animation: fadeInUp 0.5s ease-out 0.4s both;",

                    // Annual — featured
                    button {
                        class: "pricing-card pricing-card-featured",
                        div { class: "pricing-badge", "Best Value" }
                        div { class: "pricing-amount", "€4.99" }
                        div { class: "pricing-period", "per year" }
                    }

                    // Lifetime
                    button {
                        class: "pricing-card",
                        div { class: "pricing-amount", "€9.99" }
                        div { class: "pricing-period", "lifetime · one-time" }
                    }

                    // Monthly
                    button {
                        class: "pricing-card",
                        div { class: "pricing-amount", "€1.99" }
                        div { class: "pricing-period", "per month" }
                    }
                }

                // Restore / FAQ
                div {
                    class: "text-center mt-md",
                    style: "animation: fadeIn 0.5s ease-out 0.6s both;",
                    button { class: "btn btn-ghost btn-sm", "Already purchased? Restore" }
                    span { class: "text-muted mx-sm", "·" }
                    button { class: "btn btn-ghost btn-sm", "FAQ" }
                }
            }
        }
    }
}
