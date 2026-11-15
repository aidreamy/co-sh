//! Inbound-message routing: one conversation → one persistent agent session.
//!
//! WhatsApp delivers messages from every chat on one event stream. A chat's
//! harness is stateful (its context, its mode), so messages from the same
//! chat must run one at a time, while different chats may run in parallel.
//! The routing layer is therefore a map of chat → worker:
//!
//! - the first message from a chat spawns its worker and a bounded queue;
//! - every later message is sent to that queue;
//! - the worker owns the [`ChatSession`] and processes prompts FIFO, so two
//!   messages can never mutate the same harness concurrently.
//!
//! This mirrors the `serve` loop of `cosh-whatsapp` (which also isolates
//! conversations) while keeping the session state that `serve` deliberately
//! does not own.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, oneshot};

use cosh_whatsapp::{ChatKind, IncomingMessage, MessageContent};

use super::config::RuntimeConfig;
use super::session::ChatSession;

/// One prompt waiting for the chat's worker.
struct TurnRequest {
    prompt: String,
    reply: oneshot::Sender<String>,
}

/// Routes inbound messages to their per-chat session.
pub struct WhatsAppAgent {
    config: Arc<RuntimeConfig>,
    sessions: Mutex<HashMap<String, mpsc::UnboundedSender<TurnRequest>>>,
}

impl WhatsAppAgent {
    /// Build the router over an already-resolved configuration.
    #[must_use]
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config: Arc::new(config),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Handle one inbound message and return the text to send back.
    pub async fn handle(&self, message: &IncomingMessage) -> Option<String> {
        let Some(text) = reply_text(message) else {
            return Some(
                "I can only handle text messages on WhatsApp right now. \
                 Send the request as text."
                    .to_string(),
            );
        };
        let text = text.trim();
        if text.is_empty() {
            return Some("Send a message or `/help` to see the commands.".to_string());
        }

        let key = message.chat.storage_key();
        let tx = self.channel_for(key.clone());
        let (reply_tx, reply_rx) = oneshot::channel();
        let request = TurnRequest {
            prompt: text.to_string(),
            reply: reply_tx,
        };
        let reply_rx = if tx.send(request).is_err() {
            // The worker for this chat died (most likely a panic); drop the
            // stale entry so a fresh worker is spawned and retry once.
            if let Ok(mut sessions) = self.sessions.lock() {
                sessions.remove(&key);
            }
            let tx = self.channel_for(key);
            let (reply_tx, reply_rx) = oneshot::channel();
            let request = TurnRequest {
                prompt: text.to_string(),
                reply: reply_tx,
            };
            if tx.send(request).is_err() {
                return Some(
                    "The agent session for this chat ended; send the message again.".into(),
                );
            }
            reply_rx
        } else {
            reply_rx
        };
        match reply_rx.await {
            Ok(reply) => Some(reply),
            Err(_) => Some("The agent session for this chat ended unexpectedly.".into()),
        }
    }

    /// The worker queue for `chat_key`, spawning the worker on first use.
    fn channel_for(&self, chat_key: String) -> mpsc::UnboundedSender<TurnRequest> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(tx) = sessions.get(&chat_key) {
            return tx.clone();
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let config = Arc::clone(&self.config);
        let key = chat_key.clone();
        tokio::spawn(async move {
            let mut session = ChatSession::new(config, key);
            run_worker(&mut session, rx).await;
        });
        sessions.insert(chat_key, tx.clone());
        tx
    }
}

/// Serialize one chat's prompts through its session.
async fn run_worker(session: &mut ChatSession, mut rx: mpsc::UnboundedReceiver<TurnRequest>) {
    while let Some(request) = rx.recv().await {
        let reply = session.handle(&request.prompt).await;
        // The requester may have gone away (link closed); the turn's result
        // still belongs to the session context, which was already updated.
        let _ = request.reply.send(reply);
    }
}

/// The text an inbound message contributes, if any.
///
/// Media captions are treated as text: they are the user's words about the
/// attachment. The attachment bytes themselves are out of scope (the crate
/// models metadata only).
fn reply_text(message: &IncomingMessage) -> Option<&str> {
    message.text().or_else(|| message.caption())
}

/// Whether an inbound message is a conversation the agent should answer.
///
/// Echoes of this account, status/broadcast posts, channel messages,
/// protocol chatter (edits, deletes, receipts) and reactions are not: an
/// agent cannot meaningfully reply to any of them. The decision uses the
/// `chat_kind` computed at normalization time instead of re-parsing the JID.
#[must_use]
pub fn is_agent_traffic(message: &IncomingMessage) -> bool {
    if message.from_me
        || matches!(
            message.chat_kind,
            ChatKind::Broadcast | ChatKind::Newsletter
        )
    {
        return false;
    }
    !matches!(
        message.content,
        MessageContent::System | MessageContent::Unsupported | MessageContent::Reaction { .. }
    )
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use cosh_whatsapp::{ChatId, Sender};

    fn message(chat: &str, content: MessageContent) -> IncomingMessage {
        let chat_id = ChatId::parse(chat).expect("chat id");
        IncomingMessage {
            id: "MSG1".to_string(),
            chat_kind: chat_id.kind(),
            chat: chat_id,
            sender: Sender {
                id: ChatId::parse(chat).expect("sender id"),
                name: None,
            },
            from_me: false,
            timestamp: Utc::now(),
            content,
        }
    }

    const DIRECT: &str = "5511912345678@s.whatsapp.net";

    #[test]
    fn direct_text_reaches_the_agent() {
        let text = message(DIRECT, MessageContent::Text("hello".into()));
        assert!(is_agent_traffic(&text));
        assert_eq!(reply_text(&text), Some("hello"));
    }

    #[test]
    fn echoes_broadcasts_and_channels_are_ignored() {
        let mut echo = message(DIRECT, MessageContent::Text("sent by me".into()));
        echo.from_me = true;
        assert!(!is_agent_traffic(&echo));

        assert!(!is_agent_traffic(&message(
            "status@broadcast",
            MessageContent::Text("status".into())
        )));
        assert!(!is_agent_traffic(&message(
            "120363012345678901@newsletter",
            MessageContent::Text("post".into())
        )));
    }

    #[test]
    fn protocol_noise_and_reactions_are_ignored() {
        for content in [
            MessageContent::System,
            MessageContent::Unsupported,
            MessageContent::Reaction {
                emoji: "👍".into(),
                target_message_id: Some("MSG0".into()),
            },
        ] {
            assert!(!is_agent_traffic(&message(DIRECT, content)));
        }
    }

    #[test]
    fn media_captions_are_treated_as_text() {
        let image = message(
            DIRECT,
            MessageContent::Image {
                caption: Some("/help".into()),
                mime_type: Some("image/jpeg".into()),
            },
        );
        assert_eq!(reply_text(&image), Some("/help"));
        assert!(is_agent_traffic(&image));

        let uncaptioned = message(
            DIRECT,
            MessageContent::Image {
                caption: None,
                mime_type: None,
            },
        );
        assert_eq!(reply_text(&uncaptioned), None);
    }
}
