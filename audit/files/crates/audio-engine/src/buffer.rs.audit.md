# AUDIT: crates/audio-engine/src/buffer.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 94 / 3 production items + 1 test |
| File verdict | REAL — the sink-boundary data type and trait; production-reachable |
| Reachable from shipped entry point? | YES: `AudioBuffer` constructed by `AudioPipeline::mix` (audio-engine/src/pipeline.rs:377-381) on every quantum; `AudioSink::latency` called by the CLI host (apps/cli/src/pipeline_host.rs:619); the null sink implements `AudioSink` (crates/audio-output/src/null.rs:244) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Ninety-four correct lines: the `AudioBuffer` value type (format + shared
bytes + pts), a `frames()` that answers `None` on partial frames instead
of rounding (honesty over convenience), and the `AudioSink` trait
(`write` + `latency`) that keeps OS types out of the engine. One seam
nit: production hosts call the null sink's inherent `write_bytes`
directly (pipeline_host.rs:663) rather than the trait's `write`, so the
`write` vtable path is implemented-but-bypassed in the shipped build.

## 2. What actually works (verified)

— `AudioBuffer` flows out of every mixed quantum:
`AudioPipeline::mix` → `self.pending = Some(AudioBuffer { … })`
(audio-engine/src/pipeline.rs:377-381) → `AudioLimb::stage` →
`sink.write_bytes` (pipeline_host.rs:663).
— `frames()` (:35-37) delegates to `AudioFormat::frames_in` — the
partial-frame `None` is pinned by the in-file test (:87-92).
— `AudioSink::latency` is genuinely used in production
(pipeline_host.rs:619, the drift-observation path — even though the
governor is dormant, the call executes).

## 3. Findings

### F-001 [S4] `AudioSink::write` is implemented by the shipped sink but no host calls it
Where: buffer.rs:58 vs apps/cli/src/pipeline_host.rs:663, crates/audio-output/src/null.rs:244-257

Code (buffer.rs:58):
```
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), media_types::EngineError>;
```
Code (crates/audio-output/src/null.rs:244-245, 257):
```
impl AudioSink for NullSink {
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), EngineError> {
…
        self.write_bytes(data)
```
What is wrong: both shipped hosts stage audio through the inherent
`write_bytes(&[u8])` (pipeline_host.rs:663, apps/web/src/host.rs:233),
so the trait method that carries the type's whole purpose
("what the mixer feeds", :40) — taking the `AudioBuffer` this file
defines — is dead vtable surface in production. The trait still earns
its keep via `latency` (called fully-qualified at pipeline_host.rs:619)
and the stub backend (audio-output/src/stub.rs:77).
Why it matters: a future real backend could implement `write`
correctly and still never have it called, because hosts bypass the
seam; the seam is the architecture lie here, small but real.
What correct looks like: hosts call `AudioSink::write(&buffer)` and
`write_bytes` becomes the backend-internal helper.
Confidence: HIGH — Verification: read + repo-wide Grep for
`write_bytes|AudioSink`.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AudioBuffer` (struct) | 21-28 | REAL | — | pub fields; `PartialEq/Eq` on format+bytes+pts |
| `AudioBuffer::frames` | 35-37 | REAL | — | `None` on partial frames — honest, pinned by test |
| `AudioSink` (trait) | 49-63 | REAL | S4 | `write` bypassed by hosts (F-001); `latency` live |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `buffer_frame_math` | YES | 16 bytes stereo f32 → 2 frames; 7 bytes → `None` (no rounding) |

Not executed by me (no cargo); W00 workspace run green. UNVERIFIED
per-test.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine can target any sink without naming a device API" | :7-8 | REAL | trait has no OS types; null + stub backends implement it |
| "`write` runs on the pump side — may block for backpressure … must not deadlock against the device callback" | :44-48 | REAL (contract) | null backend's SPSC ring honors it (audio-output — other agent) |
| "a sink seeing [partial frames] treats it as malformed input" | :31-34 | REAL | `frames() == None` is the detector |

## 7. Dead / duplicated / unreachable code

`AudioSink::write`: implemented (null.rs:244, stub.rs:77), no
production caller (F-001).

## 8. Dependencies & flags

`media_types::{AudioFormat, SharedBytes, Duration, Stamp}`. No feature
gates; ships in the CLI binary.

## 9. Open questions for the next agent

- Wave 8 (audio-output): when a real backend lands, decide whether the
  host-side call moves to `AudioSink::write` (fixing F-001) — the
  `AudioBuffer`-typed path is the one this file exists for.
