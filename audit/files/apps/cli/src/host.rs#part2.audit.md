# AUDIT: apps/cli/src/host.rs#part2 (lines 636-1414)
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = 636-1414 (`drain_outputs`' doc comment through the end of `impl CliHost` at `setting_set`). Part1 = 1-635 (module doc, struct, pump/wall-anchor half — W13-A09); part3 = 1416-1940 (free wire fns, `engine_info`, `command_listings`, both `#[cfg(test)]` mods — W13-A10). Seam at fn boundaries; no overlap. |
| Lines / functions | 779 / 22 |
| File verdict | REAL (core session lifecycle genuinely implemented and baseline-executed) with two S2 behavioral defects: a stale-`last_fatal` poison that bricks open/close against each other, and a false `seekable: true` capability report |
| Reachable from shipped entry point? | YES — `lumen` main → run.rs REPL → dispatch.rs `lumen_dispatch` → `host.open_session` (dispatch.rs:82), `submit_and_await` (:103), `session_closed` (:109), `dispatch_seek` (:131), `media_info` (:133), `current_time` (:134), `duration` (:135), `setting_get`/`setting_set` (:149/:152). Baseline `audit/runs/baseline/mp4-smoke-open-session.txt` exercises `open_session`+pump loop verbatim. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 3 |
| Commands run | `Read` host.rs 1-1418 (full context through part3 start) + 1224-1418; `rg -n "last_fatal" apps/cli/src` (set 659/839, only clear 998); `rg -n "close\|open_session\|dispatch_seek\|setting_\|media_info" apps/cli/src/dispatch.rs` (call sites 82-152); `rg -n "enum SourceSpec" -A 12 crates/commands/src/catalog/types.rs` (File + Url variants; http(s) served via streaming per unsupported_path.rs:184-187); `rg -n "reopen\|last_fatal" apps/cli/tests/*.rs` (NO test covers open→fatal→close→reopen); `rg -n "enum SeekSupport" -A 10 crates/plugin-api/src/demux.rs` (variants Fast/SlowLinear/None); `rg -n "SeekSupport::None" crates/` (ts/mod.rs:1167 ALWAYS None; avi/mod.rs:960 unseekable-source None; asf/adts/aiff/mp4/mkv conditionally); `sed -n '955,965p' crates/demux/src/avi/mod.rs` + `sed -n '1160,1170p' crates/demux/src/ts/mod.rs` (confirmed); `rg -n "pub fn submit" -A 6 crates/playback-engine/src/session.rs` (submit returns `Result<(), SessionError>`); `rg -n "pub enum SessionError" -A 20 crates/playback-engine/src/error.rs` (SessionClosed/InboxBusy/GenerationExhausted/Config — the swallowed variants); `sed -n` spot-checks of every quoted range (verified verbatim). |

## 1. One-paragraph verdict
This is the real session lifecycle of the CLI — open/teardown/seek/settings, the pump primitives' output drain, the media-time advance, the end-of-tail play-out — and the baseline smoke proves `open_session` runs end-to-end. But the error-recovery half has a genuine wedge that the W12 web-host twin already exposed, and the CLI variant is worse: `last_fatal` is set by any engine `Fatal` (659) or any present-path error (839) and is cleared ONLY by the next `open_session` (998) — never by close/teardown — so the advertised recovery flow (open a file whose decode fatally fails → close → open a GOOD file) answers the good open with the dead session's error, and because the new session is in `Buffering` (not `Error`) at the check, `teardown_after_fatal` skips the engine `Close`, nulls the host `handle` while the engine session lives, and the process is then PERMANENTLY stuck: `open` refuses "a session is already open — close it first" while `close` refuses "no session open" (dispatch.rs:102 → submit_and_await → handle `None`). On top of that, `media_info` hard-codes `seekable: true` against demuxers that answer `SeekSupport::None` (the entire MPEG-TS family, unseekable AVI sources), the chapter-seek refusal blames "AVI v0" regardless of container, settings overrides are documented session-local but survive teardown and are never applied to anything, and `submit_and_await` relabels every submit failure as "session closed". Where does an MP4 fall out in THIS range: it does not — `open_session` answers `Accepted` for the 4:2:0 MP4 (plays) and even for the 4:4:4 one (the decode `Fatal` lands after the Opening-exit pump, so the open itself succeeds and the failure surfaces as an event later); every wait in this file is pump-bounded (≤64), so the reported "hangs forever" cannot originate here (see Open questions).

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/mp4-smoke-open-session.txt` → `ok: accepted session=1 gen=0`, `state-changed Idle → Opening`, `tracks 2 ready`, `TimingDurationKnown`, `MediaInfoReady` — `open_session` + `drain_outputs` + TracksChanged snapshot run in the shipped binary (the 4:4:4 fatal then arriving as an event proves the class-A accepted≠completed shape).
— Verified by `audit/runs/baseline/avi-firstvertical-open-session.txt` → full `Buffering → Playing → Stopped → Idle` lifecycle, and `rc-smoke-engine-only.log.txt` 6/6 — the pump/advance/teardown chain works on the first vertical.
— Verified by read: every wait in this range is bounded (`pump_pipeline_until`/`pump_until` take `bound`, `await_receipt` caps at `AWAIT_PUMPS=64`, `dispatch_seek`'s terminal wait returns a typed `Internal` error on expiry) — no unbounded loop exists in 636-1414.
— Verified by read + baseline `orchrun-test-workspace-nofailfast.log.txt` (native_seek/ttf_seek suites) : the seek flow (submit → class-A receipt → boundary duties → bounded terminal wait) is exercised by committed integration tests.

## 3. Findings

### F-001 [S2]
Where: apps/cli/src/host.rs:658-660 (set), 835-841 (set), 998-1001 (only clear), 1013-1018 (wrong-state teardown), 1024-1033 (teardown clears neither `last_fatal` nor the engine session when state ≠ Error); blast radius via dispatch.rs:102-103
Code:
```rust
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
```
```rust
    fn teardown_after_fatal(&mut self) {
        if self.state() == PlaybackState::Error {
            let _ = self.submit_and_await(SessionCommandKind::Close);
        }
        self.teardown_session_scoped();
    }
```
What is wrong: `last_fatal` is set by `drain_outputs` (engine `Fatal`, 658-660) and by `surface_video_error` (any present-path error, 839) but cleared ONLY at the top of the next `open_session` (998) — `session_closed`/`teardown_session_scoped` (1037-1039, 1024-1033) never clear it. Reachable sequence with the baseline's own samples: (1) `open sample.mp4` (yuv444p) → `Accepted` (the decode `Fatal` has not landed yet — the pump loop at 997 stops at the first non-`Opening` state, i.e. `Buffering`, per baseline mp4-smoke-open-session.txt: "ok: accepted" then "event: error [decode.unsupported]"); the REPL settle then records `last_fatal`. (2) `close` → engine Close from Error → Idle → `session_closed` → teardown (fatal NOT cleared). (3) `open sample-420.mp4` (the good file) → `open_session` proceeds (engine session id is None), `engine.open` succeeds, the pump exits at `Buffering`, and the STALE fatal from the previous session is `take()`n → the good file's open FAILS with the old file's `h264.sw.unsupported`. Worse: the new session is in `Buffering`, so `teardown_after_fatal`'s `state() == Error` guard skips the engine `Close` while `teardown_session_scoped` nulls `self.handle` — the engine session is orphaned (`engine.session_id()` stays `Some`, host handle `None`). (4) Every subsequent `open` refuses "a session is already open — close it first" (897-903) while `close` refuses "no session open" (submit_and_await, 859-861, handle is None). The process can neither open nor close anything until exit.
Why it matters: open-failed-media → close → open-good-media is the most basic error-recovery flow; here it not only answers the WRONG file's error, it bricks the whole host permanently (open and close refuse each other's precondition). A present-path video error mid-playback (839) triggers the same poison without any engine fatal. No test in apps/cli/tests covers reopen after a mid-session/post-accept fatal (grep: only seek/no-reopen compositions). This is the CLI twin of W12-A05 F-001 (web host `last_fatal` poison) with a harder terminal state.
What correct looks like: clear `last_fatal` in `teardown_session_scoped` (or timestamp/scope it per session-generation and only honor fatals minted by the current open); in `teardown_after_fatal`, submit `Close` whenever `engine.session_id().is_some()` (not only in `Error`).
Confidence: HIGH — Verification: read (all five sites traced), baseline transcript for step (1), dispatch.rs call-chain read for the brick; no execution (sandbox rule) — the sequence is a code-trace, each step individually baseline-evidenced.

### F-002 [S2]
Where: apps/cli/src/host.rs:1307 (with 1283-1314)
Code:
```rust
                seekable: true,
                is_live: false,
```
What is wrong: `media_info` hard-codes `seekable: true` for every source. The demuxer layer truth is `SeekSupport` with a real `None` variant ("The source cannot seek (streams, pipes…)" — plugin-api/src/demux.rs:142-145), produced unconditionally by the MPEG-TS demuxer (`crates/demux/src/ts/mod.rs:1166-1168` returns `SeekSupport::None` always) and conditionally by avi/asf/adts/aiff/mp4/mkv (unseekable source / no index). `SelectDemuxer` faithfully forwards `seekability()` (demux_host.rs:596-603) but nothing in host.rs reads it — the field is a constant.
Why it matters: `media.get-info` is the capability snapshot a UI/remote-control builds on. For a .ts file (a whole shipped container family) the host claims seekable while `playback.seek` can only fail typed — the exact contradiction W12-A05 rated S2 on the web twin (there the field was also hard-coded `true`).
What correct looks like: carry the demuxer's `SeekSupport` (or the engine's seek acceptance) into `MediaFacts` at open and report `seekable: matches!(support, Fast{..} | SlowLinear)`; `is_live` likewise needs a source-derived value, not a constant.
Confidence: HIGH — Verification: read of both files + ts/avi demuxer sources; UNVERIFIED only in that no baseline run opens a .ts via the CLI to paste the wire output (the TS demuxer's None is unit-asserted at crates/demux/src/ts/tests.rs:671).

### F-003 [S3]
Where: apps/cli/src/host.rs:1300-1311
Code:
```rust
                title: None,
                duration: match duration_us {
                    Some(us) => Duration::Known {
                        time: Timestamp(us),
                    },
                    None => Duration::Unknown,
                },
                seekable: true,
                is_live: false,
                metadata: BTreeMap::new(),
                tracks: self.tracks.clone(),
                chapters: Vec::<Chapter>::new(),
```
What is wrong: `title`, `metadata`, `chapters` are structurally always empty: the host has no vocabulary for them (the mkv harvest records "Chapters/Editions … demuxer-side metadata only — MediaFacts carries no chapter vocabulary (the recorded gap)", mkv.rs:112-115), even though the demuxer parses titles/tags/chapters (crates/demux/tests/mkv_editions_tags.rs tests them) and `duration` for MKV is also always `Unknown` (per-track span end `Missing` by demuxer design — mkv.rs:98-104), so `media.get-duration` on any .mkv/.mka/.webm answers Unknown while the container declares a segment Duration.
Why it matters: get-info/get-duration are user-visible surfaces; whole format families answer with permanent amnesia for data the demuxer already parsed. The gaps are self-recorded as honest nos, so this is drift/limited-surface, not fraud.
What correct looks like: extend `MediaFacts` with title/metadata/chapters/duration (segment-level) at the demux-host seam and surface them here.
Confidence: HIGH — Verification: read (mkv.rs part1 doc + mkv_editions_tags.rs existence); baseline smoke shows `TimingDurationKnown` only for the MP4 lane.
(Severity note: the MKV-duration half of this is decided in demux_host/MediaFacts — W13's demux_host chunk owners should severity-call it; I record the observable site here.)

### F-004 [S3]
Where: apps/cli/src/host.rs:180-181 (field doc), 1024-1033 (teardown), 1372-1375 + 1404 (only consumers)
Code:
```rust
    /// Session-local settings overrides (the settings-schema seam).
    settings: BTreeMap<String, SettingValue>,
```
(`teardown_session_scoped` at 1024-1033 clears handle/consumer/device/clock/facts/source/tracks/pending_receipts — `settings` is absent.)
What is wrong: (a) The overrides are documented "Session-local" but survive `teardown_session_scoped` — they are process-lifetime; a `setting.set` in session 1 still answers `setting.get` in session 3. (b) Nothing but `setting_get` ever reads the map — no engine, decoder, device or pipeline input consumes a setting; `setting.set` validates, stores, and echoes `applied` while changing zero playback behavior ("UI settings wired to nothing" class).
Why it matters: The doc lies about scope, and the command's name (`SettingApplied`) overpromises: a user setting `audio.*` or `playback.*` gets a success receipt for a no-op.
What correct looks like: clear `settings` in `teardown_session_scoped` if session-local is the contract, and route at least one real knob (e.g. the null-device volume/rate seam) through it — or rename the receipt.
Confidence: HIGH — Verification: read + `rg` for `self.settings` (exactly two sites: 1372, 1404).

### F-005 [S3]
Where: apps/cli/src/host.rs:1080-1088
Code:
```rust
            WireTarget::Chapter { index } => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotFound,
                    format!(
                        "no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"
                    ),
                ));
            }
