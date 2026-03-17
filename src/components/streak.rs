use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct StreakProps {
    pub count: u32,
    pub days: Vec<bool>, // last 7 days, index 6 = today
    pub today_filled: bool,
    pub has_freeze: bool,
    #[props(default)]
    pub previous_streak: Option<u32>,
}

#[component]
pub fn StreakDisplay(props: StreakProps) -> Element {
    let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    rsx! {
        div {
            class: "streak-container",
            role: "region",
            aria_label: "Daily streak: {props.count} days",

            // Big streak number
            div { class: "streak-counter",
                if props.count > 0 {
                    "🔥 {props.count}"
                } else {
                    "0"
                }
            }

            if props.count > 0 {
                div { class: "streak-label", "day streak" }
            } else if let Some(prev) = props.previous_streak {
                div { class: "streak-previous", "Previous streak: {prev} days" }
                div { class: "streak-label text-accent", "Start a new streak today" }
            }

            // 7-day dot strip
            div {
                class: "streak-dots",
                aria_label: "Last 7 days",

                for (i, filled) in props.days.iter().enumerate() {
                    {
                        let is_today = i == props.days.len() - 1;
                        let dot_class = if *filled {
                            if is_today {
                                "streak-dot streak-dot-today streak-dot-filled"
                            } else {
                                "streak-dot streak-dot-filled"
                            }
                        } else if is_today {
                            "streak-dot streak-dot-today"
                        } else {
                            "streak-dot"
                        };
                        let label_idx = if i < day_labels.len() { i } else { 0 };

                        rsx! {
                            div {
                                key: "day-{i}",
                                class: "{dot_class}",
                                aria_label: if *filled {
                                    format!("{} — completed", day_labels[label_idx])
                                } else if is_today {
                                    format!("{} — today, not yet completed", day_labels[label_idx])
                                } else {
                                    format!("{} — missed", day_labels[label_idx])
                                },
                                title: "{day_labels[label_idx]}",
                            }
                        }
                    }
                }
            }

            // Freeze indicator
            if props.has_freeze {
                div { class: "streak-freeze",
                    "🧊 Streak freeze available"
                }
            }
        }
    }
}
