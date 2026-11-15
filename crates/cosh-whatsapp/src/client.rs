//! The live link to WhatsApp.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use whatsapp_rust::bot::{Bot, BotHandle};
use whatsapp_rust::pair_code::PairCodeOptions;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::store::traits::Backend;
use whatsapp_rust::transport::TokioWebSocketTransportFactory;
use whatsapp_rust::waproto::whatsapp as wa;
use whatsapp_rust::{Client, TokioRuntime};

use crate::config::{Config, Pairing};
use crate::error::{Error, Result};
use crate::event::{BUFFER, Event, EventSink, Events};
use crate::http::ReqwestHttpClient;
use crate::message::ChatId;

/// A message that was handed to WhatsApp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentMessage {
    /// Server message id, usable for edits and reactions.
    pub id: String,
    /// The chat it was sent to.
    pub to: ChatId,
}

/// A linked WhatsApp account.
///
/// Cheap to clone (it is an `Arc` inside); every clone talks over the same
/// connection and shares the same lifecycle.
///
/// Call [`close`](WhatsApp::close) (or [`logout`](WhatsApp::logout)) to stop
/// the link: that is the graceful path, which flushes the device store and
/// lets the engine disconnect. Dropping the last clone is only a safety net —
/// it aborts the engine's run loop mid-flight, so nothing is flushed. A
/// long-running host should close explicitly.
#[derive(Clone, Debug)]
pub struct WhatsApp {
    inner: Arc<Inner>,
}

struct Inner {
    client: Arc<Client>,
    /// Kept alive so dropping it does not abort the run loop; `stop` aborts
    /// explicitly once the caller is done.
    run: Mutex<Option<BotHandle>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhatsApp")
            .field("connected", &self.client.is_connected())
            .finish()
    }
}

impl Inner {
    fn stop(&self) {
        if let Ok(mut run) = self.run.lock()
            && let Some(handle) = run.take()
        {
            handle.abort();
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.stop();
    }
}

impl WhatsApp {
    /// Open the device store and start the link.
    ///
    /// Returns the handle and the event stream to drive pairing and
    /// messages with. Pairing itself happens asynchronously: watch for
    /// [`Event::QrCode`] (or [`Event::PairingCode`]) and show the payload to
    /// the user, then wait for [`Event::Connected`].
    ///
    /// An existing session in the device store reconnects without pairing.
    ///
    /// The returned [`Events`] queue holds at most 1024 pending events and is
    /// never allowed to block the protocol engine. If the caller falls behind,
    /// message events are dropped with a logged warning; lifecycle events
    /// (pairing, connection, failures, logout) wait for room for up to 30
    /// seconds instead. Drain it promptly; a slow handler should move its work
    /// into a spawned task.
    pub async fn connect(config: Config) -> Result<(Self, Events)> {
        // Fail before touching the disk or the engine: an empty or truncated
        // number would otherwise surface as an opaque engine error.
        config.validate()?;

        prepare_store_dir(config.store_path()).await?;

        let store = SqliteStore::new(&config.store_path().to_string_lossy())
            .await
            .map_err(Error::storage)?;
        restrict_store_permissions(config.store_path());

        let (tx, events) = mpsc::channel(BUFFER);
        let sink = EventSink::new(tx);

        let mut builder = Bot::builder()
            .with_backend(Arc::new(store) as Arc<dyn Backend>)
            .with_transport_factory(TokioWebSocketTransportFactory::new())
            .with_http_client(ReqwestHttpClient::new().map_err(Error::session)?)
            .with_runtime(TokioRuntime)
            .on_event(move |engine_event, _client| {
                let sink = sink.clone();
                async move {
                    if let Some(event) = Event::from_engine(&engine_event) {
                        sink.send(event).await;
                    }
                }
            });

        if let Some(name) = config.push_name() {
            builder = builder.with_push_name(name);
        }
        if config.skips_history_sync() {
            builder = builder.skip_history_sync();
        }
        if let Pairing::Code { phone } = config.pairing() {
            builder = builder.with_pair_code(PairCodeOptions {
                phone_number: phone.clone(),
                ..Default::default()
            });
        }

        let mut bot = builder.build().await.map_err(Error::session)?;
        // The store may have written its WAL files while opening; harden
        // them again now that they exist.
        restrict_store_permissions(config.store_path());
        let handle = bot.run().await.map_err(Error::session)?;

        Ok((
            Self {
                inner: Arc::new(Inner {
                    client: bot.client(),
                    run: Mutex::new(Some(handle)),
                }),
            },
            events,
        ))
    }

    /// Send a text message to `chat`.
    ///
    /// Empty or whitespace-only text is rejected with [`Error::EmptyMessage`]
    /// instead of uploading a blank message.
    pub async fn send_text(&self, chat: &ChatId, text: impl Into<String>) -> Result<SentMessage> {
        let message = wa::Message {
            conversation: Some(text_body(text)?),
            ..Default::default()
        };
        let result = self
            .inner
            .client
            .send_message(chat.to_jid()?, message)
            .await
            .map_err(Error::send)?;

        Ok(SentMessage {
            id: result.message_id,
            to: ChatId::from_jid(&result.to),
        })
    }

