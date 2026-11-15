//! The agent integration: one message in, one reply out.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::{JoinError, JoinSet};

use crate::client::WhatsApp;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::event::Event;
use crate::message::{ChatKind, IncomingMessage, MessageContent};
use crate::qr;

/// How many agent calls may run at once. Replies are independent, so a slow
/// answer must not hold up the messages behind it, but unbounded parallelism
/// would let a burst of chats hammer the model at the same time.
const MAX_IN_FLIGHT: usize = 4;

/// Turns inbound WhatsApp messages into replies.
///
/// Implementations get the live [`WhatsApp`] handle too, so a reply is not
/// limited to one string: an agent can send several messages, target another
/// chat, or react, while the returned text is simply the common case.
///
/// The trait is dyn-compatible, so an agent can be chosen at runtime:
/// `let agent: Box<dyn Agent> = ...; serve(config, agent)`.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Handle one inbound message.
    ///
    /// Return `Ok(Some(text))` to reply in the same chat, `Ok(None)` to stay
    /// silent, or an error to log and skip the message. [`serve`] filters
    /// echoes of this account, status/broadcast posts, channel messages,
    /// protocol chatter and reactions before this method is called. Use
    /// [`WhatsApp::connect`] directly if reactions should reach the agent.
    ///
    /// Calls for different messages run concurrently (up to four at a time),
    /// so implementations must not assume they are serialized.
    async fn on_message(
        &self,
        message: &IncomingMessage,
        whatsapp: &WhatsApp,
    ) -> Result<Option<String>>;
}

#[async_trait]
impl Agent for Box<dyn Agent> {
    async fn on_message(
        &self,
        message: &IncomingMessage,
        whatsapp: &WhatsApp,
    ) -> Result<Option<String>> {
        (**self).on_message(message, whatsapp).await
    }
}

/// Run `agent` against a WhatsApp link until the link stops or the account is
/// logged out.
///
/// This is the batteries-included entry point: it connects, renders pairing
/// QR codes to the log, forwards inbound messages to the agent and sends the
/// replies. Use [`WhatsApp::connect`] directly when the caller owns the UI
/// (a TUI, an HTTP server) and needs the raw [`Event`] stream.
///
/// Messages are handled in spawned tasks, up to four at a time, so a slow
/// reply (an LLM call, for instance) does not stall lifecycle events or other
/// conversations. A slot is reserved **before** the task is spawned, so a
/// burst cannot pile up waiting tasks: a message that arrives while all four
/// slots are busy is shed with a logged warning instead. Lifecycle events
/// (pairing, connection, failures, logout) wait for room in the event queue
/// instead of being dropped immediately.
///
/// Finished tasks are reaped on every iteration and a panicking agent is
/// logged instead of disappearing silently.
///
/// Pairing instructions are logged at `info`; a QR code appears in the log
/// for each code the server issues. The status of every event is logged, so
/// a normal `env_logger` setup is enough to link an account.
///
/// Returns `Ok(())` only when the account is logged out (the intended way to
/// stop). A link that ends for any other reason — a fatal failure such as
/// another session taking over, or the event stream closing — returns an
/// error, so a supervisor can restart or report it.
pub async fn serve<A: Agent + 'static>(config: Config, agent: A) -> Result<()> {
    let (whatsapp, mut events) = WhatsApp::connect(config).await?;
    let agent = Arc::new(agent);
    let in_flight = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
    let mut tasks = JoinSet::new();

    let outcome = loop {
        // Collect finished replies (and surface panics) before waiting for
        // the next event, so the set does not grow for the process lifetime.
        reap(&mut tasks);

        let Some(event) = events.recv().await else {
            break Err(Error::Stopped);
        };

        match event {
            Event::QrCode { payload, .. } => match qr::render(&payload) {
                Ok(code) => log::info!(
                    "Scan this QR code from WhatsApp → Linked devices → Link a device:\n{code}"
                ),
                Err(err) => log::error!("cosh-whatsapp: {err}"),
            },
            Event::PairingCode { code, .. } => log::info!(
                "On the phone: WhatsApp → Linked devices → Link with phone number instead, \
                 then type {code}"
            ),
            Event::PairingFailed { reason } => {
                log::warn!("cosh-whatsapp: pairing failed: {reason}")
            }
            Event::Connected => log::info!("cosh-whatsapp: connected"),
            Event::Disconnected => log::warn!("cosh-whatsapp: connection dropped, reconnecting"),
            Event::LoggedOut => {
                log::warn!("cosh-whatsapp: device unlinked from the phone; stopping");
                break Ok(());
            }
            Event::Failure { kind, reason } if kind.is_fatal() => {
                log::error!("cosh-whatsapp: unrecoverable failure: {reason}");
                break Err(Error::LinkFailed { kind, reason });
            }
            Event::Failure { reason, .. } => {
                log::warn!("cosh-whatsapp: {reason}; the engine retries with backoff");
            }
            Event::Message(message) => {
                if !is_agent_traffic(&message) {
                    continue;
                }

                let Some(permit) = take_slot(&in_flight) else {
                    log::warn!(
                        "cosh-whatsapp: all {MAX_IN_FLIGHT} agent slots are busy, dropped message {} from {}",
                        message.id,
                        message.sender.id
                    );
                    continue;
                };

                let agent = Arc::clone(&agent);
                let whatsapp = whatsapp.clone();
                tasks.spawn(async move {
                    // Holding the permit for the whole reply keeps the cap
                    // honest; it is released when the task ends.
                    let _permit = permit;
                    let reply = agent.on_message(&message, &whatsapp).await;

                    match reply {
                        Ok(Some(reply)) if !reply.trim().is_empty() => {
                            if let Err(err) = whatsapp.send_text(&message.chat, reply).await {
                                log::warn!(
                                    "cosh-whatsapp: reply to {} failed: {err}",
                                    message.chat
                                );
                            }
                        }
                        Ok(_) => {}
                        Err(err) => log::warn!(
                            "cosh-whatsapp: agent failed on a message from {}: {err}",
                            message.sender.id
                        ),
                    }
                });
            }
        }
    };

    // Do not let in-flight replies outlive the link they would write to.
    tasks.abort_all();
    while let Some(finished) = tasks.join_next().await {
        observe(finished);
    }
    whatsapp.close().await;

    outcome
}

