# apps/cli/src/host.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 1579 (≈50 functions incl. conversions + 3 seam tests) |
| File verdict | **REAL** — session/pipeline/lifecycle half is real and executed; `engine_info()` is a static lie (F-001) |
| Reachable from shipped entry point | YES — `main` → `run::run` → `repl_mode`/`one_shot` → `lumen_dispatch` (dispatch.rs:62) → `host.open_session` (711) / `dispatch_seek` (880) / snapshots; chain verified end-to-end by the orchestrator's MP4 smoke run |
| Findings | F-001 [S1], F-002 [S3], F-003 [S3], F-004 [S3], F-005 [S4], F-006 [S4] |

## 1 One-paragraph verdict

The session/pipeline/lifecycle half of this file is the real engine host and it demonstrably works: it opens the h264+aac MP4 through the real demux/composition chain, pumps the engine, builds the null audio limb from generic track geometry, drives the native seek flow with real boundary duties, tears down gracefully, and answers class-S snapshots from live facts (live run: `info` → `container: mp4`, `#0 video h264 320x240`, `#1 audio aac 1ch 44100Hz`). The one outright fabrication is `engine_info()` at 1452-1482: a hand-written static table that answers `decoders: mjpeg, pcm_s16le` while the same build's provider wires 13 codec families including H264SwDecoder — a capability snapshot that contradicts the executed binary's own demonstrated behavior and is pinned-as-truth by a test. Secondary: `media_info()` hard-codes `seekable: true` with no consultation of the demuxer's actual seek support, `current_time()` fabricates the buffered-range field, and one stale AVI-specific error message survives on every container.

## 2 What actually works (verified)

