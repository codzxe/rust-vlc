//! Deny-by-default sensitive-op policy — card 7.J1 (doc 05 §5.4, MP §12).
//!
//! Over the control wire, catalog commands split into two classes:
//!
//! * **ordinary** — playback/tracks/media queries and the like: the
//!   token-authenticated session may issue them.
//! * **sensitive** — commands that mutate user settings, touch
//!   filesystem paths, or change plugin state. These are DENIED unless
//!   the operator's config explicitly grants them
//!   ([`PolicyFile::policy::sensitive_allow`]) — deny-by-default is
//!   structural, the same posture as the loopback bind.
//!
//! The classification itself ([`sensitive_command_ids`]) is a closed,
//! documented list derived from the doc 05 §2 tables' own notes (e.g.
//! §5.4: "`session.open` stays privileged because it touches the local
//! filesystem"); membership is asserted against the live catalog in
//! tests, so a catalog rename cannot silently rot it.
//!
//! Wrong-class refusals are the command layer's `permission-denied`
//! (stage `control`) — the connection SURVIVES; only the op is refused.

use std::collections::BTreeSet;

use commands::WireRequest;
use commands::catalog::ids::CommandId;
use commands::error::CommandError;
use commands::error::ErrorCode;
use commands::error::Stage;

/// The sensitive classification: catalog commands that mutate settings,
/// touch filesystem paths, or change plugin state (doc 05 §2 tables +
/// §5.4). Ordered by catalog id for stable diffs.
pub const fn sensitive_command_ids() -> &'static [CommandId] {
    &[
        // session: opens LOCAL filesystem sources (doc 05 §5.4 — "session.open
        // stays privileged because it touches the local filesystem").
        CommandId::SessionOpen,
        // subtitles: adds a subtitle FILE by path (§2.8).
        CommandId::SubtitleAddFile,
        // playlists: save/load WRITE/READ filesystem paths (§2.9).
        CommandId::PlaylistSave,
        CommandId::PlaylistLoad,
        // video: writes a screenshot file (§2.7).
        CommandId::VideoTakeScreenshot,
        // configuration: mutates user settings (§2.11 — admin-locked keys are
        // permission-denied via the control server by design).
        CommandId::ConfigSetSetting,
        CommandId::ConfigResetSetting,
        // system: log verbosity is a settings mutation (§2.12).
        CommandId::SystemSetLogLevel,
        // system: plugin lifecycle (§2.12 — MP §6 sandbox boundary).
        CommandId::SystemPluginEnable,
        CommandId::SystemPluginDisable,
    ]
}

/// Whether a catalog command is in the sensitive class.
pub fn is_sensitive(id: CommandId) -> bool {
    sensitive_command_ids().contains(&id)
}

/// The fixed safe subset admitted to UNauthenticated sessions in
/// read-only mode (doc 05 §5.4): `media.get-info`,
/// `system.get-engine-info`, `tracks.list`, the state snapshot.
pub const READ_ONLY_SAFE: &[CommandId] = &[
    CommandId::MediaGetInfo,
    CommandId::SystemGetEngineInfo,
    CommandId::TracksList,
    CommandId::SystemGetStateSnapshot,
];

/// A parse failure for the operator-facing policy file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyConfigError {
    /// The offending `sensitive_allow` entry.
    pub entry: String,
    /// The underlying unknown-id error (`commands::UnknownId`).
    pub detail: String,
}

impl std::fmt::Display for PolicyConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sensitive_allow entry {:?} is not a catalog command id: {} \
             (see the 64 ids in the doc 05 §2 tables / system.list-commands)",
            self.entry, self.detail
        )
    }
}

impl std::error::Error for PolicyConfigError {}

/// The operator's config file shape (`control-server.toml`), parsed by
/// the host app (doc 07's config family: typed model, unknown keys are
/// errors, defaults stated once — the `product-config` crate's own
/// discipline, expressed here for the control server's own keys).
///
/// ```toml
/// [server]
/// bind = "127.0.0.1"   # loopback only; anything else is refused
/// port = 0             # 0 = ephemeral
/// read_only = false    # doc 05 §5.4 read-only mode
///
/// [auth]
/// token = "…"          # absent = generated per start, printed to the terminal
///
/// [policy]
/// sensitive_allow = [] # Sensitive commands explicitly GRANTED (deny-by-default)
/// ```
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PolicyFile {
    /// Bind + port + read-only mode.
    pub server: ServerSection,
    /// Token source.
    pub auth: AuthSection,
    /// Sensitive-op grants.
    pub policy: PolicySection,
}

/// `[server]` — bind address, port, read-only mode.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerSection {
    /// Bind host; MUST be `127.0.0.1` or `::1` (refused otherwise).
    pub bind: String,
    /// Port; `0` = ephemeral.
    pub port: u16,
    /// Read-only mode (doc 05 §5.4): unauthenticated clients admitted
    /// to [`READ_ONLY_SAFE`] and nothing else.
    pub read_only: bool,
}

