use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ConfettiProps {
    pub active: bool,
}

#[component]
pub fn Confetti(props: ConfettiProps) -> Element {
    if !props.active {
        return rsx! {};
    }

    // Generate 20 particles with random-ish positions via CSS custom properties
    let colors = [
        "#f5a623", "#ff6b9d", "#c084fc", "#22d3ee",
        "#34d399", "#fbbf24", "#60a5fa", "#fb923c",
    ];

    rsx! {
        div {
            class: "confetti-container",
            aria_hidden: "true",

            for i in 0..20u32 {
                {
                    let color = colors[(i as usize) % colors.len()];
                    let angle = (i as f64) * 18.0; // spread 360 degrees
                    let distance = 80.0 + (i as f64 % 5.0) * 40.0;
                    let dx = (angle.to_radians().cos() * distance) as i32;
                    let dy = (angle.to_radians().sin() * distance - 60.0) as i32;
                    let rotation = (i as i32 * 73) % 720;
                    let size = 0.3 + (i as f64 % 3.0) * 0.2;
                    let delay = (i as f64) * 15.0;

                    let style = format!(
                        "left: 50%; top: 40%; background: {}; --dx: {}px; --dy: {}px; --dr: {}deg; width: {}rem; height: {}rem; animation-delay: {}ms;",
                        color, dx, dy, rotation, size, size, delay
                    );

                    rsx! {
                        div {
                            key: "particle-{i}",
                            class: "confetti-particle",
                            style: "{style}",
                        }
                    }
                }
            }
        }
    }
}
