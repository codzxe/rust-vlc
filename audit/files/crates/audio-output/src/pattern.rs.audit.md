# AUDIT: crates/audio-output/src/pattern.rs
| Field | Value |
|---|---|
| Auditor | W08-A10 |
| Date | 2026-11-24 |
| Lines / functions | 190 / 5 (new, sample_at, fill, instants, channels) + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for SinePattern: apps/cli uses audio_output::{NullBackend,NullConfig,NullStream,null::*} only (pipeline_host.rs:52-53, host.rs:79); the sole callers are the crate's own integration tests (tests/pcm_pipeline.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg SinePattern across repo (exit 0); rg audio_output in apps/ (exit 0); sed -n quote checks (exit 0); baseline cargo-test-workspace.txt:167-170 |

## 1. One-paragraph verdict
This is a real, small, correct test-pattern generator with honest tests: validation
rejects non-finite/non-positive frequencies, Nyquist and above, zero rate/channels, and
out-of-range amplitude; `sample_at` is a pure function of the instant (no phase
accumulator, so no drift by construction); `fill` writes whole interleaved frames and
provably leaves unaligned tails untouched. All four unit tests ran and passed in the
orchestrator baseline. The only defects are two doc-level overclaims: "any independently
written oracle agrees bit-for-bit" is false in general (ulp differences from expression
order/libm; the in-repo proof oracle is `sample_at` itself), and the API accepts a full
`AudioFormat` while ignoring its `sample` field. No production caller exists — it is a
shared fixture for the crate's integration proof, which is the idiomatic way to expose
it (tests/ cannot see #[cfg(test)] items).

## 2. What actually works (verified)
— Baseline cargo-test-workspace.txt:167-170: all 4 pattern tests passed
  (`fill_emits_whole_frames_and_leaves_the_tail_untouched`, `invalid_shapes_are_typed`,
  `quarter_rate_tone_hits_the_exact_cardinal_values`, `the_oracle_is_a_pure_function_of_the_instant`).
— Static check of the math: freq=rate/4 gives phase n·π/2; asserted cardinal values
  0/1/-1/1 at instants 0/1/3/5 are correct for sin(n·π/2)·1.0.
— rg: no `SinePattern` reference outside audio-output's src/tests — reachability
  statement above is exhaustive.

## 3. Findings
F-001 [S4]
Where: crates/audio-output/src/pattern.rs:8-12
Code:
```
//! lookup table) and cast to f32. Because every sample is a pure
//! function of its instant, the generator and any independently
//! written oracle agree **bit-for-bit by construction** — the property
//! the PCM-pipeline proof asserts against the sink's captured stream.
```
What is wrong: "any independently written oracle … bit-for-bit" is false as stated:
bit-for-bit agreement requires the identical f64 expression (`TAU * freq * n / rate`,
this order) AND the same libm `sin`. An oracle written `(n as f64 * (freq/rate) * TAU).sin()`
or on another libm can differ in the last ulp. And the actual proof does not use an
independent oracle at all — tests/pcm_pipeline.rs:135,187 compare the captured stream
against `pattern.sample_at(instant as u64)`, the same function that filled the buffer.
Why it matters: the exactness contract is the file's headline claim; it holds for
"the same expression" (which is what the proof exercises) but is worded as a universal.
What correct looks like: "the generator and any oracle evaluating the same expression
agree bit-for-bit; the proof asserts the captured stream equals `sample_at`."
Confidence: HIGH. Verification: read + rg of the proof file.

F-002 [S4]
Where: crates/audio-output/src/pattern.rs:46-50
Code:
```
    pub fn new(
        freq_hz: f64,
        format: &AudioFormat,
        amplitude: f32,
    ) -> Result<SinePattern, EngineError> {
```
What is wrong: `AudioFormat` carries `sample: SampleFormat` and `layout: AudioLayout`,
both ignored — the generator emits f32 interleaved values regardless (the `out: &mut
[f32]` type forces the value type). Constructing a pattern "for" an S16 planar format
silently produces an F32 interleaved tone. Why it matters: the accepted type promises
format fidelity it does not deliver; a caller reading the signature believes the pattern
honors the format. What correct looks like: take `rate: u32, channels: u32` directly, or
validate/require `SampleFormat::F32` + `AudioLayout::Interleaved`.
Confidence: HIGH. Verification: read (no use of `format.sample`/`format.layout` in the file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SinePattern::new | 46-69 | REAL | — | Validation complete for freq/rate/channels/amplitude incl. Nyquist boundary `2·f < rate`; errors typed `audio.pattern_invalid` (S4: ignores format.sample/layout, F-002) |
| SinePattern::sample_at | 73-76 | REAL | — | Pure function of instant; f64 first-principles, f32 cast; no accumulator → no drift; huge-instant precision loss is deterministic and irrelevant at audio timescales |
| SinePattern::fill | 82-92 | REAL | — | Whole-frames only, tail untouched, cursor advances by frames; returns frames; no allocation; no unsafe |
| SinePattern::instants | 95-97 | REAL | — | Trivial accessor, correct semantics (instants = frames emitted) |
| SinePattern::channels | 100-102 | REAL | — | Trivial accessor |
| tests (4) | 120-189 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| quarter_rate_tone_hits_the_exact_cardinal_values | YES | Asserts exact f32 values 0.0/1.0/-1.0/1.0 at instants 0/1/3/5; any phase or amplitude error shifts them (sin(n·π/2) exact in f64) |
| fill_emits_whole_frames_and_leaves_the_tail_untouched | YES | Pre-poisons the buffer with NaN and asserts the unaligned slot stays NaN; asserts oracle identity per channel; asserts continuation from instant 2 — catches cursor-advance bugs (+= out.len() or += samples would fail `instants()==2` / `next[0]==sample_at(2)`) |
| the_oracle_is_a_pure_function_of_the_instant | YES | Compares a partially-drained generator against a fresh clone at 5 instants incl. 599 (beyond generated range) — catches state leakage into sample_at |
| invalid_shapes_are_typed | YES | 6 boundary combos incl. Nyquist-equal (22050 at 44100) + zero-rate; asserts the error code string |
Gap (minor): channels=0 and NaN/∞ amplitude not in the table, though validated; both would need a broken validator to matter.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no `unsafe`, no allocation" | pattern.rs:2-3 | REAL | No unsafe blocks; fill writes in place (read) |
| "every sample is a pure function of its instant" | pattern.rs:9-10 | REAL | sample_at reads only fields set at construction; test proves it |
| "the generator and any independently written oracle agree bit-for-bit by construction" | pattern.rs:10-12 | FALSE (overclaim) | F-001: proof oracle is sample_at itself (pcm_pipeline.rs:135,187); ulp sensitivity to expression/libm |
| "an unaligned tail … left untouched" | pattern.rs:79-81 | REAL | NaN-tail test asserts it |

## 7. Dead / duplicated / unreachable code
No production caller (rg across repo): SinePattern is exported (lib.rs:139) and used
only by tests/pcm_pipeline.rs. Not dead in the crate's own evidence chain, but dead
weight in the shipped lumen-cli binary (apps/cli imports only NullBackend/null —
pipeline_host.rs:52-53). Exporting fixtures from src/ for integration tests is
idiomatic Rust, so no defect beyond the reachability note.

## 8. Dependencies & flags
media-types (AudioFormat/EngineError), testkit (dev). No features, no unsafe.

## 9. Open questions for the next agent
- If the feature matrix cites the PCM-pipeline proof as "audio output" evidence, note
  that the proof's source is this generator, not a decoder (W08-A08 already flagged the
  suite as SHALLOW on that axis).
