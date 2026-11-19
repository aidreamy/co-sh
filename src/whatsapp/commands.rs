//! Slash-command detection and the WhatsApp command registry.
//!
//! # What counts as a command
//!
//! A message is a slash command only when **all** of these hold:
//!
//! 1. the first non-whitespace character is `/`;
//! 2. the token right after `/` is a bare name (`A-Z`, `a-z`, `0-9`, `-`,
//!    `_`) — no `/`, `\`, `.` or `:`; and
//! 3. the lowercased name is a command this registry publishes.
//!
//! The registry gate is what keeps ordinary messages from being hijacked:
//! Unix paths (`/usr/bin/env`), file paths (`/home/me/report.txt`) and text
//! like `/ano que vem` start with a slash but name no command, so they are
//! sent to the agent as normal prose. This mirrors the conservative
//! detection used by coding agents such as Crush, OpenCode and Claude Code:
//! a slash command is an *explicit, known* command at the start of the
//! message, never a heuristic on the first character alone.
//!
//! Names are matched case-insensitively (`/MODE build` == `/mode build`),
//! and arguments are the trimmed tail after the first whitespace. A message
//! with only a command name is still a command: the handler reports the
//! missing arguments instead of guessing.

use crate::harness::Mode;

use super::mode;

/// One command published to WhatsApp users.
pub struct CommandSpec {
    /// Canonical lowercase name (the only spelling shown in help).
    pub name: &'static str,
    /// One-line summary for `/help`.
    pub summary: &'static str,
    /// Usage line for `/help` and for argument errors.
    pub usage: &'static str,
}

/// The command surface exposed over WhatsApp.
///
/// This is deliberately an allowlist, not a copy of the TUI menu: a command
/// ships here only after its remote behavior exists. TUI-only presentation
/// commands stay out (see [`TUI_ONLY_COMMANDS`]).
pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "help",
        summary: "List the commands available on WhatsApp",
        usage: "/help",
    },
    CommandSpec {
        name: "mode",
        summary: "Switch the harness mode",
        usage: "/mode <build|ask|yolo|command|manager>",
    },
];

/// Commands that exist in the TUI but have no meaning in a remote chat:
/// they customize the terminal UI (theme, spinner), which the
/// WhatsApp interface does not render. Kept explicit so the exclusion is a
/// tested contract instead of an accident of the allowlist.
#[cfg(test)]
pub const TUI_ONLY_COMMANDS: &[&str] = &["theme", "themes", "spinner"];

/// A syntactically valid, registered command parsed out of a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingCommand {
    /// Canonical lowercase name.
    pub name: String,
    /// Trimmed argument tail (empty when nothing followed the name).
    pub args: String,
}

/// What a parsed command asks the session to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Render the command list.
    Help,
    /// Switch the harness mode.
    SetMode(Mode),
    /// The arguments were missing or malformed: send this usage message
    /// back to the user instead of executing anything.
    Usage(String),
}

/// Look up a command by its canonical lowercase name.
#[must_use]
pub fn lookup(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|command| command.name == name)
}

/// Detect and parse a slash command. `None` means "ordinary message".
#[must_use]
pub fn parse(text: &str) -> Option<IncomingCommand> {
    let rest = text.trim_start().strip_prefix('/')?;

    // The name ends at the first whitespace; the tail is the argument text.
    let mut parts = rest.splitn(2, char::is_whitespace);
    let raw_name = parts.next().unwrap_or_default();
    let args = parts.next().unwrap_or_default().trim();

    // A bare name only. This is what rejects paths: `/usr/bin` has a `/`
    // inside the name and is never considered a command.
    if raw_name.is_empty()
        || !raw_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return None;
    }

    let name = raw_name.to_ascii_lowercase();
    lookup(&name)?;
    Some(IncomingCommand {
        name,
        args: args.to_string(),
    })
}

impl IncomingCommand {
    /// Validate the arguments and resolve what to run.
    #[must_use]
    pub fn action(&self) -> Action {
        match self.name.as_str() {
            "help" => {
                if self.args.is_empty() {
                    Action::Help
                } else {
                    Action::Usage("`/help` takes no arguments.".to_string())
                }
            }
            "mode" => {
                let mut words = self.args.split_whitespace();
                match (words.next(), words.next()) {
                    (Some(word), None) => match mode::parse(word) {
                        Some(mode) => Action::SetMode(mode),
                        None => Action::Usage(format!(
                            "Unknown mode `{word}`. Available modes: {}.",
                            mode::options()
                        )),
                    },
                    (None, _) => Action::Usage(format!(
                        "Name the mode: {}. Expected `/mode <mode>`.",
                        mode::options()
                    )),
                    (Some(_), Some(_)) => Action::Usage(
                        "`/mode` takes exactly one mode name (e.g. `/mode build`).".to_string(),
                    ),
                }
            }
            // `parse` only returns registered names, so this is unreachable
            // in practice; answering with the usage line beats a silent
            // no-op if the registry and this match ever drift.
            other => Action::Usage(format!(
                "`/{other}` has no handler. Send `/help` for the command list."
            )),
        }
    }
}