/// Reserve one of the agent slots, or report that the loop is saturated.
fn take_slot(in_flight: &Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    Arc::clone(in_flight).try_acquire_owned().ok()
}

/// Reap every finished task, logging agent panics.
fn reap(tasks: &mut JoinSet<()>) {
    while let Some(finished) = tasks.try_join_next() {
        observe(finished);
    }
}

fn observe(finished: std::result::Result<(), JoinError>) {
    if let Err(err) = finished
        && err.is_panic()
    {
        log::error!("cosh-whatsapp: an agent task panicked: {err}");
    }
}

/// Whether an inbound message is a conversation an agent should answer.
///
/// Echoes of this account, status/broadcast posts, channel messages,
/// protocol chatter (edits, deletes, receipts) and reactions are not — an
/// agent cannot meaningfully reply to any of them. The decision uses
/// [`IncomingMessage::chat_kind`], which was computed once when the message
/// was normalized, instead of parsing the JID again.
fn is_agent_traffic(message: &IncomingMessage) -> bool {
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
    use crate::message::{ChatId, ChatKind, Sender};

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

    #[test]
    fn text_traffic_reaches_the_agent() {
        assert!(is_agent_traffic(&message(
            "5511912345678@s.whatsapp.net",
            MessageContent::Text("hi".to_string())
        )));
    }

    #[test]
    fn protocol_noise_is_filtered_out() {
        for content in [MessageContent::System, MessageContent::Unsupported] {
            assert!(
                !is_agent_traffic(&message("5511912345678@s.whatsapp.net", content)),
                "protocol chatter should not reach the agent"
            );
        }
    }

    #[test]
    fn echoes_broadcasts_and_channels_are_filtered_out() {
        let mut echo = message(
            "5511912345678@s.whatsapp.net",
            MessageContent::Text("sent by me".to_string()),
        );
        echo.from_me = true;
        assert!(!is_agent_traffic(&echo));

        assert!(!is_agent_traffic(&message(
            "status@broadcast",
            MessageContent::Text("status".to_string())
        )));

        assert!(!is_agent_traffic(&message(
            "120363012345678901@newsletter",
            MessageContent::Text("channel post".to_string())
        )));
    }

    struct Recorder;

    #[async_trait]
    impl Agent for Recorder {
        async fn on_message(
            &self,
            _message: &IncomingMessage,
            _whatsapp: &WhatsApp,
        ) -> Result<Option<String>> {
            Ok(Some("ack".to_string()))
        }
    }

    #[tokio::test]
    async fn slots_are_reserved_before_spawning() {
        let in_flight = Arc::new(Semaphore::new(1));
        let slot = take_slot(&in_flight).expect("first message gets a slot");
        // A second message is shed instead of spawning a task that waits.
        assert!(take_slot(&in_flight).is_none());
        drop(slot);
        assert!(take_slot(&in_flight).is_some());
    }

    #[tokio::test]
    async fn finished_tasks_are_reaped_and_panics_surface() {
        let mut tasks = JoinSet::new();
        tasks.spawn(async {});
        tasks.spawn(async { panic!("agent blew up") });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        assert_eq!(tasks.len(), 2, "both tasks are still owned by the set");
        reap(&mut tasks);
        assert!(tasks.is_empty(), "finished tasks must be reaped");
    }

    #[test]
    fn chat_kind_decides_noise_without_reparsing_the_jid() {
        // The filter trusts the kind computed at normalization time.
        let mut channel = message(
            "5511912345678@s.whatsapp.net",
            MessageContent::Text("post".to_string()),
        );
        channel.chat_kind = ChatKind::Newsletter;
        assert!(!is_agent_traffic(&channel));
    }

    #[test]
    fn reactions_are_filtered_out() {
        // `serve` must not answer a thumbs-up with "echo: 👍".
        let reaction = message(
            "5511912345678@s.whatsapp.net",
            MessageContent::Reaction {
                emoji: "👍".to_string(),
                target_message_id: Some("MSG0".to_string()),
            },
        );
        assert!(!is_agent_traffic(&reaction));
    }

    #[test]
    fn agent_is_dyn_compatible() {
        fn takes_agent<A: Agent + 'static>(_: A) {}

        let agent: Box<dyn Agent> = Box::new(Recorder);
        // Both `&dyn Agent` and `Box<dyn Agent>` satisfy the trait.
        let _ = agent.as_ref() as &dyn Agent;
        takes_agent(agent);
    }
}
