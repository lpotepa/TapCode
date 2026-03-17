use dioxus::prelude::*;

#[component]
pub fn OfflineBanner() -> Element {
    rsx! {
        div {
            class: "offline-banner",
            role: "alert",
            aria_live: "polite",
            "Offline — progress will sync when you reconnect"
        }
    }
}

#[component]
pub fn SessionWarning(on_dismiss: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "session-warning",
            role: "alert",
            div { class: "text-lg font-bold mb-sm", "Great session!" }
            p { class: "text-sm text-secondary mb-md",
                "You've been going for 45 minutes. Taking a break helps retention."
            }
            button {
                class: "btn btn-primary btn-sm",
                aria_label: "Dismiss session warning",
                onclick: move |_| on_dismiss.call(()),
                "Got it"
            }
        }
    }
}

#[component]
pub fn NoConnectionScreen(on_retry: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "onboarding-screen",
            role: "alert",
            div { class: "text-4xl mb-md", "📡" }
            div { class: "text-xl font-bold mb-sm", "No connection" }
            p { class: "text-secondary mb-lg",
                "TapCode needs a connection to get started. Once loaded, you can use it offline."
            }
            button {
                class: "btn btn-primary",
                aria_label: "Retry connection",
                onclick: move |_| on_retry.call(()),
                "Try Again"
            }
        }
    }
}
