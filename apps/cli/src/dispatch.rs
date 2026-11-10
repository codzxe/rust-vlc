//! The one dispatch entry (card 2.D2): every host surface — argv
//! subcommands, the interactive REPL, the `--control-stdio` machine
//! mode — builds a [`WireRequest`] and calls [`lumen_dispatch`].
//! This is the CLI's shape of doc 05 §5.1's `lumen_dispatch` (the
//! Tauri shell registers the same single command; ARCH-5: one command
//! layer, no per-host re-implementations, no bypasses).
//!
//! # The served table
//!
//! Doc 05 §5.3 maps the CLI's top-30 verbs onto catalog ids; the
//! first vertical owns the **playing subset** (the state machine +
//! snapshots + the settings seam — exactly what the engine and its
//! wired pipeline can honor today). [`SERVED`] is that set as data;
//! the `--help` synopsis ([`crate::help`]) is tested against it, so
//! the two cannot drift. Every other catalog id answers the honest
//! typed refusal `unsupported-for-host` naming the wave that owns it
//! — the same posture doc 05 §5.2 prescribes for the web host's
//! capability gaps ("typed, not silent").
//!
//! # Class-A completion truth
//!
//! `session.open`, `playback.play/pause/toggle/stop/seek` answer
//! [`CommandOutcome::Accepted`] and their evidence arrives as events
//! (the host records and prints them); class-S snapshot commands
//! answer [`CommandOutcome::Completed`] with their payload (doc 05
//! §1.4 — the truthful-UI rule). `session.close` is class S in the
//! catalog and the engine's receipt agrees (`Completed`).

use commands::envelope::{CommandOutcome, Generation};
use commands::{Command, CommandError, CommandId, CommandResult, WireRequest, WireResponse};
use playback_engine::SessionCommandKind;

use crate::host::CliHost;

/// The catalog ids this build serves, in synopsis order (doc 05 §5.3's
/// table restricted to the first vertical's honest surface). Tested
/// against [`crate::help::synopsis_catalog_ids`].
pub const SERVED: &[CommandId] = &[
    CommandId::SessionOpen,
    CommandId::SessionClose,
    CommandId::PlaybackPlay,
    CommandId::PlaybackPause,
    CommandId::PlaybackTogglePlayPause,
    CommandId::PlaybackStop,
    CommandId::PlaybackSeek,
    CommandId::MediaGetInfo,
    CommandId::MediaGetCurrentTime,
    CommandId::MediaGetDuration,
    CommandId::TracksList,
    CommandId::SystemGetEngineInfo,
    CommandId::SystemListCommands,
    CommandId::ConfigGetSetting,
    CommandId::ConfigSetSetting,
];

/// **The** dispatch entry (doc 05 §5.1's shape for the CLI host):
/// correlate, route through the engine, answer the wire response.
///
/// The response's `session`/`generation` fields carry the context at
/// answering time (doc 05 §1.6); `request_id` is always echoed.
#[must_use]
pub fn lumen_dispatch(host: &mut CliHost, request: &WireRequest) -> WireResponse {
    let request_id = request.request_id;
    let result = route(host, &request.command);
    WireResponse {
        request_id,
        session: host.session_id(),
        generation: host.generation().unwrap_or(Generation(0)),
        result,
    }
}

