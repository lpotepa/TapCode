use dioxus::prelude::*;
use crate::models::*;
use crate::components::chip::TokenChip;

#[derive(Props, Clone, PartialEq)]
pub struct PickerProps {
    pub chip_groups: Vec<ChipGroupDisplay>,
    pub group_states: Vec<ChipGroupState>,
    pub used_tokens: Vec<String>,
    pub on_chip_tap: EventHandler<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChipGroupDisplay {
    pub name: String,
    pub display_name: String,
    pub css_class: String,
    pub tokens: Vec<String>,
}

#[component]
pub fn TokenPicker(props: PickerProps) -> Element {
    let mut chip_index: u32 = 0;

    rsx! {
        div {
            class: "picker",
            role: "region",
            aria_label: "Token picker — tap to add tokens",

            for group in props.chip_groups.iter() {
                {
                    let is_dimmed = props.group_states.iter()
                        .find(|s| s.group_name == group.name)
                        .map(|s| !s.is_highlighted)
                        .unwrap_or(false);

                    let group_name = group.name.clone();

                    rsx! {
                        div {
                            class: "picker-group",
                            key: "{group_name}",
                            role: "group",
                            aria_label: "{group.display_name} tokens",

                            span {
                                class: "picker-label",
                                "{group.display_name}"
                            }

                            div {
                                class: "picker-chips",

                                for token in group.tokens.iter() {
                                    {
                                        chip_index += 1;
                                        let is_used = props.used_tokens.contains(token);

                                        rsx! {
                                            TokenChip {
                                                key: "{group_name}-{token}",
                                                token: token.clone(),
                                                category: group.display_name.clone(),
                                                css_class: group.css_class.clone(),
                                                dimmed: is_dimmed,
                                                used: is_used,
                                                entrance_delay_ms: chip_index * 30,
                                                on_tap: move |t: String| {
                                                    props.on_chip_tap.call(t);
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
