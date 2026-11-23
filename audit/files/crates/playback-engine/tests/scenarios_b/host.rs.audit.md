# AUDIT: crates/playback-engine/tests/scenarios_b/host.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 2307 / ~114 (incl. methods, impls) |
| File verdict | REAL (test harness driving the REAL engine; the command layer is a LOCAL reimplementation, not the shipped dispatch — divergences flagged S2) |
| Reachable from shipped entry point? | NO — compiled only into the `scenarios_b` test binary (`cargo test -p playback-engine --test scenarios_b`); the `Engine`/`SessionHandle`/`AviDemuxer`/MJPEG/PCM stack it drives is shipped code (playback-engine, demux, codecs, input crates), but the composition (demux+codecs+input into playback-engine) exists ONLY on dev edges (boundaries.toml:294-303) — no shipped binary builds it this way. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 3 |
| Commands run | `cargo test -p playback-engine --test scenarios_b -- scenario_07` -> 1 passed, exit 0; `... -- scenario_08 scenario_11 scenario_15` -> 5 passed, exit 0; `rg "set_audio_track|set_video_track" apps/` -> 0 matches; baseline audit/runs/baseline/cargo-test-workspace.txt: 23 passed / 0 failed in 5.06s |

## 1. One-paragraph verdict
This is a real test harness, not a fake one: every engine interaction goes through the production `Engine`/`SessionHandle` API (`poll`, `drain_outputs_into`, `submit`, `seek`, `set_audio_track`), and the AVI/MJPEG/PCM/file-access stages are the production crates. I executed scenario_07 (A/V sync) and scenarios 08/11/15 — all pass (outputs above). The command layer, however, is a local re-implementation of `lumen_cli::dispatch::lumen_dispatch` (admitted up front, lines 4-39), and it is NOT a faithful mirror: it serves `TracksSetAudioTrack`/`TracksSetVideoTrack`/`PlaybackSetPosition` — commands no shipped host serves (rg over apps/: zero matches) — silently drops `start_time`, and runs a test-only `TickBudget` (1 packet/limb) whose own doc comment admits the production-shaped pacing breaks mid-media track switching ("no new-track frame ever flowed"). So: REAL engine evidence, but the "pins the command-layer CONTRACT" claim is a superset of any shipped host's contract.

## 2. What actually works (verified)
- verified by `cargo test -p playback-engine --test scenarios_b -- scenario_07` -> `test scenario_07_av_sync::full_playback_keeps_av_stamps_within_tolerance_at_checkpoints ... ok` (exit 0, 0.09s) — real open/play/checkpoint/close cycle over the real AVI fixture.
- verified by `cargo test -p playback-engine --test scenarios_b -- scenario_08 scenario_11 scenario_15` -> 5 passed (exit 0, 2.36s).
- verified by baseline audit/runs/baseline/cargo-test-workspace.txt (scenarios_b section): `23 passed; 0 failed; 0 ignored ... finished in 5.06s`.
- verified by `ls tests/fixtures/generated/valid/` -> `avi_mjpeg_pcm.avi`, `avi_mjpeg_2pcm.avi` exist (PARENT/MULTITRACK constants resolve).

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/tests/scenarios_b/host.rs:1873-1883
Code:
```
        Command::PlaybackSetPosition { fraction } => {
            host.dispatch_seek(commands::catalog::types::SeekTarget::Percent {
                fraction: *fraction,
            })
        }
        Command::TracksSetAudioTrack { selector } => {
            host.dispatch_set_track(SessionCommandKind::SelectAudio, *selector)
        }
        Command::TracksSetVideoTrack { selector } => {
            host.dispatch_set_track(SessionCommandKind::SelectVideo, *selector)
        }
```
What is wrong: The local route serves three commands that NO shipped host serves. `rg "TracksSetAudioTrack|TracksSetVideoTrack|PlaybackSetPosition" apps/` returns zero matches — apps/cli/src/dispatch.rs:75-157 serves neither (they fall to `unserved`, whose `owner_of` message is "track switching is card 3.A3"). The module doc (line 1826-1829: "the served subset this suite's scenarios need, mirroring the CLI's semantics") is therefore FALSE for these three commands: the CLI's semantics are a typed refusal, this host's are full service. Scenario 8's track-switching evidence (and any set-position evidence) covers `SessionHandle::set_audio_track` machinery that no shipped entry point reaches.
Why it matters: the suite is cited as command-layer scenario coverage; a reader concludes the shipped product does mid-session track switching and set-position — it does not.
What correct looks like: either the local host refuses these like the CLI (and scenario 8 tests the engine API directly, labeled as such), or the shipped CLI actually serves them.
Confidence: HIGH — Verification: ran rg over apps/ (0 matches) + read apps/cli/src/dispatch.rs:75-157.

