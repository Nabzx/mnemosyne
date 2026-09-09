//! ANSI styling for human-facing CLI output.
//!
//! Every helper is a no-op when stdout is not a terminal, or when `NO_COLOR` is
//! set (<https://no-color.org>), so piped and redirected output stays plain and
//! parseable.

use std::io::IsTerminal;
use std::sync::OnceLock;

fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal())
}

fn wrap(codes: &str, s: &str) -> String {
    if on() {
        format!("\x1b[{codes}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// A commit or object id.
pub fn id(s: &str) -> String {
    wrap("38;5;75", s)
}

/// A positive outcome, or an added line.
pub fn good(s: &str) -> String {
    wrap("38;5;71", s)
}

/// A removed line.
pub fn bad(s: &str) -> String {
    wrap("38;5;167", s)
}

/// A modified line.
pub fn changed(s: &str) -> String {
    wrap("38;5;179", s)
}

/// Secondary text: labels and metadata.
pub fn dim(s: &str) -> String {
    wrap("38;5;245", s)
}

/// A warning, or the one thing the reader is meant to notice.
pub fn warn(s: &str) -> String {
    wrap("38;5;173", s)
}

/// A branch name.
pub fn branch(s: &str) -> String {
    wrap("38;5;140", s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn helpers_are_plain_when_stdout_is_not_a_terminal() {
        // the test harness captures stdout, so it is not a tty
        assert_eq!(super::id("a17e42c9"), "a17e42c9");
        assert_eq!(super::warn("look here"), "look here");
        assert_eq!(super::branch("main"), "main");
    }
}
