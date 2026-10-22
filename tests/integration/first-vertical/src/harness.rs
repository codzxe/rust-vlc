//! The session harness (2.E1): one scenario's command script over the
//! command layer, recorded as a doc 10 §6.4 transcript, plus the doc
//! 10 §7.1 label axes every artifact carries.
//!
//! # Through the command layer, only
//!
//! The harness holds a [`CliHost`] (the 2.D2 headless host — the
//! composition root that wires the REAL pipeline: input → probe →
//! AVI demux → queues → MJPEG/PCM decode → audio engine → null
//! device → video scheduler → software compositor) and speaks to it
//! EXCLUSIVELY through [`lumen_dispatch`] — the CLI's shape of doc 05
//! §5.1's single dispatch entry. No engine API, no pipeline type,
//! no sink or device handle is ever touched from here: the responses,
//! the wire events, the state trail and the headless accounting ARE
//! the observation surface (the card's own rule, and ARCH-5).
//!
//! # Determinism and the two label axes
//!
//! Every observable is a pure function of the pump count (the 2.C2/2.D2
//! poll model: media time moves only via the host's `advance`), so a
//! transcript is byte-stable for a given code version — which is what
//! lets the committed evidence be re-verified by a fresh run. The
//! environment block records doc 10 §7.1's axes: `hardware_class` is
//! **always `mock` here** (null audio device + software renderer —
//! asserting anything else would be fabrication: no audio hardware or
//! GPU exists in this sandbox, and CI VMs are mock by definition),
//! while `environment_kind` and `os` name the machine that produced
//! the artifact (`local`/this OS for the committed evidence;
//! `ci-vm`/<runner OS> under a `CI` runner) — those two are per-run
//! values, normalized by the currency check (see [`normalize_run_specific`]).

use std::path::{Path, PathBuf};

use commands::catalog::command::CommandResult;
use commands::catalog::types::{Duration, MediaInfo, TrackInfo};
use commands::envelope::{CommandOutcome, RequestId};
use commands::{
    Command, CommandError, EventKind, PlaybackState, WireEvent, WireRequest, WireResponse,
};
use lumen_cli::dispatch::lumen_dispatch;
use lumen_cli::host::{CliHost, HeadlessAccounting, RUN_PUMPS};

use crate::digest::sha256_hex;
use crate::json::Json;

/// The transcript schema tag (doc 10 §6.4's `transcripts/<id>.json`).
pub const TRANSCRIPT_SCHEMA: &str = "transcript/1";

/// A frozen monotonic source (card 3.B4's determinism seam): every
/// emission window reads time 0 forever, so each buffering episode
/// permits exactly one in-episode progress emission and the damage
/// coalescer only ever flushes at its terminal points — pure
/// functions of the pump count, like every other observable here.
struct FrozenThrottle(std::sync::atomic::AtomicU64);

impl lumen_cli::MonotonicSource for FrozenThrottle {
    fn now_ns(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The doc 10 §7.1 label axes for one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    /// `local` here, `ci-vm` under a CI runner (doc 10 §7.1's third
    /// kind, `reference-hardware`, cannot occur for this suite — it
    /// would require real devices, which the suite never touches).
    pub environment_kind: &'static str,
    /// The OS that produced the artifact (`std::env::consts::OS`).
    pub os: &'static str,
    /// Always `mock` for this suite (null audio + software render).
    pub hardware_class: &'static str,
    /// The devices the pipeline actually drove.
    pub devices: Vec<&'static str>,
}

impl Labels {
    /// Detect the run's labels. `hardware_class` is pinned to `mock`:
    /// the suite's audio sink is the null adapter and its renderer is
    /// the software compositor on EVERY machine it can currently run
    /// on — a `real` label would be a fabricated claim (doc 10 §7.1:
    /// CI-produced evidence is always mock; the reference-hardware
    /// tier does not exist yet).
    #[must_use]
    pub fn detect() -> Labels {
        let ci = std::env::var_os("CI").is_some();
        Labels {
            environment_kind: if ci { "ci-vm" } else { "local" },
            os: std::env::consts::OS,
            hardware_class: "mock",
            devices: vec!["null-audio", "software-render"],
        }
    }

