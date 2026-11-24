# AUDIT: apps/web/src/host.rs
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-24 |
| Lines / functions | 2651 / 80 production fns (+ 14 tests, 3 test helpers) |
| File verdict | REAL (native/headless paths executed) — the shipped wasm32 build is an honest decoder-less shell (see F-001) |
| Reachable from shipped entry point? | NOT from lumen-cli/desktop (not a dep of either — verified). YES from the web lane's own shipped artifact: `lumen_web_dispatch` (abi.rs:357) → `AbiWorker::dispatch` (abi.rs:180) → `WebHost::handle` (host.rs:984) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 5 S4: 3 |
| Commands run | `cargo test -p lumen-web --lib host::tests` → 14 passed/0 failed (0.89s); `--lib open::tests` → 5 passed; `--lib capabilities::tests` → 5 passed; `file target/wasm32-unknown-unknown/debug/lumen_web.wasm` → WebAssembly binary, 4,243,806 B; baseline cargo-test-workspace.txt:3443 (111 passed) |

## 1. One-paragraph verdict
This is a genuinely implemented, carefully bounded cooperative engine host — pump slices, typed refusals everywhere, real state-machine driving, real fixture decode on native — and its 14 in-file tests are honest (I ran them: 14/14 pass). But the headline truth for the WEB LANE is brutal: **the shipped wasm32 module decodes nothing**. `decoder_for` on the Wasm32 profile without the `builtin-codecs` feature is a constant refusal (F-001), so every `session.open` on the shipped 1.23 MiB module fails typed with `web.decoders_absent` — the module can boot, load blobs, probe AVI, and answer class-S queries, but cannot play a single file. The web open path wires exactly one container (AVI) and refuses all others named (open.rs:238-249). Video "presentation" is a mailbox + one `PresentFrame` message per pump with RGBA bytes and NO pacing against media time (F-004); the canvas draw itself lives outside this crate (page-side, card 7.I1 `[VERIFY]`). The honest-refusal posture is consistent and test-enforced — this is the rare file whose docs under-claim rather than over-claim — but "honest" does not make a 4 MiB debug / 1.23 MiB release artifact that cannot decode a single frame any less of a non-product.

## 2. What actually works (verified)
— `cargo test -p lumen-web --lib host::tests` → `test result: ok. 14 passed; 0 failed` (real fixture `generated/valid/avi_mjpeg_pcm.avi` through the real open path: pump-to-end, position≈duration within 5 ms, bounded ticks/flushes, pause freeze, boot/ladder/loss, autoplay park/resume, present-per-pump, typed refusals).
— `cargo test -p lumen-web --lib open::tests` → 5 passed (AVI-only refusal names `wav`; source grammar refusals typed).
— `cargo test -p lumen-web --lib capabilities::tests` → 5 passed (wasm32 snapshot claims empty decoders; blocked rows typed).
— `file target/wasm32-unknown-unknown/debug/lumen_web.wasm` → real wasm module, 4,243,806 B (the Wave-0 "4.2 MB debug" artifact). Baseline rc-smoke.txt addendum: wasm build `Finished dev profile ... in 11.45s`.

## 3. Findings

F-001 [S2]
Where: apps/web/src/host.rs:477-486 and 696-705
Code:
```rust
            TargetProfile::Wasm32 => {
                #[cfg(feature = "builtin-codecs")]
                {
                    builtin_decoder_for(self.profile, track, config, &self.facts, &self.probe)
                }
                #[cfg(not(feature = "builtin-codecs"))]
                {
                    Err(decoders_absent_error())
                }
            }
```
```rust
#[cfg(all(target_arch = "wasm32", not(feature = "builtin-codecs")))]
fn builtin_decoder_for(
```
What is wrong: The SHIPPED wasm32 build (no features — the build in README §"Building the module", in `ui/scripts/web-matrix-run.mjs:444`, and in the size-report "floor" row) refuses every decoder request: `decoders_absent_error()` (L417-427) regardless of track. `session.open` on any file therefore fails. The browser-matrix runner itself pre-writes the excuse (web-matrix-run.mjs:279-281: "open-play-pause-stop fails with the typed refusal web.decoders_absent … the designed v1 posture"). Why it matters: the web lane's only shipped artifact is a media engine that cannot play media. Not S0 only because every doc in the tree (README:56-62, capabilities.rs module docs, size-report) states the gap honestly and the refusal is typed, never a crash. What correct looks like: the `[PROPOSAL]` crate split (in-house mjpeg/pcm out of `codecs`) so the floor ships `builtin-codecs` in-house set at ~1.32 MiB. Confidence: HIGH — Verification: ran (capabilities test asserts empty wasm decoder list; wasm32_profile_decoder_seam_refuses_typed passed).

