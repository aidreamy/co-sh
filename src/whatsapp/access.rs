//! Who may use the agent over WhatsApp.
//!
//! `WhatsApp::is_prompt` (in the `cosh-whatsapp` crate) decides what is a
//! conversation at all — it drops echoes, broadcasts, channels and protocol
//! chatter. This module decides the separate question: whether the account
//! owner authorized that conversation to reach the agent. The policy is
//! read from `setup.json` → `whatsapp` and fails closed: until a contact or
//! group is listed there, only the owner's own self-chat is answered.

use cosh_whatsapp::{ChatKind, IncomingMessage};

use crate::setup::WhatsAppConfig;

/// The resolved access policy, copied once from `setup.json` at startup.
#[derive(Clone, Debug, Default)]
pub struct Access {
    policy: WhatsAppConfig,
}

impl Access {
    /// Build the policy from the persisted `whatsapp` section.
    #[must_use]
    pub fn new(policy: WhatsAppConfig) -> Self {
        Self { policy }
    }

    /// Whether `message` may reach the agent.
    ///
    /// `from_me` messages are the account's own; `is_prompt` only lets the
    /// self-chat through, so the toggle alone applies to them. Every other
    /// message must match its chat's allowlist by kind — direct chats
    /// against `contacts`, groups against `groups`. Chat kinds the policy
    /// does not model are never allowed.
    #[must_use]
    pub fn allows(&self, message: &IncomingMessage) -> bool {
        if message.from_me {
            return self.policy.self_chat;
        }
        let allowed = match message.chat_kind {
            ChatKind::Direct => &self.policy.contacts,
            ChatKind::Group => &self.policy.groups,
            _ => return false,
        };
        let chat = identity(message.chat.as_str());
        allowed
            .iter()
            .any(|entry| !entry.trim().is_empty() && identity(entry) == chat)
    }
}

/// The identity part of a JID or a user-written address: ASCII letters and
/// digits only, lowercased. The `@server` suffix, device (`:12`) suffix,
/// spaces, `+` and punctuation are addressing details, so
/// `+55 (11) 91234-5678`, `5511912345678` and
/// `5511912345678@s.whatsapp.net` all compare equal.
fn identity(id: &str) -> String {
    id.split(['@', ':'])
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use cosh_whatsapp::{ChatId, MessageContent, Sender};

    const OWNER: &str = "5511999999999";

    fn message(chat: &str, from_me: bool) -> IncomingMessage {
        let chat = ChatId::parse(chat).expect("chat id");
        IncomingMessage {
            id: "MSG1".to_string(),
            chat_kind: chat.kind(),
            sender: Sender {
                id: chat.clone(),
                name: None,
            },
            chat,
            from_me,
            timestamp: Utc::now(),
            content: MessageContent::Text("hello".into()),
        }
    }

    fn policy(contacts: &[&str], groups: &[&str]) -> Access {
        Access::new(WhatsAppConfig {
            self_chat: true,
            contacts: contacts.iter().map(|s| s.to_string()).collect(),
            groups: groups.iter().map(|s| s.to_string()).collect(),
        })
    }

    #[test]
    fn the_default_policy_is_the_self_chat_only() {
        let access = Access::default();
        assert!(access.allows(&message(
            &format!("{OWNER}@s.whatsapp.net"),
            true
        )));
        assert!(!access.allows(&message(
            "5511912345678@s.whatsapp.net",
            false
        )));
        assert!(!access.allows(&message("120363012345678901@g.us", false)));
    }

    #[test]
    fn contacts_match_with_or_without_formatting() {
        let access = policy(&["+55 (19) 91234-5678"], &[]);
        assert!(access.allows(&message("5519912345678@s.whatsapp.net", false)));
        // A different number stays denied.
        assert!(!access.allows(&message("5511912345678@s.whatsapp.net", false)));
        // A group message from that same contact is a group rule, not a
        // contact rule.
        assert!(!access.allows(&message("120363012345678901@g.us", false)));
    }

    #[test]
    fn groups_match_by_id_or_digits() {
        let access = policy(&[], &["120363012345678901"]);
        assert!(access.allows(&message("120363012345678901@g.us", false)));
        assert!(!access.allows(&message("120363099999999999@g.us", false)));
        assert!(!access.allows(&message("5511912345678@s.whatsapp.net", false)));
    }

    #[test]
    fn contacts_and_groups_combine() {
        let access = policy(&["5511912345678"], &["120363012345678901@G.US"]);
        assert!(access.allows(&message("5511912345678@s.whatsapp.net", false)));
        assert!(access.allows(&message("120363012345678901@g.us", false)));
        assert!(!access.allows(&message("5511888888888@s.whatsapp.net", false)));
    }

    #[test]
    fn the_self_chat_toggle_and_blank_entries() {
        let mut off = policy(&["5511912345678"], &[]);
        off.policy.self_chat = false;
        assert!(!off.allows(&message(
            &format!("{OWNER}@s.whatsapp.net"),
            true
        )));
        // A blank entry never matches a chat with no identity left.
        let blank = policy(&["", "   "], &[]);
        assert!(!blank.allows(&message("5511912345678@s.whatsapp.net", false)));
    }

    #[test]
    fn unmodelled_chat_kinds_are_never_allowed() {
        let access = policy(&["5511912345678"], &["120363012345678901"]);
        // `ChatKind::Other` is not a direct chat or a group: the policy
        // fails closed instead of inheriting `is_prompt`'s tolerance.
        let mut unknown = message("5511912345678@s.whatsapp.net", false);
        unknown.chat_kind = ChatKind::Other;
        assert!(!access.allows(&unknown));

        let mut broadcast = message("status@broadcast", false);
        broadcast.chat_kind = ChatKind::Broadcast;
        assert!(!access.allows(&broadcast));
    }
}