    /// The labels as the transcript's JSON block.
    fn to_json(&self) -> Json {
        Json::obj(vec![
            ("environment_kind", Json::str(self.environment_kind)),
            ("os", Json::str(self.os)),
            ("hardware_class", Json::str(self.hardware_class)),
            (
                "devices",
                Json::Arr(self.devices.iter().map(|d| Json::str(*d)).collect()),
            ),
        ])
    }
}

/// One recorded claim: what was asserted, whether it held, and the
/// observed values when it did not. Scenarios answer checks instead of
/// panicking (the lib is non-test code under the workspace lint bans);
/// the test files turn them into assertions, and the ledger records
/// them as the scenario's claim table.
#[derive(Debug, Clone)]
pub struct Check {
    /// The claim (a stable sentence).
    pub claim: String,
    /// Whether it held.
    pub passed: bool,
    /// Observed vs expected, when it did not hold ("" otherwise).
    pub detail: String,
}

impl Check {
    /// A boolean claim.
    pub fn is(claim: &str, passed: bool, detail: &str) -> Check {
        Check {
            claim: claim.to_string(),
            passed,
            detail: if passed {
                String::new()
            } else {
                detail.to_string()
            },
        }
    }

    /// An integer claim (observed vs expected).
    pub fn int(claim: &str, got: i64, want: i64) -> Check {
        Check::is(
            claim,
            got == want,
            &format!("observed {got}, expected {want}"),
        )
    }

    /// A string claim (observed vs expected).
    pub fn same(claim: &str, got: &str, want: &str) -> Check {
        Check::is(
            claim,
            got == want,
            &format!("observed {got:?}, expected {want:?}"),
        )
    }
}

/// One scenario's finished run: the transcript (the artifact), the
/// checks (the ACs, machine-evaluated) and — when a video limb
/// existed — the raw final-frame PNG bytes (NOT part of the
/// transcript artifact, which records only their digest/extent; the
/// bytes feed the committed `final-frame.png` evidence and its
/// byte-identity currency check).
pub struct ScenarioRun {
    /// The doc 10 §6.4 scenario id (stable).
    pub id: &'static str,
    /// The full transcript value (labels included).
    pub transcript: Json,
    /// The scenario's claims.
    pub checks: Vec<Check>,
    /// The final-frame PNG bytes, when a video limb existed.
    pub png: Option<Vec<u8>>,
}

impl ScenarioRun {
    /// Whether every check held (the test files assert this; the
    /// ledger records it as the verdict).
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
    }

    /// The transcript's exact bytes (the artifact file's content).
    pub fn transcript_bytes(&self) -> String {
        self.transcript.render()
    }
}

/// The pump bound for run-to-end (the CLI's own `RUN_PUMPS` — the
/// same constant the `advance` verb uses, so the suite and the host
/// cannot disagree about "enough").
pub fn run_pumps() -> u32 {
    RUN_PUMPS as u32
}

/// The workspace root (compile-time-known: this crate lives at
/// `tests/integration/first-vertical`, three levels down).
#[must_use]
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Resolve a fixture under `tests/fixtures/` (existence-checked; the
/// error carries the resolved path, testkit's own shape).
///
/// # Errors
/// [`String`] when the fixture does not exist.
pub fn fixture_path(relative: &str) -> Result<PathBuf, String> {
    let path = workspace_root().join("tests/fixtures").join(relative);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("fixture missing: {}", path.display()))
    }
}

/// One scenario's driver: a `CliHost` plus the recording state.
pub struct SessionHarness {
    host: CliHost,
    next_id: u64,
    steps: Vec<Json>,
    labels: Labels,
    fixture: &'static str,
}

