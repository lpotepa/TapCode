mod models;
mod engine;
mod validator;
mod state;
mod route;
mod component_logic;
mod components;
mod screens;
mod services;

use dioxus::prelude::*;
use state::AppState;
use route::Route;
use services::platform::{HapticEngine, NoOpHaptics, SecureStorage, MemoryStorage};
use std::sync::Arc;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize global state
    use_context_provider(|| Signal::new(AppState::new()));

    // Platform abstraction — inject trait-object providers so components
    // never need #[cfg()] for platform differences.
    use_context_provider(|| Arc::new(NoOpHaptics) as Arc<dyn HapticEngine>);
    use_context_provider(|| Arc::new(MemoryStorage::new()) as Arc<dyn SecureStorage>);

    rsx! {
        // Load stylesheet via Dioxus 0.7 asset system
        document::Stylesheet { href: asset!("/assets/main.css") }

        div {
            id: "main",
            class: "app-shell",
            Router::<Route> {}
        }
    }
}
