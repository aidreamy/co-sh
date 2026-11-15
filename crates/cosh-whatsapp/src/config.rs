//! Link configuration.

use std::path::{Path, PathBuf};

use crate::error::Result;

/// How the companion device should be linked to the phone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Pairing {
    /// Show a QR code to scan (WhatsApp → Linked devices → Link a device).
    #[default]
    QrCode,
    /// Request an eight-character code to type on the phone instead. Useful
    /// when the agent runs somewhere without a screen, e.g. over SSH.
    Code {
        /// Phone number in international format, digits only
        /// (e.g. `15551234567`). [`Config::with_pairing_code`] normalizes
        /// formatted numbers to this form.
        phone: String,
    },
}

/// Everything needed to open a link.
///
/// The device store is the only required setting: it holds the Signal keys
/// and the device identity, so pointing two configs at the same file resumes
/// the same link. Everything else has a sensible default.
#[derive(Clone, Debug)]
pub struct Config {
    store_path: PathBuf,
    push_name: Option<String>,
    skip_history_sync: bool,
    pairing: Pairing,
}

impl Config {
    /// Use `store_path` as the SQLite device store.
    ///
    /// The parent directory is created on connect, so a path under the
    /// user's data directory works without extra setup.
    pub fn new(store_path: impl Into<PathBuf>) -> Self {
        Self {
            store_path: store_path.into(),
            push_name: None,
            skip_history_sync: true,
            pairing: Pairing::QrCode,
        }
    }

    /// Default store location next to the cosh data directory:
    /// `<cosh data dir>/whatsapp/device.db`.
    ///
    /// Returns `None` when the platform cannot name a data directory (for
    /// example when `HOME` is unset), in which case callers should pass an
    /// explicit path to [`Config::new`].
    pub fn default_store_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "cosh")
            .map(|dirs| dirs.data_dir().join("whatsapp").join("device.db"))
    }

    /// The name contacts see for this companion ("push name").
    ///
    /// Defaults to the engine's own default when unset; set it to make the
    /// agent recognizable (e.g. `"cosh"`).
    pub fn with_push_name(mut self, name: impl Into<String>) -> Self {
        self.push_name = Some(name.into());
        self
    }

    /// Whether to process synced chat history from the phone (default: off).
    ///
    /// An agent only needs live traffic, and history sync is a large
    /// download; turn it on if the agent needs context from messages sent
    /// before it was linked.
    pub fn with_history_sync(mut self, enabled: bool) -> Self {
        self.skip_history_sync = !enabled;
        self
    }

    /// Link with an eight-character code sent to `phone` instead of QR code
    /// pairing.
    ///
    /// Formatting is stripped, so a number copied from the phone app
    /// (`+55 (11) 91234-5678`) works as well as digits only.
    ///
    /// The number is validated when the link starts: it must have 7 to 15
    /// digits, like [`ChatId::phone`](crate::ChatId::phone). An empty or
    /// truncated number makes
    /// [`WhatsApp::connect`](crate::WhatsApp::connect) fail with
    /// [`Error::InvalidPhone`](crate::Error::InvalidPhone) before the device
    /// store is touched.
    pub fn with_pairing_code(mut self, phone: impl Into<String>) -> Self {
        let phone: String = phone
            .into()
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        self.pairing = Pairing::Code { phone };
        self
    }

    /// Path of the SQLite device store.
    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    /// Push name, when one was configured.
    pub fn push_name(&self) -> Option<&str> {
        self.push_name.as_deref()
    }

    /// Whether history sync notifications are ignored.
    pub fn skips_history_sync(&self) -> bool {
        self.skip_history_sync
    }

    /// Selected pairing mode.
    pub fn pairing(&self) -> &Pairing {
        &self.pairing
    }

    /// Reject settings that would only fail deep inside the engine.
    pub(crate) fn validate(&self) -> Result<()> {
        if let Pairing::Code { phone } = &self.pairing {
            crate::error::Error::check_phone(phone)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn defaults_skip_history_sync_and_use_qr() {
        let config = Config::new("device.db");
        assert!(config.skips_history_sync());
        assert_eq!(config.pairing(), &Pairing::QrCode);
        assert!(config.push_name().is_none());
    }

    #[test]
    fn builders_override_defaults() {
        let config = Config::new("device.db")
            .with_push_name("cosh")
            .with_history_sync(true)
            .with_pairing_code("15551234567");

        assert_eq!(config.push_name(), Some("cosh"));
        assert!(!config.skips_history_sync());
        assert_eq!(
            config.pairing(),
            &Pairing::Code {
                phone: "15551234567".to_string()
            }
        );
    }

    #[test]
    fn pairing_code_strips_phone_formatting() {
        // The engine wants digits only; a formatted number is what a human
        // copies from their phone app, so normalize it instead of failing.
        let config = Config::new("device.db").with_pairing_code("+55 (11) 91234-5678");
        assert_eq!(
            config.pairing(),
            &Pairing::Code {
                phone: "5511912345678".to_string()
            }
        );
    }

    #[test]
    fn invalid_pairing_codes_are_rejected() {
        // An empty or truncated number used to reach the engine and fail
        // later with an opaque error.
        for phone in ["", "+", "123", "1234567890123456"] {
            let config = Config::new("device.db").with_pairing_code(phone);
            assert!(
                matches!(config.validate(), Err(Error::InvalidPhone { .. })),
                "`{phone}` should be rejected"
            );
        }

        for phone in ["15551234567", "+55 (11) 91234-5678"] {
            let config = Config::new("device.db").with_pairing_code(phone);
            assert!(config.validate().is_ok(), "`{phone}` should be accepted");
        }

        assert!(Config::new("device.db").validate().is_ok());
    }

    #[test]
    fn default_store_path_sits_under_whatsapp() {
        // Depends on the platform naming a data directory; skipped when it
        // cannot (e.g. HOME unset in a sandbox).
        let Some(path) = Config::default_store_path() else {
            return;
        };
        assert!(path.ends_with("whatsapp/device.db") || path.ends_with("whatsapp\\device.db"));
    }
}