- **Open path through this file**: orchestrator run (audit/runs/baseline/mp4-smoke-open-advance.txt): `ok: accepted session=1 gen=0` → `event: tracks 2 ready` → `TimingDurationKnown { duration: Timestamp(3000000) }` → `end-of-media` → `Playing → Ended` → `ok: time 3.041814s (100.0%)` → `EXIT=0`. That is `open_session` (711-829) doing: `open_source` → facts → `open_audio_session` (audio limb from the aac track's geometry pair) → `CliFrameConsumer` + `HostDecoderProvider` → `SessionConfig` (queues from product limits) → `engine.open` → pump-to-settled → fatal check.
- **Snapshots from live facts** (my live runs, 2026-11-16, shipped binary): `info` → `container: mp4`, `duration: 3.000000s`, `#0 video h264 320x240 (selected)`, `#1 audio aac 1ch 44100Hz (selected)` — the `enrich_track_info` fold (1262-1287) of the demux-harvested dimensions/geometry is real.
- **Graceful exit**: live `quit` on a Playing session → `Playing → Stopped → Idle` (graceful_exit via run.rs; host submits real Stop/Close receipts — `submit_and_await` 675-687, `await_receipt` 975-991).
- **Native seek flow machinery is real where the demuxer supports it**: `dispatch_seek` (880-971) submits `handle.seek`, checks `failed_at_accept`, runs `seek_boundary` (1022-1059) with the write-order exact-prefix arithmetic (`audio_stale_prefix`), and awaits the terminal. The AVI lane exercises it (apps/cli/tests/native_seek.rs in the 1001-pass suite). On MP4 the same machinery surfaces the demuxer's own typed gap honestly (my live run: `event: TimingSeekFailed { … "[demux] unsupported (plugin.demux.reanchor.unsupported)" … }`, old position preserved, playback continued to Ended) — class-A truth as documented at host.rs:64-74.
- **Drain gating + end-tail**: `Playing → Ended` at exactly 100.0% in the executed smoke (not 96.5% stranding) — `drain_tail_at_end` (616-651) rendering the residue is live.
- The three in-file seam tests (1523-1578) prove the generic audio-limb seam (vorbis mono limb, pcm limb, adts limb-less) through the REAL `open_session`.

## 3 Findings

### F-001 [S1] `engine_info()` is a static string table that under-reports the build's decoders by 11 families
Where: host.rs:1452-1482 (the `decoders`/`features`/`sessions` fields at 1458-1480)
```rust
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.software-compositor".to_string(),
        ],
        plugins: Vec::new(),
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
```
Wrong: this is the engine self-description command (`system.get-engine-info`), and every capability field is a hard-coded string/vec with no path to live state. Executed proof of the lie in both directions:
- I ran `./target/release/lumen engine-info` → `features: first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor` / `decoders: mjpeg, pcm_s16le` (matches the orchestrator's run).
- I ran the SAME binary's open path on `sample-h264-aac.mp4` → both limbs decode and present to Ended (see §2), and the provider's own unavailable-codec error (pipeline_host.rs:354-361) lists the landed set as "mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264".
So the command's answer ("this engine has 2 decoders, and its identity is avi-mjpeg-pcm") is contradicted by the same process's demonstrated decode of h264+aac in an mp4. The `active: false` on both rows is also dead prose — nothing computes activation. `sessions: Vec::new()` is structurally incapable of ever being populated (the function signature `pub fn engine_info() -> EngineInfo` takes no host/engine state at all), so the field is a permanent lie of shape "0 sessions" even mid-playback. `plugins: Vec::new()` is honest (no plugin registry exists).
Correct: derive the decoder list from the same source of truth the provider dispatches on (a shared registry constant consumed by both pipeline_host.rs and engine_info), the features row from the wired container registry, and either populate `sessions` from engine state or remove the field.
Aggravating: the doc comment at 1450 claims "class S, data-driven" — false; and apps/cli/tests/command_surface.rs:125-133 PINS the stale strings as "the first vertical's honest … decisions" (`assert!(out.contains("mjpeg, pcm_s16le"))`), so the test suite cements the lie as the expected answer.
Confidence: high. Verification: `sed -n '1450,1482p' apps/cli/src/host.rs` (verbatim above); live `engine-info` run; live open+decode run; pipeline_host.rs:354-361 contrast.

### F-002 [S3] `media_info()` hard-codes `seekable: true` for every container
Where: host.rs:1128
```rust
                seekable: true,
                is_live: false,
```
Wrong: `seekability` is a real, available truth — `SelectDemuxer::seekability()` forwards the inner demuxer's `SeekSupport` (demux_host.rs:596-604), and the conformance suite's own header records a "None-seek ts family" whose `--start-time` answers the typed `demux.ts.seek.unsupported` refusal. For that same ts file, `lumen info` would still print `seekable: yes` (my live mp4 run printed `seekable: yes`; the value is unconditional in the code). `is_live: false` is likewise unconditional, defensible today (no live sources) but uncomputed.
Correct: thread the demuxer's `SeekSupport` into `MediaFacts` at open time and report it; `is_live` can stay a constant until a live lane exists, with a comment saying so.
Confidence: high on the mp4/`yes` half (live); high on the code path (single unconditional literal, no facts field exists — verified `Grep seekable` in host.rs shows exactly one assignment).
Severity S3: the wrong answer is printed for exactly the families whose seek is known-broken (ts), i.e. where a user would rely on it.

### F-003 [S3] `current_time()` fabricates the buffered-range field
Where: host.rs:1172-1178
```rust
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
```
Wrong: the wire answer "N buffered range(s)" claims the entire media `[0, duration]` is buffered at all times, for every session, from the instant of open. There is no buffering-truth anywhere in the host (no ring/queue fill is consulted). Executed evidence: `ok: time 3.041814s (100.0%), 1 buffered range(s)` — printed while the pipeline had long drained. For a local file this is cosmetic; for the wired `http(s)` ranged lane (demux_host.rs:474-500) it is a fabricated cache report.
Correct: either answer an empty list until a real buffered-range model exists, or compute ranges from the demuxer/queue state.
Confidence: high — single unconditional synthesis; executed transcript shows the fabricated "1 buffered range(s)".

### F-004 [S3] Stale AVI-specific chapter refusal on every container
Where: host.rs:901-908
```rust
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotFound,
                    format!(
                        "no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"
                    ),
                ));
```
Wrong: verified live — `seek chapter 1` against `sample-h264-aac.mp4` answers "AVI v0 carries no chapter markers" for an MP4. The message names a container the command was not run on; MP4 files can genuinely carry chapter traks (the mp4 host itself records consuming `tref/chap` traks, containers/mp4.rs:68-72). The refusal's OUTCOME is honest (no chapter source is wired anywhere — mp4.rs:70-72 records the gap); the wording is a 2023-era leftover that now misinforms.
Correct: "no chapter {index}: no container family exposes chapter markers yet (the chapter card owns this)".
Confidence: high — live binary run + sed-verified quote.

### F-005 [S4] `media_info()` source fallback fabricates an empty path
Where: host.rs:1115-1119
```rust
            source: self.source.clone().unwrap_or_else(|| {
                commands::catalog::types::SourceSpec::File {
                    path: std::path::PathBuf::new(),
                }
            }),
```
`self.source` is set unconditionally in `open_session` (811) and only cleared by teardown, after which `facts` is also None and the function refuses earlier — the fallback is unreachable in practice. Defensive shape, S4 only.

### F-006 [S4] Queue caps mirrored as constants instead of product.toml
Where: host.rs:792-795 + demux_host.rs:71-73 — the `[PROPOSAL]` is in the source itself ("The values ride the product.toml mirror constants until the product-config load lands"). Honest, registered; S4.

## 4 Per-function table (session/pipeline/lifecycle half emphasized; every function listed)

| Function (line) | Role | Verdict |
|---|---|---|
| `CliHost::new` (202-223) | idle host; empty session halves | REAL |
| `Default` (193-197) | delegates to new | REAL |
| `set_event_throttle` (232-234) | inject deterministic monotonic source (tests) | REAL — used by harnesses |
| `set_present_tap` (245-250) | w31.1 desktop frame-tap stash/install | REAL — CLI never calls it; seam tests in pipeline_host |
| `settle` (260-270) | pump until not Opening/Buffering (bound AWAIT_PUMPS) | REAL — smoke shows open settling |
| `state` (274-276) | wire state | REAL |
| `session_id` (280-282) | live session id | REAL |
| `generation` (286-288) | live generation | REAL — `gen=1` after seek accept in live run |
| `events` (292-294) | observed wire events slice | REAL |
| `state_history` (298-300) | transition trail | REAL |
| `position_us` (317-331) | audio-master counter else presentation latch/anchor | REAL — F3 fix; 100.0% end position in smoke |
| `facts` (335-337) | harvested facts | REAL |
| `track_snapshot` (342-344) | last TracksChanged snapshot | REAL — live `tracks` output |
| `engine_stats` (348-350) | engine counters | REAL |
| `audio_lane_counters` (359-367) | audio-only lane counters | REAL (observability) |
| `headless_accounting` (374-397) | full frame accounting snapshot | REAL (AC evidence) |
| `screenshot_png` (401-407) | surface → PNG | REAL (2.D1 encoder; not exercised in my runs) |
| `pump_pipeline` (419-423) | poll + drain + surface error; NO time advance | REAL — determinism primitive |
| `pump` (429-433) | pipeline + time advance + tail | REAL — `advance 512` verb's tick |
| `pump_pipeline_until` (436-448) | bounded predicate pump | REAL |
| `pump_until` (453-461) | full-pump predicate | REAL |
| `pump_until_state` (464-466) | state target pump | REAL |
| `advance` (469-473) | N full pumps (the REPL verb) | REAL — executed smoke |
| `drain_outputs` (481-523) | receipts stash, events → wire, staleness filter, TracksChanged enrich | REAL |
| `advance_time` (542-589) | render quanta (tick-before-render), bounded scheduler ticks when limb-less; pause-frozen guard | REAL |
| `audio_channels` (593-604) | geometry lookup, stereo default | REAL — live `1ch` answer proves the aac geometry flowed |
| `drain_tail_at_end` (616-651) | residue play-out at Ended | REAL — 100.0% not 96.5% |
| `surface_video_error` (656-662) | present-path failure → last_fatal once | REAL |
| `next_request` (669-671) | id allocation | REAL |
| `submit_and_await` (675-687) | submit + bounded receipt await; not-ready typed | REAL |
| `receipt_outcome` (690-708) | receipt → Accepted/Completed/Rejected wire | REAL |
| `open_session` (711-829) | THE open: reject live-session, open_source, generic audio limb, consumer + provider, SessionConfig (container truth, queue plan, throttle), engine.open, settle, fatal check, teardown-after-fatal | REAL — executed smoke + 3 seam tests |
| `teardown_after_fatal` (834-839) | close from Error, clear halves | REAL |
| `teardown_session_scoped` (845-854) | drop session halves on settled close | REAL |
| `session_closed` (858-860) | dispatch's close hook | REAL |
| `dispatch_seek` (880-971) | wire target → engine target; submit; failed-at-accept check; boundary; bounded terminal await; internal error on no-terminal | REAL on AVI; MP4 lands the typed `plugin.demux.reanchor.unsupported` (demuxer gap, containers/mp4.rs:173-178) |
| `await_receipt` (975-991) | bounded receipt matching | REAL |
| `seek_boundary` (1022-1059) | exact-prefix discard + clock re-anchor compensation + consumer boundary | REAL (6.FIX-ogg-reanchor) |
| `discard_ring_prefix` (1079-1101) | render-discard the ring prefix | REAL |
| `media_info` (1104-1135) | class-S info from facts + enrichment | REAL with F-002 (seekable) and F-005 |
| `duration` (1138-1154) | class-S duration | REAL |
| `current_time` (1156-1184) | class-S time/percent/buffered | REAL with F-003 (buffered fabricated) |
| `setting_get` (1187-1208) | schema-backed read + meta | REAL — pinned by run.rs tests |
| `setting_set` (1211-1230) | schema validate + store | REAL |
| `session_state_as_wire` (1237-1239) | identity shim | REAL |
| `is_seek_terminal` (1245-1250) | seek terminal match | REAL |
| `track_info_wire_host` (1254-1256) | reuse engine's track mapping | REAL |
| `enrich_track_info` (1262-1287) | fold facts (dims/geometry) into wire track | REAL — live `320x240`, `1ch 44100Hz` |
| `rejection_wire` (1290-1354) | 11 rejection reasons → typed errors | REAL |
| `session_error_wire` (1357-1381) | session construction errors → typed | REAL |
| `default_value` (1386-1392) | schema default → wire (product.toml refs as markers) | REAL — honest `[PROPOSAL]` |
| `toml_to_setting_value` / `setting_value_to_toml` (1394-1432) | value bridges | REAL |
| `setting_type_name` (1434-1448) | type name | REAL |
| `engine_info` (1452-1482) | static self-description | **FALSE as a description of this build** — F-001 |
| `command_listings` (1486-1497) | catalog as data | REAL — `lumen commands` → "65 command(s)" pinned in run.rs test |
| seam tests (1523-1578) | audio-limb geometry through real open_session | REAL |

## 5 Test honesty

- In-file: the 3 `audio_limb_seam_tests` (1523-1578) drive the REAL `open_session` over vorbis/pcm/adts fixtures — honest, strong.
- apps/cli/tests/command_surface.rs:125-133 `engine_info_names_the_vertical` — DISHONEST PIN: it asserts the stale `first-vertical:avi-mjpeg-pcm` and `mjpeg, pcm_s16le` strings, i.e. it tests that the lie is reproduced byte-for-byte. This test must be rewritten alongside F-001 or it will fail the fix.
- apps/cli/tests/container_conformance.rs:426 `mp4_h264_family_lane` (in the 1001-pass suite) exercises this file's open/pump/teardown against the REAL binary on the same mp4 family — passes with `ok: time 1.532517s (100.0%)` and no `cli.codec_unavailable`.
- session_script.rs / native_seek.rs / pipeline_accounting.rs / ttf_seek.rs / sync_scenarios.rs drive the REPL against this host — in the 1001-pass suite.
- Nothing tests `media_info().seekable` or `current_time().buffered` against a None-seek family — F-002/F-003 are uncovered.

## 6 Doc claims

- docs/feature-matrix.toml:152 (container-mp4 notes): "2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane — mp4_h264_aac.mp4 plays to Ended (exit 0, apps/cli mp4_h264_family_lane)" — CONSISTENT (this file's open path is the engine-side half of that claim).
- docs/feature-matrix.toml:428 cites the same e2e as h264-decode evidence — CONSISTENT.
- lib.rs:39-42 describes this host's duties accurately (owns engine, pumps, correlates receipts, records events/state, answers class-S).
- host.rs module doc 26-74 (seek) matches the code (native flow, boundary duties, class-A truth) — the CONTRADICTING text is help.rs:57 (see run.rs audit F-001), not this file.

## 7 Dead/duplicated/unreachable

- `engine_info`'s `sessions`/`active` fields: dead-by-construction (no live path, never populated, `active: false` literal) — F-001.
- `media_info` source fallback (F-005): unreachable defensive arm.
- `pending_present_tap` (161, 766-768): live seam, exercised by desktop not CLI — intentional, not dead.
- No duplicated logic found: track mapping reuses `playback_engine::track_info_wire` (1254-1256) rather than re-implementing.

## 8 Dependencies & flags

Imports the real pipeline crates (audio_output::null, media_clock, playback_engine, media_types, commands) plus the two host modules. Nothing here gates codecs — the decoder set enters through `pipeline_host::HostDecoderProvider`, whose h264 lane compiles under the shipped default feature `codec-h264-sw` (crates/codecs/Cargo.toml:47-53, `default = [… "codec-h264-sw" …]`), and apps/cli/Cargo.toml consumes codecs with default features. The one static-vec function (engine_info) is exactly the place a feature-aware list should have been generated from.

## 9 Open questions

- Is any consumer branching on `engine-info`'s decoder list (e.g. the desktop shell or scripts)? In-tree grep shows only run.rs printing it and command_surface.rs pinning it — no logic branches on the values. External scripts UNVERIFIED.
- Would threading `SeekSupport` into `MediaFacts` break the conformance pins that currently expect `seekable: yes` (unverified — I did not grep every assertion on "seekable"); the mp4 lane would stay `yes` (Fast{indexed}).
