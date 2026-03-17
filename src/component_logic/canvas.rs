// Canvas logic — indentation computation for code display.
//
// The canvas renders tokens with indentation based on brace depth.
// After `{`, indent level increases; `}` decreases it.
// Each token gets `margin-left: {depth * 1.5}rem`.

/// Compute the inline margin-left style for each token based on brace-depth indentation.
///
/// Rules:
/// - `}` decreases depth BEFORE rendering (so `}` aligns with its matching `{`)
/// - `{` increases depth AFTER rendering (so content inside the block is indented)
/// - Depth never goes below 0 (saturating subtraction)
///
/// Returns a Vec of CSS style strings, one per token.
pub fn compute_indent_styles(tokens: &[(String, String)]) -> Vec<String> {
    let mut styles = Vec::with_capacity(tokens.len());
    let mut depth: u32 = 0;

    for (token, _css_class) in tokens {
        if token == "}" {
            depth = depth.saturating_sub(1);
        }

        let indent = format!("margin-left: {}rem", depth as f64 * 1.5);
        styles.push(indent);

        if token == "{" {
            depth += 1;
        }
    }

    styles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(t: &str) -> (String, String) {
        (t.to_string(), String::new())
    }

    #[test]
    fn flat_statement_all_zero_indent() {
        let tokens = vec![tok("let"), tok("x"), tok("="), tok("5"), tok(";")];
        let styles = compute_indent_styles(&tokens);
        assert!(styles.iter().all(|s| s == "margin-left: 0rem"));
    }

    #[test]
    fn open_brace_indents_following() {
        let tokens = vec![tok("{"), tok("x")];
        let styles = compute_indent_styles(&tokens);
        assert_eq!(styles[0], "margin-left: 0rem");
        assert_eq!(styles[1], "margin-left: 1.5rem");
    }

    #[test]
    fn close_brace_dedents() {
        let tokens = vec![tok("{"), tok("x"), tok("}")];
        let styles = compute_indent_styles(&tokens);
        assert_eq!(styles[2], "margin-left: 0rem");
    }

    #[test]
    fn nested_blocks() {
        let tokens = vec![tok("{"), tok("{"), tok("x"), tok("}"), tok("}")];
        let styles = compute_indent_styles(&tokens);
        assert_eq!(styles[0], "margin-left: 0rem");
        assert_eq!(styles[1], "margin-left: 1.5rem");
        assert_eq!(styles[2], "margin-left: 3rem");
        assert_eq!(styles[3], "margin-left: 1.5rem");
        assert_eq!(styles[4], "margin-left: 0rem");
    }

    #[test]
    fn underflow_protection() {
        let tokens = vec![tok("}"), tok("x")];
        let styles = compute_indent_styles(&tokens);
        assert_eq!(styles[0], "margin-left: 0rem");
        assert_eq!(styles[1], "margin-left: 0rem");
    }

    #[test]
    fn empty_input() {
        let styles = compute_indent_styles(&[]);
        assert!(styles.is_empty());
    }
}
