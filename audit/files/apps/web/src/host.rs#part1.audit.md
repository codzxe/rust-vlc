# AUDIT: apps/web/src/host.rs#part1 (lines 1–662)
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-21 |
| Lines / functions | 662 (of 2651; chunk boundary pinned from audit/waves/wave-00/assignments.tsv:718) / 34 items |
| File verdict | UNREACHABLE (real, faithful CLI mirror, native-test-exercised; no shipped entry point links it) |
| Reachable from shipped entry point? | NO. Chain that WOULD drive it: worker.js → wasm exports (abi.rs:132 `WebHost::new(build_profile())`) → host.rs. But nothing ships that chain: `rg lumen-web` over ui/src, apps/desktop, apps/desktop-tauri, .github/workflows → no consumer (evidence recorded in audit/files/apps/web/Cargo.toml.audit.md F-001: UI transport picks Tauri/Demo never a wasm worker; browser harness excluded from CI via playwright.config.ts testIgnore `**/web/**`). `lumen` CLI and `lumen-tauri` do not depend on lumen-web. |
| Findings | S0:0 S1:0 S2:0 S3:3 S4:1 |
| Commands run | Read host.rs 1–240/241–480/481–665/666–725/1718–1782/1990–2070/2225–2280; rg `WebHost|WebDecoderProvider|WebFrameConsumer|AudioLane|PumpClock|SliceBudgets|RENDER_QUANTUM|RING_FRAMES|AWAIT_POLLS` over apps/web (use sites mapped); rg `AudioPipeline` (users: cli/pipeline_host.rs:660 twin confirmed); sed pipeline_host.rs 630–690 (stage twin); rg `pub struct AudioFrame` (samples is PCM **bytes** → frame math correct); sed time.rs 715–760 (`Stamp::map_timestamp` → Option<U>, flatten correct); rg `map_timestamp` frame.rs:202 pts: Stamp; rg `store|refused|displaced` present.rs (refusals counted in stats); rg `write_bytes|buffered_frames|ring_full` audio-output/null.rs (typed ring_full); rg `1470|131_072|AWAIT_PUMPS` apps/cli (all three constants are real mirrors); rg `builtin-codecs|codec-` cfg sites in apps/web; rg `decoder_for` outside host.rs (zero — no dispatch tests); sed capabilities.rs 358–395 (descriptor-list test, not dispatch). |

## 1. One-paragraph verdict
Part 1 of the web engine host is genuine, disciplined engineering: the constants, the pump clock, the audio lane, the frame consumer, and the decoder provider are faithful, line-checked mirrors of the shipped CLI host (`apps/cli/src/pipeline_host.rs:92,118,508,658-666`), with typed refusals everywhere the web target lacks a capability. It is also dead weight in the product sense: the crate's only consumers are its own tests and a wasm module that no shipped lane builds or loads (parity web=0/252, baseline #9), and the *shipped* wasm32 configuration is decoder-less by design — `decoder_for` returns `web.decoders_absent` for every codec (lines 477–486), so the module this code exists for can demux but never decode. The findings below are evidence-inflation and swallowed-error issues, not breakage.

## 2. What actually works (verified)
- Constants are real mirrors: `RENDER_QUANTUM=1470` = apps/cli/src/pipeline_host.rs:92; `RING_FRAMES=131_072` = pipeline_host.rs:118; `AWAIT_POLLS=64` = AWAIT_PUMPS apps/cli/src/host.rs:103 (rg verified).
- `AudioLane::stage` (229–236) is a verbatim mirror of the CLI's `AudioLimb::stage` (pipeline_host.rs:658–666, sed compared). `AudioFrame.samples` is PCM bytes (media-types/src/frame.rs:196–200), so the `frames = bytes / (channels × bytes_per_sample)` math in `accept` (362–369) is correct.
- The native end-to-end lane is genuinely tested: `pump_slices_play_the_fixture_to_its_end` (host.rs:2024, part4) opens the real AVI MJPEG+PCM fixture through the real open path and asserts Ended + position≈duration — this exercises part1's `builtin_decoder_for` mjpeg/pcm rows and `DrainGatedDecoder`. Baseline #3 (`cargo test --workspace`) compiled and ran lumen-web tests (test-workspace.log.txt:21); worklog.md:749 records 111 passed for `-p lumen-web`.
- The wasm32 module builds: baseline #20 rc-smoke check 6 `cargo build -p lumen-web --target wasm32-unknown-unknown --locked` → PASS (rc-smoke-engine-only.log.txt:25–62).
- `PresentMailbox::store` refusals are counted, not lost: present.rs:356–382 increments `refused`/`displaced` (rg verified).

