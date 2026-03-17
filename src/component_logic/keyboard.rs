// Keyboard handler logic — maps key presses to lesson actions.
//
// This is a pure function that can be tested without a DOM.
// The Dioxus onkeydown handler in lesson.rs calls this function
// and dispatches the returned KeyAction.

/// Actions that a keyboard press can resolve to in the lesson screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// No action — key is not mapped or is blocked by current state.
    None,
    /// Submit the current answer (equivalent to tapping Check).
    Check,
    /// Undo the last placed token (equivalent to tapping Undo / Backspace).
    Undo,
    /// Advance to the next challenge (when feedback is Correct).
    Next,
    /// Retry the challenge (when feedback is Wrong).
    TryAgain,
}

/// Resolve a key press to a `KeyAction` based on the current lesson state.
///
/// Arguments:
/// - `key`: the string representation of the pressed key (e.g. "Enter", "Backspace")
/// - `has_feedback`: true when the feedback panel is visible (Correct or Wrong)
/// - `feedback_is_correct`: when `has_feedback` is true, indicates whether it's Correct
pub fn resolve_key_action(key: &str, has_feedback: bool, feedback_is_correct: bool) -> KeyAction {
    match key {
        "Enter" => {
            if has_feedback {
                if feedback_is_correct {
                    KeyAction::Next
                } else {
                    KeyAction::TryAgain
                }
            } else {
                KeyAction::Check
            }
        }
        "Backspace" => {
            if has_feedback {
                KeyAction::None
            } else {
                KeyAction::Undo
            }
        }
        _ => KeyAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_during_correct_feedback() {
        assert_eq!(resolve_key_action("Enter", true, true), KeyAction::Next);
    }

    #[test]
    fn enter_during_wrong_feedback() {
        assert_eq!(resolve_key_action("Enter", true, false), KeyAction::TryAgain);
    }

    #[test]
    fn enter_no_feedback() {
        assert_eq!(resolve_key_action("Enter", false, false), KeyAction::Check);
    }

    #[test]
    fn backspace_no_feedback() {
        assert_eq!(resolve_key_action("Backspace", false, false), KeyAction::Undo);
    }

    #[test]
    fn backspace_during_feedback() {
        assert_eq!(resolve_key_action("Backspace", true, false), KeyAction::None);
    }

    #[test]
    fn unknown_key() {
        assert_eq!(resolve_key_action("x", false, false), KeyAction::None);
    }
}