```
What is wrong: The chapter-seek refusal message names "AVI v0" for every container — the host serves mp4/mkv/wav/ogg/ts/…; a user seeking a chapter in an MKV (whose demuxer HAS parsed 50 chapters, mkv corpus lane 0007) is told AVI v0 has no chapter markers. The refusal itself is honest (no chapter source is wired, per the comment at 1067-1071); the message is a copy-paste from the AVI-era vertical.
Why it matters: Error text is a user-facing contract; it asserts a falsehood about the open container and misdirects triage.
What correct looks like: "no chapter {index}: this build resolves no chapter source for {container}" — the facts already carry `container`.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: apps/cli/src/host.rs:862-864
Code:
```rust
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
```
What is wrong: `SessionHandle::submit` returns `Result<(), SessionError>` with distinct typed variants (`SessionClosed`, `InboxBusy` "the caller retries or sheds load", `GenerationExhausted` — playback-engine/src/error.rs:66-83); the closure discards the real error and always reports session-closed.
Why it matters: An `InboxBusy` refusal would be reported as "session closed" — an actionable retry condition masked as a dead session.
What correct looks like: `map_err(|err| session_error_wire(&err))`.
Confidence: HIGH — Verification: read + engine signature read. (Reachability of InboxBusy is narrow in this host — it awaits each receipt before the next submit — hence S4.)

### F-007 [S4]
Where: apps/cli/src/host.rs:1351-1357
Code:
```rust
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
```
What is wrong: The `buffered` range is fabricated as [0, full duration] with no data source — no demux position, no ring fill, no network buffer state feeds it. For the http lane (SourceSpec::Url is served, unsupported_path.rs:184-187) the answer claims the entire remote file is buffered at open.
Why it matters: `media.get-current-time`'s buffered field is a lie for any non-local source; local files make it trivially true, so the blast radius is the streaming lane.
What correct looks like: derive buffered from the demuxer's known-contiguous range (or drop the field until a source of truth exists).
Confidence: HIGH — Verification: read.

### F-008 [S4]
Where: apps/cli/src/host.rs:725, 823, 1273
Code:
```rust
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
```
What is wrong: The render/discard scratch buffer is heap-allocated per quantum on the media-time hot path (advance_time 725, drain_tail_at_end 823, discard_ring_prefix 1273) — while the module's own D-1 discipline ("steady state allocates nothing on the drain path", 638-641) is applied to `outbox_scratch` three functions above. At 48 kHz stereo this is a ~8 KB alloc × ~47/s; a 6-channel 96 kHz lane churns ~2.2 MB/s.
Why it matters: Contradicts the file's stated allocation discipline on the path that runs most often; measurable churn, not a correctness bug.
What correct looks like: one reusable `Vec<f32>` field sized at open (channels known), like `outbox_scratch`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CliHost::drain_outputs` | 636-684 | REAL | — | Receipt stash + state_history + last_fatal set (659) + TracksChanged snapshot (zip of parallel slices — W12-verified engine invariant) + staleness filter; buffer reuse per D-1 |
| `CliHost::advance_time` | 686-750 | REAL | F-008 | Paused-clock guard correct (the frozen-clock spin analysis in the doc is sound); per-quantum alloc; bounded loops |
| `CliHost::audio_channels` | 752-765 | REAL | — | First audio fact with geometry; stereo fallback; consistent with the limb opened in open_session |
| `CliHost::audio_rate_hz` | 767-783 | REAL | — | 44 100 fallback; used by wall due-frames math (part1) |
| `CliHost::drain_tail_at_end` | 785-830 | REAL | — | Ended-state residue play-out; resume→render→pause; error breaks leave clock paused (829) — correct |
| `CliHost::surface_video_error` | 832-841 | REAL | F-001 | Sets last_fatal on present-path error while state stays Playing — feeds the poison; last-wins overwrite |
| `CliHost::next_request` | 845-850 | REAL | — | Trivial, correct |
| `CliHost::submit_and_await` | 852-866 | REAL | F-006 | Error relabeled SessionClosed; await bounded (64) |
| `CliHost::receipt_outcome` | 868-887 | REAL | — | Accepted/Completed/Rejected mapping; `let _ = session` ignored id is fine (echoed elsewhere) |
| `CliHost::open_session` | 889-1008 | REAL | F-001, F-004 | The MP4 open lane (baseline-proven); class-A Accepted even when a decode Fatal is imminent; audio-limb geometry gate; queue caps; the stale-fatal take at 998 is the poison site |
| `CliHost::teardown_after_fatal` | 1010-1018 | REAL | F-001 | Close only in Error → orphaned session in the Buffering case |
| `CliHost::teardown_session_scoped` | 1020-1033 | REAL | F-001, F-004 | Clears 8 halves; leaves `last_fatal` and `settings` |
| `CliHost::session_closed` | 1035-1039 | REAL | — | Trivial delegation |
| `CliHost::dispatch_seek` | 1041-1150 | REAL | F-005 | Bounded terminal wait; failed-at-accept correctly skips boundary duties (D4); chapter message lies about container; ms/µs→ns saturating conversions correct |
| `CliHost::await_receipt` | 1152-1170 | REAL | — | Bounded 64 pumps; linear match; typed Internal on expiry |
| `CliHost::seek_boundary` | 1172-1238 | REAL | — | Exact-prefix discard via consumer write-order bookkeeping; fast-completion clock compensation; typed re-anchor error |
| `CliHost::discard_ring_prefix` | 1240-1280 | REAL | F-008 | min(bounded prefix, ring content); returns actually-discarded; per-render alloc |
| `CliHost::media_info` | 1282-1314 | REAL | F-002, F-003 | seekable/is_live constants; title/metadata/chapters constants; not-ready refusal honest |
| `CliHost::duration` | 1316-1333 | REAL | F-003 | Answers Unknown for every mkv/mka/webm (span-end Missing) |
| `CliHost::current_time` | 1335-1363 | REAL | F-007 | position via audio-master clock or latched present (correct µs math); fraction clamped; buffered fabricated |
| `CliHost::setting_get` | 1365-1387 | REAL | F-004 | Schema meta + stored-or-default; reads the only consumer of the map |
| `CliHost::setting_set` | 1389-1409 | REAL | F-004 | Validate+store+echo; no behavioral consumer |