## 3. Findings

### F-001 [S3] — "Everything here is exercised headless-native by this crate's tests": false for the codec-* dispatch rows
Where: apps/web/src/host.rs:63-69 (claim) vs 613-665 (unexercised rows)
Code:
```
//! Everything here is exercised headless-native by this crate's tests
//! (the SAME message paths the JS glue drives, through
//! [`crate::abi`]). Browser-runtime behavior (worker instantiation,
//! `postMessage`, `setTimeout` re-arming) is `[VERIFY]` — no browser
//! exists in this sandbox.
```
and the rows the claim covers:
```
        #[cfg(feature = "codec-vorbis")]
        "vorbis" => {
            let mut decoder = codecs::audio::vorbis::VorbisDecoder::new();
```
What is wrong: No test in this crate dispatches any of the codec-* rows (adpcm_ 613–618, flac 622–627, mp3 628–638, vorbis 639–644, opus 645–650, aac 654–659, alac 660–665) in ANY feature combination. The only `decoder_for` test is `wasm32_profile_decoder_seam_refuses_typed` (2243), which is `#[cfg(not(feature = "builtin-codecs"))]` and asserts the refusal only. `rg decoder_for` over apps/web outside host.rs: zero matches — there are no "decoder_for dispatch tests", though the comment at 2241 (part4) claims exactly that for the with-codecs contract. Under `--all-features` (baseline #4) the rows compile but nothing executes them.
Why it matters: The module header sells the file as fully test-proven; in the only configurations where two thirds of `builtin_decoder_for`'s body exists, that body has zero execution evidence. A broken dispatch row (wrong constructor, wrong gating) would pass every gate the repo runs.
What correct looks like: cfg-gated dispatch tests (e.g. `#[cfg(all(feature = "codec-flac", not(target_arch = "wasm32")))]` asserting `decoder_for("flac")` returns a configured decoder), or delete the "Everything here is exercised" sentence and name the exercised subset.
Confidence: HIGH — Verification: read + rg (no dispatch tests exist).

### F-002 [S3] — Video-present refusals are swallowed and never surfaced on the wire
Where: apps/web/src/host.rs:401
Code:
```
                // never fail decode.
                let _ = guard.mailbox.store(&video);
                Ok(())
```
What is wrong: `store` returns `Result<(), UploadError>`; oversize/unsupported-format refusals are discarded here. They are counted in `PresentStats.refused` (present.rs:356–382) and `WebHost::present_stats()` exists (host.rs:977, part2), but `rg present_stats` shows its only caller outside the definition is a test (2445) — no pump report, no protocol message, no ABI export carries it. The operator of a future canvas lane gets a frozen picture with zero typed signal.
Why it matters: The checklist's swallowed-error class: a real media file whose frames exceed the mailbox bounds or carry an unsupported format degrades silently. The doc comment defends "presentation bounds never fail decode" — fine — but nothing re-surfaces the refusal anywhere a human or the page can see.
What correct looks like: Fold `PresentStats.refused` deltas into the pump's `PumpReport` (or a class-S snapshot), so the degrade is observable outside unit tests.
Confidence: HIGH — Verification: read + rg callers.

### F-003 [S3] — Single audio lane hardcodes pipeline track 0; multi-audio-track media aliases or mis-stages
Where: apps/web/src/host.rs:229-231
Code:
```
    fn stage(&mut self, frame: &AudioFrame) -> Result<(), EngineError> {
        self.pipeline.process(0, frame)?;
        self.pipeline.mix()?;
```
What is wrong: Every audio frame from every track stages into pipeline slot 0. The lane is built once per session with `PipelineSpec { tracks: 1 }` (host.rs:1744–1752, part4) from the first audio fact's geometry. A valid AVI with two audio tracks: if geometries match, both tracks' frames interleave into the one staged slot (mixed/aliased audio); if they differ, the second track's frames fail `audio.channel_count_mismatch`/`rate_mismatch` (audio-engine/pipeline.rs:284–294) and surface as a decode error on a playable file. Nothing in part1 or the open path refuses multi-audio-track media up front.
Why it matters: Wrong behavior on valid input; blast radius limited to multi-audio-track containers on a v1 web host that (today) ships decoder-less and unreachable — hence S3, not S2.
What correct looks like: Refuse `web.multi_audio_track_unsupported` at open (typed, like the CLI's own v1 posture) or build one pipeline slot per audio limb.
Confidence: MEDIUM — Verification: read + traced pipeline checks; the engine's per-track limb wiring is part3/part4 territory (cross-part question logged below).

### F-004 [S4] — `lock_or_clone` never clones
Where: apps/web/src/host.rs:339-344
Code:
```
fn lock_or_clone(state: &Mutex<ConsumerState>) -> std::sync::MutexGuard<'_, ConsumerState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
```
What is wrong: The name promises a clone fallback; the body recovers poisoning by taking the guard. No clone exists or is needed (the doc comment at 334–338 describes the real behavior).
Why it matters: Naming that lies (checklist §architecture); a future reader will hunt for the clone path.
What correct looks like: `lock_or_recover` / `lock_unpoisoned`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-69 | REAL | — | Honest about count-form deviation, null-device clock, [VERIFY] browser gap; F-001's "everything exercised" sentence is the one inflation. |
| consts RENDER_QUANTUM / RING_FRAMES / AWAIT_POLLS | 110-114 / 116-120 / 128-126 | REAL | — | Exact mirrors of CLI pipeline_host.rs:92/118 and host.rs:103 (rg verified). Used at 536, 1090, 1229–1247, 1609, 1732, 2102. |
| SliceBudgets (struct) | 147-162 | REAL | — | Count-form bounds; doc math (16×1470=23520 fr ≈ 533 ms @44.1 kHz) checks out. |
| Default for SliceBudgets | 164-174 | REAL | — | 16/4/64/32/8 matches doc text at 137–143. |
| PumpClock (struct) | 182-185 | REAL | — | AtomicU64; no overflow risk at ns scale. |
| PumpClock::new | 189-192 | REAL | — | Trivial, correct. |
| PumpClock::advance_ns | 195-197 | REAL | — | Release store; single-threaded worker anyway. |
| PumpClock::nanos | 201-203 | REAL | — | Acquire load. |
| MonotonicSource::now_ns | 206-210 | REAL | — | Delegates. |
| AudioLane (struct) | 218-221 | REAL | — | Private fields; constructed only at 1752 (part4). |
| AudioLane::stage | 229-236 | REAL | F-003 | Verbatim CLI mirror (pipeline_host.rs:658-666); hardcoded track 0. |
| AudioLane::buffered_frames | 239-241 | REAL | — | Delegates to sink. |
| ConsumerCounters (struct) | 247-264 | REAL | — | Plain counters + Option latch. |
| ConsumerState (struct) | 266-273 | REAL | — | lane + mailbox + counters. |
| WebFrameConsumer (struct) | 280-282 | REAL | — | Mutex state; Send+Sync for the trait bound. |
| WebFrameConsumer::new | 288-296 | REAL | — | — |
| WebFrameConsumer::counters | 300-302 | REAL | — | Used at 901 (part2), tests 2052/2555/2576. |
| WebFrameConsumer::buffered_frames | 306-311 | REAL | — | Used by drain-gate probe (516). |
| WebFrameConsumer::take_present | 316-318 | REAL | — | Used by pump at 1152 (part2/3). |
| WebFrameConsumer::present_pending | 322-324 | REAL | — | Used at 1366/1375 (busy truth). |
| WebFrameConsumer::present_stats | 329-331 | REAL | F-002 | Not wired to any wire surface (only test 2445). |
| lock_or_clone | 339-344 | REAL | F-004 | Poison recovery; name lies. |
| FrameConsumer::accept | 347-411 | REAL | F-002/F-003 | Audio path math verified against SharedBytes; lane-absent refusal typed (356–360); video latch via map_timestamp().flatten() correct (time.rs:742); `_track` ignored (single-lane v1). |
| decoders_absent_error | 418-427 | REAL | — | cfg(not(builtin-codecs)); on native reachable only from the cfg-gated test; on wasm32 default it IS the shipped answer for every codec. |
| WebDecoderProvider (struct) | 439-443 | REAL | — | — |
| WebDecoderProvider::new | 448-454 | REAL | — | probe starts None; open path must call attach_consumer (verified in open path, part4/tests). |
| WebDecoderProvider::attach_consumer | 459-461 | REAL | — | — |
| DecoderProvider::decoder_for | 465-492 | REAL | — | Wasm32 arm: builtin-codecs → table, else typed refusal; Native arm → table. cfg matrix sound (baseline builds green under both feature sets). |
| DrainGatedDecoder (struct + audio_backlog) | 506-517 | REAL | — | Mirror of pipeline_host.rs:508 gate. |
| DrainGatedDecoder::configure | 522-524 | REAL | — | Pure delegation. |
| DrainGatedDecoder::decode | 526-532 | REAL | — | Pure delegation; PCM decode never self-reports Drained, so un-gated decode is right. |
| DrainGatedDecoder::flush | 534-543 | REAL | — | `Drained if backlog >= RENDER_QUANTUM → NeedMoreInput`; residue < 1470 releases (doc 494–499); consistent with pump render logic at 1237–1247 (part3). Livelock impossible only because the pump keeps rendering at Ended — cross-part, see §9. |
| builtin_decoder_for (+ inner gated) | 574-662 (continues 663-690 in part2) | REAL | F-001 | mjpeg + pcm_ rows exercised by the fixture test; codec-* rows zero execution evidence; chunk boundary cuts the alac arm — vp9/dnxhd/catch-all (663–690) audited by A05. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pump_slices_play_the_fixture_to_its_end (2024, part4) | YES | Real fixture through real open path; asserts Ended, position≈duration, video_frames>0, audio_frames>0, Playing in history — mutation of accept()/stage/decoder rows breaks at least one assertion. |
| wasm32_profile_decoder_seam_refuses_typed (2243, part4) | YES (in default features) | Constructs Wasm32 profile natively, asserts the typed `web.decoders_absent`; cfg-gated OFF under --all-features, so the with-codecs seam is untested (F-001). |
| codec-* dispatch rows | NO test exists | F-001. |
| capabilities decoder_claims_match_the_profile (capabilities.rs:358) | Only for descriptor STRINGS | Tests the capability list, not dispatch; does not exercise host.rs code. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything here is exercised headless-native by this crate's tests" | host.rs:63-65 | FALSE (for codec-* rows) | F-001; no dispatch tests exist (rg). |
| Count-form slice is an honest deviation from doc 09 §3.2 deadlines | host.rs:22-35 | REAL | SliceBudgets implements counts; `std::time::Instant` on wasm32-unknown-unknown indeed has no monotonic source (std stub); documented mapping at 134–146. |
| "The served command surface is the first vertical's playing subset (open/close/play/pause/toggle/stop …)" | host.rs:50-54 | REAL-UNPROVEN (native only) | Plays in native tests (fixture test); the SHIPPED wasm32 module is decoder-less (apps/web/Cargo.toml "The SHIPPED module enables none of them"), so on the target this file exists for, decode-bearing opens refuse `web.decoders_absent`. |
| Shipped wasm32 module ships no builtin decoders ("the honest capability snapshot") | host.rs:429-432, Cargo.toml | REAL | decoder_for 477–486 + capabilities.rs blocked row (tests at capabilities.rs:383–390). |
| RENDER_QUANTUM/RING_FRAMES/AWAIT_POLLS "the CLI host's own constant, mirrored" | host.rs:110-126 | REAL | rg: pipeline_host.rs:92/118, cli host.rs:103. |

## 7. Dead / duplicated / unreachable code
- Whole crate is product-unreachable (see header). Within the file: `decoders_absent_error` is dead on native production paths (reachable only via the cfg-gated test constructing a Wasm32 profile); the wasm32 stub `builtin_decoder_for` (696–705, part2 range) exists to keep the Native arm compiling on decoder-less wasm32 — unreachable per `build_profile()`'s cfg. Duplication: `builtin_decoder_for` duplicates the CLI host's decoder table rows by design (mirrored v1 twin) — acceptable, but every fix must be applied twice (drift risk, noted for the wave summary).

## 8. Dependencies & flags
- Target-gated `codecs` edge: native unconditional, wasm32 optional `default-features = false` behind `builtin-codecs`/`codec-*` (apps/web/Cargo.toml, verified). Shipped wasm32 config enables none → decoder-less module is the shipped truth.
- Feature gates in this range: `#[cfg(feature = "builtin-codecs")]` (adpcm row 613, wasm32 arm 478), `codec-flac/mp3/vorbis/opus/aac/alac` rows — none enabled by any CI/shipping lane for wasm (the tree-shake measurement builds are manual, docs/web/size-report.md).

## 9. Open questions for the next agent
- A05 (part2): does anything surface `PresentStats` on the wire (F-002)? And audit the vp9/dnxhd/catch-all tail of `builtin_decoder_for` (663–690).
- A06/A03 (part3/4): confirm the pump renders the end-tail residue after `Ended` (the drain-gate's release depends on it — else `flush` spins NeedMoreInput forever); confirm the engine never wires two audio limbs into one consumer (F-003's aliasing premise).
- Wave gate: if the product ever ships the web lane, the decoder-less wasm module means the web target cannot play ANY media — parity web=0/252 (baseline #9) is the honest external reflection of this.
