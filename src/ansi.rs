// ANSI color codes for cross-platform terminal output
// No external dependencies — pure escape codes

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN: &str = "\x1b[36m";

/// Colored key for hotkey display: "KEY" in cyan
pub fn key(s: &str) -> String {
    format!("{}{}{}", CYAN, s, RESET)
}

/// Success indicator with text
pub fn ok(text: &str) -> String {
    format!("{}{}✓{} {}", GREEN, BOLD, RESET, text)
}

/// Error indicator with text
pub fn err(text: &str) -> String {
    format!("{}{}✗{} {}", RED, BOLD, RESET, text)
}

/// Warning indicator with text
pub fn warn(text: &str) -> String {
    format!("{}{}!{} {}", YELLOW, BOLD, RESET, text)
}

/// Section heading
pub fn heading(text: &str) -> String {
    format!("{}{}{}{}", BOLD, CYAN, text, RESET)
}

/// Muted/secondary text
pub fn muted(text: &str) -> String {
    format!("{}{}{}", DIM, text, RESET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_formatting() {
        assert!(key("scroll lock").contains("scroll lock"));
    }

    #[test]
    fn test_ok_formatting() {
        assert!(ok("member").contains("✓"));
    }

    #[test]
    fn test_err_formatting() {
        assert!(err("failed").contains("✗"));
    }
}