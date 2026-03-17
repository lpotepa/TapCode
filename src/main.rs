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
use services::supabase::{ReqwestHttpClient, SupabaseClient, SUPABASE_URL, SUPABASE_ANON_KEY};
use services::sync::{SyncService, ProdSyncService};
use std::sync::Arc;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize global state
    let mut state = use_context_provider(|| Signal::new(AppState::new()));

    // Platform abstraction — inject trait-object providers so components
    // never need #[cfg()] for platform differences.
    use_context_provider(|| Arc::new(NoOpHaptics) as Arc<dyn HapticEngine>);
    let storage = Arc::new(MemoryStorage::new());
    use_context_provider(|| storage.clone() as Arc<dyn SecureStorage>);

    // SyncService — initialized asynchronously, provided as context.
    // Components read this signal; None means init is still in progress.
    let mut sync_signal: Signal<Option<Arc<ProdSyncService>>> =
        use_context_provider(|| Signal::new(None::<Arc<ProdSyncService>>));

    // Initialize SyncService on first render
    use_future(move || {
        let storage = storage.clone();
        async move {
            let http = ReqwestHttpClient::new();
            let client = Arc::new(SupabaseClient::new(
                SUPABASE_URL,
                SUPABASE_ANON_KEY,
                http,
                storage,
            ));

            let mut svc = SyncService::new(client);
            let authed = svc.init().await.unwrap_or(false);

            if !authed {
                state.write().is_offline = true;
            }

            // After init, is_authenticated is set. Wrap in Arc for shared access.
            sync_signal.set(Some(Arc::new(svc)));
        }
    });

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
