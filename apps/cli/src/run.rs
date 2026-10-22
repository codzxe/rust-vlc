//! The runner (card 2.D2): the three host modes of doc 05 §5.3 —
//! argv one-shot, the interactive REPL (controller mode), and
//! `--control-stdio` NDJSON machine mode — all funneling through
//! [`crate::dispatch::lumen_dispatch`] (ARCH-5: every surface builds a
//! [`WireRequest`]).
//!
//! # Pacing (the honest v0 shape, recorded)
//!
//! The engine advances only when the host pumps it (2.C2's
//! synchronous single-writer model). The REPL pumps at **command
//! boundaries**: each dispatched class-A command is awaited to its
//! receipt and then [`CliHost::settle`]d (pumped until the engine
//! leaves the transient states, bounded) — so `open`/`play`/`seek`
//! visibly land before the next line is read. Between commands
//! nothing pumps; the explicit `advance [ticks]` host verb is the
//! run-to-end driver (deterministic, no wall-clock sleeps anywhere).
//! Continuous wall-time pumping joins with the session-thread card
//! (3.B3). `[PROPOSAL: the settle-after-command policy is this
//! host's; a `--wait` flag or event-driven pumping is a later card]`
//!
//! # Output shapes (deterministic, asserted by the AC suite)
//!
//! Human mode prints, per command: `ok: …` / `error: [stage.code] …`
//! (the [`crate::exit`] printer) plus one `event: …` line per newly
//! observed wire event, in causal order. `--json` prints the response
//! envelope as one NDJSON line instead. Machine mode
//! (`--control-stdio`) always prints response envelopes as NDJSON;
//! with `--json` the observed events are emitted as NDJSON lines
//! after each response (doc 05 §5.3: "`--json` emits every event as
//! one NDJSON line on stdout").
//!
//! # Graceful exit (M17)
//!
//! `quit`/EOF never abandons a live session: the REPL (and the
//! machine mode's EOF and protocol-error paths) runs a stop → close
//! teardown through the same dispatch entry as the typed verbs,
//! bounded by the engine's await-pump seam, and answers 0 — the
//! teardown's state trail prints like any command's (doc 10 §6
//! scenario 16: "exit during playback leaves no temp state"). See
//! [`graceful_exit`].
//!
//! # The argv-only bench verb
//!
//! `decode-bench <file>` (W4-R2) bypasses the catalog dispatch by
//! design: it is not a player command but the ffmpeg speed-parity
//! runner — it drives the open path + decoders directly
//! ([`crate::bench`]) and prints one machine-parseable BENCH line.

use std::io::{BufRead, Write};

use commands::{Command, CommandOutcome, CommandResult, PlaybackState, WireEvent, WireRequest};

use crate::bench;
use crate::dispatch::lumen_dispatch;
use crate::exit;
use crate::help;
use crate::host::CliHost;
use crate::parse::{self, CliCommand, ParseError, USAGE};

/// The process exit code `run` answers (the honest runner contract:
/// 0 success, the [`exit`] table otherwise).
///
/// # Errors
/// Never panics; every failure is a typed [`CommandError`] printed by
/// [`exit::print_error`] and mapped by [`exit::exit_code`].
pub fn run<R: BufRead, W: Write, E: Write>(
    argv: &[String],
    input: &mut R,
    out: &mut W,
    err: &mut E,
    interactive: bool,
) -> u8 {
    let invocation = match parse::parse_argv(argv) {
        Ok(invocation) => invocation,
        Err(failure) => {
            print_parse_error(err, &failure);
            return exit::exit_code(commands::ErrorCode::InvalidArgument);
        }
    };
    if invocation.version {
        let _ = writeln!(out, "{}", help::version_text());
        return 0;
    }
    if let Some(area) = invocation.help.as_ref() {
        return match area {
            Some(area) => match help::help_for_area(area) {
                Ok(text) => {
                    let _ = writeln!(out, "{text}");
                    0
                }
                Err(message) => {
                    let _ = writeln!(err, "error: {message}");
                    exit::exit_code(commands::ErrorCode::InvalidArgument)
                }
            },
            None => match help::help_text() {
                Ok(text) => {
                    let _ = writeln!(out, "{text}");
                    0
                }
                Err(message) => {
                    let _ = writeln!(err, "error: {message}");
                    exit::exit_code(commands::ErrorCode::Internal)
                }
            },
        };
    }
    if invocation.control_stdio {
        return machine_mode(input, out, err, invocation.json);
    }
    match invocation.command {
        None => {
            let _ = writeln!(err, "{USAGE}");
            exit::exit_code(commands::ErrorCode::InvalidArgument)
        }
        Some(CliCommand::Help) => match help::help_text() {
            Ok(text) => {
                let _ = writeln!(out, "{text}");
                0
            }
            Err(message) => {
                let _ = writeln!(err, "error: {message}");
                exit::exit_code(commands::ErrorCode::Internal)
            }
        },
        Some(CliCommand::Open {
            source,
            start_time,
            start_paused,
            no_video,
            no_audio,
        }) => repl_mode(
            &source,
            start_time,
            start_paused,
            no_video,
            no_audio,
            input,
            out,
            err,
            invocation.json,
            interactive,
        ),
        Some(CliCommand::DecodeBench { source }) => bench::run_bench(&source, out, err),
        Some(command) => one_shot(&command, invocation.json, out, err),
    }
}

