//! One WhatsApp conversation's agent session.
//!
//! Each chat gets its own worker (see [`super::agent`]) and therefore its own
//! [`ChatSession`]. The session owns exactly three pieces of state:
//!
//! - the harness **mode** (`/mode`), initially the user's global selection;
//! - the model-facing **context** ([`ContextManagerState`]), carried from
//!   one turn to the next in memory (the WhatsApp transcript is never fed
//!   back to the model);
//! - the immutable [`RuntimeConfig`] resolved at startup.
//!
//! A turn assembles a fresh harness (the ONE assembly in `crate::assembly`),
//! runs it while draining its events, and stores the returned context. This
//! is the same per-turn contract the TUI follows, and it is why a mode
//! change is a plain field write: the next turn assembles with the new mode.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::mpsc;

use morpheus_tools::question::types::AnswerItem;

use crate::harness::{ContextManagerState, Harness, HarnessEvent, Mode, PermissionAction};
use crate::setup::Setup;

use super::commands::{self, Action, IncomingCommand};
use super::config::RuntimeConfig;
use super::mode;

/// Upper bound on one outbound message. WhatsApp accepts long texts, but a
/// runaway shell output or model answer must not flood the phone; the tail
/// says how much was cut.
const MAX_REPLY_CHARS: usize = 16_000;

/// How the agent loop of one turn ended.
#[derive(Default)]
struct Turn {
    /// Streamed assistant text, kept as a fallback when the context manager
    /// has no sealed final answer (e.g. a stopped turn).
    tokens: String,
    /// The context snapshot returned by `Done`/`Stopped`/`Error`.
    context: Option<ContextManagerState>,
    /// The fatal error message, when the turn failed.
    error: Option<String>,
    /// Whether the turn was interrupted.
    stopped: bool,
    /// Tools the loop asked approval for. There is no approval dialog on
    /// WhatsApp, so they were denied; the reply tells the user why.
    denied_tools: Vec<String>,
}

impl Turn {
    /// Collect one harness event, answering the loop where it would
    /// otherwise block forever waiting for a UI.
    fn absorb(
        &mut self,
        event: HarnessEvent,
        answer_tx: &mpsc::UnboundedSender<Result<Vec<AnswerItem>, String>>,
        perm_tx: &mpsc::UnboundedSender<PermissionAction>,
    ) {
        match event {
            HarnessEvent::Token { text } => self.tokens.push_str(&text),
            HarnessEvent::Done { context, .. } => self.context = Some(context),
            HarnessEvent::Stopped { context } => {
                self.context = Some(context);
                self.stopped = true;
            }
            HarnessEvent::Error {
                message, context, ..
            } => {
                self.error = Some(message);
                if context.is_some() {
                    self.context = context;
                }
            }
            HarnessEvent::PermissionRequest { tool, .. } => {
                // Deny rather than hang: the loop waits for an answer, and
                // there is no interactive surface here. Denying is the safe
                // choice; `/mode yolo` is the explicit opt-in to run tools
                // without approval.
                if !self.denied_tools.contains(&tool) {
                    self.denied_tools.push(tool);
                }
                let _ = perm_tx.send(PermissionAction::Deny);
            }
            HarnessEvent::QuestionRequest { .. } => {
                // `ask_questions` is hidden from the model on this surface,
                // but a stale registration must not wedge the loop.
                let _ =
                    answer_tx.send(Err("Interactive questions are not available on WhatsApp. \
                     Ask the user directly in your reply instead."
                        .to_string()));
            }
            HarnessEvent::Toast { message, .. } => {
                // Toasts are TUI chrome; keep them observable without
                // spamming the chat.
                log::warn!("morpheus whatsapp: {message}");
            }
            _ => {}
        }
    }

    /// The user-facing reply for this turn.
    fn reply(self, harness: &Harness) -> String {
        let body = if let Some(error) = self.error {
            format!("The agent could not complete this turn: {error}")
        } else {
            harness
                .context_manager
                .final_answer()
                .filter(|answer| !answer.trim().is_empty())
                .or_else(|| {
                    let tokens = self.tokens.trim();
                    (!tokens.is_empty()).then(|| tokens.to_string())
                })
                .unwrap_or_else(|| {
                    if self.stopped {
                        "The turn was stopped before it produced a reply.".to_string()
                    } else {
                        "The agent finished without a textual reply.".to_string()
                    }
                })
        };

        let mut reply = truncate(body);
        if !self.denied_tools.is_empty() {
            reply.push_str(&format!(
                "\n\n_Tools requiring approval were denied (no approval dialog on WhatsApp): {}. \
                 Use `/mode yolo` if you want them to run._",
                self.denied_tools.join(", ")
            ));
        }
        reply
    }
}

/// The state of one WhatsApp chat.
pub struct ChatSession {
    config: Arc<RuntimeConfig>,
    chat_key: String,
    mode: Mode,
    context: Option<ContextManagerState>,
}

impl ChatSession {
    /// Start a chat worker in the user's persisted mode, with no history.
    #[must_use]
    pub fn new(config: Arc<RuntimeConfig>, chat_key: String) -> Self {
        let mode = config.initial_mode;
        Self {
            config,
            chat_key,
            mode,
            context: None,
        }
    }

    /// The current harness mode, asserted by the command tests.
    #[cfg(test)]
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Handle one inbound message: a slash command, a shell command
    /// (Command mode) or a normal agent turn.
    pub async fn handle(&mut self, text: &str) -> String {
        if let Some(command) = commands::parse(text) {
            return self.execute(command);
        }
        if self.mode == Mode::Command {
            return self.run_shell(text).await;
        }
        self.run_turn(text).await
    }