impl SessionHarness {
    /// A harness for a scenario over `fixture` (the repo-relative
    /// fixture name — recorded in the transcript; machine-absolute
    /// paths are never recorded, V8).
    #[must_use]
    pub fn new(scenario_fixture: &'static str) -> SessionHarness {
        let mut host = CliHost::new();
        // Card 3.B4: freeze the event-emission windows (doc 05 §3.4)
        // on the same virtual-time doctrine as the media clock — a
        // frozen monotonic source keeps the in-episode
        // `session.buffering` progress and the non-fatal damage
        // coalescing byte-deterministic in the transcripts (the
        // time-tick stream is position-judged and needs no help).
        host.set_event_throttle(std::sync::Arc::new(FrozenThrottle(
            std::sync::atomic::AtomicU64::new(0),
        )));
        SessionHarness {
            host,
            next_id: 1,
            steps: Vec::new(),
            labels: Labels::detect(),
            fixture: scenario_fixture,
        }
    }

    /// Dispatch one catalog command through the ONE entry (the
    /// scenario's every engine interaction goes through here).
    pub fn dispatch(&mut self, name: &str, command: Command) -> WireResponse {
        let request = WireRequest {
            request_id: RequestId(self.next_id),
            command,
        };
        self.next_id += 1;
        let response = lumen_dispatch(&mut self.host, &request);
        self.steps.push(step_json(name, &response));
        response
    }

    /// Settle to a non-transient state (the host's own bounded
    /// pipeline-only pacing — no media time moves).
    pub fn settle(&mut self) {
        self.host.settle();
    }

    /// Advance media time (the deterministic session-thread stand-in).
    pub fn advance(&mut self, ticks: u32) {
        self.host.advance(ticks);
    }

    /// The live wire state.
    pub fn state(&self) -> PlaybackState {
        self.host.state()
    }

    /// The live position (µs).
    pub fn position_us(&self) -> i64 {
        self.host.position_us()
    }

    /// The live session id.
    pub fn session_id(&self) -> Option<u64> {
        self.host.session_id().map(|id| id.0)
    }

    /// The headless accounting (when a session wired a consumer).
    pub fn accounting(&self) -> Option<HeadlessAccounting> {
        self.host.headless_accounting()
    }

    /// The full state trail (from the `state-changed` events).
    pub fn state_history(&self) -> Vec<(PlaybackState, PlaybackState)> {
        self.host.state_history().to_vec()
    }

    /// The final surface as a PNG screenshot (when a video limb
    /// exists) — `None` otherwise; `Some(Err(_))` is a present-path
    /// failure the scenario records as a failed check.
    pub fn screenshot_png(&self) -> Option<Result<Vec<u8>, String>> {
        match self.host.screenshot_png() {
            Some(Ok(png)) => Some(Ok(png)),
            Some(Err(error)) => Some(Err(error.to_string())),
            None => None,
        }
    }

    /// Assemble the transcript: the command script with outcomes, the
    /// full wire event log, and the final observables.
    #[must_use]
    pub fn finish(self, id: &'static str) -> ScenarioRun {
        // The final pump state is whatever the scenario left; the
        // observables are read through the host's wire surfaces only.
        let state = self.host.state();
        let position = self.host.position_us();
        let session = self.host.session_id().map(|sid| sid.0);
        let accounting = self
            .host
            .headless_accounting()
            .as_ref()
            .map(accounting_json);
        let png = self.screenshot_png();
        let events = self.host.events().iter().map(event_json).collect();
        let mut observables = vec![
            ("final_state".to_string(), state_json(state)),
            ("position_us".to_string(), Json::Int(position)),
            (
                "session".to_string(),
                session.map_or(Json::Null, Json::UInt),
            ),
        ];
        if let Some(accounting) = accounting {
            observables.push(("accounting".to_string(), accounting));
        }
        let png_bytes = match png {
            Some(Ok(bytes)) => {
                observables.push(("final_png".to_string(), png_json(&bytes)));
                Some(bytes)
            }
            Some(Err(message)) => {
                observables.push(("final_png_error".to_string(), Json::Str(message)));
                None
            }
            None => None,
        };
        let transcript = Json::obj(vec![
            ("schema", Json::str(TRANSCRIPT_SCHEMA)),
            ("scenario", Json::str(id)),
            ("fixture", Json::str(self.fixture)),
            ("labels", self.labels.to_json()),
            ("commands", Json::Arr(self.steps.clone())),
            ("events", Json::Arr(events)),
            ("observables", Json::Obj(observables)),
        ]);
        ScenarioRun {
            id,
            transcript,
            checks: Vec::new(),
            png: png_bytes,
        }
    }