/// One argv command, dispatched once, printed, exited (doc 05 §5.3's
/// one-shot use: `lumen info`, `lumen --json engine-info`, …).
fn one_shot<W: Write, E: Write>(command: &CliCommand, json: bool, out: &mut W, err: &mut E) -> u8 {
    let mut host = CliHost::new();
    let catalog = match to_catalog(command) {
        Ok(catalog) => catalog,
        Err(failure) => {
            print_parse_error(err, &failure);
            return exit::exit_code(commands::ErrorCode::InvalidArgument);
        }
    };
    let request_id = host.next_request();
    let request = WireRequest {
        request_id,
        command: catalog,
    };
    let response = lumen_dispatch(&mut host, &request);
    print_response(&response, json, out, err);
    match &response.result {
        Ok(_) => 0,
        Err(error) => exit::exit_code(error.code),
    }
}

/// The interactive controller mode (doc 05 §5.3: "default when a media
/// is given"): the argv `open` seeds the session, then stdin lines
/// drive it. EOF or `quit`/`q` leaves; the exit code is 0 unless the
/// OPEN itself failed (its typed error is the process answer).
/// Leaving with a live session runs the graceful-exit teardown (M17,
/// [`graceful_exit`]) — never an abandoned device mid-stream.
#[allow(clippy::too_many_arguments)]
fn repl_mode<R: BufRead, W: Write, E: Write>(
    source: &str,
    start_time: Option<i64>,
    start_paused: bool,
    no_video: bool,
    no_audio: bool,
    input: &mut R,
    out: &mut W,
    err: &mut E,
    json: bool,
    interactive: bool,
) -> u8 {
    let mut host = CliHost::new();
    let catalog = parse::open_command(source, start_time, start_paused, no_video, no_audio);
    // Events observed from the very first pump count: the open's own
    // state trail prints after its response line.
    let mut printed = 0;
    let request_id = host.next_request();
    let response = lumen_dispatch(
        &mut host,
        &WireRequest {
            request_id,
            command: catalog,
        },
    );
    print_response(&response, json, out, err);
    print_new_events(&host, printed, out);
    printed = host.events().len();
    if let Err(error) = &response.result {
        // The open failed: the typed error is the process answer (the
        // REPL would have nothing to drive).
        return exit::exit_code(error.code);
    }
    // The open answered `Accepted` from `Buffering`; let it settle
    // (auto-play — the `Primed`/`EofReached` signal) so the next
    // command meets a settled engine.
    host.settle();
    print_new_events(&host, printed, out);
    printed = host.events().len();
    let _ = interactive;
    loop {
        if interactive {
            let _ = write!(out, "lumen> ");
            let _ = out.flush();
        }
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        if words.is_empty() {
            continue;
        }
        let command = match parse::parse_command(&words) {
            Ok(command) => command,
            Err(failure) => {
                print_parse_error(err, &failure);
                continue;
            }
        };
        match command {
            CliCommand::Quit => break,
            CliCommand::Help => match help::help_text() {
                Ok(text) => {
                    let _ = writeln!(out, "{text}");
                }
                Err(message) => {
                    let _ = writeln!(err, "error: {message}");
                }
            },
            CliCommand::Advance { ticks } => {
                host.advance(ticks);
                print_new_events(&host, printed, out);
                printed = host.events().len();
                let _ = writeln!(
                    out,
                    "state: {:?} (position {})",
                    host.state(),
                    format_us(host.position_us())
                );
            }
            other => {
                let catalog = match to_catalog(&other) {
                    Ok(catalog) => catalog,
                    Err(failure) => {
                        print_parse_error(err, &failure);
                        continue;
                    }
                };
                let class_a = is_class_a(&catalog);
                let request_id = host.next_request();
                let response = lumen_dispatch(
                    &mut host,
                    &WireRequest {
                        request_id,
                        command: catalog,
                    },
                );
                if class_a {
                    host.settle();
                }
                print_new_events(&host, printed, out);
                printed = host.events().len();
                print_response(&response, json, out, err);
            }
        }
    }
    // M17: the graceful exit — `quit`/EOF must not abandon a live
    // session (doc 10 §6 scenario 16: "teardown within deadline;
    // device closed; exit code clean"). The teardown goes through
    // the same dispatch the typed verbs use; its trail prints here,
    // and the answer stays 0.
    graceful_exit(&mut host, err);
    print_new_events(&host, printed, out);
    0
}

