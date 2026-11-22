# AUDIT: crates/playback-engine/tests/scenarios_b/host.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 2307 / ~55 production items (harness: 3 fakes, 1 host struct with ~30 methods, 20 free helpers) |
| File verdict | REAL (production stack under a parallel local command layer) |
| Reachable from shipped entry point? | N/A (test harness) — drives the REAL Engine + AviDemuxer + MjpegDecoder/PcmDecoder + input::FileAccess through a LOCAL re-implementation of the CLI dispatch layer. Part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:3 |
| Commands run | Read in 8 chunks; cross-checks: apps/cli host.rs (dispatch/receipt shapes), input::ScriptSource semantics, media-probe builtins claim; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The scenario harness: a scenario-facing command layer (`ScenarioHost` + `lumen_dispatch`) over the real engine and the real AVI/MJPEG/PCM production stack, with three named fakes (a fault-scriptable access source, an on-demand timeline demuxer, an observing/vanishing sink) and a genuinely impressive byte-surgery AVI shortener whose comments even document the historical bug it fixed. The module header is the most honest architecture note in the repo: it explains exactly why this suite cannot drive the production `lumen_cli::dispatch::lumen_dispatch` (dev-edge cycle rejected by boundaries.toml), names every fake, and files a [PROPOSAL] to close the gap. That gap is also the file's central defect: everything above the engine — command routing, seek/set-track composition, wire conversions, position reporting, error mapping — is a hand-maintained parallel implementation of the CLI host, so production regressions in the real dispatch layer are invisible to this suite, and two harness behaviors (position semantics, the never-exercised audio-master clock wiring) diverge from what ships.

## 2. What actually works (verified)

