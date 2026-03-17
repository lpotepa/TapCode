use dioxus::prelude::*;
use crate::screens::*;
use crate::state::AppState;
use crate::components::nav_shell::NavShell;

#[derive(Routable, Clone, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Landing {},

    #[route("/onboarding")]
    Onboarding {},

    #[route("/home")]
    Home {},

    #[route("/lesson/:id")]
    Lesson { id: String },

    #[route("/module/:id")]
    ModuleMap { id: String },

    #[route("/profile")]
    Profile {},

    #[route("/compose/:module_id")]
    FreeCompose { module_id: String },

    #[route("/paywall")]
    Paywall {},
}

/// Landing route — redirects to Onboarding or Home based on state
#[component]
fn Landing() -> Element {
    let state = use_context::<Signal<AppState>>();
    let nav = navigator();

    if state.read().is_onboarded {
        let _ = nav.replace(Route::Home {});
    } else {
        let _ = nav.replace(Route::Onboarding {});
    }

    rsx! {}
}

#[component]
fn Home() -> Element {
    rsx! {
        HomeScreen {}
        NavShell {}
    }
}

#[component]
fn Lesson(id: String) -> Element {
    rsx! { LessonScreen { id: id } }
}

#[component]
fn ModuleMap(id: String) -> Element {
    rsx! {
        ModuleMapScreen { id: id }
        NavShell {}
    }
}

#[component]
fn Profile() -> Element {
    rsx! {
        ProfileScreen {}
        NavShell {}
    }
}

#[component]
fn FreeCompose(module_id: String) -> Element {
    rsx! { ComposeScreen { module_id: module_id } }
}

#[component]
fn Onboarding() -> Element {
    rsx! { OnboardingScreen {} }
}

#[component]
fn Paywall() -> Element {
    rsx! { PaywallScreen {} }
}