/// The `--control-stdio` machine mode: NDJSON requests in, NDJSON
/// response envelopes out (the conformance-suite transport of doc 05
/// §5.3/§7.2). With `--json`, the observed events are emitted as
/// NDJSON lines after each response.
fn machine_mode<R: BufRead, W: Write, E: Write>(
    input: &mut R,
    out: &mut W,
    err: &mut E,
    json_events: bool,
) -> u8 {
    let mut host = CliHost::new();
    let mut printed = 0;
    let mut line = String::new();
    loop {
        line.clear();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => {
                // M17: EOF tears a live session down before the exit —
                // the machine twin of the REPL's graceful quit. The
                // typed refusals land on stderr; the trail rides the
                // NDJSON event stream when `--json` asked for it
                // (doc 05 §5.3: "`--json` emits every event").
                graceful_exit(&mut host, err);
                if json_events {
                    print_new_events_json(&host, printed, out);
                }
                return 0;
            }
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let request: WireRequest = match serde_json::from_str(trimmed) {
            Ok(request) => request,
            Err(failure) => {
                // W2-17 (M17 attack): a protocol error must not abandon
                // a live session either — the EOF twin tears down before
                // its exit, this path must too. The reply shape stays
                // the control-protocol card's (doc 05 §5.3 does not
                // define the malformed-request envelope; that card owns
                // it); only the teardown ordering is fixed here.
                graceful_exit(&mut host, err);
                if json_events {
                    print_new_events_json(&host, printed, out);
                }
                let _ = writeln!(err, "error: [control.invalid-argument] {failure}");
                return exit::exit_code(commands::ErrorCode::InvalidArgument);
            }
        };
        let response = lumen_dispatch(&mut host, &request);
        match serde_json::to_string(&response) {
            Ok(line) => {
                let _ = writeln!(out, "{line}");
            }
            Err(failure) => {
                // The serialization twin (W2-17): same
                // teardown-before-exit — a live session is never
                // dropped just because its envelope could not be
                // printed.
                graceful_exit(&mut host, err);
                if json_events {
                    print_new_events_json(&host, printed, out);
                }
                let _ = writeln!(err, "error: [control.internal] {failure}");
                return exit::exit_code(commands::ErrorCode::Internal);
            }
        }
        if json_events {
            print_new_events_json(&host, printed, out);
            printed = host.events().len();
        }
    }
}

