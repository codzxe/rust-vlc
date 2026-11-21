# AUDIT: crates/audio-output/src/ring.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-21 |
| Lines / functions | 319 / 7 methods + 2 types + 6 tests |
| File verdict | REAL (sequential data structure executed in baseline; the cross-thread SPSC/memory-ordering contract is REAL-UNPROVEN — see F-001) |
| Reachable from shipped entry point? | YES (call chain: `lumen open`/`play` → apps/cli/src/host.rs:926 `open_session` → `pipeline_host::open_audio_session` (pipeline_host.rs:1587-1601, `RING_FRAMES = 131_072` at :118) → `NullBackend::open_stream` (null.rs:370-387) → `Shared::open` null.rs:85 `SpscRing::new`; consumer side: host.rs:613-617/726/824/1274 `device.render` → `render_into` render.rs:100 → `SpscRing::read`. Same wiring in `apps/web/src/host.rs:1732`.) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `rg -n "SpscRing" --type rs` (caller census: null.rs:61/85, render.rs:90/130, tests/callback_budget.rs:273, lib.rs:142 — no other production callers) · `rg -n "NullBackend|builtin_backends|audio_output::"` (wiring: apps/cli/src/pipeline_host.rs:1599 + apps/web/src/host.rs:1732 are the only shipped construction sites; `builtin_backends` has zero callers) · `rg -n "open_audio_session|RING_FRAMES"` (host.rs:926; RING_FRAMES=131_072) · `rg -n "MAX_PCM_CHANNELS"` (demux ceilings: avi/header.rs:349 = 64, wav/format.rs + pcm decoder = 64) · read crates/codecs/src/audio/vorbis/headers.rs (channels u8, only zero rejected → ≤255) · sed-n verified every quote below · `rg -n "thread::spawn|thread::Builder|loom"` in crates/audio-output + apps/cli/tests (zero matches — no concurrent test exists anywhere) · baseline `audit/runs/baseline/test-workspace.log.txt:184-189` (6/6 ring tests ok) · docs/03-engine-plan.md:644-648 (§4.9 c) · docs/feature-matrix.toml:2352-2362 (row `platform-aout-cpal`) |

## 1. One-paragraph verdict
This is the one file in the audio-output domain that is not a seam: a complete, careful,
correct-by-inspection SPSC ring with real shape validation, real wraparound handling, and six
meaningful sequential tests that all executed and passed in the baseline run. The memory-ordering
protocol (Relaxed slot stores, Release cursor publishes, Acquire cursor loads) is textbook-correct
under the stated SPSC discipline, and the bounded-loop/no-allocation/no-lock claims are
structurally true. Two structural holes remain: (1) the file's entire reason to exist — safe
cross-thread transfer — is never exercised by two threads anywhere in the workspace, not in tests
and not in the shipped CLI (both halves run on one thread; the "device thread" is simulated
comments), so the wait-free claim rests on code review alone, exactly the half of doc 03's gate
("code review plus these tests") that the tests cannot check — a Release→Relaxed mutation passes
all six tests; (2) `new()` bounds nothing but usize arithmetic overflow, so the no-OOM property
is an accident of out-of-crate demuxer ceilings. Hostile domain answer: the ring is real, but its
only consumer is the null device, whose rendered buffers the CLI drops on the floor
(host.rs:613-617) — a user never hears sound in any shipped build.

## 2. What actually works (verified)
- All 6 in-module tests ran and passed — verified by baseline
  `audit/runs/baseline/test-workspace.log.txt:184-189`:
  `test ring::tests::full_ring_refuses_without_blocking_or_tearing ... ok`,
  `push_read_roundtrip_in_fifo_order ... ok`, `long_stream_wrap_conservation ... ok`,
  `shape_validation_is_typed ... ok`, `short_read_reports_what_was_available ... ok`,
  `unaligned_buffers_are_refused_whole ... ok` (audio-output unit suite: 33 passed).
- `read` path is allocation-free at runtime — verified by baseline
  `test-workspace.log.txt` `zero_alloc_null_sink` suite (1 passed; the guarded window wraps
  `device.render` → `render_into` → `SpscRing::read`, asserts zero bytes over 256 cycles).
