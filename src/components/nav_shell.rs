use dioxus::prelude::*;
use crate::route::Route;

#[component]
pub fn NavShell() -> Element {
    let nav = navigator();

    rsx! {
        nav {
            class: "bottom-nav safe-bottom",
            role: "navigation",
            aria_label: "Main navigation",

            button {
                class: "nav-item active",
                aria_label: "Home",
                onclick: move |_| { let _ = nav.push(Route::Home {}); },
                span { class: "nav-icon", "🏠" }
                span { "Home" }
            }

            button {
                class: "nav-item",
                aria_label: "Profile",
                onclick: move |_| { let _ = nav.push(Route::Profile {}); },
                span { class: "nav-icon", "👤" }
                span { "Profile" }
            }
        }
    }
}
