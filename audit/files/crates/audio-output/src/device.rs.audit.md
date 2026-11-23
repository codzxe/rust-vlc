# AUDIT: crates/audio-output/src/device.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-23 |
| Lines / functions | 132 / 7 (DeviceId::{new,handle}, DeviceDesc, SinkEvent, trait AudioBackend: enumerate/open/pause/resume/clock_source/underruns) |
| File verdict | REAL (vocabulary + trait, shipped and implemented) — but the enumeration story it anchors is: no real device enumeration exists in any shipped binary |
| Reachable from shipped entry point? | YES — audio-output is a direct dependency of lumen-cli (apps/cli/Cargo.toml:49); trait implemented by NullBackend/UnsupportedBackend, exercised by baseline suites |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg (AudioBackend impls/callers, SinkEvent uses, backend construction in apps/), sed quote verification, baseline citation |

## 1. One-paragraph verdict
The hostile question — "device enumeration: real enumeration or empty vec?" —
answers: NEITHER, because nothing real is enumerated and nothing calls the
method at all. `AudioBackend::enumerate` has zero callers in the entire
shipped tree (rg `\.enumerate\(\)` matches only std `Iterator::enumerate`);
the CLI constructs the null sink directly
(apps/cli/src/pipeline_host.rs:1599) without ever listing devices. The only
implementations are NullBackend, which returns one synthetic
`"null (headless discard)"` device (null.rs:434-445), and
UnsupportedBackend, which returns `Vec::new()` (stub.rs:88-91). The cpal
adapter is a compile-time-gated typed refusal (`AUDIO_ADR005_ADMITTED =
false`, cpal.rs:43). Real enumeration code exists only in the unreachable
platform-* crates behind a different trait. The trait file itself is clean,
honest, and well-documented — the defect is the error-channel-less
`enumerate` signature and the dead vocabulary around it.

## 2. What actually works (verified)
- The trait + types compile into the shipped binary and its contract is
  implemented and tested: baseline cargo-test-workspace.txt:145-240 —
  audio-output 33 unit + 13 integration + 1 doctest, all pass, including
  `null::tests::open_stream_roundtrip_and_capture ... ok` and
  `stub::tests::every_operation_answers_typed_not_supported ... ok` and
  `cpal::tests::the_gate_is_recorded_closed ... ok`.
- The cpal gate cannot be flipped by accident: `const _: () =
  assert!(!AUDIO_ADR005_ADMITTED, ...)` (cpal.rs:48-52) fails the build on a
  bare const flip — verified by reading; `the_gate_is_recorded_closed`
  re-asserts it at test time (baseline: ok).
- Doc honesty is maintained downstream: the feature matrix row
  `platform-aout-cpal` says "status is partial-seam, not done … no
  real-device evidence exists (typed marker refuses on every platform)"
  (feature-matrix.toml:2361-2362) — the docs do NOT claim audible output.

## 3. Findings
F-001 [S3]
Where: crates/audio-output/src/device.rs:88-91
Code:
```
pub trait AudioBackend: AudioSink {
    /// Devices this backend can see right now. Empty is honest ("no
    /// devices"), never a placeholder entry.
    fn enumerate(&self) -> Vec<DeviceDesc>;
```
What is wrong: `enumerate` has no error channel, so "no devices" and
"subsystem unavailable" are indistinguishable. When the planned real adapters
land (doc 01 §2.7 / ADR-005), an adapter whose library is missing (the exact
case platform-linux's `AudioDevices::enumerate -> Result<…, PlatformError>`
handles via dlopen refusal) can only return an empty vec — which this doc
blesses as "honest" — or panic. The L5 seam has the typed error; the L4 seam
this file defines drops it. Why it matters: for an audio player, "no devices"
vs "sound server down" is a user-facing distinction that will be silently
erased at this boundary. What correct looks like:
`fn enumerate(&self) -> Result<Vec<DeviceDesc>, EngineError>`.
Confidence: HIGH — Verification: read + rg (platform-linux audio seam
signature compared).

