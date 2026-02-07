use std::io::IsTerminal;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Ansi {
    enabled: bool,
}

impl Ansi {
    pub(crate) fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub(crate) fn from_env() -> Self {
        if std::env::var_os("NO_COLOR").is_some() {
            return Self::new(false);
        }
        Self::new(std::io::stdout().is_terminal())
    }

    fn esc(self, code: &str) -> &'static str {
        if !self.enabled {
            return "";
        }
        match code {
            "reset" => "\x1b[0m",
            "dim" => "\x1b[2m",
            "bold" => "\x1b[1m",
            "cyan" => "\x1b[36m",
            "green" => "\x1b[32m",
            "yellow" => "\x1b[33m",
            _ => "\x1b[0m",
        }
    }

    pub(crate) fn h(self, s: &str) -> String {
        format!(
            "{}{}{}{}",
            self.esc("bold"),
            self.esc("cyan"),
            s,
            self.esc("reset")
        )
    }

    pub(crate) fn good(self, s: &str) -> String {
        format!(
            "{}{}{}{}",
            self.esc("bold"),
            self.esc("green"),
            s,
            self.esc("reset")
        )
    }

    pub(crate) fn warn(self, s: &str) -> String {
        format!(
            "{}{}{}{}",
            self.esc("bold"),
            self.esc("yellow"),
            s,
            self.esc("reset")
        )
    }

    pub(crate) fn dim(self, s: &str) -> String {
        format!("{}{}{}", self.esc("dim"), s, self.esc("reset"))
    }
}