    /// The labels this run detected (for the ledger assembly).
    pub fn labels(&self) -> &Labels {
        &self.labels
    }
}

/// One dispatched step's transcript record: the catalog id, the echoed
/// correlation fields, and the outcome's shape (accepted / completed /
/// the typed error). Result payloads are recorded for the snapshot
/// commands this suite uses (see [`result_json`]) — never raw `Debug`
/// forms, which could embed machine paths.
fn step_json(name: &str, response: &WireResponse) -> Json {
    let result = match &response.result {
        Ok(CommandOutcome::Accepted {
            session,
            generation,
        }) => Json::obj(vec![
            ("form", Json::str("accepted")),
            (
                "session",
                session.map(|s| Json::UInt(s.0)).unwrap_or(Json::Null),
            ),
            ("gen", Json::UInt(generation.0)),
        ]),
        Ok(CommandOutcome::Completed(result)) => Json::obj(vec![
            ("form", Json::str("completed")),
            ("payload", result_json(result)),
        ]),
        Err(error) => Json::obj(vec![
            ("form", Json::str("error")),
            ("error", error_json(error)),
        ]),
    };
    Json::obj(vec![
        ("cmd", Json::str(name)),
        ("request_id", Json::UInt(response.request_id.0)),
        (
            "session",
            response
                .session
                .map(|s| Json::UInt(s.0))
                .unwrap_or(Json::Null),
        ),
        ("gen", Json::UInt(response.generation.0)),
        ("result", result),
    ])
}

/// A typed command error's transcript record (the stage.code wire
/// identity + the redacted message — the layer's own constructor has
/// already applied V8 redaction to user paths).
fn error_json(error: &CommandError) -> Json {
    Json::obj(vec![
        ("stage", Json::str(error.stage.as_str())),
        ("code", Json::str(error.code.as_str())),
        ("message", Json::str(error.to_string())),
    ])
}

/// The snapshot payloads this suite dispatches, as transcript records.
/// Unrecorded variants answer a placeholder — an honest "not recorded"
/// rather than a `Debug` dump (payloads can carry machine paths).
fn result_json(result: &CommandResult) -> Json {
    match result {
        CommandResult::Unit => Json::str("unit"),
        CommandResult::MediaInfo { info } => media_info_json(info),
        CommandResult::CurrentTime {
            time,
            position,
            buffered,
        } => Json::obj(vec![
            ("time_us", Json::Int(time.0)),
            ("position", Json::str(format!("{:.3}", position))),
            ("buffered_ranges", Json::UInt(buffered.len() as u64)),
        ]),
        CommandResult::Duration { duration } => match duration {
            Duration::Known { time } => Json::obj(vec![
                ("known", Json::Bool(true)),
                ("duration_us", Json::Int(time.0)),
            ]),
            Duration::Unknown => Json::obj(vec![("known", Json::Bool(false))]),
            Duration::Infinite => Json::str("infinite"),
        },
        CommandResult::TrackList { tracks } => Json::obj(vec![
            ("count", Json::UInt(tracks.len() as u64)),
            ("tracks", Json::Arr(tracks.iter().map(track_json).collect())),
        ]),
        _ => Json::str("(result variant not recorded by this suite)"),
    }
}