- Reachability from the shipped `lumen` binary — verified by rg call-chain:
  host.rs:926 → pipeline_host.rs:1599 → null.rs:85 (producer), host.rs:614 → null.rs:292-306 →
  render.rs:100 (consumer).
- Module claim "never allocates after construction" (ring.rs:23-25) — verified by reading:
  `push`/`read` take `&self`, construct no owned values, touch only atomics and the boxed slots.

## 3. Findings

### F-001 [S2]
Where: crates/audio-output/src/ring.rs:193-319 (test module) and 124/144/162/164/184 (the ordering-sensitive loads/stores)
Code:
```
        if count > 0 {
            self.write.store(write, Ordering::Release);
        }
```
(and `let mut read = self.read.load(Ordering::Relaxed);` at :164, `available_frames`'s
`Acquire` loads at :101)
What is wrong: The SPSC ring's headline property — safe cross-thread producer/consumer
transfer via the Release/Acquire cursor protocol — has zero test coverage and zero production
exercise. `rg "thread::spawn|thread::Builder|loom"` over crates/audio-output and apps/cli/tests:
no matches. Every test (and the shipped CLI) runs `push` and `read` on one thread; the crate's
own AC suite admits it — tests/sine_sink_hash.rs:108-109: "The device drains exactly one tick
per callback period — // the shape an OS device thread would drive." The shipped CLI drives
both halves sequentially (host.rs:613-617), and no session/device thread exists ("Phase-3
wiring", lib.rs:74-76).
Why it matters: Mutation analysis: change `Ordering::Release` → `Relaxed` at ring.rs:144/184 or
drop the `Acquire` at :101 — all 6 tests still pass, `cargo test --workspace` stays green, and
on weakly-ordered targets (AArch64 macOS, the repo's own macos target) the consumer can observe
torn/stale slots. Doc 03 §4.9 c's gate is "code review plus these tests"; the tests gate
geometry and FIFO only, so half the gate is vapor. The module doc's "wait-free" and
"release/acquire pairing" claims (ring.rs:1, 17-21) are asserted, not tested.
What correct looks like: at minimum a two-thread hammer test (producer thread pushing a
known sequence, consumer thread draining, assert exact FIFO + conservation on join); ideally a
loom/shuttle model test of push/read interleavings. No new dependencies needed for the hammer.
Confidence: HIGH — Verification: read + rg (no concurrent test exists); baseline run cited for
what did execute (sequential only).

### F-002 [S3]
Where: crates/audio-output/src/ring.rs:70-78
Code:
```
    pub fn new(channels: u32, buffer_frames: usize) -> Result<SpscRing, EngineError> {
        if channels == 0 || buffer_frames == 0 || !buffer_frames.is_power_of_two() {
            return Err(error::unsupported("audio.ring_shape_invalid"));
        }
        let slots = buffer_frames
            .checked_mul(channels as usize)
            .ok_or_else(|| error::unsupported("audio.ring_shape_invalid"))?;
        let mut storage = Vec::with_capacity(slots);
```
What is wrong: The only overflow guard is `checked_mul` at usize width. `Vec::with_capacity`
ABORTS the process (allocation-failure abort, not `Err`) for any large product below that: e.g.
`channels = 255` (Vorbis identification header, codecs/vorbis/headers.rs:117 — u8, only zero
rejected; OpusHead byte 9 same width) × `buffer_frames = 131_072` (the CLI's RING_FRAMES,
pipeline_host.rs:118) = 33.4 M slots ≈ 128 MiB per session open — survivable, but nothing in
this crate enforces it. The bound is an accident of out-of-crate demuxer ceilings
(`MAX_PCM_CHANNELS = 64`, demux/avi/header.rs:349, mirrored in wav/format.rs and the pcm
decoder) plus format-native field widths (u8 for Vorbis/Opus).
Why it matters: The ring is a public API (`pub use ring::{RingRead, SpscRing}`, lib.rs:142).
The moment a future backend (the ADR-005 cpal adapter, or any caller) passes file-influenced
channels with a bigger `buffer_frames`, a hostile media file converts to a process-abort OOM
with no typed error. The doc comment even claims the error path covers "a byte size that
overflows `usize`" — arithmetic overflow, which is the one failure mode users can never hit;
the reachable failure mode (allocation abort) is unguarded. null.rs:174-175's "channels > 256 —
no container parses that wide" is true only of today's parser set, not of this API.
What correct looks like: a sane product ceiling in `new()` (e.g. reject slots > some MB bound
with `audio.ring_shape_invalid`), or a fallible allocation strategy; the invariant should not
live in two demuxer crates.
Confidence: HIGH — Verification: read + rg of MAX_PCM_CHANNELS/vorbis headers/wav harvest
(wav.rs:101 `u32::from(fmt.channels)` raw from file, ceiling applied in demux parse).

### F-003 [S3]
Where: crates/audio-output/src/ring.rs:113-120 (and 154-160)
Code:
```
    pub fn push(&self, data: &[f32]) -> usize {
        let channels = self.channels;
        if !data.len().is_multiple_of(channels) {
            // A whole-frames contract violation: refuse rather than
            // tear a frame. (Callers validate and type this; the ring
            // itself is the last line of defense.)
            return 0;
        }
```
What is wrong: `push` returns `0` both for "ring full" (documented: "0 when full — the pump's
backpressure signal", ring.rs:111-112) and for "misaligned buffer" (undocumented in the return
contract). Same conflation on `read`: misaligned `out` returns `RingRead::default()`, the exact
value of "empty ring". The doc at :110-112 is therefore wrong by omission: 0 does not only mean
full.
Why it matters: A caller that hits the misaligned case reads it as backpressure and retries
forever — a silent livelock, not an error. Current callers are safe (null.rs:184-189 pre-checks
`frame_bytes` alignment; null.rs:293-294 `render` types `audio.render_buffer_unaligned`;
render.rs:100 slices an aligned prefix; the pump propagates typed errors), so this is latent —
but the ring is re-exported as public API and its last-line-of-defense return value is
indistinguishable from normal flow control.
What correct looks like: a distinguishable outcome (e.g. `Option<usize>`/`Result`, or at
minimum a debug_assert + doc that 0 also means misaligned).
Confidence: HIGH — Verification: read of all four call sites (null.rs:182-226/292-294,
render.rs:98-100, pump.rs:186-218).

### F-004 [S4]
Where: crates/audio-output/src/ring.rs:1-3
Code:
```
//! The output ring: a wait-free SPSC ring of interleaved f32 frames
//! (doc 03 §4.9 c — "the SPSC ring's wait-free index swap is the only
//! synchronization on the callback path").
```
What is wrong: The string in quotation marks is presented as a doc quote but is not verbatim:
doc 03 §4.9 c actually says (docs/03-engine-plan.md:644-647) "it is **forbidden**: any lock
except the SPSC ring's wait-free index swap; allocation; logging; syscalls; unbounded loops."
It is also imprecise about the code: the callback path (`render_into`) synchronizes three
atomic structures — the ring cursors, the `SampleCounter` advance, and the `underruns`
`fetch_add` (render.rs:108-112). All wait-free, but the ring is not "the only synchronization
on the callback path" literally.
Why it matters: Quotation-marks-as-paraphrase is how evidence strings drift; this audit wave
exists because of that habit (baseline INDEX row 8).
What correct looks like: paraphrase without quotes, or the verbatim fragment ("any lock except
the SPSC ring's wait-free index swap").
Confidence: HIGH — Verification: read both files; `rg -n "wait-free index swap" docs/` →
single hit at 03-engine-plan.md:646.

### F-005 [S4]
Where: crates/audio-output/src/ring.rs:100-107, 124, 162-164
Code:
```
    pub fn available_frames(&self) -> usize {
        self.write.load(Ordering::Acquire) - self.read.load(Ordering::Acquire)
    }
```
What is wrong: On the real-time consumer path, `read()` pays 3 cursor loads (write-Acquire +
read-Acquire inside `available_frames`, then its own `read.load(Relaxed)` again at :164); `push`
likewise re-loads its own cursor at :124 after `free_frames()` already loaded it. Harmless
redundancy — but this crate bills per-callback operation counts (`RenderStats`, "branch-cheap
RT core") and never counts these.
Why it matters: Cosmetic next to F-001, but the op-ledger discipline the crate advertises
should either count or eliminate them.
What correct looks like: single loads of own cursor, ledger note.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: crates/audio-output/src/ring.rs:100-101, 139-141, 179-181
Code:
```
        self.write.load(Ordering::Acquire) - self.read.load(Ordering::Acquire)
```
What is wrong: Monotonic-cursor arithmetic (`write - read`, `write += run`) panics on integer
overflow in debug builds. On 64-bit this needs 2^64 frames (unreachable); on a 32-bit target at
48 kHz the cursors wrap after ~24.9 h and a debug build panics (release wraps correctly — the
difference stays < capacity, so modular arithmetic holds).
Why it matters: Only 32-bit debug builds are exposed; no such target ships. Recorded for
completeness, not urgency.
What correct looks like: `wrapping_sub`/`wrapping_add` on cursors if 32-bit ever becomes a
target.
Confidence: HIGH — Verification: read + arithmetic.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RingRead` (struct) | 37-43 | REAL | — | Plain data carrier; `Default` doubles as the read-refusal sentinel (F-003) |
| `SpscRing` (struct) | 53-60 | REAL | — | 5 fields, all atomics/plain — `Sync` sound for the SPSC contract |
| `SpscRing::new` | 70-86 | REAL | S3 (F-002) | Shape validation real (zero/pow-2/checked_mul); no allocation ceiling; abort path unguarded |
| `SpscRing::channels` | 89-91 | REAL | — | Trivial getter, correct |
| `SpscRing::capacity_frames` | 94-96 | REAL | — | `mask + 1`, correct |
| `SpscRing::available_frames` | 100-102 | REAL | S4 (F-005/F-006) | Acquire/Acquire snapshot; correct pairing for both halves; debug-underflow caveat |
| `SpscRing::free_frames` | 105-107 | REAL | — | `capacity - available`, correct |
| `SpscRing::push` | 113-147 | REAL (cross-thread UNPROVEN) | S2 (F-001), S3 (F-003) | Bounded ≤2-segment loop; Relaxed slots + Release publish — protocol correct by review; misaligned→0 conflated with full |
| `SpscRing::read` | 154-190 | REAL (cross-thread UNPROVEN) | S2 (F-001), S3 (F-003) | Same protocol mirrored; segments ledger honest (verified by wrap test); RT path zero-alloc per baseline `zero_alloc_null_sink` |
| `tests::shape_validation_is_typed` | 198-207 | REAL | — | Fails if validation deleted |
| `tests::push_read_roundtrip_in_fifo_order` | 209-221 | REAL | — | Fails on mask/count/order mutation |
| `tests::full_ring_refuses_without_blocking_or_tearing` | 223-240 | REAL | — | Asserts segments==2 on wrap — fails on wrap bug |
| `tests::short_read_reports_what_was_available` | 242-253 | REAL | — | Fails if read over-reports |
| `tests::unaligned_buffers_are_refused_whole` | 255-265 | REAL | — | Pins F-003's behavior (documents the conflation) |
| `tests::long_stream_wrap_conservation` | 267-318 | REAL | — | 100-iteration saturation loop, conservation + FIFO closed exactly — strongest test in the file; still single-threaded |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| shape_validation_is_typed | YES | Deleting the pow-2/zero checks makes `new(0,8)`/`new(2,7)` return Ok; `.err().must()` panics |
| push_read_roundtrip_in_fifo_order | YES | Asserts exact vector equality + free==0 |
| full_ring_refuses_without_blocking_or_tearing | YES | Over-write past free corrupts `[3.0,4.0,5.0,6.0]`; segments==2 pins the wrap split |
| short_read_reports_what_was_available | YES | NAN-tail assert catches over-read |
| unaligned_buffers_are_refused_whole | YES (by design of F-003) | Returns-0 pinned; cannot catch the conflation itself |
| long_stream_wrap_conservation | YES (geometry/FIFO/conservation) | Exact prefix compare each iteration + final conservation identity |
| ALL SIX | NO for memory-ordering bugs | Single-threaded execution makes Release/Acquire vs Relaxed indistinguishable — the F-001 gap |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the SPSC ring's wait-free index swap is the only synchronization on the callback path" | ring.rs:2-3, attributed to doc 03 §4.9 c | SHALLOW (paraphrase-as-quote; imprecise — counter + underrun atomics also on path) | docs/03-engine-plan.md:644-647 verbatim text differs; render.rs:108-112 |
| "wait-free SPSC ring" | ring.rs:1 | REAL-UNPROVEN | Structurally bounded (≤2 segments, no locks — verified by reading); never executed cross-thread (F-001) |
| "The ring never allocates after construction" | ring.rs:23-25 | REAL | push/read take `&self`, no owned values; baseline `zero_alloc_null_sink` 1 passed |
| Ring is "the only synchronization" the RT rules permit | doc 03 §4.9 c (docs/03-engine-plan.md:644-648) | REAL | render_into's traced graph: ring read + counter advance + fetch_add + slice fill, nothing else |
| `audio.null-device` shipped feature | baseline `lumen engine-info` (INDEX.md:46) | REAL | NullBackend reachable from `lumen open` (chain in header); ring is its core |
| "status is partial-seam … no real-device evidence exists (typed marker refuses on every platform)" | feature-matrix row `platform-aout-cpal` (docs/feature-matrix.toml:2352-2362) | REAL (honest) | cpal.rs:43 `AUDIO_ADR005_ADMITTED = false`; `cpal_backend()` always `Err(BackendNotAdmitted::CPAL)` |

## 7. Dead / duplicated / unreachable code
None in this file: every method has a reachable caller (`available_frames`/`free_frames` via
null.rs:142-144/191-196 and render path; `channels` via render.rs:98; `capacity_frames` via
null.rs:194). Adjacent dead code owned by other files (confirmed this wave by W08-A04):
`RenderCallback` trait + `RingRenderer` + `builtin_backends()` (lib.rs:152) have zero non-test
callers. Domain fact for the synthesis: the shipped CLI's only "device" is `NullDevice`, and
its rendered buffers are dropped (host.rs:613-617 renders into a scratch `Vec` that is never
read, `capture: false` at pipeline_host.rs:1599) — no hardware path exists in any build.

## 8. Dependencies & flags
Std-only (`std::sync::atomic`), plus `media_types::EngineError` and `crate::error` — matches
the crate's zero-external-deps manifest (crates/audio-output/Cargo.toml: deps audio-engine,
media-clock, media-types only; no feature flags, no optional deps, no `unsafe`, no `#[allow]`,
workspace lints inherited). No cfg-gating anywhere in the file. Confirmed: nothing here is
compiled out of the shipped build.

## 9. Open questions for the next agent
1. **OpusHead channels=0 inconsistency (demux wave):** `TrackFact::audio_geometry`
   (apps/cli/src/demux_host.rs:202-232) takes OpusHead byte 9 verbatim with no zero check
   (`Some((u32::from(data[9]), 48_000))`), so a corrupt OpusHead with 0 channels fails the
   WHOLE `lumen open` (Shared::open's `channels == 0` → `audio.format_unsupported`), while the
   analogous corrupt Vorbis ident (zero rejected in codecs/vorbis/headers.rs:117-122) opens the
   session audio-less on the wall clock. Inconsistent containment; confirm and severity it.
2. Doc 03 §4.9 f promises testkit `VanishingSink` (device loss mid-stream) and `StallingSink`
   (stops pulling) — neither exists anywhere (single comment mention,
   tests/integration/first-vertical/src/scenarios.rs:21). Device-loss/switch behavior
   (`SinkEvent::DeviceLost`) is therefore untested end-to-end; belongs to the null.rs/device.rs
   follow-up.
3. If the orchestrator can run it: a two-thread hammer over `SpscRing` (the F-001 test) is the
   single cheapest way to convert this file's UNPROVEN half into evidence. No new deps needed.
