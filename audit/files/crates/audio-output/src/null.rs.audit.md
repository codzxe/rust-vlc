# AUDIT: crates/audio-output/src/null.rs
| Field | Value |
|---|---|
| Auditor | W08-A01 |
| Date | 2026-11-21 |
| Lines / functions | 748 / 35 (+8 tests, 2 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open <media-with-audio>` → apps/cli/src/host.rs:919-927 → pipeline_host::open_audio_session → apps/cli/src/pipeline_host.rs:1598-1599 `NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })` → NullSink::write (producer) + NullDevice::render driven by the CLI pump loop apps/cli/src/host.rs:613-617/726/824/1274. Same shape in apps/web/src/host.rs:1732. `lumen engine-info` ships it as feature `audio.null-device` — baseline INDEX.md.) |
| Findings | S0: 1 S1: 1 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "NullBackend|NullSink|NullDevice|NullConfig|open_stream|take_device"` (repo-wide: callers found only in apps/cli, apps/web, audio-output tests — none elsewhere); `rg -n "CpalBackend|cpal::|audio_output::cpal"` (cpal seam has zero production callers); `rg -n "builtin_backends"` (definition only — zero callers); `rg -n "audio_geometry"` (geometry sources: containers/*.rs); `sed -n` verbatim verification of every quote below; baseline cites: audit/runs/baseline/test-workspace.log.txt:155-236, INDEX.md. |

## 1. One-paragraph verdict
The null backend is real, tested, reachable code that does exactly what it says: it moves decoded f32 frames through a genuine wait-free SPSC ring, counts samples for the audio-master clock, records underruns, and then **throws every sample away** (or, in test mode, captures them into a Vec). The mechanics (ring round-trip, backpressure, pause-freeze, typed errors) are proven by 8 unit tests + 13 integration tests in the baseline run. But the hostile question's answer is: **a user never hears sound, in any shipped build, by design** — this file is the terminal sink of every shipped audio path, the only OS-audio seam (cpal) is a compile-time-closed typed refusal, and the "device thread" is the CLI's own loop rendering into a scratch `Vec` that goes out of scope. The file is honest about being null; the product-level consequence (a media player with no audible output, advertised via `engine-info` feature string `audio.null-device`) is an S0 blocker, plus one genuine S1: a crafted file's channel count flows unbounded into a 131,072-frame ring allocation.

## 2. What actually works (verified)
- Ring round-trip, capture bytes, backpressure FIFO integrity, pause freeze (NaN untouched), shared-counter clock, typed error codes, latency math — verified by `audit/runs/baseline/test-workspace.log.txt:164-171` (`null::tests::*` 8/8 `ok`).
- Zero-allocation write path after warm-up — verified by `test-workspace.log.txt:230-234` (`null_sink_write_path_is_zero_allocation_after_warm_up ... ok`).
- Callback op-budget and clock truth over this sink — `test-workspace.log.txt:194-209` (callback_budget 4/4 ok, clock_truth 1/1 ok), `pcm_pipeline.rs` 4/4 ok, `sine_sink_hash.rs` 3/3 ok.
- Reachability of the sink from the shipped CLI — verified by call chain in the table above (rg callers + read of host.rs/pipeline_host.rs).

## 3. Findings

### F-001 [S0] — Every shipped audio path ends in this discard sink: no build of this "media player" can produce sound
Where: null.rs:346-348 (anchor) + crates/audio-output/src/cpal.rs:43,98-107 + apps/cli/src/pipeline_host.rs:1598-1599 + apps/cli/src/host.rs:613-617 + apps/desktop-tauri/src (no audio wiring)
Code (null.rs:346-348):
```rust
/// The null backend: enumerates one virtual device, opens sessions
/// that discard or capture audio, and never touches a real OS
/// device. CI-safe by construction.
```
Code (cpal.rs:43 + 98-107):
```rust
pub const AUDIO_ADR005_ADMITTED: bool = false;
...
pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, BackendNotAdmitted> {
    if !AUDIO_ADR005_ADMITTED {
        // The refusal arm (the flip point): delete together with the
        // gate and the marker when the admission change lands.
        return Err(BackendNotAdmitted::CPAL);
    }
```
Code (apps/cli/src/host.rs:613-617 — the shipped "device", a discarded scratch buffer):
```rust
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
                        guard.note_video_error(error);
                        break;
                    }
```
What is wrong: the user-visible core (audible playback) is absent, not broken en route. The shipped CLI and web hosts construct exactly one backend — `NullBackend` (pipeline_host.rs:1598-1599, web host.rs:1732); `cpal_backend()` can only return `Err(BackendNotAdmitted)` behind a compile-time `assert!(!AUDIO_ADR005_ADMITTED)` gate; `UnsupportedBackend` is a typed refusal stub; `apps/desktop-tauri/src` contains zero `audio_output` usage (rg: no matches). The real-time callback the architecture documents (doc 01 §7 "reach the audio-output backend's real-time callback") is the CLI's own synchronous loop, and its output buffer is dropped on the floor.
Why it matters: Wave 7 proved 9+ live audio decoders feeding this sink. `lumen engine-info` lists `audio.null-device` as a shipped feature (baseline INDEX.md) — technically true, but for a VLC reimplementation "audio output = discard" means every decoded sample is compute-for-nothing. Baseline standing defect already records it: "Audio 'plays' only into the null device; no sound output path exists" (INDEX.md §Correction v2); sample-aac.mp4 "Playing … audible output: none".
What correct looks like: at least one admitted OS backend (cpal or direct) behind `AudioBackend`, wired into `builtin_backends()` and selected by the hosts; until then, `engine-info`'s feature list should say "audio: none (null discard only)", not present a feature string.
Confidence: HIGH — Verification: read (full chain traced: callers rg'd, cpal gate read, tauri rg empty) + baseline INDEX.md cites. Docs are *not* fraudulent here (feature-matrix row `platform-aout-cpal` honestly says "partial-seam, not done … no real-device evidence exists"), so the S0 is BLOCKER (user-visible core), not doc-fraud.

### F-002 [S1] — File-controlled channel count → ~34 GB ring allocation → allocation-failure abort at `lumen open`
Where: null.rs:77-85 (anchor: `Shared::open` validates only `channels == 0`, no upper bound) + apps/cli/src/containers/mp4.rs:386-389 + crates/audio-output/src/ring.rs:74-78 + apps/cli/src/pipeline_host.rs:118,1598-1599
Code (null.rs:77-85):
```rust
        if format.sample != SampleFormat::F32
            || format.layout != AudioLayout::Interleaved
            || format.channels == 0
            || format.rate == 0
        {
            return Err(error::format_unsupported(format));
        }
        Ok(Shared {
            ring: SpscRing::new(format.channels, buffer_frames)?,
```
Code (containers/mp4.rs:386-389 — the only validation of a file's channel count on the MP4 open path):
```rust
fn audio_geometry(entry: &[u8]) -> Option<(u32, u32)> {
    let channels = u16be(entry, 16)?;
    let rate = u16be(entry, 24)?;
    (channels > 0 && rate > 0).then_some((u32::from(channels), u32::from(rate)))
```
Code (ring.rs:74-78 — the allocation, `checked_mul` guards usize overflow, not memory):
```rust
        let slots = buffer_frames
            .checked_mul(channels as usize)
            .ok_or_else(|| error::unsupported("audio.ring_shape_invalid"))?;
        let mut storage = Vec::with_capacity(slots);
        storage.resize_with(slots, || AtomicU32::new(0));
```
What is wrong: the open path builds the session from container *facts* before any decode: `lumen open crafted.mp4` → host.rs:919-927 reads the stsd `channelcount` (u16, only `> 0` checked) → `open_audio_session(channels, …)` → `NullBackend::open_stream(format, RING_FRAMES=131_072, …)` → `SpscRing::new(65_535, 131_072)` → `Vec::with_capacity(8_589_803_520 × AtomicU32)` ≈ 34.4 GB, then `resize_with` commits every page. On typical hardware the allocation fails → `handle_alloc_error` → process abort; on overcommit it OOM-kills. WAV (`MAX_CHANNELS=64`, wav/format.rs:41) and AIFF (chunks.rs:262 `channels > 64`) cap; **MP4 does not** (MKV's `channels: Option<u64>` path unverified here — next agent).
Why it matters: a crafted ~100-byte stsd field turns `lumen open` into a crash/OOM DoS with zero decoder involvement; this is the classic "unbounded allocation from file-controlled counts" the checklist names.
What correct looks like: an upper bound on channels at the fact boundary (e.g. ≤ 64 like WAV/AIFF) or a byte-budget check in `Shared::open`/`SpscRing::new` (typed `audio.ring_shape_invalid` when `slots * 4` exceeds a sane budget, before `with_capacity`).
Confidence: HIGH on the chain (every hop read and quoted; arithmetic checked) — the abort itself UNVERIFIED at runtime (builds forbidden). REQUEST-ORCH-RUN: `printf` a stsd channelcount=0xffff MP4 via the existing sample tooling and run `target/release/lumen open` on it (expect abort/OOM, exit ≠ 0).

### F-003 [S3] — Capture buffer grows without bound for the session lifetime
Where: null.rs:307-311
Code:
```rust
        if let Some(capture) = self.capture.as_mut() {
            for sample in out.iter() {
                capture.extend_from_slice(&sample.to_le_bytes());
            }
        }
```
What is wrong: with `NullConfig { capture: true }`, every rendered sample (data *and* underrun silence) is appended forever; only `take_capture()` drains it. A driven 48 kHz stereo session accumulates ~384 kB/s ≈ 1.35 GB/hour.
Why it matters: the module is the documented observation point for the 2.E1 integration ("records every byte 'played'"); a long driven capture session is an OOM by design. Blast radius is limited: both shipped callers pass `capture: false` (pipeline_host.rs:1599, web host.rs:1732 — verified), so only tests/integrations can trip it.
What correct looks like: a byte cap with typed overflow (or ring-buffered capture), or a documented hard session-duration limit.
Confidence: HIGH — Verification: read; shipped callers verified capture-off.

### F-004 [S3] — Trait-mode NullBackend is a fill-and-refuse sink, and the API that ships it (`builtin_backends`) has zero callers
Where: null.rs:19-23 + 447-465; crates/audio-output/src/lib.rs:152-157
Code (null.rs:19-23, module docs admitting it):
```rust
//! The [`AudioBackend`] trait path (`open`) parks the driver inside
//! the backend — the sink is live (backpressure, counter, pause) but
//! nobody drains it, so an undriven session fills its ring and
//! refuses further writes with a typed `audio.ring_full` (bounded
//! memory, honest state).
```
Code (lib.rs:152-157):
```rust
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
    vec![
        Box::new(NullBackend::new()),
        Box::new(UnsupportedBackend::for_current_os()),
    ]
}
```
What is wrong: any consumer that takes the documented "backend set v0 ships" and writes through the `AudioBackend` trait gets a sink that accepts `buffer_frames` frames, then refuses everything (`audio.ring_full`) — playback stalls by construction. Meanwhile `rg -n "builtin_backends"` over the whole repo returns only the definition: dead exported API whose only real member is non-functional as a playback sink.
Why it matters: the name "the backend set v0 ships" overstates: it ships nothing (no caller), and its null member cannot play. The open_stream path in the CLI bypasses it entirely.
What correct looks like: either drive the parked device on a real session thread (doc 03 §9 Phase-3 wiring) before exposing the set, or delete/`#[doc(hidden)]` `builtin_backends` until it has a consumer.
Confidence: HIGH — Verification: read + rg (zero callers).

### F-005 [S4] — `AudioBackend::open` silently replaces an open session
Where: null.rs:447-465
Code:
```rust
        let shared = Arc::new(Shared::open(format, buffer_frames)?);
        // The driver is parked inside the backend (discard mode);
        // `take_device` hands it to whoever wants to drive.
        self.device = Some(NullDevice {
```
What is wrong: a second `open` overwrites `self.session`/`self.device` with no "already open" refusal; the previous session's state (counters, underruns) is dropped silently. Contrast `CliHost::open` which refuses double-opens with `NotReady` (host.rs:897-903).
Why it matters: API hygiene; a caller that re-opens loses its clock truth source without an error.
What correct looks like: typed `audio.sink_already_open` or documented replacement semantics.
Confidence: HIGH — Verification: read.

### F-006 [S4] — Duplicated half-vs-half accessor surface and a near-duplicate test
Where: null.rs:128-144 vs 331-343; tests at 733-747 vs 518-538
Code (null.rs:128-136 and 331-338 — same three lines twice):
```rust
    pub fn underruns(&self) -> u64 {
        self.shared.underruns.load(Ordering::Relaxed)
    }
    ...
    pub fn sample_count(&self) -> u64 {
        self.shared.counter.count()
    }
```
What is wrong: `underruns/sample_count/buffered_frames` are implemented identically on both halves (documented as views, so intentional), and `renderer_callback_composes_over_the_shared_state` (733-747) re-runs `open_stream_roundtrip_and_capture` (518-538) minus the capture/frame-count assertions — it would not catch anything the first test misses.
Why it matters: duplication drift risk; the second test adds no mutation-killing power.
What correct looks like: one `Shared`-level accessor each half delegates to (they already do — drop the duplicate method bodies via a macro or keep one public surface); delete or differentiate the weak test.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `NULL_DEVICE_ID` (const) | 53 | REAL | — | DeviceId::new(0); used by enumerate/open + tests. |
| `struct Shared` | 60-71 | REAL | — | Ring + counter + underruns + paused + written; all single-word atomics. |
| `Shared::open` | 74-91 | REAL (defect) | S1 | F-002: no upper bound on channels/rate geometry; format gate F32/Interleaved/≠0 correct. |
| `struct NullConfig` | 96-101 | REAL | — | `capture` bool; relocation from testkit documented (null.rs:30-34). |
| `struct NullStream` | 106-111 | REAL | — | Split halves; the shipped mode. |
| `struct NullSink` | 115-118 | REAL | — | Producer half. |
| `NullSink::clock_source` | 123-125 | REAL | — | Returns the shared counter — verified by clock_truth baseline test. |
| `NullSink::underruns` | 128-130 | REAL | — | Relaxed load; bookkeeping only. Duplication F-006. |
| `NullSink::sample_count` | 134-136 | REAL | — | Delegates to counter. |
| `NullSink::buffered_frames` | 142-144 | REAL | — | Write-side view of ring fill; used by engine drain gate (doc claim consistent). |
| `NullSink::written_frames` | 154-156 | REAL | — | Monotonic write twin; seek-boundary arithmetic (6.FIX-ogg-reanchor) — consumer is host.rs:1105. |
| `NullSink::write_bytes` | 182-226 | REAL | — | Frame-alignment + free-check correct (step math verified for 1..256 channels stack, >256 heap one-frame scratch); zero-alloc verified baseline line 230-234; all-or-nothing push with honest-refusal guard. |
| `NullSink::push_aligned` | 232-241 | REAL | — | `as_chunks::<4>()` + zip refill; partial last chunk handled (`&chunk[..part.len()]`); byte-identical LE f32 reinterpretation. |
| `AudioSink::write` (NullSink) | 245-258 | REAL | — | Format equality gate (`audio.format_mismatch`), partial-frame gate, delegates. |
| `AudioSink::latency` (NullSink) | 260-265 | REAL | — | u128 math, graceful None on u64 overflow; 16 frames @ 8 kHz = 2 ms verified baseline. |
| `struct NullDevice` | 276-280 | REAL | — | Consumer half; channels + capture. |
| `NullDevice::render` | 292-313 | REAL (defect) | S3 | Unaligned gate, paused no-op (no callback/counter/capture — correct freeze semantics), render_into, then capture — F-003 unbounded growth. |
| `NullDevice::take_capture` | 318-323 | REAL | — | `mem::take` drains; empty when off. |
| `NullDevice::paused` | 326-328 | REAL | — | Acquire load. |
| `NullDevice::underruns` | 331-333 | REAL | — | Duplicate view (F-006). |
| `NullDevice::sample_count` | 336-338 | REAL | — | Duplicate view (F-006). |
| `NullDevice::buffered_frames` | 341-343 | REAL | — | Duplicate view (F-006). |
| `struct NullBackend` | 350-353 | REAL | — | Session + parked device. |
| `NullBackend::new` | 357-359 | REAL | — | Default. |
| `NullBackend::open_stream` | 370-387 | REAL | — | The constructor the shipped CLI/web use; builds both halves. |
| `NullBackend::take_device` | 391-393 | REAL | — | Option::take; None when absent. |
| `NullBackend::is_open` | 396-398 | REAL | — | Trivial; no non-test caller found in shipped apps (test-only surface, hygiene note). |
| `NullBackend::shared` (private) | 400-405 | REAL | — | `audio.sink_not_open` typed error. |
| `AudioSink::write` (NullBackend) | 409-421 | REAL | — | Temporary NullSink per call (2 words, no alloc). |
| `AudioSink::latency` (NullBackend) | 423-430 | REAL | — | Delegates; None when not open. |
| `AudioBackend::enumerate` | 434-445 | REAL | — | One hardcoded "null (headless discard)" desc; honest. |
| `AudioBackend::open` | 447-466 | REAL (defect) | S3/S4 | F-004 (parked driver, fill-and-refuse) + F-005 (silent re-open). |
| `AudioBackend::pause` | 468-471 | REAL | — | Release store on shared flag. |
| `AudioBackend::resume` | 473-476 | REAL | — | Symmetric. |
| `AudioBackend::clock_source` | 478-481 | REAL | — | Counter handle. |
| `AudioBackend::underruns` | 483-487 | REAL | — | 0 when not open. |
| tests `f32_format`/`buffer_of` | 497-516 | REAL | — | Helpers. |
| 8 tests | 518-747 | REAL | — | See §5. |

## 5. Test honesty (tests in this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `open_stream_roundtrip_and_capture` (518) | YES | Asserts exact rendered bytes AND exact captured bytes AND take-drains. Mutation: swap a byte in capture → fail. |
| `underruns_are_counted_and_silence_captured` (541) | YES | Asserts silence_filled=2, underrun=1, counter=3 (data+silence), captured silence bytes. |
| `backpressure_is_typed_and_all_or_nothing` (563) | YES | Asserts `audio.ring_full` AND FIFO integrity of prior write (a partial-write bug would surface as wrong `out`). |
| `pause_freezes_counter_and_renders` (591) | YES | Asserts NaN-untouched buffer (render side-effect bug), counter 0→2, clock-source identity. |
| `stream_mode_shares_the_counter_between_the_halves` (626) | YES | Cross-half counter/underrun identity — catches split-state bugs. |
| `format_and_shape_errors_are_typed` (652) | YES | Seven distinct error codes asserted (S16, planar, non-pow2, unknown device, not-open, mismatch, unaligned, partial-frame). |
| `latency_reports_the_ring_fill` (715) | YES | Exact 2 ms assertion; enumerate desc sanity. |
| `renderer_callback_composes_over_the_shared_state` (733) | WEAK-YES | Would fail if render broke, but adds no assertion `open_stream_roundtrip_and_capture` lacks (F-006). |
All 8 pass in baseline: test-workspace.log.txt:164-171.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a null backend for tests/headless" | doc 01 §2.6 | REAL | This file; baseline tests 164-171. |
| "first vertical runs dummy/null aout in CI, real sinks labeled … status is partial-seam, not done" | feature-matrix `platform-aout-cpal` (toml:2360-2362) | REAL (honest) | cpal.rs:43/98-107 gate closed; no real-device evidence anywhere (rg). |
| "AC suites sine_sink_hash/callback_budget/clock_truth" as evidence | same row evidence string | REAL | Baseline test-workspace.log.txt:194-229 (4+1+4+3 all ok). |
| `audio.null-device` as shipped feature | `lumen engine-info` (baseline INDEX.md) | REAL-but-deceptive | True it ships; it is a discard sink — no audible output exists (F-001). |
| "The backend set v0 ships" | lib.rs:145-157 doc on `builtin_backends` | UNREACHABLE | rg repo-wide: zero callers (F-004). |
| "doc 03 §4.9 f `NullSink` … for determinism"; capture relocated out of testkit | null.rs:30-34 (deviation note) | REAL (recorded deviation) | Capture lives here as `NullConfig::capture`; consistent with tests. |
| Wave-7 context: "all output goes to the null device" | W07-A01 log §File7 | CONFIRMED | This audit's call chain (pipeline_host.rs:1598-1599 is the sole CLI sink). |

## 7. Dead / duplicated / unreachable code
- `builtin_backends()` (lib.rs:152-157) — UNREACHABLE (zero callers repo-wide); its null member is fill-and-refuse in trait mode (F-004). Referenced only by a worklog archive entry.
- `NullBackend::is_open` — no non-test caller found (hygiene).
- Duplicate accessor surface (F-006).
- Trait-mode `AudioBackend for NullBackend` (`open`/`pause`/`resume`/`enumerate`/`clock_source`/`underruns`, null.rs:433-488) — exercised only by this file's own tests (pause test), no production caller uses trait mode; parked-driver semantics make it unusable for playback as-is.

## 8. Dependencies & flags
- Crate is std-only, no unsafe (consistent with lib.rs:86 claim — verified: no `unsafe` token in null.rs).
- No feature flags gate this file; compiled in the default workspace build (baseline build #1 clean; audio_output tests ran in default test run #3).
- Depends on audio-engine (AudioSink trait), media-clock (SampleCounterSource), media-types (formats/errors). Layering consistent with boundaries (L4).

## 9. Open questions for the next agent
1. MKV channel-count path (`crates/demux/src/mkv/header.rs:171` `channels: Option<u64>`) — same F-002 chain? Who casts u64→u32 and is it bounded?
2. Ogg Vorbis ID channels byte (max 255 → 134 MB ring) — survivable but rude; worth a bound anyway.
3. desktop-tauri audio wiring is absent entirely (rg `audio_output|NullBackend|cpal` in apps/desktop-tauri/src → no matches) while its Cargo.lock pulls audio-output transitively — the desktop wave should decide whether that is dead weight or the Phase-3 wiring seam.
4. `afeat-volume` matrix row (toml:1659-1668) claims live volume/mute — those Gain/Mute stages feed a discard sink; the UI wave should record that the slider controls silence.
5. REQUEST-ORCH-RUN (runtime confirmation of F-002): craft an MP4 with stsd channelcount=0xFFFF (ffmpeg `-ac` won't go that high; hex-patch the existing sample-420.mp4 at the audio sample entry offset 16) and run `target/release/lumen open <file>`; expect alloc abort / OOM, non-zero exit.