F-002 [S2]
Where: crates/playback-engine/tests/scenarios_b/host.rs:972-990
Code:
```
/// (observed:
/// scenario 8's switch landed at the end, the new limb
/// `can_never_deliver`-completed, no new-track frame ever flowed).
fn multitrack_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 1,
        flush_calls_per_limb: 8,
    }
}
```
What is wrong: The comment admits that under the 4-packet `avi_budget` slice (and a fortiori under the production `TickBudget::default()` of 16 packets/limb, pipeline.rs:469-476), the open "races the whole stream to EOS while still Buffering" and mid-media track switching never delivers a new-track frame. The harness fixes this by shrinking the test budget to 1 packet/limb — tuning the test environment until the scenario passes instead of fixing the engine's priming-vs-decode ordering. No shipped host uses 1-packet budgets (the CLI relies on `TickBudget::default()` + its audio-ring decode-park, apps/cli/src/pipeline_host.rs:105).
Why it matters: the engine-level defect (queue high-watermark starvation on coarse-granularity multi-track media) is documented in a test comment and then routed around; the production configuration is untested for this scenario.
What correct looks like: an engine-side fix (watermark evaluated pre-decode, or decode slice bounded by the high-watermark distance) with the scenario passing under the default budget.
Confidence: HIGH — Verification: read + compared pipeline.rs:469-476 (default 16) and apps/cli/src/pipeline_host.rs:105.

F-003 [S3]
Where: crates/playback-engine/tests/scenarios_b/host.rs:1275-1277
Code:
```
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
```
What is wrong: any submit failure — including `SessionError::InboxBusy` (a bounded-inbox refusal this same file maps to a ResourceLimit error at lines 846-850) — is reported as "the session closed before the command landed". The error is discarded (`|_|`), so the message can be factually wrong. Inherited verbatim from the production CLI (apps/cli/src/host.rs:862-864 has the identical construct) — a production defect mirrored here.
Why it matters: an inbox-full condition is misreported as session death; callers cannot distinguish transient from fatal.
What correct looks like: `.map_err(session_error_wire)` like the seek path at line 1323 already does.
Confidence: HIGH — Verification: read both sites.

F-004 [S3]
Where: crates/playback-engine/tests/scenarios_b/host.rs:1429-1454
Code:
```
    fn open_session(
        &mut self,
        source: SourceSpec,
        options: &OpenOptions,
    ) -> Result<CommandOutcome<CommandResult>, CommandError> {
        if self.engine.session_id().is_some() {
```
What is wrong: `options.start_time` is silently ignored. The real CLI serves it (apps/cli/src/dispatch.rs:81-87 passes `options.start_time.map(|time| time.0)` into `open_session`). Here a `session.open` with `start_time` would open at 0 with no error — a silent semantic divergence in a host whose doc claims to mirror the CLI. (No current scenario uses `start_time`, verified by rg over scenarios_b/, so no test is currently corrupted by it.)
Why it matters: the harness accepts and drops a field; any future scenario using it gets a false pass.
What correct looks like: refuse `start_time` typed (like `no_video`/`no_audio` at lines 1463-1471) or implement it.
Confidence: HIGH — Verification: read + rg confirmed no scenario passes start_time.

