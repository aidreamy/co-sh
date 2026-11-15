//! Minimal echo bot: links a personal account and echoes every inbound
//! message back to the same chat.
//!
//! Run with:
//!
//! ```text
//! cargo run -p cosh-whatsapp --example echo
//! ```
//!
//! A QR code is printed to stdout the first time; scan it from
//! WhatsApp → Linked devices → Link a device. The device store defaults to
//! `cosh-whatsapp.db` in the working directory, so a second run resumes the
//! link without pairing again.

use cosh_whatsapp::{Config, Event, Result, WhatsApp, qr};

#[tokio::main]
async fn main() -> Result<()> {
    let store = Config::default_store_path().unwrap_or_else(|| "cosh-whatsapp.db".into());
    println!("device store: {}", store.display());

    let (whatsapp, mut events) = WhatsApp::connect(Config::new(store)).await?;

    loop {
        let event = tokio::select! {
            event = events.recv() => event,
            // Closing flushes the device store; dropping the handle would
            // abort the engine's run loop instead.
            _ = tokio::signal::ctrl_c() => {
                println!("shutting down");
                break;
            }
        };
        let Some(event) = event else {
            break;
        };
        match event {
            Event::QrCode { payload, .. } => {
                println!("\n{}", qr::render(&payload)?);
                println!("scan the QR code above from WhatsApp → Linked devices\n");
            }
            Event::PairingCode { code, .. } => {
                println!("pairing code: {code}");
            }
            Event::PairingFailed { reason } => {
                eprintln!("pairing failed: {reason}");
            }
            Event::Connected => println!("connected"),
            Event::Disconnected => eprintln!("connection dropped, reconnecting"),
            Event::Failure { kind, reason } => {
                eprintln!("failure ({kind:?}): {reason}");
                if kind.is_fatal() {
                    break;
                }
            }
            Event::LoggedOut => {
                eprintln!("device unlinked from the phone; stopping");
                break;
            }
            Event::Message(message) => {
                if message.from_me {
                    continue;
                }
                let Some(text) = message.text() else {
                    println!(
                        "ignoring a {} message in {}",
                        message.content.kind(),
                        message.chat
                    );
                    continue;
                };
                println!("{}: {text}", message.sender.id);
                whatsapp
                    .send_text(&message.chat, format!("echo: {text}"))
                    .await?;
            }
            _ => {}
        }
    }

    whatsapp.close().await;
    Ok(())
}