## 5. Test honesty
No `#[cfg(test)]` in this range (both test mods are part3, W13-A10; part3's 1849 asserts `last_fatal.is_none()` only for a backpressure scenario). Integration coverage touching this range: `apps/cli/tests/native_seek.rs`, `ttf_seek.rs`, `session_script.rs`, `pipeline_accounting.rs` (baseline green) — none covers open→fatal→close→reopen (F-001's sequence), and none asserts `media.get-info.seekable` against an unseekable source (F-002).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "After a fatal open failure the engine sits in Error … close it so the host is reusable" (doc 1010-1012) | host.rs itself | FALSE for the stale-fatal path | The poisoned open tears down in Buffering (no Close) and the previous fatal survives close — F-001 trace |
| "class-A open: accepted ≠ completed … an immediate open failure must surface in THIS response" (doc 993-996) | host.rs itself | PARTLY SHALLOW | Only fatals landing within the Opening pumps surface in-response; the yuv444p decode fatal lands after (baseline transcript) — by class-A design, but the doc's "immediate" is doing heavy lifting |
| "The flow's own typed failures … surface as the timing.seek-failed event with the old position (contract D4 — never a fake success, never a reopen)" (doc 1054-1058) | host.rs | REAL | dispatch_seek 1125-1131 + native_seek.rs baseline-green |
| seekable media info for TS/streams | plugin-api SeekSupport::None contract | FALSE | host.rs:1307 constant vs ts/mod.rs:1167 always-None — F-002 |
| "Session-local settings overrides" (doc 180) | host.rs field doc | FALSE | teardown never clears; no consumer applies them — F-004 |