/// A media-info snapshot's transcript record (machine paths excluded:
/// `source` is deliberately not recorded — the fixture name is already
/// the transcript's `fixture` field).
fn media_info_json(info: &MediaInfo) -> Json {
    let duration = match &info.duration {
        Duration::Known { time } => Json::Int(time.0),
        Duration::Unknown => Json::str("unknown"),
        Duration::Infinite => Json::str("infinite"),
    };
    Json::obj(vec![
        (
            "container",
            info.container.clone().map(Json::Str).unwrap_or(Json::Null),
        ),
        ("duration_us", duration),
        ("seekable", Json::Bool(info.seekable)),
        ("is_live", Json::Bool(info.is_live)),
        ("track_count", Json::UInt(info.tracks.len() as u64)),
        (
            "tracks",
            Json::Arr(info.tracks.iter().map(track_json).collect()),
        ),
    ])
}

/// One track's transcript record.
fn track_json(track: &TrackInfo) -> Json {
    let mut fields = vec![
        ("id".to_string(), Json::UInt(u64::from(track.id.0))),
        ("kind".to_string(), Json::str(kind_str(track.kind))),
        (
            "codec".to_string(),
            track
                .codec
                .description
                .clone()
                .map(Json::Str)
                .unwrap_or(Json::Null),
        ),
        ("selected".to_string(), Json::Bool(track.is_selected)),
    ];
    if let Some(video) = &track.video {
        fields.push(("width".to_string(), Json::UInt(u64::from(video.width))));
        fields.push(("height".to_string(), Json::UInt(u64::from(video.height))));
        if let Some(rate) = video.frame_rate {
            fields.push(("frame_rate".to_string(), Json::str(format!("{rate:.3}"))));
        }
    }
    if let Some(audio) = &track.audio {
        fields.push((
            "channels".to_string(),
            Json::UInt(u64::from(audio.channels)),
        ));
        if let Some(rate) = audio.sample_rate {
            fields.push(("sample_rate".to_string(), Json::UInt(u64::from(rate))));
        }
    }
    Json::Obj(fields)
}

/// The wire state's stable name (the enum's kebab-case wire form).
fn state_json(state: PlaybackState) -> Json {
    Json::str(format!("{state:?}"))
}

