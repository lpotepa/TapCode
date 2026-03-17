// ══════════════════════════════════════════════════════════════
// Component Logic — Pure functions extracted from UI components
//
// These functions contain the data-transformation logic that drives
// component rendering. They are kept separate from the Dioxus RSX
// so they can be unit-tested without a DOM or framework runtime.
// ══════════════════════════════════════════════════════════════

pub mod canvas;
pub mod keyboard;
pub mod picker;