/// One catalog command → its outcome (the served table + the honest
/// typed refusals).
fn route(
    host: &mut CliHost,
    command: &Command,
) -> Result<CommandOutcome<CommandResult>, CommandError> {
    match command {
        // ---- session (§2.3) ----
        Command::SessionOpen { source, options } => {
            let outcome = host.open_session(
                source.clone(),
                options.start_time.map(|time| time.0),
                options.no_video,
                options.no_audio,
            )?;
            if options.start_paused {
                // `--start-paused` composes as open → settle → pause:
                // the engine's `Buffering` has no pause row (the
                // transient states are card 3.C1's), so the honest
                // composition lets the open settle (auto-play — the
                // `Primed` signal) and pauses from `Playing`. A
                // bounded number of frames may present during the
                // settle window; recorded here rather than hidden.
                host.settle();
                host.submit_and_await(SessionCommandKind::Pause)
                    .and_then(|receipt| host.receipt_outcome(receipt))?;
            }
            Ok(outcome)
        }
        Command::SessionClose => host
            .submit_and_await(SessionCommandKind::Close)
            .and_then(|receipt| host.receipt_outcome(receipt))
            .inspect(|_| {
                // A settled Close drops the host's session-scoped
                // halves: later snapshots refuse with not-ready (the
                // close test's honest-not-ready path).
                host.session_closed();
            }),
        // ---- playback (§2.4) ----
        Command::PlaybackPlay => host
            .submit_and_await(SessionCommandKind::Play)
            .and_then(|receipt| host.receipt_outcome(receipt)),
        Command::PlaybackPause => host
            .submit_and_await(SessionCommandKind::Pause)
            .and_then(|receipt| host.receipt_outcome(receipt)),
        Command::PlaybackTogglePlayPause => {
            // The primary keymap target: toggle by the live state.
            let kind = if host.state() == commands::PlaybackState::Playing {
                SessionCommandKind::Pause
            } else {
                SessionCommandKind::Play
            };
            host.submit_and_await(kind)
                .and_then(|receipt| host.receipt_outcome(receipt))
        }
        Command::PlaybackStop => host
            .submit_and_await(SessionCommandKind::Stop)
            .and_then(|receipt| host.receipt_outcome(receipt)),
        Command::PlaybackSeek { target } => host.dispatch_seek(*target),
        // ---- snapshots (§2.6/§2.12 — class S) ----
        Command::MediaGetInfo => Ok(CommandOutcome::Completed(host.media_info()?)),
        Command::MediaGetCurrentTime => Ok(CommandOutcome::Completed(host.current_time()?)),
        Command::MediaGetDuration => Ok(CommandOutcome::Completed(host.duration()?)),
        Command::TracksList => Ok(CommandOutcome::Completed(CommandResult::TrackList {
            tracks: host.track_snapshot(),
        })),
        Command::SystemGetEngineInfo => Ok(CommandOutcome::Completed(CommandResult::EngineInfo {
            info: crate::host::engine_info(),
        })),
        Command::SystemListCommands => {
            Ok(CommandOutcome::Completed(CommandResult::CommandListings {
                commands: crate::host::command_listings(),
            }))
        }
        // ---- the settings seam (§2.11) ----
        Command::ConfigGetSetting { area, key } => {
            Ok(CommandOutcome::Completed(host.setting_get(area, key)?))
        }
        Command::ConfigSetSetting { area, key, value } => Ok(CommandOutcome::Completed(
            host.setting_set(area, key, value.clone())?,
        )),
        // ---- everything else: the honest typed refusal ----
        other => Err(unserved(other)),
    }
}

/// The typed refusal for a catalog command this host does not serve
/// (doc 05 §5.2's "capability gaps are typed, not silent", applied to
/// the build's subset). The message names the owning subsystem — a
/// script can branch on `[command-layer.unsupported-for-host]` and
/// the detail rows without parsing prose.
fn unserved(command: &Command) -> CommandError {
    let id = command_id_string(command);
    let owner = owner_of(&id);
    CommandError::new(
        commands::Stage::Control,
        commands::ErrorCode::UnsupportedForHost,
        format!("`{id}` is not served by this build's command-line host ({owner})"),
    )
    .with_detail("command", id)
    .with_detail("owner", owner)
}

