# AUDIT: crates/audio-output/src/pattern.rs
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-21 |
| Lines / functions | 190 / 5 methods + 1 derive + 1 test helper + 4 tests |
| File verdict | UNREACHABLE (real, correct, test-executed code — but a test instrument; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `rg SinePattern` repo-wide: only `lib.rs:48,139` (doc + re-export), `pattern.rs` (self + unit tests), `tests/pcm_pipeline.rs`. `apps/cli/src/pipeline_host.rs:52-53` and `apps/web/src/host.rs:75-76` import only null-backend items. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | `rg -n SinePattern` (all constructors = tests only); `rg -n "audio_output::\|use audio_output" apps/` (CLI/web touch only null backend); `rg PcmPump` outside crate (none in production); `sed -n` on pattern.rs:1-14/46-58/73-76/82-92, lib.rs:124-143, cpal.rs (AUDIO_ADR005_ADMITTED=false at :43); `rg` in docs/feature-matrix.toml (row `platform-aout-cpal` 2352-2362), ADR-005 §82-193; baseline `test-workspace.log.txt:172-175,192,211-219,221-228`. |

## 1. One-paragraph verdict
The sine generator is genuine, correct arithmetic — `amplitude·sin(2π·freq·n/rate)` computed in f64 per instant, no table, no phase accumulator, no unsafe, no allocation in the non-test module — and its four unit tests plus the `pcm_pipeline` acceptance suite pass in the baseline run (`audit/runs/baseline/test-workspace.log.txt:172-175`, `:211-219`). But the answer to the hostile question for this file is: it is an instrument, not a feature. No production code anywhere constructs a `SinePattern`; it exists so tests have an oracle. Domain context this file sits in: the crate's only shipped sinks are `NullBackend` and `UnsupportedBackend` (`lib.rs:152-157`), the cpal gate is hard-closed (`cpal.rs:43 pub const AUDIO_ADR005_ADMITTED: bool = false;`), and the CLI plays into the null stream — so nothing this file feeds ever reaches a speaker; the user hears nothing, by architecture. The three S3s: the constructor ignores half its `AudioFormat` parameter; the test-only module is compiled into every shipped binary via the unconditional `pub mod pattern`; the module doc's "bit-for-bit by construction … independently written oracle" claim overstates what the shipped proof asserts (the pipeline test re-uses `sample_at` itself as the oracle).

## 2. What actually works (verified)
— `SinePattern::new/sample_at/fill/instants/channels` produce the documented exact sine, whole-frame discipline, and typed refusals — verified by reading the code AND by baseline execution: `pattern::tests::{fill_emits_whole_frames_and_leaves_the_tail_untouched, invalid_shapes_are_typed, quarter_rate_tone_hits_the_exact_cardinal_values, the_oracle_is_a_pure_function_of_the_instant} … ok` (`audit/runs/baseline/test-workspace.log.txt:172-175`); `tests/pcm_pipeline.rs` 4/4 ok including `clock_synced_pump_delivers_the_exact_sine_stream` (`test-workspace.log.txt:211-219`).

## 3. Findings

### F-001 [S3]
Where: crates/audio-output/src/pattern.rs:51-58
Code:
```
        let valid = freq_hz.is_finite()
            && freq_hz > 0.0
            && format.rate > 0
            && format.channels > 0
            && 2.0 * freq_hz < f64::from(format.rate)
            && amplitude.is_finite()
            && amplitude > 0.0
            && amplitude <= 1.0;
```
What is wrong: `new` takes a full `&AudioFormat` but reads only `rate` and `channels`; `format.sample` and `format.layout` are silently ignored. The generator always emits interleaved f32 regardless of the negotiated sample format/layout. ADR-005 (docs/adr/ADR-005-audio-output.md:170-171) makes "interleaved f32 … the seam's v0 contract (`error::format_unsupported` otherwise)" — this constructor does not apply that typed refusal.
Why it matters: the API accepts a shape it will not honor; an S16 or planar `AudioFormat` yields f32-interleaved values with no error, which a downstream sink negotiating that format would misinterpret byte-for-byte. Blast radius today is zero: the only in-tree constructors (this file's tests, `tests/pcm_pipeline.rs:93,152,208`) all pass `SampleFormat::F32`/`AudioLayout::Interleaved` — this is a defect of the shipped public API, not a live bug.
What correct looks like: either refuse `format.sample != F32 || format.layout != Interleaved` with the crate's typed `error::format_unsupported`, or shrink the parameter to `(rate, channels)` so the signature stops claiming format-wide meaning.
Confidence: HIGH — Verification: read (callers enumerated by `rg SinePattern`).

### F-002 [S3]
Where: crates/audio-output/src/lib.rs:129 and crates/audio-output/src/lib.rs:139
Code (lib.rs:129):
```
pub mod pattern;
```
Code (lib.rs:139):
```
pub use pattern::SinePattern;
```
What is wrong: a test-pattern generator is an unconditional public module of a production crate that `apps/cli`, `apps/web`, and `platform-{linux,windows,macos}` all depend on (their Cargo.tomls), so it is compiled into every shipped binary — yet nothing outside test code ever constructs it (`rg SinePattern`: lib.rs re-export, pattern.rs's own tests, tests/pcm_pipeline.rs — that is the complete list). The shipped `lumen` binary carries a public tone-generator API no entry point uses.
Why it matters: dead code shipped as public API; it also invites exactly the misreading this audit wave is hostile to — "pattern.rs exists, so audio generation works" — when the file is a test oracle by design and by fact.
What correct looks like: gate the module `#[cfg(test)]`-style (dev-only helper, like `testkit` is dev-only per Cargo.toml:37-41), or move the oracle next to its sole consumer; alternatively, if it is meant to be a feature, wire it to a real shipped surface (e.g. a `lumen tone` command).
Confidence: HIGH — Verification: read + `rg SinePattern` repo-wide; `rg "audio_output::|use audio_output" apps/` shows only null-backend imports.

### F-003 [S3]
Where: crates/audio-output/src/pattern.rs:8-12
Code:
```
//! computed in **f64 from first principles** (no phase accumulator, no
//! lookup table) and cast to f32. Because every sample is a pure
//! function of its instant, the generator and any independently
//! written oracle agree **bit-for-bit by construction** — the property
//! the PCM-pipeline proof asserts against the sink's captured stream.
```
What is wrong: two overstatements. (a) "any independently written oracle agree bit-for-bit by construction" is false as stated: f64 results are expression-dependent (e.g. `sin(TAU*(f*n/r))` vs `sin(TAU*f*n/r)` can differ in the last ulp), so only an oracle using the *identical* expression is guaranteed bit-stable — "any independently written" is exactly the case floating point does not guarantee. (b) "the property the PCM-pipeline proof asserts": the proof (`crates/audio-output/tests/pcm_pipeline.rs:132-138`) compares the captured stream against `pattern.sample_at(instant)` — the same struct's own method. It is a self-oracle: it proves pipeline transparency (pattern → pump → ring → null render → capture is the identity), not agreement with an independent implementation. No second sine implementation is asserted anywhere in the suite.
Why it matters: the doc comment inflates the evidence the shipped tests provide; absolute value truth rests on four cardinal-point assertions (`pattern.rs:128-131`: 0, 1, -1, 1) plus trust in `f64::sin`. Under this audit's rules, evidence claims that promise more than the body proves are findings.
What correct looks like: the doc should say — the proof asserts the capture equals *this generator's* oracle (transparency); bit-stability holds for the identical expression; any independent oracle must reuse the exact expression `TAU * freq * n / rate` (amplitude applied after `sin`).
Confidence: HIGH — Verification: read of doc + tests/pcm_pipeline.rs:130-138.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `SinePattern::new` | 46-69 | REAL | S3 | F-001: `format.sample`/`format.layout` ignored. Validates freq finite/positive, rate/channels > 0, strict sub-Nyquist (`2·f < rate` — Nyquist itself refused, matching the doc), amplitude finite in (0,1]. O(1), no panic, no alloc. Typed `audio.pattern_invalid` via `crate::error::unsupported` (error.rs:13-19, verified). |
| `SinePattern::sample_at` | 73-76 | REAL | — | Pure function of instant; single f64 expression; f32 cast. `instant as f64` rounds beyond 2^53 instants (≈6.6 years at 44.1 kHz per channel-instant) — deterministic, so the exactness-by-identical-expression property survives; theoretical only. No panic, no alloc. |
| `SinePattern::fill` | 82-92 | REAL | — | O(n) writes, zero alloc, zero panic: `frames = len/channels`, max index `frames*channels ≤ out.len()`. Whole-frames discipline: unaligned tail untouched (test-proven with a NAN sentinel, pattern.rs:139-142). Cursor advances by frames only. |
| `SinePattern::instants` | 95-97 | REAL | — | Trivial getter; monotonic (only incremented). |
| `SinePattern::channels` | 100-102 | REAL | — | Trivial getter. |
| `derive(Debug, Clone)` | 28 | REAL | — | Derive only; `Clone` used by the purity test (pattern.rs:159). |
| `tests::stereo_44k` (helper) | 111-118 | REAL | — | Fixture builder; no assertions. |
| `tests::quarter_rate_tone_hits_the_exact_cardinal_values` | 120-132 | REAL | — | Asserts exact 0.0/1.0/-1.0 at 11 025 Hz (phase n·π/2); comment correctly notes sin(π) residual. |
| `tests::fill_emits_whole_frames_and_leaves_the_tail_untouched` | 134-153 | REAL | — | NAN sentinel proves tail untouched; both channels of a frame equal; continuation instant exact. |
| `tests::the_oracle_is_a_pure_function_of_the_instant` | 155-168 | REAL | — | State advance does not perturb the oracle; clones before fill. |
| `tests::invalid_shapes_are_typed` | 170-189 | REAL | — | 7 invalid shapes (zero/negative freq, Nyquist, above-Nyquist, amp 0 and 2) + zero-rate; asserts the exact code `audio.pattern_invalid`. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| quarter_rate_tone_hits_the_exact_cardinal_values | YES | Mutation: use degrees instead of TAU-radians, apply amplitude inside sin, or an off-by-one in the phase → sample_at(1) ≠ 1.0 exactly. Asserts literal values, not tautologies. |
| fill_emits_whole_frames_and_leaves_the_tail_untouched | YES | Mutation: fill the tail (`out[4]` written) → NAN sentinel assertion fails; wrong frame math (per-sample instead of per-frame) → channel-equality assertions fail; cursor not advanced → continuation fill (pattern.rs:149-152) returns wrong values. |
| the_oracle_is_a_pure_function_of_the_instant | YES | Mutation: bake state into sample_at (e.g. use next_instant as base) → after two fills, `a.sample_at(0)` ≠ `b.sample_at(0)`. |
| invalid_shapes_are_typed | YES | Mutation: loosen any of the 8 validation clauses (e.g. accept `freq == rate/2`) → the corresponding `.err().must()` panics; wrong error code → `assert_eq!(err.context.code, …)` fails. |
| (consumer) tests/pcm_pipeline.rs::clock_synced_pump_delivers_the_exact_sine_stream | YES | Compares 44 100 × 2 captured samples against `sample_at` per instant; any pipeline mutation (drop, reorder, silence, truncation) breaks equality — but note F-003(b): the oracle is the generator itself, so a *sine-math* mutation consistent in both places would pass this test and be caught only by the cardinal-point unit test. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "wave-14 PCM pipeline proof … clock-synced bounded-queue pump, exact sine oracle, drop/backpressure policy" (as evidence for row `platform-aout-cpal` "Default audio output path (cpal-first with escape hatches) on all desktops", status partial×3/not_started web) | docs/feature-matrix.toml:2352-2362 | REAL (as a *seam* proof) — but it evidences the null-device seam, not any desktop output path | Baseline `tests/pcm_pipeline.rs` 4/4 ok (test-workspace.log.txt:211-219); `AUDIO_ADR005_ADMITTED=false` (cpal.rs:43) — the row's own notes concede "no real-device evidence exists (typed marker refuses on every platform)" (feature-matrix.toml:2362). Honest as written; the partial status rests on test-only seam evidence. |
| "the generator and any independently written oracle agree bit-for-bit by construction — the property the PCM-pipeline proof asserts" | pattern.rs:8-12 | FALSE (overbroad; see F-003) | pcm_pipeline.rs:135 uses `pattern.sample_at` as the oracle — self-oracle; FP expression-dependence refutes "any independently written". |
| "std-only, no `unsafe`, no allocation" | pattern.rs:3 | REAL | Read of the whole file: no `unsafe`, no allocation in the non-test module; `media-types`/`crate::error` are the only imports (pattern.rs:22-25). |
| "A frame carries the same value on every channel (a mono-compatible test tone; channel *mapping* is downstream negotiation, doc 03)" | pattern.rs:13-14 | REAL | fill writes the same `value` to every channel (pattern.rs:86-88); test asserts it (pattern.rs:144-147). |
| "the PcmPump pacing proof (wave 14: `audio_output::pump` + `audio_output::pattern`) must hold on-device" (post-admission conformance plan item 3) | docs/adr/ADR-005-audio-output.md:189-193 | REAL-UNPROVEN (future obligation, honestly labeled as a conformance *plan*) | No on-device run exists; ADR records cpal NOT admitted (ADR-005:127-165) and CI lacks audio devices. |
| ADR-005: "crates/audio-output — 3,097 LOC, std-only, zero dependencies; 32 tests" | docs/adr/ADR-005-audio-output.md:84-86 | REAL (counts shift with wave-14 additions) | Baseline lib suite 33 passed (test-workspace.log.txt:192) + 4 pcm_pipeline + 3 sine_sink_hash + 1 clock_truth + 1 zero_alloc — consistent scale; "zero dependencies" means zero external deps (Cargo.toml:32-35 has 3 path deps). |

## 7. Dead / duplicated / unreachable code
- `SinePattern` is unreachable from `lumen`/`lumen-tauri` (F-002) — it is dead weight in the shipped binary and live tooling in the test suite.
- Adjacent-but-distinct sine assets: `tests/sine_sink_hash.rs` uses the testkit corpus NCO tone plan (`TONE_RATE`, `corpus_pcm`, fixture bytes — sine_sink_hash.rs:49), NOT `SinePattern`. Two sine sources with different purposes (corpus byte-stability vs pipeline oracle); not copy-paste duplication, but a future agent may want to confirm no third appears.
- Nothing else in the file is dead: every method has either a test consumer or the re-export.

## 8. Dependencies & flags
- Imports: `media_types::{EngineError, frame::AudioFormat}` and `crate::error` only (pattern.rs:22-25) — matches the crate's std-only claim. `testkit` is dev-only (Cargo.toml:37-41) — the `Must` helper used by tests is a panicking unwrap with caller context (crates/testkit/src/lib.rs:44-56), appropriate for fixture contracts.
- No feature flags gate this module: `pub mod pattern;` is unconditional (lib.rs:129), so every shipped build compiles it. cpal is NOT a dependency of this crate (not admitted; the seam lives in cpal.rs as a typed refusal).

## 9. Open questions for the next agent
- pump.rs / null.rs / render.rs owners: confirm whether the shipped CLI playback path writes decoded PCM straight into `NullSink` (apps/cli/src/pipeline_host.rs:52-53 suggests yes) — i.e. whether `PcmPump` (like `SinePattern`) is test-proven but CLI-unwired. That decides if the whole wave-14 "PCM pipeline proof" is architecture evidence only.
- `AudioFormat.channels` is `u32` (media-types frame.rs:60); `as usize` casts here are safe on all supported targets — fine, but the platform-* owners should confirm no u16 truncation elsewhere in the seam.