- Real stack below the harness: `AviDemuxer::new(stream)` (1496), `MjpegDecoder`/`PcmDecoder::with_format` via `RealProvider` (705-742), `input::FileAccess` (1475-1477), the real `Engine`/`SessionHandle`/`SessionConfig`, and real committed fixtures (`generated/valid/avi_mjpeg_pcm.avi`, `avi_mjpeg_2pcm.avi` — testkit fixture paths).
- Determinism: media time advances ONLY when the sink accepts audio (the sample-counter model, 649-658, with the CLI's frozen-clock guard mirrored at 654), the event throttle reads a permanently frozen source (1627-1630), every pump/await is bounded (`AWAIT_PUMPS` 133; `run_until_terminal` answers pumps-spent, 1156-1165), and there is not a single sleep in 2307 lines (rg-verified).
- Bounded harness memory: frame ring capped at 1024 with roll-out (508-520), events/state cleared at every settled close (1659-1666), reused drain scratch (D-1, 1230-1257).
- `build_short_avi` (2162-2287): real RIFF walking with asserted preconditions (RIFF/LIST/movi/idx1 fourcc checks, kept==2×frames, video==audio==frames, idx1 coverage), honest header patching (avih dwTotalFrames, strh dwLength per fccType incl. sample-size scaling for 'auds'), RIFF size patched last; the comment at 2213-2217 documents the slice-offset bug the original walk had. This builds genuine well-formed tiny media — the strongest fixture-surgery code I have audited in this repo.
- The fault-injection design is right: `ScriptHandle` shares state with the engine-owned stream so faults arm mid-play (148-201); `pace_next_open` (1093-1100) exists precisely because the default caps let the sink race the whole stream during the open's own pumps — the comment explains the observed failure mode (1087-1092).
- Teardown oracles: drop-hook counters (322-370, 683-689), `/proc/self/fd` and thread counts (2292-2307, Linux-gated, None elsewhere — documented).

## 3. Findings

### F-001 [S2] The command layer under test is a parallel implementation, not the shipped one
Where: crates/playback-engine/tests/scenarios_b/host.rs:1807-1885 (lumen_dispatch/route) vs apps/cli/src/host.rs (the production host)
Code (host.rs:1807-1811):
```rust
/// **The** dispatch entry of this suite — the local shape of doc 05
/// §5.1's `lumen_dispatch` (see the module docs for why it is local
/// and what is fake). Correlate, route through the engine, answer
/// the wire response.
#[must_use]
pub fn lumen_dispatch(host: &mut ScenarioHost, request: &WireRequest) -> WireResponse {
```
Code (host.rs:1281-1284, the divergence note):
```rust
    /// Submit `playback.seek` with its target (the CLI
    /// `dispatch_seek` shape, minus the CLI's audio ring — this
    /// host's sink is the direct seam): map the wire target, submit,
```
What is wrong: `route` (1830-1885) hand-implements the served command set, receipt translation, seek composition, track-switch composition, class-S snapshots, and typed refusals — a from-scratch copy of the CLI host's behavior. The production host additionally owns the null-device audio ring, the DrainGatedDecoder gate, the `advance` pump semantics, filtered re-wiring for `--no-audio`, and the real `lumen_dispatch`'s command surface. The suite therefore proves the command-layer CONTRACT against the real engine, but a regression introduced in apps/cli's dispatch (wrong refusal text, a broken receipt correlation, a seek bug in the ring-coupled path) passes this suite untouched. The module doc admits this in full (4-39) and proposes the two real fixes — but the tree ships the gap.
Why it matters: These sixteen scenarios are doc 10 §6.4's acceptance evidence. Any consumer of that evidence believes it covers the product's command path; it covers a lookalike. Realistic divergence, silent — S2.
What correct looks like: The module's own [PROPOSAL] (32-39): move the suite where it can dev-depend on lumen-cli, or extract the CLI host core below playback-engine so both hosts share it. Until then, this file's audit trail must travel with every scenario verdict.
Confidence: HIGH — Verification: read (both hosts).
### F-002 [S3] The shipped audio-master clock wiring is never exercised at engine level
Where: crates/playback-engine/tests/scenarios_b/host.rs:1596-1603 (finish_wiring) and 1205-1211 (position_us)
Code (1596-1603):
```rust
        let clock = Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::clone(&self.now) as Arc<dyn MonotonicSource>,
                Timestamp::new(0, base),
                media_types::Rate::ONE,
            )
            .map_err(|err| wire_error(&err))?,
        );
```
Code (1207-1211):
```rust
    pub fn position_us(&self) -> i64 {
        self.wired
            .as_ref()
            .map_or(0, |wired| wired.clock.now().ticks / 1_000)
    }
```
What is wrong: Every scenario session wires `system_monotonic` over the shared virtual source and models the sample counter by advancing that source from the sink (649-658). The SHIPPED audio path wires `audio_master` over the null-backend sink counter (pipeline_host.rs:1605, my clock.rs audit) and reports position through the device/latch logic (apps/cli host.rs:346-358). So the engine-level scenarios never run the clock construction users actually get, and this host's `position_us` (raw `clock.now()`) has different semantics from the production position (device-gated / last-presented latch). The structurally-equivalent-at-the-seam argument holds for the engine (the clock is injected either way), but the *integration* truth — audio-master clock + engine + sink — is tested only in audio-output's crate tests, never end-to-end here.
Why it matters: The no-audio-device question this wave was asked (who drives the clock) has its audio-present half untested at scenario level; a regression in the audio-master wiring would pass all sixteen scenarios. Coverage gap, not a proven defect — S3.
What correct looks like: One scenario (or harness mode) wiring `audio_master` over a fake `SampleCounterSource` advanced by the sink, asserting the same positions.
Confidence: HIGH — Verification: read (all three wiring sites).
### F-003 [S4] `submit_and_await` collapses every submit error into `SessionClosed`
Where: crates/playback-engine/tests/scenarios_b/host.rs:1275-1277
Code:
```rust
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
```
What is wrong: The real `SessionError` (InboxBusy, GenerationExhausted, …) is discarded; every submit failure is reported as "the session closed". A harness-side swallowed error — the scenario sees a plausible-but-wrong refusal.
Why it matters: Test-harness honesty: an inbox-full condition would masquerade as session-closed in a scenario's assertion surface. Low likelihood (bounded command volume), high confusion when hit.
What correct looks like: `.map_err(|error| session_error_wire(&error))` — the function already exists and handles all arms.
Confidence: HIGH — Verification: read.
### F-004 [S4] Hand-mirrored wire vocabulary: 26 event serde names + Debug-form command-id derivation
Where: crates/playback-engine/tests/scenarios_b/host.rs:2038-2067 (event_name) and 1916-1955 (command_id_string/catalog_prefix)
Code (1916-1923):
```rust
fn command_id_string(command: &Command) -> String {
    let debug = format!("{command:?}");
    debug
        .split(['{', ' ', '('])
        .next()
        .unwrap_or("(unnamed)")
        .to_string()
}
```
What is wrong: `event_name` mirrors the serde tags of 26 event kinds by hand; `command_id_string`/`catalog_prefix` reconstruct dotted command ids from Debug-form token structure ("serde_json is not on this crate's dependency set, and this is error-path only" — 1912-1915). Both are parallel vocabularies that can drift from the authoritative definitions in the commands crate.
Why it matters: Drift in `event_name` makes `count_events` count zero → scenario asserts fail loudly (acceptable); drift in the Debug-derivation mislabels the `unserved` refusal message only (error-path). Brittle-by-necessity test infrastructure, documented as such. Hygiene.
What correct looks like: A `commands`-crate API exposing the serde name of an `EventKind` and the dotted id of a `Command` (the authoritative source both hosts then share).
Confidence: HIGH — Verification: read.

## 4. Per-function table (harness inventory)

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `stamp_ns` | 124-130 | REAL | Checked conversion. |
| `ScriptHandle::{shrink_to, fail_next_read, visible_len, read_stats, lock}` | 167-201 | REAL | Mid-play fault arming; stats as assertion surface. |
| `ScriptedStream::{new, read, read_into, seek, position, uri, size, supports_seek, lock_state}` | 208-301 | REAL | The named access fake; byte semantics per input::ScriptSource; typed seek errors. |
| `seek_typed` | 303-311 | REAL | Local typed error. |
| `DropHooks`/`demuxer_dropped`/`sink_dropped` | 322-342 | REAL | Teardown oracle. |
| `DropCountingDemuxer` (Demuxer + Drop) | 347-370 | REAL | Forwards all three trait methods. |
| `TimelineDemuxer::{audio_timeline, packet}` + Demuxer impl | 377-446 | REAL | On-demand bounded timeline; not-started/not-seekable typed refusals. |
| `timeline_track` | 450-470 | REAL | UNKNOWN span for zero-packet case. |
| `ObservingSink::{ring_push, frame_ring, last_pts_ns_of_track, counting, vanish, frames_accepted, audio_accepted, frames_while_vanished, last_pts_ns, frame_duration_ns}` | 506-629 | REAL | Bounded ring (cap 1024); PCM sample-derived durations with per-format byte sizes; dead-code allow on one query is honest (539-548). |
| `FrameConsumer for ObservingSink::accept` | 632-681 | REAL | Vanish arm returns cloned typed error; audio advances virtual time only when clock unpaused (the CLI guard); video rides the ring. `unreachable!` at 672 for the third frame kind — acceptable in a v0 harness. |
| `Drop for ObservingSink` | 683-689 | REAL | Sink-drop oracle. |
| `SharedNow::advance_ns` | 691-696 | REAL | The sample-counter model. |
| `RealProvider::decoder_for` | 709-742 | REAL | Real MJPEG/PCM decoders; typed no-strf-facts refusal. |
| `harvest_avi_facts`/`pcm_fact_map` | 759-798 | REAL | CLI harvest shape. |
| `wire_error`/`access_refusal`/`session_error_wire` | 802-857 | REAL | Local copies of the CLI conversions (F-001 surface); V8 redaction note honest. |
| `file_uri` | 864-893 | REAL | Canonicalize; not-a-regular-file refusal; UTF-8 refusal. |
| `sniff_format`/`probe_refusal` | 899-928 | REAL | Local media-probe mirror; mp4/unknown → typed probe refusal (documents D-018 first-vertical AVI-only). |
| `avi_queues`/`timeline_queues`/`avi_budget`/`multitrack_budget`/`timeline_budget` | 938-998 | REAL | The multitrack_budget comment (972-983) documents the observed priming race it fixes — honest engineering trail. |
| `ScenarioHost::{new, arm_drop_hooks, pace_next_open}` | 1049-1100 | REAL | Pacing override with rationale. |
| `pump_pipeline`/`settle`/`pump_pipeline_until`/`pump_pipeline_until_within`/`run_until_terminal` | 1107-1165 | REAL | All bounded; run-to-end answers pumps spent. |
| `state`/`session_id`/`generation`/`events`/`state_history`/`stats`/`position_us`/`sink`/`script` | 1170-1223 | REAL | Observation surface; position_us semantics diverge from production (F-002). |
| `drain_outputs` | 1230-1258 | REAL | CLI shape, reused scratch. |
| `submit_and_await` | 1262-1279 | REAL | S4 F-003 error collapse. |
| `dispatch_seek` | 1287-1339 | REAL | Bounded terminal wait; Chapter typed refusal. |
| `dispatch_set_track` | 1346-1383 | REAL | Id/None accepted; Auto refused per §2.5. |
| `await_receipt`/`receipt_outcome` | 1387-1425 | REAL | Bounded; truthful-UI mapping. |
| `open_session` | 1429-1454 | REAL | Settles open; fatal surfaced in-response; teardown after fatal. |
| `wire_source` | 1458-1486 | REAL | no_video/no_audio typed refusal naming the real owner (honest). |
| `wire_avi` | 1489-1526 | REAL | ≥2 audio tracks ⇒ coarse budget (documented). |
| `wire_scripted` | 1532-1578 | REAL | script:// and timeline:// schemes; other URLs refused typed. |
| `finish_wiring` | 1584-1643 | REAL | system_monotonic wiring (F-002); #[allow(too_many_arguments)] documented workaround. |
| `teardown_after_fatal`/`teardown_session_scoped` | 1647-1666 | REAL | Stale-last_fatal dies with session (documented recovery shape). |
| `media_info`/`duration`/`current_time` | 1669-1720 | REAL | Class-S snapshots; clamped fraction. |
| `not_ready`/`parse_timeline`/`duration_wire`/`rejection_wire` | 1723-1801 | REAL | Rejection table mirrors the CLI's, reason-by-reason. |
| `lumen_dispatch`/`route`/`is_seek_terminal`/`unserved` | 1811-1910 | REAL | F-001 subject. |
| `command_id_string`/`catalog_prefix`/`owner_of` | 1916-1979 | REAL | S4 F-004. |
| `dispatch`/`take_request_id`/`error_events`/`count_events`/`event_name` | 1988-2067 | REAL | Scenario-side surface; F-004. |
| `TempDir::{new, path, entries, copy_parent}` + Drop | 2071-2115 | REAL | Process-id uniqueness only — same-tag collision within a process possible (noted §7). |
| `corrupt_bytes` | 2119-2123 | REAL | Deterministic corruption pattern. |
| `u32_le`/`put_u32_le`/`fourcc_at` | 2132-2144 | REAL | Panics on short slices — broken-fixture-is-broken-test, documented. |
| `TempDir::build_short_avi` | 2162-2287 | REAL | The byte-surgery fixture builder (§2). |
| `open_fds`/`thread_count` | 2292-2307 | REAL | Linux-gated oracles, None elsewhere. |

## 5. Test honesty

No tests in this file (harness). The scenarios that consume it are files #19-21 (mine) and the other thirteen (others'). The harness's own honesty properties: bounded everywhere, no sleeps, fakes named, determinism contract stated and mechanically enforced (frozen throttle; sink-driven time).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the real entry is unreachable from here: a lumen-cli dev-dependency would close a dev-edge cycle … which boundaries.toml check 2 rejects" (6-13) | module doc | REAL-UNPROVEN | The dependency direction is verified (playback-engine is a dep of apps/cli); the boundaries.toml rejection of the dev-cycle is its file's audit (W01). |
| "What is a fake is named below; everything else is the production stack" (29-30) | module doc | REAL | Verified against imports (84-121): only ScriptedStream/TimelineDemuxer/ObservingSink/sniff/wire-conversions are local. |
| "no test ever sleeps" (79) | module doc | REAL | rg: zero sleeps in the file. |
| "Byte semantics are copied verbatim from input::ScriptSource/SyntheticStream" (48-49) | module doc | UNVERIFIED | input crate's fakes are W-input's files; the read/seek shapes look consistent. |
| "the xtask generator's byte-stable contract" (2159-2160) | comment | UNVERIFIED | xtask fixture generation is W01's domain. |

## 7. Dead / duplicated / unreachable code

- `last_pts_ns_of_track` (539-548): `#[allow(dead_code)]` with the honest gate-doc provenance comment — kept deliberately, disclosed.
- Duplication: the whole local command layer duplicates apps/cli host semantics by design (F-001); `event_name` duplicates the serde vocabulary (F-004).
- `TempDir::new` uniqueness is process-id-only; two same-tag TempDirs in one process share a directory (drop order then matters). Not hit by current scenarios; noted.

## 8. Dependencies & flags

Dev-edges: demux (AVI), codecs (MJPEG/PCM), input, media-clock, packet-queue, commands, plugin-api, codec-api, testkit — all production crates except testkit. No feature gates. Linux-specific oracles degrade to None elsewhere.

## 9. Open questions for the next agent

- W13 (apps/cli host): diff the production `dispatch_seek`/`receipt_outcome`/`rejection_wire` against this harness's copies line by line — every unshared difference is a blind spot of this suite (F-001 inventory).
- W01 (boundaries.toml / xtask): confirm the dev-cycle rejection claim and whether the [PROPOSAL] at 32-39 (move suite or extract shared host core) was ever scheduled — the worklog says.