## 7. Dead / duplicated / unreachable code
None dead in range. Note `teardown_after_fatal`'s Error-only Close guard makes its submit unreachable exactly in the case that needs it (F-001). `receipt_outcome`'s `let _ = session;` (882) is a deliberate ignore.

## 8. Dependencies & flags
Consumes workspace crates only (playback-engine, commands, media-clock, media-types, audio-output::null, settings-schema, toml via part3 converters). No feature gates inside this range; `RENDER_QUANTUM`/`HostDecoderProvider`/`CliFrameConsumer` from crate::pipeline_host. The null-device render-discard flush is a `[PROPOSAL]`-documented seam (1252-1257), honestly marked.

## 9. Open questions for the next agent
- W13-A02 (run.rs): the wave's known context says a 4:4:4 MP4 "HANGS forever". Every wait in 636-1414 is pump-bounded (≤64 pumps, typed errors on expiry) — the hang must live in run.rs's REPL/scripted loop or a blocking read; check the `play`/`wait` verbs' termination predicates against a session parked in `Error`/`Buffering`.
- W13-A10 (host.rs part3): `engine_info()`'s hard-coded decoder list (baseline INDEX standing defect) is yours; note it also under-reports h264/vp9/av1 which DO decode (mkv corpus TRUTHS + unsupported_path.rs:142-160).
- W13 demux_host owner: F-003's duration half (span-end Missing for mkv) is a MediaFacts design decision — severity-call it there; also consider carrying SeekSupport into MediaFacts so F-002's fix has a data path.
- REQUEST-ORCH-RUN (would prove F-001 end-to-end in the real binary): a scripted session `open <yuv444 sample> \n close \n open <yuv420 sample> \n close \n open <any>` against the baseline samples — expected per code trace: second open errors `h264.sw.unsupported`, third open errors "a session is already open", close errors "no session open".
