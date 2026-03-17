use dioxus::prelude::*;
use crate::state::AppState;
use crate::route::Route;
use crate::services::sync::ProdSyncService;
use std::sync::Arc;

#[component]
pub fn PaywallScreen() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let sync_ctx = use_context::<Signal<Option<Arc<ProdSyncService>>>>();
    let nav = navigator();

    // TAP-30: Helper to unlock modules and fire sync in background
    let mut handle_purchase = move || {
        state.write().unlock_all_modules();

        // Fire-and-forget sync to Supabase
        spawn(async move {
            if let Some(sync) = sync_ctx.read().clone() {
                let state_snap = state.read().clone();
                sync.sync_purchase(&state_snap).await;
            }
        });

        let _ = nav.push(Route::Home {});
    };

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
                        onclick: move |_| {
                            handle_purchase();
                        },
                        div { class: "pricing-badge", "Best Value" }
                        div { class: "pricing-amount", "€4.99" }
                        div { class: "pricing-period", "per year" }
                    }

                    // Lifetime
                    button {
                        class: "pricing-card",
                        onclick: move |_| {
                            handle_purchase();
                        },
                        div { class: "pricing-amount", "€9.99" }
                        div { class: "pricing-period", "lifetime · one-time" }
                    }

                    // Monthly
                    button {
                        class: "pricing-card",
                        onclick: move |_| {
                            handle_purchase();
                        },
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
