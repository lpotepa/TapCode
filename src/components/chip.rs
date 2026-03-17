use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ChipProps {
    pub token: String,
    pub category: String,
    pub css_class: String,
    #[props(default = false)]
    pub dimmed: bool,
    #[props(default = false)]
    pub used: bool,
    pub on_tap: EventHandler<String>,
    #[props(default = 0)]
    pub entrance_delay_ms: u32,
}

#[component]
pub fn TokenChip(props: ChipProps) -> Element {
    let state_class = if props.dimmed {
        "chip-dimmed"
    } else if props.used {
        "chip-used"
    } else {
        "chip-highlighted"
    };

    let delay = format!("animation-delay: {}ms", props.entrance_delay_ms);
    let class = format!("chip {} {} chip-enter", props.css_class, state_class);
    let label = format!("{} — {}", props.token, props.category);
    let token = props.token.clone();

    rsx! {
        button {
            class: "{class}",
            style: "{delay}",
            role: "button",
            aria_label: "{label}",
            tabindex: 0,
            onclick: move |_| {
                props.on_tap.call(token.clone());
            },
            "{props.token}"
        }
    }
}
