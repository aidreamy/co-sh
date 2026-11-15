//! cosh-whatsapp: WhatsApp Web (multi-device) for agent communication.
//!
//! The crate links a personal WhatsApp account as a companion device — the
//! same flow as "WhatsApp Web" in a browser — and exposes it as a small,
//! engine-agnostic surface: pairing events, normalized inbound messages and
//! typed outbound sends. It deliberately does **not** use Meta's official
//! (and business-only) Cloud API: the transport is the WhatsApp Web
//! multi-device protocol, authenticated by scanning a QR code or by entering
//! an eight-character pairing code.
//!
//! # Layers
//!
//! * [`Config`] — device store location, push name and pairing mode.
//! * [`WhatsApp`] — the live link: [`connect`](WhatsApp::connect) starts it,
//!   [`send_text`](WhatsApp::send_text) talks, [`close`](WhatsApp::close)
//!   stops it.
//! * [`Event`] — a bounded [`tokio::sync::mpsc`] stream of pairing,
//!   connection, failure and message events. It never blocks the protocol
//!   engine: message events are dropped (with a warning) if the consumer
//!   falls behind, while lifecycle events wait up to 30 seconds for room so
//!   a disconnected or logged-out link cannot go unnoticed. Pairing payloads
//!   are raw strings; [`qr::render`] turns one into a terminal QR code.
//! * [`message`] — engine-independent types: [`ChatId`],
//!   [`IncomingMessage`], [`MessageContent`].
//! * [`Agent`] + [`serve`] — the agent-shaped integration: one message in,
//!   one reply out, with the live [`WhatsApp`] handle available for anything
//!   richer.
//!
//! # The engine underneath
//!
//! The protocol work (Noise handshake, Signal sessions, binary node codec,
//! media encryption) is [whatsapp-rust], the actively maintained Rust
//! implementation of the same WhatsApp Web protocol that whatsmeow (Go) and
//! Baileys (TypeScript) implement. Keeping it behind this crate's types is
//! what makes the surface stable: swapping engines later only touches
//! `client.rs`, `event.rs`, `message.rs` and the HTTP seam in `http.rs`, not
//! agent code.
//!
//! # Example
//!
//! ```no_run
//! use async_trait::async_trait;
//! use cosh_whatsapp::{Agent, Config, IncomingMessage, Result, WhatsApp, serve};
//!
//! struct Echo;
//!
//! #[async_trait]
//! impl Agent for Echo {
//!     async fn on_message(
//!         &self,
//!         message: &IncomingMessage,
//!         _whatsapp: &WhatsApp,
//!     ) -> Result<Option<String>> {
//!         Ok(message.text().map(|text| format!("echo: {text}")))
//!     }
//! }
//!
//! # async fn run() -> Result<()> {
//! serve(Config::new("cosh-whatsapp.db"), Echo).await
//! # }
//! ```
//!
//! # Scope
//!
//! Text conversations, groups, reactions and the common media *metadata*
//! (captions, file names, durations) are modeled. Downloading media bytes,
//! calling, status posts and newsletters are intentionally out of scope for
//! this first cut; the underlying engine supports them, so widening the
//! surface later is additive.
//!
//! [whatsapp-rust]: https://github.com/oxidezap/whatsapp-rust

pub mod agent;
mod client;
pub mod config;
pub mod error;
mod event;
mod http;
pub mod message;
pub mod qr;

pub use agent::{Agent, serve};
pub use client::{SentMessage, WhatsApp};
pub use config::Config;
pub use error::{Error, Result};
pub use event::{Event, Events, FailureKind};
pub use message::{ChatId, ChatKind, IncomingMessage, MessageContent, Sender};