/// The catalog id of one command, read from the layer's own wire form
/// (the adjacent `"cmd"` tag IS the stable id — the serde path is the
/// truth, no second table to drift; error-path only, so the Value
/// allocation is irrelevant).
fn command_id_string(command: &Command) -> String {
    match serde_json::to_value(command) {
        Ok(value) => value
            .get("cmd")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "(unnamed)".to_string()),
        Err(_) => "(unserializable)".to_string(),
    }
}
fn owner_of(id: &str) -> &'static str {
    if id.starts_with("playlist.") {
        "the playlist model is a Phase-4 card"
    } else if id.starts_with("audio.") {
        "audio device control joins with the real backends (ADR-005)"
    } else if id.starts_with("video.") {
        "video output control joins with the windowed renderer (2.D3)"
    } else if id.starts_with("subtitle.") {
        "the subtitle path is a Phase-5 card"
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
    } else if id.starts_with("system.plugin") {
        "the plugin registry is a Phase-6 card"
    } else if id.starts_with("playback.set-") || id.starts_with("playback.step-") {
        "rate and frame stepping are card 3.A1"
    } else if id.starts_with("media.") {
        "chapters and their navigation are the chapter card"
    } else {
        "not in the first vertical's served set (see `lumen commands`)" // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::catalog::types::{SeekTarget, SettingValue, SourceSpec, Timestamp};
    use commands::envelope::RequestId;

    fn request(command: Command) -> WireRequest {
        WireRequest {
            request_id: RequestId(1),
            command,
        }
    }

    /// The unserved surface answers the typed refusal (exit class 69)
    /// with the owning subsystem named — never a panic, never silence.
    #[test]
    fn unserved_commands_refuse_typed() {
        let mut host = CliHost::new();
        let response = lumen_dispatch(&mut host, &request(Command::PlaybackSetRate { rate: 1.0 }));
        match response.result {
            Err(error) => {
                assert_eq!(
                    error.code,
                    commands::ErrorCode::UnsupportedForHost,
                    "{error}"
                );
                assert_eq!(error.stage, commands::Stage::Control, "{error}");
                assert_eq!(
                    error.details.get("command").map(String::as_str),
                    Some("playback.set-rate")
                );
                assert!(error.details.contains_key("owner"), "{error}");
            }
            Ok(_) => unreachable!("set-rate must not be served"),
        }
    }

    /// A snapshot command with no session: the honest not-ready.
    #[test]
    fn snapshots_need_a_session() {
        let mut host = CliHost::new();
        let response = lumen_dispatch(&mut host, &request(Command::MediaGetInfo));
        match response.result {
            Err(error) => assert_eq!(error.code, commands::ErrorCode::NotReady, "{error}"),
            Ok(_) => unreachable!("info without a session must refuse"),
        }
    }

    /// The settings seam validates through the schema: a good write
    /// applies, a bad one refuses with the schema's own sentence.
    #[test]
    fn settings_seam_validates_through_the_schema() {
        let mut host = CliHost::new();
        let response = lumen_dispatch(
            &mut host,
            &request(Command::ConfigSetSetting {
                area: "engine".to_string(),
                key: "log-level".to_string(),
                value: SettingValue::String("debug".to_string()),
            }),
        );
        match response.result {
            Ok(CommandOutcome::Completed(CommandResult::SettingApplied {
                applied,
                restart_required,
            })) => {
                assert_eq!(applied, SettingValue::String("debug".to_string()));
                assert!(!restart_required);
            }
            other => unreachable!("valid setting write: {other:?}"),
        }
        let response = lumen_dispatch(
            &mut host,
            &request(Command::ConfigSetSetting {
                area: "engine".to_string(),
                key: "log-level".to_string(),
                value: SettingValue::String("bogus".to_string()),
            }),
        );
        match response.result {
            Err(error) => {
                assert_eq!(error.code, commands::ErrorCode::InvalidArgument, "{error}");
            }
            Ok(_) => unreachable!("bogus enum value must refuse"),
        }
    }

    /// A seek with no session: the honest not-ready (not a crash).
    #[test]
    fn seek_without_session_refuses() {
        let mut host = CliHost::new();
        let response = lumen_dispatch(
            &mut host,
            &request(Command::PlaybackSeek {
                target: SeekTarget::Absolute { time: Timestamp(0) },
            }),
        );
        match response.result {
            Err(error) => assert_eq!(error.code, commands::ErrorCode::NotReady, "{error}"),
            Ok(_) => unreachable!("seek without a session must refuse"),
        }
    }

    /// URL sources refuse through the open path's own typed answer.
    #[test]
    fn url_open_refuses_the_phase_named() {
        let mut host = CliHost::new();
        let response = lumen_dispatch(
            &mut host,
            &request(Command::SessionOpen {
                source: SourceSpec::Url {
                    url: "https://example.org/x.mp4".to_string(), // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
                },
                options: Default::default(),
            }),
        );
        match response.result {
            Err(error) => {
                assert_eq!(
                    error.code,
                    commands::ErrorCode::UnsupportedForHost,
                    "{error}"
                );
            }
            Ok(_) => unreachable!("URL open must refuse"),
        }
    }

    /// The served table and the help synopsis are the same set — the
    /// two cannot drift (the card's own cross-check).
    #[test]
    fn served_table_matches_the_help_synopsis() {
        let synopsis = crate::help::synopsis_catalog_ids();
        let served = SERVED.to_vec();
        assert_eq!(synopsis.len(), served.len(), "synopsis rows vs SERVED");
        for id in served {
            assert!(synopsis.contains(&id), "{id} missing from --help");
        }
        for id in synopsis {
            assert!(SERVED.contains(&id), "{id} in --help but not SERVED");
        }
    }
}