    /// Run a slash command. Commands never reach the model.
    fn execute(&mut self, command: IncomingCommand) -> String {
        match command.action() {
            Action::Help => commands::help_text(),
            Action::SetMode(mode) => {
                self.mode = mode;
                // The selection is a global preference, like the TUI's Tab
                // cycle: restore it on the next run instead of starting at
                // Build. Reading the file fresh (and letting `save` merge)
                // avoids clobbering another process's settings.
                let mut setup = Setup::load();
                setup.set_mode_selection(mode);
                if mode == Mode::Command {
                    "Mode set to *command*. Send a shell command as your next message; \
                     no LLM call is made. Use `/mode build` to return to the agent."
                        .to_string()
                } else {
                    format!(
                        "Mode set to *{}*.\n{}",
                        mode::name(mode),
                        mode::description(mode)
                    )
                }
            }
            Action::Usage(message) => message,
        }
    }

    /// Assemble the harness for this turn and run the agent loop, answering
    /// the loop's UI requests and waiting for it to finish.
    async fn run_turn(&mut self, input: &str) -> String {
        let assembly = self.config.assembly(
            self.mode,
            self.context.clone(),
            Some(crate::assembly::session_affinity_id(&format!(
                "whatsapp:{}",
                self.chat_key
            ))),
        );
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let mut harness = match crate::assembly::assemble(assembly, &event_tx).await {
            Ok(harness) => harness,
            Err(error) => {
                return format!("Could not start the agent for this chat: {error}");
            }
        };

        let (answer_tx, answer_rx) = mpsc::unbounded_channel();
        let (perm_tx, perm_rx) = mpsc::unbounded_channel();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let mut turn = Turn::default();

        {
            let mut running = std::pin::pin!(harness.run_agent_loop(
                input,
                event_tx,
                answer_rx,
                perm_rx,
                stop_signal,
            ));
            loop {
                tokio::select! {
                    _ = &mut running => break,
                    event = event_rx.recv() => match event {
                        Some(event) => turn.absorb(event, &answer_tx, &perm_tx),
                        None => break,
                    },
                }
            }
        }

        // `run_agent_loop` returns right after sending its terminal event;
        // drain what is still queued so `Done` is never missed.
        while let Ok(event) = event_rx.try_recv() {
            turn.absorb(event, &answer_tx, &perm_tx);
        }

        if let Some(context) = turn.context.take() {
            self.context = Some(context);
        }
        turn.reply(&harness)
    }

    /// Command mode: the message IS a shell command. No LLM call, no
    /// context mutation — the conversation history is preserved untouched.
    async fn run_shell(&self, command: &str) -> String {
        use crate::harness::Tools;

        if command.trim().is_empty() {
            return "Send a shell command, or `/mode build` to return to the agent.".to_string();
        }

        if self.config.disabled_tools.contains("bash_run") {
            return "Shell commands are disabled in your tool settings (`bash_run` is off). \
                    Re-enable it in `.cosh-tools.json` or `setup.json` to use command mode."
                .to_string();
        }

        let tools = crate::harness::CoshTools::new(&self.config.cwd);
        match tools
            .dispatch("bash_run", serde_json::json!({ "command": command }))
            .await
        {
            Ok(output) if output.trim().is_empty() => {
                "Command finished with no output.".to_string()
            }
            Ok(output) => truncate(output),
            Err(error) => format!("Command failed: {error}"),
        }
    }
}

/// Keep an outbound message under [`MAX_REPLY_CHARS`], saying what was cut.
fn truncate(text: String) -> String {
    let total = text.chars().count();
    if total <= MAX_REPLY_CHARS {
        return text;
    }
    let kept: String = text.chars().take(MAX_REPLY_CHARS).collect();
    format!("{kept}\n\n… truncated: {total} characters total.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::Setup;

    fn session(setup: &Setup) -> ChatSession {
        ChatSession::new(
            Arc::new(RuntimeConfig::from_setup(setup, "/tmp/project")),
            "chat".to_string(),
        )
    }

    #[test]
    fn mode_command_changes_the_session_mode() {
        let mut session = session(&Setup::default());
        assert_eq!(session.mode(), Mode::Build);

        let reply = session.execute(commands::parse("/MODE yolo").unwrap());
        assert!(reply.contains("*yolo*"), "{reply}");
        assert_eq!(session.mode(), Mode::Yolo);
    }

    #[test]
    fn mode_command_requires_an_explicit_valid_argument() {
        let mut session = session(&Setup::default());
        let usage = session.execute(commands::parse("/mode").unwrap());
        assert!(usage.contains("Name the mode"), "{usage}");

        let unknown = session.execute(commands::parse("/mode turbo").unwrap());
        assert!(unknown.contains("Unknown mode"), "{unknown}");
        assert_eq!(
            session.mode(),
            Mode::Build,
            "a bad argument changes nothing"
        );
    }

    #[test]
    fn help_command_lists_the_modes() {
        let mut session = session(&Setup::default());
        let help = session.execute(commands::parse("/help").unwrap());
        assert!(help.contains("/mode"), "{help}");
        assert!(help.contains("manager"), "{help}");
    }

    #[test]
    fn truncation_is_bounded_and_explains_itself() {
        let long = "x".repeat(MAX_REPLY_CHARS + 10);
        let cut = truncate(long);
        assert!(cut.chars().count() <= MAX_REPLY_CHARS + 60);
        assert!(cut.contains("truncated"));
        assert_eq!(truncate("short".to_string()), "short");
    }
}