impl Default for ServerSection {
    fn default() -> Self {
        ServerSection {
            bind: "127.0.0.1".to_owned(),
            port: 0,
            read_only: false,
        }
    }
}

/// `[auth]` — token source.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AuthSection {
    /// A fixed token (operator-pinned / tests). Absent ⇒ generated per
    /// start and printed to the controlling terminal.
    pub token: Option<String>,
}

/// `[policy]` — the sensitive-op grants.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PolicySection {
    /// Sensitive commands explicitly allowed over the wire (stable
    /// catalog ids). Everything else in the sensitive class stays
    /// denied.
    pub sensitive_allow: Vec<String>,
}

/// The resolved, server-side policy (deny-by-default).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerPolicy {
    /// Granted sensitive commands (explicit config opt-in only).
    grants: BTreeSet<CommandId>,
    /// Read-only mode: unauthenticated sessions get [`READ_ONLY_SAFE`].
    read_only: bool,
}

impl ServerPolicy {
    /// The maximally strict policy: every sensitive command denied,
    /// token authentication required. This is the default.
    pub fn deny_all_sensitive() -> ServerPolicy {
        ServerPolicy::default()
    }

    /// Build from the operator's config file. Unknown ids are typed
    /// errors (typos must surface at startup — doc 07 §3.5's rule,
    /// never silent no-ops).
    ///
    /// # Errors
    /// [`PolicyConfigError`] for any `sensitive_allow` entry that is
    /// not a catalog command id.
    pub fn from_file(file: &PolicyFile) -> Result<ServerPolicy, PolicyConfigError> {
        let mut grants = BTreeSet::new();
        for entry in &file.policy.sensitive_allow {
            match entry.parse::<CommandId>() {
                Ok(id) => {
                    grants.insert(id);
                }
                Err(detail) => {
                    return Err(PolicyConfigError {
                        entry: entry.clone(),
                        detail: detail.to_string(),
                    });
                }
            }
        }
        Ok(ServerPolicy {
            grants,
            read_only: file.server.read_only,
        })
    }

    /// Whether `id` may execute for a session with this auth state.
    ///
    /// * unauthenticated ⇒ only [`READ_ONLY_SAFE`] in read-only mode;
    /// * authenticated ⇒ everything except ungranted sensitive ids.
    ///
    /// Errors carry the command layer's `permission-denied` (V8-safe
    /// message; the command id is not secret).
    pub fn check(&self, id: CommandId, authenticated: bool) -> Result<(), CommandError> {
        if !authenticated {
            if self.read_only && READ_ONLY_SAFE.contains(&id) {
                return Ok(());
            }
            return Err(CommandError::new(
                Stage::Control,
                ErrorCode::PermissionDenied,
                "control server requires token authentication for this command",
            ));
        }
        if is_sensitive(id) && !self.grants.contains(&id) {
            return Err(CommandError::new(
                Stage::Control,
                ErrorCode::PermissionDenied,
                format!(
                    "{id} is a sensitive operation and is not in the server's \
                     sensitive_allow grant list"
                ),
            )
            .with_detail("command", id.as_str()));
        }
        Ok(())
    }

    /// The granted sensitive ids (for the welcome capabilities/log).
    pub fn granted(&self) -> impl Iterator<Item = CommandId> + '_ {
        self.grants.iter().copied()
    }

    /// Read-only mode flag.
    pub fn read_only(&self) -> bool {
        self.read_only
    }
}

