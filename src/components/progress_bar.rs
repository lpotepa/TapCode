use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ProgressBarProps {
    pub current: usize,
    pub total: usize,
}

#[component]
pub fn ProgressBar(props: ProgressBarProps) -> Element {
    let pct = if props.total > 0 {
        (props.current as f64 / props.total as f64 * 100.0) as u32
    } else {
        0
    };
    let width = format!("width: {}%", pct);

    rsx! {
        div {
            class: "progress-bar",
            role: "progressbar",
            aria_valuenow: "{props.current}",
            aria_valuemin: "0",
            aria_valuemax: "{props.total}",
            aria_label: "Challenge progress: {props.current} of {props.total}",

            div { class: "progress-fill", style: "{width}" }

            div { class: "progress-segments",
                for i in 0..props.total {
                    div {
                        key: "seg-{i}",
                        class: "progress-segment",
                    }
                }
            }
        }
    }
}