F-002 [S3]
Where: apps/web/src/host.rs:725-740
Code:
```rust
        decoders: capabilities::decoder_descriptors(profile)
            .into_iter()
            .map(|name| DecoderInfo {
                kind: if name == "mjpeg" {
                    commands::catalog::types::TrackKind::Video
                } else {
                    commands::catalog::types::TrackKind::Audio
                },
```
What is wrong: Every descriptor except the literal `"mjpeg"` is classified `Audio`. In `codec-vp9`/`codec-dnx` feature builds the descriptors include `vp9` and `dnxhd` (capabilities.rs:217-220) — both VIDEO codecs — so `system.get-engine-info` reports them as audio decoders. Why it matters: class-S snapshot lies about codec kind in exactly the tree-shake measurement builds the size report documents. What correct looks like: a kind table keyed by descriptor, or derive from the codec registry. Confidence: HIGH — Verification: read (cfg arithmetic verified: `codec-dnx` ⇒ `set.push("dnxhd")`, L219-220 of capabilities.rs; default/native builds are unaffected — empty / [mjpeg, pcm_s16le]).

F-003 [S3]
Where: apps/web/src/host.rs:1650-1662
Code:
```rust
        Ok(CommandResult::MediaInfo {
            info: MediaInfo {
                source: self.source.clone().ok_or_else(not_ready)?,
                container: Some(facts.container.clone()),
                title: None,
                duration: wire_duration(facts.duration_us()),
                seekable: true,
```
What is wrong: `media.get-info` hard-codes `seekable: true` while `playback.seek` answers `UnsupportedForHost` for every request (`owner_of`, L1837-1839: "the seek boundary's host duties … land with the H-pillar pump integration"). Why it matters: a UI honoring this metadata shows a seek bar whose every use errors — the snapshot contradicts the served surface. What correct looks like: `seekable: false` until the seek host duties land (the source IS seekable; the HOST is not). Confidence: HIGH — Verification: read + ran `unserved_surface_refuses_typed` (seek refuses typed, test passed).