/// The `/help` body.
#[must_use]
pub fn help_text() -> String {
    let mut text = String::from("*Morpheus on WhatsApp*\n\nCommands (case-insensitive):\n");
    for command in COMMANDS {
        text.push_str(&format!("• {} — {}\n", command.usage, command.summary));
    }
    text.push_str("\nModes:\n");
    for (name, description) in mode::ALL {
        text.push_str(&format!("• {name} — {description}\n"));
    }
    text.push_str(
        "\nSend any other message to talk to the agent. A message that only \
         looks like a path (e.g. /usr/bin/env) is treated as normal text.",
    );
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(name: &str, args: &str) -> IncomingCommand {
        IncomingCommand {
            name: name.to_string(),
            args: args.to_string(),
        }
    }

    #[test]
    fn registered_commands_are_detected() {
        assert_eq!(
            parse("/help"),
            Some(cmd("help", "")),
            "a bare command is still a command"
        );
        assert_eq!(parse("/mode build"), Some(cmd("mode", "build")));
        assert_eq!(
            parse("   /mode   yolo   "),
            Some(cmd("mode", "yolo")),
            "leading whitespace and the argument tail are trimmed"
        );
    }

    #[test]
    fn commands_are_case_insensitive() {
        for spelling in ["/help", "/HELP", "/Help", "/hElP"] {
            assert_eq!(parse(spelling), Some(cmd("help", "")), "{spelling}");
        }
        // The command name is normalized; the argument tail is preserved as
        // typed (mode parsing is itself case-insensitive).
        for (spelling, args) in [
            ("/mode BUILD", "BUILD"),
            ("/MODE Build", "Build"),
            ("/Mode bUiLd", "bUiLd"),
        ] {
            assert_eq!(parse(spelling), Some(cmd("mode", args)), "{spelling}");
        }
    }

    #[test]
    fn paths_and_prose_are_not_commands() {
        // The false positives this grammar exists to avoid: a leading slash
        // is not enough, and the name must be a registered command.
        for text in [
            "/usr/bin/env",
            "/home/me/report.txt is broken",
            "/etc",
            "/ano que vem",
            "and/or",
            "https://example.com/mode",
            "/",
            "/ help",
            "/mode:build",
            "/mod build",
            "/mode.build",
        ] {
            assert_eq!(parse(text), None, "{text:?} must stay a normal message");
        }
    }

    #[test]
    fn a_slash_inside_the_argument_is_allowed() {
        // Only the NAME is restricted; arguments can be anything the
        // command accepts (today `/mode build`, but the grammar must not
        // bake that in).
        assert_eq!(parse("/mode build/extra"), Some(cmd("mode", "build/extra")));
    }

    #[test]
    fn mode_arguments_are_validated_explicitly() {
        assert_eq!(cmd("mode", "build").action(), Action::SetMode(Mode::Build));
        assert_eq!(cmd("mode", "YOLO").action(), Action::SetMode(Mode::Yolo));

        match cmd("mode", "").action() {
            Action::Usage(message) => assert!(message.contains("Name the mode"), "{message}"),
            other => panic!("expected usage, got {other:?}"),
        }
        match cmd("mode", "turbo").action() {
            Action::Usage(message) => assert!(message.contains("Unknown mode"), "{message}"),
            other => panic!("expected usage, got {other:?}"),
        }
        match cmd("mode", "build extra").action() {
            Action::Usage(message) => assert!(message.contains("exactly one"), "{message}"),
            other => panic!("expected usage, got {other:?}"),
        }
    }

    #[test]
    fn help_takes_no_arguments() {
        assert_eq!(cmd("help", "").action(), Action::Help);
        assert!(matches!(cmd("help", "mode").action(), Action::Usage(_)));
    }

    #[test]
    fn tui_only_commands_are_not_published() {
        for name in TUI_ONLY_COMMANDS {
            assert!(
                lookup(name).is_none(),
                "TUI-only command `/{name}` must not be exposed on WhatsApp"
            );
            assert_eq!(parse(&format!("/{name}")), None);
            assert!(!help_text().contains(&format!("/{name}")));
        }
    }

    #[test]
    fn help_lists_every_published_command_and_mode() {
        let help = help_text();
        for command in COMMANDS {
            assert!(
                help.contains(command.usage),
                "help is missing {}",
                command.usage
            );
        }
        for (name, _) in mode::ALL {
            assert!(help.contains(name), "help is missing mode {name}");
        }
    }
}
