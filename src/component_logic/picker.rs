// Picker logic — chip group filtering and staggered entrance delay computation.

/// Display model for a chip group in the picker.
/// Mirrors the struct in components::picker but lives here to avoid
/// coupling test code to the Dioxus component module.
#[derive(Debug, Clone, PartialEq)]
pub struct ChipGroupDisplay {
    pub name: String,
    pub display_name: String,
    pub css_class: String,
    pub tokens: Vec<String>,
}

/// Filter out chip groups that have no tokens.
/// Empty groups should not be rendered in the picker.
pub fn filter_nonempty_groups(groups: &[ChipGroupDisplay]) -> Vec<&ChipGroupDisplay> {
    groups.iter().filter(|g| !g.tokens.is_empty()).collect()
}

/// Compute staggered entrance delay for every chip across ALL groups.
///
/// The counter is GLOBAL — it does NOT reset per group. This ensures
/// chips in later groups animate in after chips in earlier groups,
/// creating a smooth top-to-bottom cascade effect.
///
/// Each chip gets delay = (global_index + 1) * 30ms.
/// Returns a flat Vec of delay values in ms, one per chip across all groups.
pub fn compute_global_chip_delays(groups: &[ChipGroupDisplay]) -> Vec<u32> {
    let mut delays = Vec::new();
    let mut chip_index: u32 = 0;

    for group in groups {
        for _token in &group.tokens {
            chip_index += 1;
            delays.push(chip_index * 30);
        }
    }

    delays
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(name: &str, tokens: &[&str]) -> ChipGroupDisplay {
        ChipGroupDisplay {
            name: name.to_string(),
            display_name: name.to_string(),
            css_class: format!("chip-{}", name),
            tokens: tokens.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn filter_removes_empty() {
        let groups = vec![group("kw", &["fn"]), group("empty", &[]), group("sym", &[";"])];
        let filtered = filter_nonempty_groups(&groups);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|g| !g.tokens.is_empty()));
    }

    #[test]
    fn delays_increment_globally() {
        let groups = vec![
            group("a", &["x", "y"]),
            group("b", &["z"]),
        ];
        let delays = compute_global_chip_delays(&groups);
        assert_eq!(delays, vec![30, 60, 90]);
    }

    #[test]
    fn delays_skip_empty_groups() {
        let groups = vec![
            group("a", &["x"]),
            group("empty", &[]),
            group("b", &["y"]),
        ];
        let delays = compute_global_chip_delays(&groups);
        assert_eq!(delays, vec![30, 60]);
    }
}