/// One wire event's transcript record: the envelope fields plus the
/// payload — structured for the kinds this vertical produces, kind
/// name + `Debug` payload for the rest (events never carry user
/// paths: error messages arrive through the redaction-safe layer
/// constructor).
///
/// # Seq gaps are engine-private events, never loss
///
/// The engine mints `seq` at emission for EVERY event, including the
/// engine-private kinds that never reach the wire (doc 05 §4.2:
/// `FirstFrame` is "the internal trigger of the playing transition,
/// no separate v1 wire event"; `DamagedData` is bounded-recovery
/// observability). The private kinds are filtered here, so their seq
/// numbers are absent from every transcript — the deterministic
/// `…4, 6, 7…` open pattern is exactly one private `FirstFrame`
/// consuming seq 5 during priming (and the truncated-midplay `…4,
/// 7…` adds the private `DamagedData` of the recovery). This is the
/// doc 05 §4.3.1 rule — "gaps indicate filtered events … never
/// reordering" — not a dropped or double-minted counter (audit-1
/// finding, root-caused by card 3.B4).
fn event_json(event: &WireEvent) -> Json {
    let mut fields = vec![
        ("seq".to_string(), Json::UInt(event.seq)),
        (
            "session".to_string(),
            event.session.map(|s| Json::UInt(s.0)).unwrap_or(Json::Null),
        ),
        (
            "gen".to_string(),
            event
                .generation
                .map(|g| Json::UInt(g.0))
                .unwrap_or(Json::Null),
        ),
    ];
    match &event.kind {
        EventKind::SessionStateChanged { from, to } => {
            fields.push(("kind".to_string(), Json::str("session.state-changed")));
            fields.push(("from".to_string(), state_json(*from)));
            fields.push(("to".to_string(), state_json(*to)));
        }
        EventKind::SessionBuffering { progress } => {
            fields.push(("kind".to_string(), Json::str("session.buffering")));
            // The transcript normalizes mid-episode progress to
            // coarse buckets: the exact fill fraction at the moment
            // the drain observed it is load-dependent (a busy host
            // decodes a different number of packets between two
            // drains), so 0.02-vs-0.07 flapping is machine noise,
            // not behavior. The EPISODE shape is the contract: entry
            // (0.00), the exit bookend (1.00), and at most one
            // coarse mid marker (0.50 bucket) when a mid observation
            // exists at all.
            let bucket = if *progress <= 0.25 {
                "0.00"
            } else if *progress < 0.75 {
                "0.50"
            } else {
                "1.00"
            };
            fields.push(("progress".to_string(), Json::str(bucket)));
        }
        EventKind::TracksListReady { tracks } => {
            fields.push(("kind".to_string(), Json::str("tracks.list-ready")));
            fields.push(("tracks".to_string(), Json::UInt(tracks.len() as u64)));
        }
        EventKind::TimingTimeTick { time, position } => {
            fields.push(("kind".to_string(), Json::str("timing.time-tick")));
            fields.push(("time_us".to_string(), Json::Int(time.0)));
            fields.push(("position".to_string(), Json::str(format!("{position:.3}"))));
        }
        EventKind::TimingDurationKnown { duration } => {
            fields.push(("kind".to_string(), Json::str("timing.duration-known")));
            fields.push(("duration_us".to_string(), Json::Int(duration.0)));
        }
        EventKind::MediaInfoReady { info } => {
            // The open-time snapshot (card 3.B4): the same facts
            // `media.get-info` answers, as a compact record.
            fields.push(("kind".to_string(), Json::str("media.info-ready")));
            fields.push(("seekable".to_string(), Json::Bool(info.seekable)));
            fields.push(("is_live".to_string(), Json::Bool(info.is_live)));
            fields.push((
                "duration_us".to_string(),
                match info.duration {
                    commands::catalog::types::Duration::Known { time } => Json::Int(time.0),
                    commands::catalog::types::Duration::Unknown => Json::Null,
                    _ => Json::Null,
                },
            ));
            fields.push(("tracks".to_string(), Json::UInt(info.tracks.len() as u64)));
        }
        EventKind::PlaybackEndOfTrack { track_kind, id } => {
            fields.push(("kind".to_string(), Json::str("playback.end-of-track")));
            fields.push(("track_kind".to_string(), Json::str(kind_str(*track_kind))));
            fields.push(("track_id".to_string(), Json::UInt(u64::from(id.0))));
        }
        EventKind::PlaybackEndOfMedia => {
            fields.push(("kind".to_string(), Json::str("playback.end-of-media")));
        }
        EventKind::PlaybackError {
            stage,
            code,
            message,
            details,
            fatal,
        } => {
            fields.push(("kind".to_string(), Json::str("playback.error")));
            fields.push(("stage".to_string(), Json::str(stage.as_str())));
            fields.push(("code".to_string(), Json::str(code.as_str())));
            fields.push(("message".to_string(), Json::str(message.clone())));
            // The fatal/non-fatal split and the non-fatal coalescing
            // count (card 3.B4, doc 05 §3.2): a transcript reader
            // must be able to tell the session-killing failure from
            // the bounded-recovery report without parsing prose.
            fields.push(("fatal".to_string(), Json::Bool(*fatal)));
            match details.get("count") {
                Some(count) => {
                    fields.push(("count".to_string(), Json::str(count.clone())));
                }
                None => {
                    fields.push(("count".to_string(), Json::Null));
                }
            }
        }
        other => {
            fields.push(("kind".to_string(), Json::str("(unrecorded event kind)")));
            fields.push(("debug".to_string(), Json::str(format!("{other:?}"))));
        }
    }
    Json::Obj(fields)
}