/// The wire events observed since `printed`, one NDJSON line each
/// (the `--json` machine mode).
fn print_new_events_json<W: Write>(host: &CliHost, printed: usize, out: &mut W) {
    for event in &host.events()[printed..] {
        match serde_json::to_string(event) {
            Ok(line) => {
                let _ = writeln!(out, "{line}");
            }
            Err(_) => {
                // A wire event that cannot serialize is a layer bug;
                // surfaced, never swallowed.
                let _ = writeln!(out, "{{\"error\":\"event-serialization-failed\"}}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Graceful exit (M17 — doc 10 §6 scenario 16)
// ---------------------------------------------------------------------------

/// The bounded tail flush after a landed teardown: the close's
/// receipt and its `→ Idle` event co-drain in the pump that answers
/// the await, but the exit's observability must not lean on that
/// coupling — a fixed pipeline-only flush drains whatever the hub
/// still holds (in `Idle` every pump is a no-op, so the bound is
/// cheap, deterministic, and exact).
const TEARDOWN_TAIL_PUMPS: usize = 8;

/// The graceful-exit teardown (M17): `quit`/EOF with a live session
/// runs stop → close through [`lumen_dispatch`] — the same single
/// command entry the typed verbs use (ARCH-5; no host-internal
/// bypass — the runner owns no engine bypasses) — then a bounded
/// tail flush, and the process answers 0. Doc 10 §6 scenario 16:
/// "exit during playback — teardown within deadline; device closed;
/// exit code clean". The bare `break` this replaces left the audio
/// device abandoned mid-stream, dropped the session without the
/// close protocol, and printed no `Stopped`/`Idle` trail.
///
/// The steps:
///
/// 1. `playback.stop` from the active states (`Playing`/`Paused` —
///    the seek composition's own gating; the engine's transition
///    table rejects stop elsewhere and answers `AlreadySettled` in
///    the settled ones),
/// 2. `session.close` while a session survives (the close protocol:
///    destroy → `Idle`, the host's session-scoped halves dropped),
/// 3. the tail flush ([`TEARDOWN_TAIL_PUMPS`]) so the `→ Idle` trail
///    is drained and printable.
///
/// W2-17's re-drive: the stop → close drive runs **twice**, with one
/// bounded [`CliHost::settle`] window between the rounds. A refusal
/// from a transient state (`Buffering` — the mid-seek/rebuffer quit;
/// card 3.C1 owns the table row) can clear on its own a few pumps
/// later (the priming signal makes `Playing` — and stop — legal
/// again), so the settle window gives the engine that chance and the
/// drive repeats once. This composes only wire-legal rows (stop from
/// `Playing`/`Paused`, close from `Stopped`); the engine's table and
/// the typed refusal are untouched — if the transient state persists
/// through the settle bound, the refusal lines stand and the exit
/// still proceeds (never blocks). Worst case
/// `2 × (AWAIT_PUMPS + AWAIT_PUMPS) + TEARDOWN_TAIL_PUMPS` of
/// pipeline-only work — no wall clock anywhere.
///
/// Every await rides the engine's existing bound (the host's
/// `AWAIT_PUMPS` pipeline pumps — no wall clock anywhere), so the
/// exit cannot hang. A refused or expired step is printed on stderr
/// through the one error printer and never blocks the exit: the quit
/// itself succeeded, and the engine's typed refusal names the owning
/// card (the transient-state close from `Buffering` is 3.C1's — the
/// registered follow-up; the re-drive is the bounded host-side
/// composition until that card lands).
fn graceful_exit<E: Write>(host: &mut CliHost, err: &mut E) {
    if host.session_id().is_none() {
        // Nothing live (e.g. `quit` after a typed close): plain exit.
        return;
    }
    for round in 0..2 {
        if matches!(host.state(), PlaybackState::Playing | PlaybackState::Paused) {
            teardown_step(host, Command::PlaybackStop, err);
        }
        if host.session_id().is_none() {
            break;
        }
        teardown_step(host, Command::SessionClose, err);
        if host.session_id().is_none() {
            break;
        }
        if round == 0 {
            // The bounded settle window: pumps (pipeline-only) until
            // the engine leaves the transient states, so a
            // `Buffering` refusal that was about to clear on its own
            // gets its chance before the second drive. On a settled
            // engine this is a no-op (the predicate holds before the
            // first pump).
            host.settle();
        }
    }
    if host.session_id().is_none() {
        // The teardown landed: flush the trailing `→ Idle` events out
        // of the hub (each pump in `Idle` is a no-op — bounded and
        // cheap). On a refusal the engine still holds the session;
        // pumping it further would advance a session we were told to
        // leave, so the flush is success-only and the refusal line on
        // stderr is the report.
        host.pump_pipeline_until(|_| false, TEARDOWN_TAIL_PUMPS);
    }
}

/// One teardown step through the single dispatch entry: a typed
/// failure is reported on stderr (the one honest printer) and the
/// exit proceeds — the quit must never hang on a refused close.
fn teardown_step<E: Write>(host: &mut CliHost, command: Command, err: &mut E) {
    let request_id = host.next_request();
    let response = lumen_dispatch(
        host,
        &WireRequest {
            request_id,
            command,
        },
    );
    if let Err(error) = response.result {
        exit::print_error_to(&error, err);
    }
}

// ---------------------------------------------------------------------------
// Printing (deterministic shapes; the AC suite pins them)
// ---------------------------------------------------------------------------

/// Print one response envelope (human or NDJSON), errors through the
/// one honest printer.
fn print_response<W: Write, E: Write>(
    response: &commands::WireResponse,
    json: bool,
    out: &mut W,
    err: &mut E,
) {
    if json {
        match serde_json::to_string(response) {
            Ok(line) => {
                let _ = writeln!(out, "{line}");
                return;
            }
            Err(failure) => {
                let _ = writeln!(err, "error: [control.internal] {failure}");
                return;
            }
        }
    }
    match &response.result {
        Ok(outcome) => print_outcome(outcome, out),
        Err(error) => exit::print_error_to(error, err),
    }
}

/// The human form of one successful outcome.
fn print_outcome<W: Write>(outcome: &CommandOutcome<CommandResult>, out: &mut W) {
    match outcome {
        CommandOutcome::Accepted {
            session,
            generation,
        } => match session {
            Some(session) => {
                let _ = writeln!(
                    out,
                    "ok: accepted session={} gen={}",
                    session.0, generation.0
                );
            }
            None => {
                let _ = writeln!(out, "ok: accepted gen={}", generation.0);
            }
        },
        CommandOutcome::Completed(result) => print_result(result, out),
    }
}

/// The human form of one class-S payload (the served subset).
fn print_result<W: Write>(result: &CommandResult, out: &mut W) {
    match result {
        CommandResult::Unit => {
            let _ = writeln!(out, "ok");
        }
        CommandResult::MediaInfo { info } => {
            let _ = writeln!(out, "ok: media");
            let source = match &info.source {
                commands::catalog::types::SourceSpec::File { path } => path
                    .file_name()
                    .map(std::ffi::OsStr::to_string_lossy)
                    .unwrap_or_else(|| std::borrow::Cow::Borrowed("?")),
                commands::catalog::types::SourceSpec::Url { url } => {
                    std::borrow::Cow::Borrowed(url.as_str())
                }
            };
            let _ = writeln!(out, "  source: {source}");
            match &info.container {
                Some(container) => {
                    let _ = writeln!(out, "  container: {container}");
                }
                None => {
                    let _ = writeln!(out, "  container: (unknown)");
                }
            }
            match &info.duration {
                commands::catalog::types::Duration::Known { time } => {
                    let _ = writeln!(out, "  duration: {}", format_us(time.0));
                }
                commands::catalog::types::Duration::Unknown => {
                    let _ = writeln!(out, "  duration: (unknown)");
                }
                commands::catalog::types::Duration::Infinite => {
                    let _ = writeln!(out, "  duration: (infinite)");
                }
            }
            let _ = writeln!(
                out,
                "  seekable: {}",
                if info.seekable { "yes" } else { "no" }
            );
            let _ = writeln!(out, "  tracks: {}", info.tracks.len());
            for track in &info.tracks {
                print_track(track, out);
            }
        }
        CommandResult::CurrentTime {
            time,
            position,
            buffered,
        } => {
            let _ = writeln!(
                out,
                "ok: time {} ({:.1}%), {} buffered range(s)",
                format_us(time.0),
                position * 100.0,
                buffered.len()
            );
        }
        CommandResult::Duration { duration } => match duration {
            commands::catalog::types::Duration::Known { time } => {
                let _ = writeln!(out, "ok: duration {}", format_us(time.0));
            }
            commands::catalog::types::Duration::Unknown => {
                let _ = writeln!(out, "ok: duration (unknown)");
            }
            commands::catalog::types::Duration::Infinite => {
                let _ = writeln!(out, "ok: duration (infinite)");
            }
        },
        CommandResult::TrackList { tracks } => {
            let _ = writeln!(out, "ok: {} track(s)", tracks.len());
            for track in tracks {
                print_track(track, out);
            }
        }
        CommandResult::EngineInfo { info } => {
            let _ = writeln!(out, "ok: engine");
            let _ = writeln!(out, "  version: {}", info.engine_version);
            let _ = writeln!(out, "  host: {}", info.host);
            let _ = writeln!(out, "  features: {}", info.features.join(", "));
            let _ = writeln!(
                out,
                "  decoders: {}",
                info.decoders
                    .iter()
                    .map(|decoder| decoder.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        CommandResult::CommandListings { commands } => {
            let _ = writeln!(out, "ok: {} command(s)", commands.len());
            for listing in commands {
                let _ = writeln!(out, "  {} [{}]", listing.id, listing.class.as_str());
            }
        }
        CommandResult::SettingResult { value, schema_meta } => {
            let name = schema_meta.get("flag").cloned().unwrap_or_default();
            let _ = writeln!(out, "ok: {} ({name})", format_setting_value(value));
        }
        CommandResult::SettingApplied {
            applied,
            restart_required,
        } => {
            let _ = writeln!(
                out,
                "ok: set {}{}",
                format_setting_value(applied),
                if *restart_required {
                    " (restart required)"
                } else {
                    ""
                }
            );
        }
        other => {
            // A result variant outside the served subset cannot be
            // produced by this host's dispatch — printed generically
            // rather than panicked on.
            let _ = writeln!(out, "ok: {other:?}");
        }
    }
}

/// One track row (the `info`/`tracks` tables).
fn print_track<W: Write>(track: &commands::catalog::types::TrackInfo, out: &mut W) {
    let kind = match track.kind {
        commands::catalog::types::TrackKind::Video => "video",
        commands::catalog::types::TrackKind::Audio => "audio",
        commands::catalog::types::TrackKind::Subtitle => "subtitle",
        commands::catalog::types::TrackKind::Data => "data",
    };
    let codec = track
        .codec
        .description
        .clone()
        .or_else(|| track.codec.fourcc.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut line = format!("  #{} {} {}", track.id.0, kind, codec);
    if let Some(video) = &track.video {
        line.push_str(&format!(" {}x{}", video.width, video.height));
        if let Some(rate) = video.frame_rate {
            line.push_str(&format!(" @{rate:.3}"));
        }
    }
    if let Some(audio) = &track.audio {
        line.push_str(&format!(" {}ch", audio.channels));
        if let Some(rate) = audio.sample_rate {
            line.push_str(&format!(" {rate}Hz"));
        }
    }
    if track.is_selected {
        line.push_str(" (selected)");
    }
    let _ = writeln!(out, "{line}");
}

/// The wire events observed since `printed`, one line each (human).
fn print_new_events<W: Write>(host: &CliHost, printed: usize, out: &mut W) {
    for event in &host.events()[printed..] {
        print_event(event, out);
    }
}

/// One wire event's human line (the stable event vocabulary of doc 05
/// §3.2, this host's subset).
fn print_event<W: Write>(event: &WireEvent, out: &mut W) {
    use commands::EventKind;
    let what = match &event.kind {
        EventKind::SessionStateChanged { from, to } => {
            format!("state-changed {from:?} → {to:?}")
        }
        EventKind::SessionBuffering { progress } => format!("buffering {progress:.2}"),
        EventKind::TracksListReady { tracks } => format!("tracks {} ready", tracks.len()),
        EventKind::PlaybackEndOfMedia => "end-of-media".to_string(),
        EventKind::PlaybackError {
            stage,
            code,
            message,
            ..
        } => format!("error [{stage}.{code}] {message}"),
        EventKind::TimingTimeTick { time, position } => {
            format!("time-tick {} ({:.1}%)", format_us(time.0), position * 100.0)
        }
        other => format!("{other:?}"),
    };
    let _ = writeln!(out, "event: {what}");
}

/// One parse failure's honest line(s).
fn print_parse_error<E: Write>(err: &mut E, failure: &ParseError) {
    let _ = writeln!(err, "error: {}", failure.message);
    let _ = writeln!(err, "  {}", failure.hint);
}

/// Microseconds → a fixed-decimal seconds string (`2.000000s`).
fn format_us(us: i64) -> String {
    format!("{:.6}s", us as f64 / 1.0e6)
}

/// A setting value's human form (the wire type has no `Display`).
fn format_setting_value(value: &commands::catalog::types::SettingValue) -> String {
    use commands::catalog::types::SettingValue;
    match value {
        SettingValue::Bool(on) => if *on { "on" } else { "off" }.to_string(),
        SettingValue::Number(number) => format_setting_number(*number),
        SettingValue::String(text) => text.clone(),
        SettingValue::List(items) => format!("[{}]", items.join(", ")),
    }
}

/// Integral numbers print bare, fractional with decimals.
fn format_setting_number(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 9.0e15 {
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

/// Whether the catalog command is class A (its evidence arrives as
/// events — the host settles after it).
fn is_class_a(command: &Command) -> bool {
    matches!(
        command,
        Command::SessionOpen { .. }
            | Command::SessionClose
            | Command::PlaybackPlay
            | Command::PlaybackPause
            | Command::PlaybackTogglePlayPause
            | Command::PlaybackStop
            | Command::PlaybackSeek { .. }
    )
}

/// A parsed CLI command → its catalog payload (the argv/REPL grammar's
/// whole vocabulary; `advance`/`help`/`quit` are host verbs handled by
/// the caller and never reach here).
///
/// # Errors
/// [`ParseError`] for grammar the catalog payload cannot express.
fn to_catalog(command: &CliCommand) -> Result<Command, ParseError> {
    match command {
        CliCommand::Open {
            source,
            start_time,
            start_paused,
            no_video,
            no_audio,
        } => Ok(parse::open_command(
            source,
            *start_time,
            *start_paused,
            *no_video,
            *no_audio,
        )),
        CliCommand::Play => Ok(Command::PlaybackPlay),
        CliCommand::Pause => Ok(Command::PlaybackPause),
        CliCommand::Toggle => Ok(Command::PlaybackTogglePlayPause),
        CliCommand::Stop => Ok(Command::PlaybackStop),
        CliCommand::Close => Ok(Command::SessionClose),
        CliCommand::Seek(target) => Ok(Command::PlaybackSeek {
            target: target.to_target(),
        }),
        CliCommand::Info => Ok(Command::MediaGetInfo),
        CliCommand::Time => Ok(Command::MediaGetCurrentTime),
        CliCommand::Duration => Ok(Command::MediaGetDuration),
        CliCommand::Tracks => Ok(Command::TracksList),
        CliCommand::EngineInfo => Ok(Command::SystemGetEngineInfo),
        CliCommand::Commands => Ok(Command::SystemListCommands),
        CliCommand::SettingGet { key } => {
            let (area, rest) = split_setting_key(key)?;
            Ok(Command::ConfigGetSetting { area, key: rest })
        }
        CliCommand::SettingSet { key, value } => {
            let (area, rest) = split_setting_key(key)?;
            Ok(Command::ConfigSetSetting {
                area,
                key: rest,
                value: parse_setting_value(value),
            })
        }
        CliCommand::DecodeBench { .. } => Err(ParseError::new(
            "`decode-bench` is an argv-only host verb (lumen decode-bench <file>)",
            "usage: lumen decode-bench <file>",
        )),
        CliCommand::Advance { .. } | CliCommand::Help | CliCommand::Quit => Err(ParseError::new(
            "host verbs (`advance`, `help`, `quit`) are REPL-only",
            USAGE,
        )),
    }
}

/// Split `area.key` at the first dot (the settings-schema's own key
/// grammar; the catalog carries the two halves separately).
///
/// # Errors
/// [`ParseError`] when the key has no area.
fn split_setting_key(key: &str) -> Result<(String, String), ParseError> {
    match key.split_once('.') {
        Some((area, rest)) => Ok((area.to_string(), rest.to_string())),
        None => Err(ParseError::new(
            format!("setting keys are `<area>.<key>` — `{key}` has no area"),
            "e.g. setting get playback.autoplay-next",
        )),
    }
}

/// A setting value's CLI scalar grammar: bare `on`/`off`/`true`/
/// `false` → bool, a number → Number, `[a, b]` → list, else string.
fn parse_setting_value(value: &str) -> commands::catalog::types::SettingValue {
    use commands::catalog::types::SettingValue;
    match value {
        "on" | "true" => SettingValue::Bool(true),
        "off" | "false" => SettingValue::Bool(false),
        _ => {
            if let Ok(number) = value.parse::<f64>()
                && number.is_finite()
            {
                return SettingValue::Number(number);
            }
            if value.starts_with('[') && value.ends_with(']') {
                let items = value[1..value.len() - 1]
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_string)
                    .collect();
                return SettingValue::List(items);
            }
            SettingValue::String(value.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_string).collect()
    }

    /// `run` with in-memory I/O: the test twin of `main`.
    fn run_lines(argv: &str, stdin: &str) -> (String, String, u8) {
        let mut input = stdin.as_bytes();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run(&words(argv), &mut input, &mut out, &mut err, false);
        (
            String::from_utf8_lossy(&out).into_owned(),
            String::from_utf8_lossy(&err).into_owned(),
            code,
        )
    }

    #[test]
    fn version_and_help_exit_zero() {
        let (out, err, code) = run_lines("--version", "");
        assert_eq!(code, 0);
        assert!(out.starts_with("lumen 0."), "{out}");
        assert!(err.is_empty(), "{err}");
        let (out, _err, code) = run_lines("--help", "");
        assert_eq!(code, 0);
        assert!(out.contains("Commands (the playing subset"), "{out}");
        assert!(out.contains("session.open"), "{out}");
        assert!(out.contains("Areas:"), "{out}");
        let (out, _err, code) = run_lines("--help engine", "");
        assert_eq!(code, 0);
        assert!(out.contains("--engine-log-level"), "{out}");
    }

    #[test]
    fn unknown_area_is_a_usage_error() {
        let (_out, err, code) = run_lines("--help no-such-area", "");
        assert_eq!(code, 64);
        assert!(err.contains("unknown settings area"), "{err}");
    }

    #[test]
    fn bare_invocation_is_usage() {
        let (_out, err, code) = run_lines("", "");
        assert_eq!(code, 64);
        assert!(err.contains("usage:"), "{err}");
    }

    #[test]
    fn unknown_verb_is_a_usage_error() {
        let (_out, err, code) = run_lines("bogus", "");
        assert_eq!(code, 64);
        assert!(err.contains("unknown command `bogus`"), "{err}");
    }

    /// The one-shot class-S commands answer without a session.
    #[test]
    fn one_shot_engine_info_and_commands() {
        let (out, err, code) = run_lines("engine-info", "");
        assert_eq!(code, 0);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("ok: engine"), "{out}");
        assert!(out.contains("audio.null-device"), "{out}");
        assert!(out.contains("video.software-compositor"), "{out}");
        let (out, _err, code) = run_lines("commands", "");
        assert_eq!(code, 0);
        assert!(out.contains("ok: 61 command(s)"), "{out}");
        assert!(out.contains("session.open [a]"), "{out}");
        assert!(out.contains("config.set-setting [s]"), "{out}");
    }

    /// One-shot play without a session: the honest not-ready (exit 69).
    #[test]
    fn one_shot_play_without_session_is_not_ready() {
        let (_out, err, code) = run_lines("play", "");
        assert_eq!(code, 69);
        assert!(err.contains("[session.not-ready]"), "{err}");
    }

    /// `--json` one-shot: the response envelope as one NDJSON line.
    #[test]
    fn json_one_shot_prints_the_envelope() {
        let (out, _err, code) = run_lines("--json engine-info", "");
        assert_eq!(code, 0);
        assert!(out.contains("\"requestId\":"), "{out}");
        assert!(out.contains("\"completed\":"), "{out}");
        assert!(out.contains("\"kind\":\"engine-info\""), "{out}");
    }

    /// The settings seam through the runner: get, set, invalid set.
    #[test]
    fn settings_get_set_through_the_grammar() {
        let (out, _err, code) = run_lines("setting get privacy.telemetry", "");
        assert_eq!(code, 0);
        assert!(out.contains("ok: "), "{out}");
        let (_out, err, code) = run_lines("setting set engine.log-level bogus", "");
        assert_eq!(code, 64);
        assert!(err.contains("[config.invalid-argument]"), "{err}");
        let (_out, err, code) = run_lines("setting get no.such.key", "");
        assert_eq!(code, 64);
        assert!(err.contains("unknown setting key"), "{err}");
    }

    /// The machine mode: NDJSON in, NDJSON out (doc 05 §5.3).
    #[test]
    fn control_stdio_answers_ndjson() {
        let stdin = "{\"requestId\": 7, \"cmd\": \"system.get-engine-info\"}\n\
                     {\"requestId\": 8, \"cmd\": \"playback.set-rate\", \"args\": {\"rate\": 2.0}}\n";
        let (out, err, code) = run_lines("--control-stdio", stdin);
        assert_eq!(code, 0);
        assert!(err.is_empty(), "{err}");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "{out}");
        assert!(lines[0].contains("\"requestId\":7"), "{out}");
        assert!(lines[0].contains("\"completed\""), "{out}");
        assert!(lines[1].contains("\"requestId\":8"), "{out}");
        assert!(lines[1].contains("unsupported-for-host"), "{out}");
    }

    /// A malformed machine request: typed error, usage exit.
    #[test]
    fn control_stdio_malformed_request_is_typed() {
        let (_out, err, code) = run_lines("--control-stdio", "not json\n");
        assert_eq!(code, 64);
        assert!(err.contains("[control.invalid-argument]"), "{err}");
    }

    /// The µs formatter is fixed-decimal (assertable transcripts).
    #[test]
    fn microsecond_formatting_is_fixed_decimal() {
        assert_eq!(format_us(2_000_000), "2.000000s");
        assert_eq!(format_us(1_500_000), "1.500000s");
        assert_eq!(format_us(0), "0.000000s");
    }

    /// The setting scalar grammar covers the wire vocabulary.
    #[test]
    fn setting_value_grammar() {
        use commands::catalog::types::SettingValue;
        assert_eq!(parse_setting_value("on"), SettingValue::Bool(true));
        assert_eq!(parse_setting_value("false"), SettingValue::Bool(false));
        assert_eq!(parse_setting_value("250"), SettingValue::Number(250.0));
        assert_eq!(
            parse_setting_value("[en, de]"),
            SettingValue::List(vec!["en".to_string(), "de".to_string()])
        );
        assert_eq!(
            parse_setting_value("auto"),
            SettingValue::String("auto".to_string())
        );
    }

    /// Host verbs are REPL-only in one-shot mode (typed usage error).
    #[test]
    fn host_verbs_are_repl_only() {
        let (_out, err, code) = run_lines("advance", "");
        assert_eq!(code, 64);
        assert!(err.contains("REPL-only"), "{err}");
    }
}
