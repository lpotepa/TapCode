use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct XpDisplayProps {
    pub xp: u32,
    #[props(default = false)]
    pub bouncing: bool,
    #[props(default)]
    pub float_amount: Option<u32>,
}

#[component]
pub fn XpDisplay(props: XpDisplayProps) -> Element {
    let bounce_class = if props.bouncing { "xp-bounce" } else { "" };

    rsx! {
        div {
            class: "xp-counter {bounce_class}",
            role: "status",
            aria_label: "{props.xp} experience points",

            span { "⚡" }
            span { class: "font-extrabold", "{props.xp}" }
            span { class: "text-sm text-secondary font-medium", "XP" }

            if let Some(amount) = props.float_amount {
                span {
                    class: "xp-float",
                    "+{amount}"
                }
            }
        }
    }
}
