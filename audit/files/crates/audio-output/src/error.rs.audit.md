# AUDIT: crates/audio-output/src/error.rs
| Field | Value |
|---|---|
| Auditor | W08-A09 |
| Date | 2026-11-21 |
| Lines / functions | 97 / 5 (+1 const +1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for 4/5 constructors (call chain: `lumen` CLI → apps/cli/src/host.rs:926 `open_audio_session` → pipeline_host.rs:1598-1599 `NullBackend::open_stream` → null.rs:82 `format_unsupported`; play loop `AudioLimb::stage` → `NullSink::write_bytes` → null.rs:185/193/214 `partial_frame`/`ring_full`; render loop host.rs:614/726/824 → null.rs:294 `unsupported`). NO for `pump_backpressure` — its only caller `PcmPump::push` (pump.rs:206) has no production caller. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "format_unsupported\|ring_full\|pump_backpressure\|partial_frame\|error::unsupported" --glob *.rs` (every caller of every constructor); `rg -n "PcmPump" apps/ crates/` (production callers of the pump: none — tests/pcm_pipeline.rs only); `sed -n` on error.rs:23-36,53-67, audio-engine/src/error.rs:1-58, null.rs:74-129,174-260, host.rs:1815-1844, pipeline_host.rs:540-660, stub.rs:55-75; `grep -n "error::tests" audit/runs/baseline/test-workspace.log.txt` (line 163: test ran, ok) |

## 1. One-paragraph verdict
Five small, honest error constructors — no stubs, no panics, no swallowed errors — and four of the five sit on live shipped paths inside `NullBackend` (the only audio output that exists: `lumen` open → `format_unsupported` on a bad shape; play → `partial_frame`/`ring_full` on write; render → `unsupported` on an unaligned buffer). The defects are around the edges: `pump_backpressure` documents a "PCM pump" pacing regime that exists only in `tests/pcm_pipeline.rs` — no shipped entry point constructs a `PcmPump`; the stable code `audio.partial_frame` is minted in **two crates with two different stage tags**, breaking the "one auditable home" rule the sibling audio-engine module claims; `ring_full`'s doc says "the caller retries" when the shipped CLI historically treated this error as a session FATAL and now prevents it by decode-parking, never retrying; and `format_unsupported` throws away the requested format, keeping it only in a human-readable string. Answering the wave question for this file: the error taxonomy of the null device is real; the pump-backpressure half of it is seam.

## 2. What actually works (verified)
— verified by `rg -n` caller sweep → `format_unsupported` called at null.rs:82 (`Shared::open`), `ring_full` at null.rs:193/214 (`NullSink::write_bytes`/`write`), `partial_frame` at null.rs:185/252 and pump.rs:190, `unsupported` at null.rs:247/294/404/415/454, ring.rs:72/76, counter.rs:44 — all inside `NullBackend`/`SpscRing`, which the shipped CLI constructs (pipeline_host.rs:1598-1599).
— verified by `audit/runs/baseline/test-workspace.log.txt:163` → `error::tests::errors_carry_stage_and_stable_codes ... ok` (stage tag, codes, and Display strings asserted).
— verified by `audit/runs/baseline/INDEX.md` → the null-device lane is the shipped audio path (`audio.null-device` feature; sample-aac.mp4 plays "audible output: none").

## 3. Findings
### F-001 [S3]
Where: crates/audio-output/src/error.rs:53-67
Code:
```
/// The PCM pump's pacing refusal — the device is more than the
/// configured high-water mark behind (bounded-queue backpressure,
/// MP §8; the wave-14 `PcmPump`). The whole buffer is refused; the
/// caller waits for the playback clock to advance and retries.
pub fn pump_backpressure(high_water: u64, attempted: u64) -> EngineError {
```
What is wrong: unreachable from any shipped entry point. Sole caller: pump.rs:206 (`PcmPump::push`, `DropPolicy::Backpressure` arm). `PcmPump` is constructed nowhere in production — `rg -n "PcmPump" apps/ crates/` returns only audio-output's own `lib.rs` re-export, `pump.rs`, and `tests/pcm_pipeline.rs`. The shipped CLI stages audio through `AudioLimb::stage` → `NullSink::write_bytes` (pipeline_host.rs:659+), which has no high-water pacing — its bound is the ring itself (`ring_full`).
Why it matters: this constructor plus its doc sells "the PCM pump's pacing" as an output-path behavior; in the shipped binary it is dead code exercised only by one integration test. The lib.rs narrative ("the same wave shipped the PCM pipeline proof behind the seam") is accurate about it being a proof — but a reader of this file alone would believe a pacing pump exists in production. Dead public API in a `pub use` surface is maintenance debt and inflates the perceived capability of the crate.
What correct looks like: either wire `PcmPump` into the production staging path (replacing raw `write_bytes`) or mark the constructor and its doc as test/seam-only (`#[doc(hidden)]` or a `PROPOSAL` note), so the crate's public surface does not advertise an output regime that cannot run.
Confidence: HIGH — Verification: read + rg caller sweep (production reachability disproven by exhaustive grep of apps/ and crates/).

### F-002 [S3]
Where: crates/audio-output/src/error.rs:69-78 and crates/audio-engine/src/error.rs:50-58
Code:
```
/// A payload that is not a whole number of frames.
pub fn partial_frame(expected: u64, found: u64) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated { expected, found },
        },
        ErrorContext::new("audio.partial_frame"),
    )
}
```
(audio-output, `STAGE = PipelineStage::AudioOutput`; audio-engine has the byte-for-byte same constructor body with `STAGE = PipelineStage::Mix`, error.rs:13.)
What is wrong: the stable dotted code `audio.partial_frame` is minted in two crates with two different stage tags. audio-engine/src/error.rs:5-8 explicitly claims: "The codes are part of the compatibility surface (tests may key on them) and are minted here so the mapping from failure to (kind, code) has one auditable home." For this code there are two homes and two stages.
Why it matters: any consumer (test, UI, log triage) keying on `audio.partial_frame` cannot tell whether the mix/convert layer (audio-engine, `Mix` stage, e.g. convert.rs:152/264) or the output ring layer (audio-output, `AudioOutput` stage, e.g. null.rs:185/252) refused the payload — same string, same `Corrupt/Truncated` shape, different Display prefix and different layer. "Stable codes" as a compatibility surface require exactly one meaning per code; this one has two.
What correct looks like: one code per layer: `audio.mix.partial_frame` (audio-engine) vs `audio.output.partial_frame` (audio-output), or one shared constructor in media-types used by both.
Confidence: HIGH — Verification: read (both files; both callers' sites listed above).

### F-003 [S4]
Where: crates/audio-output/src/error.rs:38-41
Code:
```
/// The output queue is full — the pump's backpressure signal
/// (bounded queues, MP §8). The whole buffer is refused; the
/// caller retries.
pub fn ring_full(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: "the caller retries" describes a caller that does not exist in production. The shipped CLI's history: `audio.ring_full` was a user-visible session FATAL (host.rs:1825-1831: "THE GUI regression (the user-visible `audio.ring_full` fatal, v0.1.2)… the next staged frame is a FATAL at the audio-output stage"); the fix was prevention by decode-parking (`ring_ready`, pipeline_host.rs:568-581), not a retry loop. The only retry-on-`ring_full` loop in the workspace is a test helper (tests/sine_sink_hash.rs:273-288).
Why it matters: the doc prescribes an error-handling contract ("refused; caller retries") that the one real caller violates by design — a maintainer implementing a new staging path from this doc would add a retry loop where the engine's actual strategy is park-and-prevent. Doc-behavior drift on the crate's most consequential error (the one that was a shipped GUI fatal).
What correct looks like: doc states the actual contract: "the whole buffer is refused; production callers must prevent this (decode-park) — treating it as fatal is the v0.1.2 regression this error encoded."
Confidence: HIGH — Verification: read (host.rs:1825-1831, pipeline_host.rs:568-581, sine_sink_hash.rs:273-288).

### F-004 [S4]
Where: crates/audio-output/src/error.rs:23-36
Code:
```
pub fn format_unsupported(want: &AudioFormat) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Device {
            which: DeviceSlot::AudioOutput,
            cause: DeviceCause::FormatUnsupported,
        },
        ErrorContext::new("audio.format_unsupported"),
    )
    .with_source(std::io::Error::other(format!(
        "wanted {:?} {}ch @ {}Hz",
        want.sample, want.channels, want.rate
    )))
}
```
What is wrong: the requested format survives only in a human-readable `io::Error` source string. The typed payload (`ErrorKind::Device { cause: FormatUnsupported }`) carries no machine-readable description of `want`, so a caller cannot programmatically learn what was wanted (sample format, channel count, rate) — only that something was refused.
Why it matters: this is the error a real device adapter will return for every format the (future) backend rejects; without typed detail, UI/logic that wants to suggest a supported shape or pick a fallback has to parse an English string. The crate's own taxonomy (structured `CorruptDetail::Truncated { expected, found }` two functions below) shows the codebase knows how to carry structured detail and chose not to here.
What correct looks like: a `FormatInfo`/detail field carrying sample format, channels, rate (mirroring `CorruptDetail`), with the string as Display only.
Confidence: HIGH — Verification: read (media-types error kinds; `FormatInfo` already exists — stub.rs:67-70 uses it).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `const STAGE` | 9 | REAL | — | `PipelineStage::AudioOutput`; asserted by the test. |
| `unsupported` | 13-19 | REAL | — | Generic code-parameter constructor; no validation of `code` (caller's responsibility — all 12 in-crate call sites pass literals). Reachable via null.rs:247/294/404/415/454, ring.rs:72/76. NOTE: doc's "(shapes, **stub backends**, unknown devices)" overstates — the stub backend (`stub.rs:61-73`) mints its own `audio.os_backend_pending` inline and never calls this. Also used for state/contract violations (`audio.sink_not_open`, `audio.render_buffer_unaligned`) that are not "unsupported values" — taxonomy stretch, harmless. |
| `format_unsupported` | 23-36 | REAL | S4 (F-004) | `want` used only for the source string (not an ignored param, but typed detail discarded). Reachable: null.rs:82 on every `open_stream` with a non-F32/non-interleaved/zero-ch/zero-rate format — shipped path (pipeline_host.rs:1599). Message content untested anywhere (null.rs:663/674 assert the code only). |
| `ring_full` | 41-51 | REAL | S4 (F-003) | Correct shape (`ResourceLimit`/`QueueSize`, budget vs attempted). Reachable: null.rs:193/214 via `NullSink::write_bytes` on the shipped staging path. Doc's "caller retries" is not what production does (F-003). |
| `pump_backpressure` | 57-67 | UNREACHABLE (production) | S3 (F-001) | Correct shape; sole caller `PcmPump::push` (pump.rs:206) has no production caller. |
| `partial_frame` | 70-78 | REAL | S3 (F-002) | Correct shape; reachable via null.rs:185/252. Same stable code minted at a different stage in audio-engine/src/error.rs:50 — F-002. |
| test `errors_carry_stage_and_stable_codes` | 84-95 | REAL | — | Covers 2 of 5 constructors (ring_full, partial_frame); asserts stage, code, and Display. Would fail on STAGE/code/Display mutations. `unsupported`, `format_unsupported` (message), `pump_backpressure` untested in this file. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `errors_carry_stage_and_stable_codes` | YES | `assert_eq!(e.stage, PipelineStage::AudioOutput)` fails if STAGE changes; `assert_eq!(e.context.code, …)` fails if the code string drifts; the two `to_string().contains` asserts fail if the Display stage tag "[audio-output]" or code rendering breaks. |

Ran and passed in baseline: `audit/runs/baseline/test-workspace.log.txt:163`. Coverage gap (not a lie, a hole): 3 of 5 constructors have zero assertions in this file.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Typed error constructors for the output path" — real, stage-tagged constructors | error.rs:1-2 | REAL | 4/5 on shipped paths (§2); baseline test pass. |
| ring_full: "the caller retries" | error.rs:38-41 | FALSE (as a description of shipped behavior) | Production parks/fatals, never retries (host.rs:1825-1831, pipeline_host.rs:568-581); only a test retries (sine_sink_hash.rs:286). F-003. |
| pump_backpressure: "the PCM pump's pacing refusal" as output-path behavior | error.rs:53-57 | UNREACHABLE | `PcmPump` has no production constructor (rg sweep; F-001). |
| audio-engine: codes "minted here so the mapping from failure to (kind, code) has one auditable home" | audio-engine/src/error.rs:5-8 | FALSE (for `audio.partial_frame`) | Same code minted in this file at a different stage (F-002). |
| engine-info feature `audio.null-device` (this crate's errors are its taxonomy) | baseline INDEX.md:45-46 | REAL | NullBackend is the shipped output; its refusals flow through these constructors. |

## 7. Dead / duplicated / unreachable code
- `pump_backpressure` — production-dead (F-001); lives only via `tests/pcm_pipeline.rs`.
- `partial_frame` — duplicated stable code across crates (F-002).
- `unsupported`'s doc parenthetical "stub backends" — `stub.rs` never calls any constructor in this file (it builds its `audio.os_backend_pending` inline, stub.rs:61-73).
- `counter.rs:44`'s use of `unsupported("audio.sample_rate_invalid")` is shadowed in production by null.rs:80's earlier `format_unsupported` (see counter.rs audit §7).

## 8. Dependencies & flags
- `media-types` only (error types + `AudioFormat`), no `audio-engine`/`media-clock` dependency in this file. No features, no `unsafe`, no allocation beyond the two `format!`/`io::Error::other` strings (error construction, not RT path — the no-blocking contract governs `render_into`, not error constructors). Matches Cargo.toml. Crate compiled and tested in baseline (`test-workspace.log.txt:27,163`).

## 9. Open questions for the next agent
- pump.rs auditor: is `PcmPump`'s `Backpressure` arm's "wait for the playback clock to advance" retry protocol implemented by ANY non-test code? (rg says no — if confirmed, the whole pump module is UNREACHABLE and lib.rs's "production pump" phrasing at line 608-609 of pipeline_host.rs is a doc defect.)
- Who reads `ErrorKind::ResourceLimit.budget/attempted` — does any consumer surface these numbers, or do they exist only for tests? (grep `QueueSize` consumers in apps/ and crates/playback-engine.)
- `audio.os_backend_pending` (stub.rs) vs this file's `unsupported`: two refusal taxonomies for "backend cannot do this" in one crate — the stub.rs/device.rs auditor should decide whether that is deliberate layering or drift.