/// The headless accounting as a transcript record (the per-frame
/// evidence — both limbs, the scheduler, the staging measurements).
fn accounting_json(accounting: &HeadlessAccounting) -> Json {
    Json::obj(vec![
        ("frames_accepted", Json::UInt(accounting.frames_accepted)),
        ("frames_refused", Json::UInt(accounting.frames_refused)),
        ("video_presented", Json::UInt(accounting.video_presented)),
        ("video_refused", Json::UInt(accounting.video_refused)),
        ("scheduler_pushed", Json::UInt(accounting.scheduler_pushed)),
        (
            "scheduler_presented",
            Json::UInt(accounting.scheduler_presented),
        ),
        (
            "scheduler_repeats",
            Json::UInt(accounting.scheduler_repeats),
        ),
        (
            "scheduler_dropped_late",
            Json::UInt(accounting.scheduler_dropped_late),
        ),
        // The seek-boundary observability (W1-06): the pre-seek
        // scheduler window purged as stale at the generation change,
        // the presentation queue's publish drops, and the queue's
        // live published generation (the mint a seek moves).
        (
            "scheduler_dropped_stale",
            Json::UInt(accounting.scheduler_dropped_stale),
        ),
        ("queue_flushed", Json::UInt(accounting.queue_flushed)),
        ("queue_published", Json::UInt(accounting.queue_published)),
        ("staging_frames", Json::UInt(accounting.staging_frames)),
        ("staging_bytes", Json::UInt(accounting.staging_bytes)),
        ("staging_copies", Json::UInt(accounting.staging_copies)),
    ])
}

/// The final PNG's transcript record: length, SHA-256 digest, and the
/// IHDR extent (parsed from the bytes — a real render-path assertion,
/// not a "looks like a PNG" sniff). A PNG shorter than its own IHDR
/// extent records `null` fields (never a panic — the scenario turns
/// that into a failed check via the extent assertions).
fn png_json(png: &[u8]) -> Json {
    // PNG: 8-byte signature, then chunks; IHDR is first (offset 8):
    // 4 length + 4 type + 4 width + 4 height (big-endian).
    let extent = if png.len() >= 24 {
        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        (Json::UInt(u64::from(width)), Json::UInt(u64::from(height)))
    } else {
        (Json::Null, Json::Null)
    };
    Json::obj(vec![
        ("bytes", Json::UInt(png.len() as u64)),
        ("sha256", Json::str(sha256_hex(png))),
        ("width", extent.0),
        ("height", extent.1),
    ])
}

/// The track-kind wire name.
fn kind_str(kind: commands::catalog::types::TrackKind) -> &'static str {
    match kind {
        commands::catalog::types::TrackKind::Video => "video",
        commands::catalog::types::TrackKind::Audio => "audio",
        commands::catalog::types::TrackKind::Subtitle => "subtitle",
        commands::catalog::types::TrackKind::Data => "data",
    }
}

/// Normalize the per-run label values in a rendered transcript so a
/// fresh run and a committed artifact compare byte-for-byte: the
/// `environment_kind` and `os` values name the PRODUCING machine
/// (honest per-artifact truth, doc 10 §7.1) and therefore differ
/// between `local` and `ci-vm` runs by design. Exactly those two
/// label lines are rewritten to a fixed placeholder; every other
/// byte must match. The currency checks assert the STABLE labels
/// (`hardware_class` = `mock`, the mock device pair) semantically
/// instead — so nothing volatile escapes scrutiny, and nothing
/// stable is excused from it.
#[must_use]
pub fn normalize_run_specific(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("\"environment_kind\":") || trimmed.starts_with("\"os\":") {
            let indent = &line[..line.len() - trimmed.len()];
            out.push_str(indent);
            out.push_str("\"normalized\": \"per-run\",\n");
        } else {
            out.push_str(line);
        }
    }
    out
}
