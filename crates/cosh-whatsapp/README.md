# cosh-whatsapp

[![Crates.io](https://img.shields.io/crates/v/cosh-whatsapp.svg)](https://crates.io/crates/cosh-whatsapp)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/DiegoZaluski/co-sh/blob/main/LICENSE)

WhatsApp for agents — a small, typed Rust surface over the WhatsApp Web
multi-device protocol.

`cosh-whatsapp` links a **personal** WhatsApp account as a companion device
(QR code or eight-character pairing code) and exposes it as a stream of
normalized events plus typed sends. It does **not** use Meta's official
Cloud API: the protocol underneath is the same one WhatsApp Web, whatsmeow
and Baileys speak, implemented by [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust).

## Why this crate

- **Engine-independent types.** Agents match on `MessageContent::Text`,
  `MessageContent::Image { .. }` and friends — never on generated protobuf
  messages or wire nodes.
- **Agent-shaped.** Implement `Agent::on_message`, call `serve`, and you have
  a bot with pairing, reconnection and reply sending handled.
- **Escape hatches.** `WhatsApp::connect` returns the raw `Event` stream and
  an `Arc`-backed handle, so a TUI or HTTP server can own the UI while the
  crate owns the protocol.
- **Cross-platform.** Bundled SQLite and rustls — no system SQLite, no
  OpenSSL, no external service or native WhatsApp dependency. The only build
  requirement beyond Rust is a C toolchain for the bundled SQLite.

## Quick start

```toml
[dependencies]
cosh-whatsapp = "0.1"
tokio = { version = "1", features = ["full"] }
```

```rust,no_run
use async_trait::async_trait;
use cosh_whatsapp::{Agent, Config, IncomingMessage, Result, WhatsApp, serve};

struct Echo;

#[async_trait]
impl Agent for Echo {
    async fn on_message(
        &self,
        message: &IncomingMessage,
        _whatsapp: &WhatsApp,
    ) -> Result<Option<String>> {
        Ok(message.text().map(|text| format!("echo: {text}")))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    serve(Config::new("cosh-whatsapp.db"), Echo).await
}
```

On the first run `serve` logs a QR code (pairing instructions are logged at
`info`). Scan it from **WhatsApp → Linked devices → Link a device**. The
device store remembers the session; later runs reconnect without pairing.

You do not need a second number: messages to the account's own "Message
yourself" chat are prompts too. WhatsApp delivers them with `from_me` set,
like the bot's own replies, so `serve` recognises its own messages by id —
every send reserves its id before going out, and a message carrying one is
dropped in every chat. The rule lives in `WhatsApp::is_prompt`, which is also
what a custom event loop should call instead of testing `message.from_me`.

Replies to that chat keep the id the chat carries (its LID on modern
accounts): that is the chat the phone displays as "Message yourself", and
rewriting it to the phone number lands the stanza in a chat the phone never
shows. Every device of the account receives the stanza, the phone included: the
engine fans a direct message out to the recipient's devices, aligned to the
recipient's addressing namespace. A send returns once the stanza is on the
socket — the server's refusal is not an error here — so "sent" means handed
over, not acknowledged.

`serve` handles messages in spawned tasks (up to four at a time), so one slow
LLM reply does not block pairing or other conversations. A slot is reserved
before the task is spawned, so a burst cannot pile up waiting tasks: messages
that arrive while all four slots are busy are shed with a logged warning
(lifecycle events are never dropped). Finished tasks are reaped each
iteration, and an agent that panics is logged instead of vanishing. It
returns only when the account is logged out; a fatal protocol failure comes
back as an error that carries its `FailureKind`. Agents are dyn-compatible
(`Box<dyn Agent>`), so the implementation can be chosen at runtime.

Prefer a terminal UI? Drive the events yourself:

```rust,no_run
# use cosh_whatsapp::{Config, Event, Result, WhatsApp, qr};
# async fn demo() -> Result<()> {
let (whatsapp, mut events) = WhatsApp::connect(Config::new("device.db")).await?;
while let Some(event) = events.recv().await {
    if let Event::QrCode { payload, .. } = event {
        println!("{}", qr::render(&payload)?);
    }
}
# Ok(())
# }
```

Code-based pairing (for a machine without a screen):

```rust,no_run
# use cosh_whatsapp::Config;
let config = Config::new("device.db").with_pairing_code("15551234567");
```

## What is modeled

| Area | Support |
| --- | --- |
| Pairing | QR code, pairing code, persisted session, automatic reconnect |
| Failures | Typed `Event::Failure` (session takeover, temp ban, outdated client, …) |
| Messages | Text, extended text (link previews), captions, files (metadata) |
| Chats | Direct, groups, broadcast/status, channels |
| Self chat | The account's own "Message yourself" chat drives the agent; the agent's own messages are recognised by id, so no loop |
| Interactions | Outbound `send_text`; inbound reactions and polls as typed content (`serve` filters reactions) |
| History | Off by default (`Config::with_history_sync(true)` to opt in) |
| Media bytes | Out of scope — metadata only |
| Calls, status posts, newsletters | Out of scope |

## Architecture

```
src/
├── lib.rs       crate docs + public surface
├── config.rs    store path, push name, pairing mode
├── client.rs    WhatsApp handle: connect / send_text / close / logout
├── event.rs     domain events mapped from the engine
├── message.rs   ChatId, IncomingMessage, MessageContent (engine-independent)
├── qr.rs        terminal QR rendering for pairing payloads
├── agent.rs     Agent trait + serve loop
└── http.rs      reqwest-backed HttpClient seam for the engine
```

The engine is confined to `client.rs`, `event.rs`, `message.rs` and the
`HttpClient` seam in `http.rs`; the rest of the crate compiles against this
crate's own types. Swapping the protocol library later is a change to those
files, not to agent code.

## Notes

- Using a custom WhatsApp client may violate Meta's Terms of Service and can
  lead to account suspension. Use it on a number you can afford to lose.
- One device store = one linked account. For several accounts, give each its
  own `Config` path.
- The event stream is bounded (1024 events) and never blocks the protocol
  loop. Message events are lossy under pressure (logged warning); lifecycle
  events — pairing, connection, failures, logout — wait up to 30 seconds for
  room instead of being dropped, so a stalled UI cannot miss a dead link.
  With `connect`, still drain promptly and move slow work into tasks.
- Call `close()` (or `logout()`) to stop the link — that flushes the device
  store. Dropping the last `WhatsApp` clone aborts the run loop as a safety
  net, so a long-running host should close explicitly.
- The device store contains Signal keys and the device identity, unencrypted.
  On Unix the crate creates it `0600` and, when it has to create the store
  directory, uses `0700` for it (an existing directory keeps its mode); the
  SQLite `-wal`/`-shm` sidecars are hardened too, since they hold recently
  written state. On Windows it relies on the user profile's ACLs. Treat the
  files like a password.
- `send_text` rejects empty or whitespace-only text with
  `Error::EmptyMessage`, and `with_pairing_code` numbers are validated at
  connect time (7–15 digits) instead of failing inside the engine.
- The crate plugs `reqwest` into the engine's HTTP seam, replacing the
  bundled `ureq` client, so no second HTTP stack is built or linked.

## License

Apache-2.0. See [LICENSE](https://github.com/DiegoZaluski/co-sh/blob/main/LICENSE).