/// Extract the catalog id of a validated request (cold path: the id
/// rides the adjacent `"cmd"` tag, recovered via the wire form).
/// `None` is an internal invariant breach (a validated `Command`
/// always carries a known id) — callers must report it as such, never
/// guess.
pub fn request_command_id(request: &WireRequest) -> Option<CommandId> {
    let value = serde_json::to_value(&request.command).unwrap_or(serde_json::Value::Null);
    value
        .get("cmd")
        .and_then(serde_json::Value::as_str)
        .and_then(|text| text.parse::<CommandId>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::catalog::command::Command;
    use commands::envelope::RequestId;

    fn request_for(command: Command) -> WireRequest {
        WireRequest {
            request_id: RequestId(1),
            command,
        }
    }

    #[test]
    fn sensitive_list_is_exactly_the_documented_categories() {
        let ids = sensitive_command_ids();
        assert_eq!(ids.len(), 10, "the closed list: settings, paths, plugins");
        for id in ids {
            // Every member is a real catalog id (FromStr round-trip).
            assert_eq!(id.as_str().parse::<CommandId>().as_ref(), Ok(id));
        }
        // Spot-check the three categories.
        assert!(is_sensitive(CommandId::ConfigSetSetting));
        assert!(is_sensitive(CommandId::SessionOpen));
        assert!(is_sensitive(CommandId::SystemPluginEnable));
        // Ordinary commands are not sensitive.
        assert!(!is_sensitive(CommandId::PlaybackPlay));
        assert!(!is_sensitive(CommandId::MediaGetInfo));
        assert!(!is_sensitive(CommandId::SystemListCommands));
    }

    #[test]
    fn deny_by_default_and_granted_when_enabled() {
        let policy = ServerPolicy::deny_all_sensitive();
        match policy.check(CommandId::ConfigSetSetting, true) {
            Err(err) => {
                assert_eq!(err.code, ErrorCode::PermissionDenied);
                assert_eq!(err.stage, Stage::Control);
                assert!(err.message.contains("sensitive"));
                assert!(!err.message.contains("token"), "no token material");
            }
            Ok(()) => unreachable!("sensitive op allowed by default"),
        }
        // Granted via config.
        let file = PolicyFile {
            policy: PolicySection {
                sensitive_allow: vec!["config.set-setting".to_owned()],
            },
            ..PolicyFile::default()
        };
        let policy = match ServerPolicy::from_file(&file) {
            Ok(policy) => policy,
            Err(err) => unreachable!("valid grant rejected: {err}"),
        };
        assert!(policy.check(CommandId::ConfigSetSetting, true).is_ok());
        // Other sensitive ids stay denied.
        assert!(policy.check(CommandId::SystemPluginEnable, true).is_err());
        // Ordinary commands pass.
        assert!(policy.check(CommandId::PlaybackPlay, true).is_ok());
    }

    #[test]
    fn unknown_grant_entries_are_typed_errors() {
        let file = PolicyFile {
            policy: PolicySection {
                sensitive_allow: vec!["config.set-stuff".to_owned()],
            },
            ..PolicyFile::default()
        };
        match ServerPolicy::from_file(&file) {
            Err(err) => assert!(err.to_string().contains("config.set-stuff")),
            Ok(policy) => unreachable!("typo accepted: {policy:?}"),
        }
    }

    #[test]
    fn unauthenticated_sessions_get_the_read_only_subset_only() {
        // Default (not read-only): nothing without a token.
        let policy = ServerPolicy::deny_all_sensitive();
        assert!(policy.check(CommandId::MediaGetInfo, false).is_err());
        // Read-only: the fixed safe subset passes, everything else fails.
        let file = PolicyFile {
            server: ServerSection {
                read_only: true,
                ..ServerSection::default()
            },
            ..PolicyFile::default()
        };
        let policy = match ServerPolicy::from_file(&file) {
            Ok(policy) => policy,
            Err(err) => unreachable!("read-only config rejected: {err}"),
        };
        assert!(policy.check(CommandId::MediaGetInfo, false).is_ok());
        assert!(policy.check(CommandId::SystemGetEngineInfo, false).is_ok());
        assert!(policy.check(CommandId::TracksList, false).is_ok());
        assert!(policy.check(CommandId::PlaybackPlay, false).is_err());
        assert!(policy.check(CommandId::SessionOpen, false).is_err());
        // Even read-only sessions never reach sensitive ops.
        assert!(policy.check(CommandId::ConfigSetSetting, false).is_err());
    }

    #[test]
    fn policy_file_serde_shape_is_strict() {
        // The HOST app owns TOML parsing (it carries the `toml`
        // external); this test pins the serde CONTRACT of the file
        // shape via its JSON projection — unknown keys are errors,
        // defaults stated once. Real-TOML parsing is covered by the
        // app's integration tests.
        let json = r#"{"server":{"bind":"127.0.0.1","port":4355,"read_only":true},
                      "auth":{"token":"abc"},
                      "policy":{"sensitive_allow":["system.set-log-level"]}}"#;
        match serde_json::from_str::<PolicyFile>(json) {
            Ok(file) => {
                assert_eq!(file.server.bind, "127.0.0.1");
                assert_eq!(file.server.port, 4355);
                assert!(file.server.read_only);
                assert_eq!(file.auth.token.as_deref(), Some("abc"));
                assert_eq!(file.policy.sensitive_allow, ["system.set-log-level"]);
            }
            Err(err) => unreachable!("valid shape rejected: {err}"),
        }
        let dirty = r#"{"server":{"hostname":"x"}}"#;
        assert!(
            serde_json::from_str::<PolicyFile>(dirty).is_err(),
            "unknown keys must be errors (deny_unknown_fields)"
        );
        // Defaults: the deny-everything posture.
        match serde_json::from_str::<PolicyFile>("{}") {
            Ok(file) => {
                assert_eq!(file.server.bind, "127.0.0.1");
                assert_eq!(file.server.port, 0);
                assert!(!file.server.read_only);
                assert!(file.policy.sensitive_allow.is_empty());
                assert_eq!(file.auth.token, None);
            }
            Err(err) => unreachable!("empty file rejected: {err}"),
        }
    }

    #[test]
    fn request_id_extraction_covers_payload_shapes() {
        let request = request_for(Command::PlaybackPlay);
        assert_eq!(request_command_id(&request), Some(CommandId::PlaybackPlay));
        let request = request_for(Command::SessionClose);
        assert_eq!(request_command_id(&request), Some(CommandId::SessionClose));
        let request = request_for(Command::AudioSetVolume { volume: 0.5 });
        assert_eq!(
            request_command_id(&request),
            Some(CommandId::AudioSetVolume)
        );
    }
}