F-002 [S3]
Where: crates/audio-output/src/device.rs:13-16 (file header) + shipped wiring
Code:
```
//! v0 implementations: [`crate::NullBackend`] (complete) and
//! [`crate::UnsupportedBackend`] (typed not-supported stubs — the
//! real-device adapters' placeholders until ADR-005 lands them in
//! `platform-*`, doc 01 §2.7).
```
with the shipped consumer:
```
    let NullStream { sink, device } =
        NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })?;
```
(apps/cli/src/pipeline_host.rs:1599-1600)
What is wrong: the shipped product's entire audio output is a discard sink —
`capture: false` means rendered samples are thrown away; no user on any
platform hears anything, and `enumerate` is never invoked by any shipped code
path (rg: zero non-Iterator callers). The file's own doc admits this state,
the matrix row is honest, and the cpal gate is a disciplined refusal — so
this is a disclosed gap, not fraud — but the hostile headline stands: NOT ONE
shipped binary performs real device enumeration or real audio output.
Confidence: HIGH — Verification: rg + read.

F-003 [S4]
Where: crates/audio-output/src/device.rs:56-70
Code:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkEvent {
    /// The device went away mid-playback (unplug, server restart).
    DeviceLost,
```
What is wrong: `SinkEvent` is dead vocabulary — never constructed anywhere in
the tree (rg: only doc-comment mentions in playback-engine recovery.rs:726 /
hotplug.rs:169 and the re-export lib.rs:137). The doc says "the event stream
joins the session event bus in the 3.B cards" — recorded future vocabulary,
not current behavior. Confidence: HIGH — Verification: rg.

F-004 [S4]
Where: crates/audio-output/src/device.rs:25-39
Code:
```
/// A device's stable identifier (opaque; minted by
/// [`AudioBackend::enumerate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);
```
What is wrong: the doc says DeviceId is "minted by enumerate", but with zero
enumerate callers the only minting that occurs is NullBackend's constant
NULL_DEVICE_ID; `DeviceId::new` is a pub API whose only in-tree user is the
null backend. Minor doc-overreach on a currently single-tenant type.
Confidence: HIGH — Verification: rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DeviceId::new | 30-34 | REAL | S4 | pub const ctor; only null backend mints |
| DeviceId::handle | 36-39 | REAL | — | used by tests + platform crates' id minting comparison |
| DeviceDesc (struct) | 42-54 | REAL | — | pure data; shipped vocabulary |
| SinkEvent (enum) | 56-70 | DEAD (documented future) | S4 | F-003: never constructed |
| AudioBackend::enumerate | 88-91 | REAL API / ZERO CALLERS | S3 | F-001/F-002 |
| AudioBackend::open | 93-108 | REAL | — | implemented by null (ok) + stub (typed refusal); called via open_stream path |
| AudioBackend::pause/resume | 110-121 | REAL | — | null implements freeze/continue (baseline tests) |
| AudioBackend::clock_source | 123-127 | REAL | — | Some after open; drives MediaClock audio_master (clock_truth suite) |
| AudioBackend::underruns | 129-131 | REAL | — | null counts atomically (baseline: underruns_are_counted_and_silence_captured) |

## 5. Test honesty
No tests in this file (trait module). The contract's implementing suites are
real and green (baseline cited in §2); the stub test
`every_operation_answers_typed_not_supported` genuinely pins the refusal
path, and the cpal gate tests pin the closed gate.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v0 implementations: NullBackend (complete) and UnsupportedBackend (typed … stubs)" | device.rs:13-16 | REAL | both impls exist; baseline tests pass |
| "Empty is honest ('no devices'), never a placeholder entry" | device.rs:89-90 | REAL-BUT-FRAGILE | stub returns Vec::new(); design conflates empty with failure (F-001) |
| "the sample counter is monotonic while the device runs, frozen while paused, and never rewinds across device switches" | device.rs:85-87 | REAL | counter.rs tests (baseline: counter_is_monotonic_and_rate_carrying, offset_carry_never_rewinds_across_devices) |
| "devices are DeviceIds + DeviceDescs, our own vocabulary" (ARCH-6) | device.rs:9-11 | REAL | no OS types in signatures (read) |

## 7. Dead / duplicated / unreachable code
`SinkEvent` dead (F-003). `enumerate` uncalled (F-002). The trait shape is
mirrored as a *different* trait (`AudioDevices`) in the three platform crates
— a planned ADR-005 unification point.

## 8. Dependencies & flags
audio-engine, media-clock, media-types only (Cargo.toml:36-39). No features,
no unsafe, no lint suppressions in this file.

## 9. Open questions for the next agent
- When ADR-005 lands cpal: does the platform adapter implement AudioBackend
  (then F-001's missing error channel bites immediately) or a fourth trait?
- playback-engine's hotplug.rs/recovery.rs reference SinkEvent::DeviceLost in
  docs only — whoever audits those files should check whether the claimed
  recovery-on-device-loss has any real trigger today.