    /// Wait until the link is connected, authenticated and done with its
    /// critical app-state sync (or until `timeout` elapses).
    pub async fn wait_until_connected(&self, timeout: Duration) -> Result<()> {
        self.inner
            .client
            .wait_for_connected(timeout)
            .await
            .map_err(Error::session)
    }

    /// Whether a network connection is currently up.
    pub fn is_connected(&self) -> bool {
        self.inner.client.is_connected()
    }

    /// Whether this device completed pairing and is authenticated.
    pub fn is_linked(&self) -> bool {
        self.inner.client.is_logged_in()
    }

    /// Unlink this companion from the phone and stop.
    ///
    /// The Signal keys are wiped on the server side; the local device store
    /// keeps its bytes but is useless, so delete it to start over cleanly.
    pub async fn logout(&self) -> Result<()> {
        // Stop the run loop even when the server-side logout fails, so a
        // failed unlink does not leave the link (and its tasks) running.
        let result = self.inner.client.logout().await.map_err(Error::session);
        self.inner.stop();
        result
    }

    /// Disconnect gracefully and stop the link.
    ///
    /// Flushes the device state first, so the next connect resumes the
    /// session without re-pairing.
    pub async fn close(&self) {
        self.inner.client.disconnect().await;
        self.inner.stop();
    }
}

/// Create the device-store directory when it is missing.
///
/// Only a directory created by this call is hardened to `0700`: an existing
/// directory belongs to the caller (it may be a shared path such as `/tmp`),
/// and changing its mode would be a surprising, system-wide side effect.
async fn prepare_store_dir(store_path: &Path) -> Result<()> {
    let Some(parent) = store_path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    let existed = parent.exists();
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(Error::storage)?;
    if !existed {
        restrict_dir_permissions(parent);
    }
    Ok(())
}

/// The text of a message, rejecting blank input.
fn text_body(text: impl Into<String>) -> Result<String> {
    let text = text.into();
    if text.trim().is_empty() {
        return Err(Error::EmptyMessage);
    }
    Ok(text)
}

/// The device store holds Signal keys and the device identity, so keep it
/// readable only by its owner where the platform supports it.
///
/// SQLite runs the store in WAL mode, which puts recently written state in
/// `-wal` and `-shm` files next to the database; those are created after the
/// database file (with the process umask), so they need the same hardening.
#[cfg(unix)]
fn restrict_store_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    for candidate in [
        path.to_path_buf(),
        with_suffix(path, "-wal"),
        with_suffix(path, "-shm"),
    ] {
        match std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(0o600)) {
            Ok(()) => {}
            // The sidecars only exist once the store has written something.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => log::warn!(
                "cosh-whatsapp: could not restrict permissions of {}: {err}",
                candidate.display()
            ),
        }
    }
}

#[cfg(not(unix))]
fn restrict_store_permissions(_path: &Path) {}

#[cfg(unix)]
fn with_suffix(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut os = path.as_os_str().to_owned();
    os.push(suffix);
    std::path::PathBuf::from(os)
}

#[cfg(unix)]
fn restrict_dir_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Err(err) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)) {
        log::warn!(
            "cosh-whatsapp: could not restrict permissions of {}: {err}",
            path.display()
        );
    }
}

#[cfg(not(unix))]
fn restrict_dir_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_text_is_rejected() {
        assert!(matches!(text_body(""), Err(Error::EmptyMessage)));
        assert!(matches!(text_body("  \n\t "), Err(Error::EmptyMessage)));
        assert_eq!(text_body(" hi ").expect("non-blank text"), " hi ");
    }

    fn temp_store_dir(name: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "cosh-whatsapp-{name}-{}-{serial}",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn missing_store_directories_are_created_and_hardened() {
        let dir = temp_store_dir("created").join("nested");
        let db = dir.join("device.db");

        prepare_store_dir(&db).await.expect("create directory");
        assert!(dir.is_dir());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&dir)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o700, "a directory we created is owner-only");
        }

        let _ = std::fs::remove_dir_all(dir.parent().expect("parent"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn existing_store_directories_keep_their_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_store_dir("existing");
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        prepare_store_dir(&dir.join("device.db"))
            .await
            .expect("prepare");

        let mode = std::fs::metadata(&dir)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o755,
            "an existing directory (e.g. /tmp) must not be re-permissioned"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn sidecar_files_are_hardened_with_the_store() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "cosh-whatsapp-perms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let db = dir.join("device.db");
        let wal = with_suffix(&db, "-wal");
        let shm = with_suffix(&db, "-shm");

        // Missing sidecars (the common case before the first write) are fine.
        restrict_store_permissions(&db);

        for path in [&db, &wal, &shm] {
            std::fs::write(path, b"x").expect("write store file");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        }
        restrict_store_permissions(&db);

        for path in [&db, &wal, &shm] {
            let mode = std::fs::metadata(path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "{} is readable by others", path.display());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
