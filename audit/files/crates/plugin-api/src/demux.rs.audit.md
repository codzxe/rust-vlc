# AUDIT: crates/plugin-api/src/demux.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-24 |
| Lines / functions | 400 / 11 production items (+3 tests, 2 test doubles) |
| File verdict | REAL (trait core, shipped and executed); 3 dead items |
| Reachable from shipped entry point? | YES: lumen-cli `main` → apps/cli/src/demux_host.rs:61 (`use plugin_api::{Access, DemuxEvent, Demuxer, ...}`) → apps/cli/src/containers/* (`impl Demuxer`, e.g. mp4.rs:79,165-177; ogg.rs:82 `demuxer.start()`; ts.rs:111) — and apps/web/src/open.rs:32. `PacketSink`/`container_duration`/`track_timing_conflict` are NOT reachable (no callers). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -q -p plugin-api demux` -> 3 passed, 0 failed, exit 0; rg PacketSink/container_duration/track_timing_conflict across repo; sed verifications |

## 1. One-paragraph verdict
The `Demuxer` trait, `SeekSupport`, and `DemuxEvent` are the real, shipped demux contract — lumen-cli's twelve container parsers and the web app implement and drive them, and the trait defaults are honest typed refusals (default `reanchor` returns `Unsupported`, default `seekability` `None`, both tested, tests run and pass). But the module documents a push model that does not exist: `PacketSink` has zero implementors and zero call sites in the entire repo (codec-api defines its own unrelated `PacketSink` for encoders), and the two helper functions `container_duration` and `track_timing_conflict` are called by nothing except their own unit tests. The doc's dataflow story is wrong about the engine it ships in.

## 2. What actually works (verified)
— verified by `cargo test -q -p plugin-api demux` -> "3 passed; 0 failed" (events_flow_packets_then_eos incl. idempotent EOS; default_seek_hooks_refuse_typed asserting stage/kind/context-code of the typed default refusal; typed_error_shapes asserting Corrupt{Timing} + source chain + container_duration Some/None). Shipped usage verified by rg: `demuxer.start()` at apps/cli/src/containers/ogg.rs:82, mp4.rs:101, ts.rs:111; `next_event()` at ogg.rs:305+ and mp4.rs:587+; `seekability()` consumed at apps/web/src/open.rs:358 and mp4.rs:582.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/demux.rs:169-183 (and 5-9)
Code:
```
/// Where demuxed packets go: the engine's queue, created and consumed
/// on the demux worker thread (no `Send` bound — the codec-api
/// `FrameSink` precedent).
pub trait PacketSink {
```
What is wrong: `plugin_api::demux::PacketSink` has ZERO implementors and ZERO users anywhere (rg over the whole repo: the only other `PacketSink` is codec-api's own distinct trait at crates/codec-api/src/encode.rs:99, re-exported separately as `EncodePacketSink` at plugin-api/src/codec.rs:20). The module doc (L6-8) claims the demuxer "pushes demuxed packets into the engine's [`PacketSink`]" — FALSE: every real engine path pulls via `next_event`. Why it matters: doc-vs-code drift on the core dataflow; readers implement against a phantom seam. What correct looks like: delete the trait or wire it. Confidence: HIGH — Verification: ran rg (all matches listed in log).
Also dead per the same rg: `container_duration` (L215-230) and `track_timing_conflict` (L188-198) are referenced only by their own tests (L377, L397-398) — merged into this finding (same defect class, same file). Counted as F-001 + F-002 below.

F-002 [S3]
Where: crates/plugin-api/src/demux.rs:185-230
Code:
```
pub fn track_timing_conflict(track: &TrackId) -> EngineError {
```
What is wrong: Dead helpers. `track_timing_conflict` and `container_duration` have no callers outside `mod tests` (verified by repo-wide rg; crates/demux, playback-engine, apps/* never import them). Tested-but-unused production API surface. Confidence: HIGH — Verification: ran rg.

F-003 [S4]
Where: crates/plugin-api/src/demux.rs:277-279
Code:
```
            duration: Some(Duration::Known(media_types::Nanoseconds(
                dur.as_nanos().map(|n| n.0).unwrap_or(0),
```
What is wrong: Test helper swallows an `as_nanos` error into `Some(Known(0ns))` — precisely the "value vs absence" conflation the module's own doc denounces (L213-214: "a value and its absence are different types, MP §8"). Test-only blast radius. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/plugin-api/src/demux.rs:259-261
Code:
```
        fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
            Ok(TimeSpan::UNKNOWN)
        }
```
What is wrong: The FakeDemux test double returns a fake seek success, while the trait doc (L84-85) demands "never fake successes (MP §8 bullet 4)". Harmless here (seek is not under test in this file), but the double teaches the anti-pattern it documents against. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Demuxer::start (required) | 52-58 | REAL | – | 12+ shipped impls |
| Demuxer::next_event (required) | 60-68 | REAL | – | pull model; the real contract |
| Demuxer::seek (required) | 70-80 | REAL | – | pure query; engine decides |
| Demuxer::seekability (default None) | 82-91 | REAL | – | tested; consumed by web open path |
| Demuxer::reanchor (default refusal) | 93-120 | REAL | – | typed Unsupported; tested |
| enum SeekSupport | 123-145 | REAL | – | Fast{indexed}/SlowLinear/None; used |
| enum DemuxEvent | 147-167 | REAL | – | Packet/Recovered/Discontinuity/EndOfStream; used everywhere |
| trait PacketSink::accept | 169-183 | UNREACHABLE (dead) | S3 | F-001 |
| track_timing_conflict | 185-198 | UNREACHABLE (dead) | S3 | F-002 |
| DemuxSourceInfo Display/Error | 200-211 | REAL (as part of F-002 chain) | S3 | only reachable via dead helper |
| container_duration | 213-230 | UNREACHABLE (dead) | S3 | F-002; logic itself correct (checked sub, base conversion) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| events_flow_packets_then_eos (287) | YES for trait shape, weak for real demuxers | in-memory FakeDemux; only proves the enum flow contract |
| default_seek_hooks_refuse_typed (344) | YES | asserts exact stage/kind/context-code of defaults |
| typed_error_shapes (375) | YES | asserts Corrupt{Timing}, source chain, code; container_duration Some/None |
Coverage gap: no test exercises `Recovered{skipped,reason}` or `Discontinuity` values; no negative test that `seek` target-out-of-range maps to the documented error kinds (that lives in impl crates).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pushes demuxed packets into the engine's [PacketSink]" | demux.rs:6-8 | FALSE | rg: no implementor/caller of this trait |
| "one container chunk per next_event, never an internal read-the-whole-file loop" | demux.rs:20-22 | UNVERIFIED here | contract on impl crates (demux, apps/cli containers) — other agents' files |
| "Invariant D4: on Err the demuxer is left at its pre-seek position" | demux.rs:100-103 | UNVERIFIED here | impl-side invariant; not enforceable in trait file |
| Default reanchor = typed refusal | demux.rs:110-120 | REAL | test ran, passed |

## 7. Dead / duplicated / unreachable code
PacketSink (169-183), track_timing_conflict (188-198), DemuxSourceInfo (203-211, only used by the dead helper), container_duration (215-230). All tested-but-dead.

## 8. Dependencies & flags
Only media-types (types: TrackId, TimeSpan, Duration, EngineError, Packet, Track). No serde, no external crates. No unsafe. No features.

## 9. Open questions for the next agent
- Was PacketSink ever consumed by playback-engine's demux worker, and was that path deleted? The doc comment reads like a design that was never wired (or was rewired to pull). Whoever audits playback-engine should confirm the pull loop is the only demux consumption path.