F-005 [S3]
Where: crates/playback-engine/tests/scenarios_b/probe.rs:1-2 (sibling wired by this suite's main.rs)
Code:
```
//! Temporary probe — measure the scripted source's read pattern (to
//! be deleted before commit).
```
What is wrong: a self-declared temporary debug probe ("to be deleted before commit") is committed, wired (`mod probe;` in main.rs), and runs in the suite (baseline shows `probe::probe_read_pattern ... ok`). It asserts nothing about product behavior — it only eprintlns read stats.
Why it matters: dead scaffolding shipped in the evidence suite; the test count is inflated by a non-assertion.
Confidence: HIGH — Verification: read probe.rs (55 lines) + baseline test list.
(Note: probe.rs itself is not in my assigned list; recorded here as an adjacent finding.)

F-006 [S4]
Where: crates/playback-engine/tests/scenarios_b/host.rs:534-548
Code:
```
    /// Harness observation API kept for track-scoped stamp queries
    /// (symmetric with `frame_ring`); no scenario asserts on it yet —
    /// the gate doc §2 row 4c dead-code hit that surfaced this.
    #[allow(dead_code)]
    pub fn last_pts_ns_of_track(&self, track: u32) -> Option<i64> {
```
What is wrong: knowingly-dead API retained with an `#[allow(dead_code)]` suppress (one of the tree's 121 allow-lints). The doc comment itself concedes nothing uses it.
Confidence: HIGH — Verification: read + rg (only definition, no callers in scenarios_b).

F-007 [S4]
Where: crates/playback-engine/tests/scenarios_b/host.rs:2038-2067
Code:
```
fn event_name(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::SessionStateChanged { .. } => "session.state-changed",
```
What is wrong: a 26-entry hand copy of the commands crate's serde names. Drift would make `count_events` return 0 — self-detecting only where a nonzero count is asserted (scenario_07 asserts ==1, so the highest-traffic names are checked; an entry nothing asserts could drift silently).
Confidence: MEDIUM — Verification: read; cross-checked names against the EventKind enum arms (no drift found today).

F-008 [S4]
Where: crates/playback-engine/tests/scenarios_b/host.rs:2132-2134, 2199, 672
Code:
```
fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
```
What is wrong: indexing panics on short slices (documented as intentional: "a broken parent fixture is a broken test, never a product bug"), plus three `unreachable!`s (672, 1378, 2199). Test-only blast radius; consistent with the tree-wide 4126-unreachable pattern. Also `TempDir::new` (line 2077-2081) keys uniqueness on `process::id()` + tag only — two same-tag TempDirs in one process would collide (no current scenario does).
Confidence: HIGH — Verification: read.

## 4. Per-function table (every function/method; trivial accessors grouped)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| stamp_ns | 124-130 | REAL | — | ns via NANO_BASE convert; ok |
| ScriptHandle::{shrink_to, fail_next_read, visible_len, read_stats, lock} | 167-201 | REAL | — | fault-injection seam; poison-tolerant locks |
| ScriptedStream::new | 216-230 | REAL | — | shared-handle fake, named in docs |
| ScriptedStream::{uri,size,supports_seek} | 233-244 | REAL | — | size = visible_len (scripted) |
| ScriptedStream::read / read_into | 246-269 | REAL | — | truncation + next-failure; bounds-safe |
| ScriptedStream::seek / position | 271-292 | REAL | — | typed before-start/out-of-range refusals |
| ScriptedStream::lock_state | 296-301 | REAL | — | poison-tolerant |
| seek_typed | 303-311 | REAL | — | typed Inaccessible |
| DropHooks::{demuxer_dropped,sink_dropped} | 331-342 | REAL | — | relaxed loads, ok |
| DropCountingDemuxer::{start,next_event,seek,drop} | 352-370 | REAL | — | forwarding wrapper + drop counter |
| TimelineDemuxer::audio_timeline | 388-397 | REAL | — | checked_mul via .must() (panics on absurd input — test-only) |
| TimelineDemuxer::packet | 399-416 | REAL | — | 64-byte static payload, bounded |
| TimelineDemuxer::{start,next_event,seek} | 420-446 | REAL | — | unseekable typed refusal; not-started typed |
| timeline_track | 450-470 | REAL | — | codec "fake-pcm" |
| ObservingSink::ring_push / frame_ring | 511-532 | REAL | — | capped 1024 (D-2) |
| ObservingSink::last_pts_ns_of_track | 539-548 | DEAD | S4 | F-006 |
| ObservingSink::counting | 551-569 | REAL | — | constructor |
| ObservingSink::vanish | 573-585 | REAL | — | arms Device{AudioOutput,Disconnected} |
| ObservingSink::{frames_accepted,audio_accepted,frames_while_vanished,last_pts_ns} | 587-605 | REAL | — | counters |
| ObservingSink::frame_duration_ns | 609-629 | REAL | — | PCM samples-derived; guards ch/rate 0; S24/S32 both 4 bytes correct |
| ObservingSink::accept (FrameConsumer) | 633-680 | REAL | — | vanish arm returns typed error; clock advances only on audio & unpaused; `ns.max(0) as u64` safe |
| Drop for ObservingSink | 683-689 | REAL | — | sink-drop hook |
| SharedNow::advance_ns | 691-696 | REAL | — | sample-counter model |
| RealProvider::decoder_for | 709-741 | REAL | — | real MjpegDecoder/PcmDecoder; typed errors on missing strf facts |
| harvest_avi_facts | 759-782 | REAL | — | max span end; ns→µs divide |
| pcm_fact_map | 785-798 | REAL | — | per-track PCM facts |
| wire_error | 802-816 | REAL | — | stage-prefix strip; mirrors CLI |
| access_refusal | 823-831 | REAL | — | redacted name appended once |
| session_error_wire | 834-857 | REAL | — | all five SessionError variants mapped |
| file_uri | 864-893 | REAL | — | canonicalize-failure-is-truth |
| sniff_format | 899-909 | REAL | — | local mirror of media-probe builtins (named fake); `let _ = seek` ignores seek failure — benign (start-0 seek on just-opened stream) |
| probe_refusal | 913-928 | REAL | — | D-018 AVI-only sentence |
| avi_queues / timeline_queues | 938-962 | REAL | — | test pacing constants |
| avi_budget / multitrack_budget / timeline_budget | 964-998 | REAL | S2 | F-002 (multitrack tuned to pass) |
| ScenarioHost::new / Default | 1049-1073 | REAL | — | |
| arm_drop_hooks | 1078-1085 | REAL | — | |
| pace_next_open | 1093-1100 | REAL | — | one-shot queue override; honest doc on why needed |
| pump_pipeline / settle / pump_pipeline_until / _until_within / run_until_terminal | 1107-1165 | REAL | — | bounded, no sleeps (determinism claim holds — no sleep/timer in file) |
| state/session_id/generation/events/state_history/stats/position_us/sink/script | 1170-1223 | REAL | — | position_us correct: clock.now() is NANO_BASE ticks (clock.rs:772-776), /1000 = µs |
| drain_outputs | 1230-1258 | REAL | — | receipts stashed; state/fatal/tracks recorded; reused buffer |
| submit_and_await | 1262-1279 | REAL | S3 | F-003 swallows submit error |
| dispatch_seek | 1287-1339 | REAL | — | chapter → typed refusal; bounded terminal wait with honest Internal error |
| dispatch_set_track | 1346-1383 | REAL | S2-context | only reachable via F-001's unserved-in-production commands |
| await_receipt | 1387-1403 | REAL | — | 64-pump bound, typed timeout |
| receipt_outcome | 1407-1425 | REAL | — | Accepted/Completed/Rejected mapped |
| open_session | 1429-1454 | REAL | S3 | F-004 start_time ignored |
| wire_source | 1458-1486 | REAL | — | no_video/no_audio typed refusal (honest named gap) |
| wire_avi | 1489-1526 | REAL | — | ≥2 audio tracks ⇒ multitrack_budget (F-002) |
| wire_scripted | 1532-1578 | REAL | — | script://avi-parent, timeline://N x Us, else typed refusal |
| finish_wiring | 1584-1643 | REAL | — | clock/sink/config; frozen throttle source |
| teardown_after_fatal / teardown_session_scoped | 1647-1666 | REAL | — | stale last_fatal cleared (reopen-after-fatal honest) |
| media_info / duration / current_time | 1669-1720 | REAL | — | seekable:true hard-coded even for timeline (whose demuxer refuses seek at 439-445 — the CLI's info would disagree; test-fixture lie, S4-grade, noted) |
| not_ready / parse_timeline / duration_wire | 1723-1741 | REAL | — | |
| rejection_wire | 1744-1801 | REAL | — | all 10 RejectionReason variants mapped |
| lumen_dispatch | 1812-1821 | REAL | S2-context | the local dispatch entry — see F-001 |
| route | 1826-1886 | REAL | S2 | F-001 |
| is_seek_terminal | 1891-1896 | REAL | — | |
| unserved / command_id_string / catalog_prefix / owner_of | 1900-1979 | REAL | S4 | owner_of's "track switching is card 3.A3" contradicts the route serving it (F-001); catalog_prefix Debug-form parsing is fragile but error-path only |
| dispatch / take_request_id | 1988-2003 | REAL | — | |
| error_events / count_events / event_name | 2008-2067 | REAL | S4 | F-007 mirror drift surface |
| TempDir::{new,path,entries,copy_parent,build_short_avi,Drop} | 2071-2287 | REAL | S4 | F-008 (panic helpers, pid-keyed uniqueness); build_short_avi surgery self-documents a fixed off-by-12 bug (2210-2218) and asserts every structural invariant |
| corrupt_bytes | 2119-2123 | REAL | — | deterministic pattern |
| open_fds / thread_count | 2292-2307 | REAL | — | Linux-only, Option-honest elsewhere |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| scenario_01..16 (suite) | YES (verified for 07/08/11/15 by execution; baseline covers all 23) | Assertions on real engine state trails, wire events, frame rings, drop counters, allocator deltas — mutation of engine seek/switch/EOM logic would trip them |
| probe::probe_read_pattern | NO | eprintln-only observations; zero product assertions (F-005) |
| scenario_08 | PARTIALLY | real generation/no-old-frame assertions, but passes only under the harness's 1-packet budget (F-002); under production pacing the scenario's precondition (mid-media position at switch time) does not occur |
| scenario_12 vanish arm | YES | sink refusal → typed DeviceLost/fatal path asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "What is a fake is named below; everything else is the production stack" | host.rs:29-30 | REAL | ScriptedStream/TimelineDemuxer/ObservingSink/sniff are the only fakes; Engine/SessionHandle/AviDemuxer/MjpegDecoder/PcmDecoder/FileAccess verified production imports (host.rs:90-119) |
| "the served subset this suite's scenarios need, mirroring the CLI's semantics" | host.rs:1824-1825 | FALSE (for 3 commands) | F-001: CLI refuses tracks.set-*/set-position; local host serves them |
| "no test ever sleeps" (determinism) | host.rs:79-80 | REAL | rg over host.rs: no thread::sleep/instant/timer constructs; pure pump-count model |
| "Relocating the suite ... would reach the real entry" (cycle claim) | host.rs:9-18 | REAL | boundaries.toml:294-303: playback-engine dev_deps = [testkit, demux, codecs, input]; lumen-cli absent (and lumen-cli→playback-engine normal edge would close the cycle) |
| "acceptance command is `cargo test -p playback-engine --test scenarios_b`" | host.rs:17-18 | REAL | ran it (filtered); compiles in 1.75s, tests pass |

## 7. Dead / duplicated / unreachable code
- `last_pts_ns_of_track` (539-548): dead, allow-suppressed (F-006).
- `probe.rs` (suite sibling): self-labeled temporary, asserts nothing (F-005).
- `owner_of` "tracks.set-" arm (1968-1970): unreachable in this host — the route serves tracks.set-* so `unserved` never sees them; the string is a stale copy contradicting the route (F-001).
- Duplication: `wire_error`/`rejection_wire`/`session_error_wire`/`unserved`/`owner_of`/`command_id_string` are near-verbatim copies of apps/cli/src/dispatch.rs + host.rs equivalents — the exact drift hazard the module doc's [PROPOSAL] (a)-(b) acknowledges (host.rs:32-39).

## 8. Dependencies & flags
- Dev-only composition: demux, codecs, input, testkit (boundaries.toml:303 — Lead-flagged test-only edges). No feature flags involved; the suite builds under plain `cargo test -p playback-engine`.
- No new external crates; serde avoided via Debug-form parsing (command_id_string) and the event_name mirror (F-007).

## 9. Open questions for the next agent
- Does any feature-matrix row cite scenarios 8/4/6 as evidence of SHIPPED track-switch/set-position/seek capability? If yes, that row is FALSE for every shipped host (F-001) — matrix auditors should cross-check.
- The engine-side priming race admitted at 972-983 (open races stream to EOS under coarse budgets): does any shipped CLI path hit it (CLI uses default 16-packet budget but has the audio-ring decode-park)? Worth an engine-agent look at pipeline.rs watermark ordering.
- `media_info` reports `seekable: true` for timeline sessions whose demuxer always refuses seek (host.rs:1679 vs 439-445) — CLI's own media_info would presumably disagree; low-stakes test fixture lie.