F-004 [S3]
Where: apps/web/src/host.rs:1120-1131 and 1152-1157
Code:
```rust
        for _ in 0..ticks {
            self.fill_audio();
            for _ in 0..self.budgets.polls_per_tick {
                self.poll_engine();
            }
            events_flushed += self.flush_events();
        }
        self.present_one();
```
```rust
        if let Some(upload) = consumer.take_present() {
            let message = WorkerToHost::PresentFrame {
                width: upload.width,
                height: upload.height,
                pts_ns: upload.pts_ns,
```
What is wrong: One frame ships per pump message, taken latest-wins from the mailbox, with `pts_ns` as metadata only — nothing gates the present against the media clock. The JS glue re-arms at `setTimeout(0)` speed (worker.js:114), so decoded video is dumped at message rate (decode-ahead), not presented at frame times; A/V sync on the web lane does not exist in v1. Why it matters: any codec-carrying feature build would blast frames at macrotask speed while audio advances at the sample-counter rate. The module docs admit rVFC pacing is card 7.I1 — but the wire message carries a pts that nothing consumes, implying timing that is not implemented. What correct looks like: present scheduling keyed on pts vs clock (the CLI presenter's discipline), or dropping pts from the v1 message. Confidence: HIGH — Verification: read + ran `the_present_path_ships_one_frame_per_pump` (frames ship per pump, no timing assertions exist).

F-005 [S3]
Where: apps/web/src/host.rs:1227-1230
Code:
```rust
        // One scratch render buffer per fill (D-1): reused across the
        // slice's quanta; the tail render uses a prefix of it.
        let mut scratch = vec![0.0_f32; RENDER_QUANTUM * channels];
```
What is wrong: The render scratch (1470 × channels f32 = 11,760 B stereo) is allocated on EVERY `fill_audio` call — i.e. once per pump tick, up to `max_ticks_per_pump` (32) times per pump message — while the sibling buffer `outbox_scratch` gets the true D-1 take/reuse treatment (L818-820, L1296-1332). At browser re-arm rates this is continuous allocation churn in the audio hot path. Why it matters: the file's own discipline (D-1: "steady state allocates nothing") is implemented for the output drain but not the audio fill. What correct looks like: a scratch field on WebHost, cleared per fill. Confidence: HIGH — Verification: read (allocation site confirmed per-call, not per-host).

F-006 [S3]
Where: apps/web/src/host.rs:102-108 (imports) vs the whole file
Code:
```rust
use crate::audio::autoplay::{AutoplayGate, GateChange};
use crate::capabilities::{self, HostCapabilities, TargetProfile};
use crate::open::{self, MediaFacts, TrackFact, WebOpenOptions};
```
What is wrong: `WebHost` never touches `crate::io` — the entire 7.H4 fetch pillar (`io/fetch_port.rs` 381 loc, `io/seekability.rs` 448, `io/staged_source.rs` 1218, `io/mock_fetch.rs` 776 = 2,823 loc) is unwired from the host dispatch: `resolve_source` refuses http/https with "network streaming is card 7.H4 (fetch + Range seekability detection)" (open.rs:182-186), and `protocol.rs` carries no fetch messages. The e2e harness pins the non-wiring as expected behavior (harness.js:513: "http open must refuse (the staged fetch lane is not wired to open)"). Why it matters: "the fetch seam … landed (7.H4)" (README:71-73) is library-landing only — no shipped entry point reaches it; every byte of that pillar is UNREACHABLE from the wasm module's dispatch. What correct looks like: either wire StagedSource into open for http sources or mark the modules explicitly as future work. Confidence: HIGH — Verification: rg for `crate::io|FetchPort|StagedSource` in host.rs/abi.rs/open.rs → zero hits (lib.rs `pub mod io` only).

F-007 [S4]
Where: apps/web/src/host.rs:1682-1688
Code:
```rust
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
```
What is wrong: `media.get-current-time` reports the buffered range as the full 0..duration regardless of demux progress. Defensible for in-memory blobs (whole file preloaded), but it is a hard-coded claim, not an observed extent; no other source kind can even reach this host today. Confidence: MEDIUM — Verification: read.

F-008 [S4]
Where: apps/web/src/host.rs:814-815, 1304-1306
Code:
```rust
    /// The state-transition trail (tests + the lane evidence).
    state_history: Vec<(PlaybackState, PlaybackState)>,
```
What is wrong: `state_history` grows on every pipeline StateChanged for the session's life and is never drained (only cleared at teardown). Entries are tiny and transitions are bounded by user input, so this is hygiene, not a leak that matters. Confidence: HIGH — Verification: read (only writes: L1305; reads: accessor + tests).

F-009 [S4]
Where: apps/web/src/host.rs:716
Code:
```rust
        protocol_version: "unwired (control-protocol is a later card)".to_string(),
```
What is wrong: a class-S snapshot field carries a prose sentence instead of a version — honest, but stringly-typed and useless to a client that wants to branch on protocol. Confidence: HIGH — Verification: read.

## 4. Per-function table (every function; "cfg" = feature/target-gated)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SliceBudgets::Default | 164-174 | REAL | — | count-form budgets, sane |
| PumpClock::new/advance_ns/nanos | 187-204 | REAL | — | atomic counter, wasm-safe |
| PumpClock::now_ns (MonotonicSource) | 206-210 | REAL | — | trivial delegate |
| AudioLane::stage | 229-236 | REAL | — | pipeline→mix→ring, typed errors |
| AudioLane::buffered_frames | 239-241 | REAL | — | delegate |
| WebFrameConsumer::new | 288-296 | REAL | — | |
| WebFrameConsumer::counters | 300-302 | REAL | — | |
| WebFrameConsumer::buffered_frames | 306-311 | REAL | — | |
| WebFrameConsumer::take_present | 316-318 | REAL | — | latest-wins drain |
| WebFrameConsumer::present_pending | 322-324 | REAL | — | |
| WebFrameConsumer::present_stats | 329-331 | REAL | — | |
| lock_or_clone | 339-344 | REAL | — | poison recovery, single-threaded |
| FrameConsumer::accept | 347-411 | REAL | — | audio lane / video mailbox / subtitle count; div guarded `.max(1)` |
| decoders_absent_error (cfg) | 417-427 | REAL | F-001 | the shipped wasm refusal |
| WebDecoderProvider::new/attach_consumer | 448-461 | REAL | — | |
| DecoderProvider::decoder_for | 465-492 | REAL | F-001 | wasm32-default = constant refusal |
| DrainGatedDecoder::audio_backlog/configure/decode/flush (cfg) | 513-543 | REAL | — | drain waits on ring, sub-quantum residue released |
| builtin_decoder_for (cfg native/feature) | 574-690 | REAL | — | mjpeg+pcm always; per-codec rows gated |
| builtin_decoder_for (cfg wasm stub) | 697-705 | STUB | F-001 | by-design typed refusal |
| engine_info | 712-741 | REAL | F-002 | kind misclassification |
| command_listings | 746-757 | REAL | — | catalog as data |
| SERVED (const) | 762-775 | REAL | — | 12 ids |
| WebHost::new | 826-855 | REAL | — | |
| WebHost::profile/set_budgets/budgets | 859-872 | REAL | — | |
| WebHost::events | 877-888 | REAL | — | test evidence helper |
| WebHost::state_history | 892-894 | REAL | F-008 | |
| WebHost::counters/state/session_id/generation | 898-920 | REAL | — | |
| WebHost::position_us | 926-931 | REAL | — | ns→µs of sample-counter clock |
| WebHost::take_outbound/take_pending_frame | 935-944 | REAL | — | ABI pulls |
| WebHost::booted_capabilities | 951-958 | REAL | — | renderer truth patched in |
| WebHost::render_snapshot/render_backend/present_stats | 964-981 | REAL | — | |
| WebHost::handle | 984-1070 | REAL | — | backpressure bound (4096) + boot gate; Boot/LoadBlob/Command/Pump/SurfaceLost/AudioContext/Shutdown |
| WebHost::handle_control_error | 1075-1080 | REAL | — | |
| WebHost::shutdown | 1085-1107 | REAL | — | bounded close, flush, ack |
| WebHost::pump | 1120-1138 | REAL | F-004 | ticks capped, roles ordered |
| WebHost::present_one | 1145-1166 | REAL | F-004 | one per message, lane-gated |
| WebHost::observe_autoplay | 1171-1176 | REAL | — | once-per-cycle latch |
| WebHost::wants_audio | 1181-1188 | REAL | — | |
| WebHost::fill_audio | 1199-1255 | REAL | F-005 | silence clock for limb-less; end-tail play-out |
| WebHost::lane_exists/audio_channels | 1260-1281 | REAL | — | facts-driven |
| WebHost::poll_engine | 1288-1291 | REAL | — | |
| WebHost::drain_outputs | 1295-1333 | REAL | — | reused scratch, receipts+events+tracks+fatal |
| WebHost::flush_events | 1339-1353 | REAL | — | bounded per tick |
| WebHost::busy/present_pending/ring_backlog | 1360-1381 | REAL | — | re-arm truth |
| WebHost::dispatch | 1389-1398 | REAL | — | |
| WebHost::route | 1402-1437 | REAL | — | 12 served + typed unserved |
| WebHost::open_session | 1441-1554 | REAL | — | options refusals typed; settle polls; fatal teardown |
| WebHost::teardown_after_fatal | 1558-1563 | REAL | — | |
| WebHost::teardown_session_scoped | 1567-1577 | REAL | — | |
| WebHost::session_closed | 1580-1582 | REAL | — | |
| WebHost::submit_and_await | 1586-1602 | REAL | — | |
| WebHost::await_receipt | 1605-1625 | REAL | — | 64-poll bound, typed timeout |
| WebHost::receipt_outcome | 1628-1645 | REAL | — | |
| WebHost::media_info | 1650-1667 | REAL | F-003 | seekable:true |
| WebHost::current_time | 1670-1694 | REAL | F-007 | |
| WebHost::duration | 1697-1704 | REAL | — | |
| frames_to_ns | 1708-1714 | REAL | — | saturating, rate-0 guard, tested |
| open_null_lane | 1720-1757 | REAL | — | null device + audio_master clock; channels bounded by demuxer (MAX_PCM_CHANNELS=64) |
| not_ready/wire_duration | 1760-1776 | REAL | — | |
| enrich_track_info | 1781-1806 | REAL | — | strf fold |
| unserved/command_id_string/owner_of | 1810-1861 | REAL | — | typed refusals naming owners |
| rejection_wire | 1864-1923 | REAL | — | full reason mapping |
| session_error_wire | 1927-1953 | REAL | — | |
| tests (14) + helpers | 1955-2651 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pump_slices_play_the_fixture_to_its_end | YES | asserts state≠Opening, Ended, !busy, position within 5 ms of duration, video/audio counters > 0, trail crossed Playing — over the real fixture through the real open path |
| pump_ticks_are_capped_and_flushes_bounded | YES | pump(u32::MAX) must stay under max_ticks×events bound |
| audio_fill_is_bounded_per_tick | YES | position advance bounded by exact quantum math (multiply-first noted) |
| unserved_surface_refuses_typed | YES | exact code + details for seek/config/not-ready |
| snapshots_answer_over_the_open_fixture | YES | duration>0, fraction in 0..=1, 2 tracks w/ geometry, host=="web", mjpeg in decoders |
| pause_freezes_the_counter_clock | YES | position frozen across pumps after pause |
| wasm32_profile_decoder_seam_refuses_typed (cfg) | YES | asserts stage Decode + code `web.decoders_absent` |
| shutdown_closes_and_acknowledges | YES | ShutdownComplete queued, session none, later snapshots NotReady |
| commands_before_boot_refuse_typed | YES | exact control-error code |
| frames_to_ns_is_exact_and_saturating | YES | exact + u64::MAX saturation |
| the_boot_snapshot_reports_the_active_backend | YES | webgpu/webgl2/unavailable(...)/unprobed(...) over all four probe reports |
| the_present_path_ships_one_frame_per_pump | YES | ≤1 PresentFrame per pump, dims == container's, pts present, stats.refused==0, unprobed ⇒ 0 |
| surface_loss_falls_then_degrades_to_audio_only | YES | one transition per loss, stale ignored, floor presents, total loss ⇒ 0 presents + clock advances + counters keep counting |
| the_autoplay_gate_parks_freezes_and_resumes | YES | event exactly once, frozen position (0), duplicate reports don't re-fire, resume advances |
All 14 executed by me: `14 passed; 0 failed` (2026-11-24). No #[ignore], no cfg-gated skip in the default feature set (one cfg on `not(feature = "builtin-codecs")`, active by default).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the wasm32 module ships the engine core WITHOUT the builtin decoders" | apps/web/README.md:56-58 | REAL | decoder_for L477-486 + ran capabilities tests (empty list asserted) |
| "served command surface is the first vertical's playing subset (open/close/play/pause/toggle/stop + class-S + engine-info/list-commands)" | host.rs module docs:52-54 | REAL | SERVED L762-775; route L1402-1437 |
| "playback.seek answers the typed refusal naming the missing host duty" | host.rs module docs:54-57 | REAL | owner_of L1837-1839; test unserved_surface_refuses_typed passed |
| "everything here is exercised headless-native by this crate's tests" | host.rs module docs:65-66 | REAL | 14/14 ran, 111/111 in baseline:3443 |
| "Browser-runtime behavior … is [VERIFY]" | host.rs module docs:67-69 | REAL-UNPROVEN | no browser in sandbox; e2e harness exists (ui/e2e/web) but not run here |
| "Native builds dispatch the first vertical's real decoders (MJPEG + PCM)" | host.rs:429-432 | REAL | builtin_decoder_for L590-608; host tests decode real fixture |

## 7. Dead / duplicated / unreachable code
- `crate::io` (2,823 loc): unreachable from the host dispatch — see F-006. `pub mod io` in lib.rs is the only reference.
- wasm32 stub `builtin_decoder_for` (L696-705): intentionally a refusal — reachable on the shipped target, by design.
- `DrainGatedDecoder` + `builtin_decoder_for` real table: dead in the default wasm32 build (cfg'd out), live on native + feature builds.
- Duplication: `open_null_lane`/`enrich_track_info`/`rejection_wire`/`session_error_wire` are admitted mirrors of the CLI host's twins (module docs say so); drift risk between the two hosts is real but currently test-pinned.

## 8. Dependencies & flags
- Web lane artifact: cdylib+rlib (Cargo.toml:49-50); wasm32 codecs edge optional, `default-features = false` (Cargo.toml:89-90); NO default features ⇒ decoder-less floor. `builtin-codecs-external` pulls 8 codec lanes; `codec-av1` absent (rav1d not wasm32-buildable).
- Direct externals: serde + serde_json only (both already the command layer's family — commands/Cargo.toml:39-40); "zero new dependencies" (README:9) is fair with that caveat.
- `lumen-web` is NOT a dependency of lumen-cli, desktop, or desktop-tauri (rg exit 1) — the web lane ships as its own artifact.

## 9. Open questions for the next agent
- Does any feature-build wasm module actually decode in a real browser (7.I1 matrix)? The harness exists (ui/e2e/web) but matrix captures in-tree show the decoder-less failures; a codec-carrying browser run is UNVERIFIED here.
- Does the product UI (ui/) ever instantiate apps/web's worker.js in a shipped page? rg shows only the e2e harness does — the React app's canvas path is the DESKTOP frame channel (ui/src/client/frame-channel.ts, LUF1), not this lane. If true, the whole apps/web lane has no product entry point yet — worth a dedicated doc-claim pass on ui/README.
- size-report floor raw_bytes=1,294,311 (size-record.toml:53) was recorded on the w24a lane; not re-measured here (release build forbidden by load discipline) — REAL-UNPROVEN at audit time.
