# AUDIT: crates/audio-engine/src/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 119 / 7 constructors + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every constructor is called from product modules (channel_map.rs ×8, convert.rs ×15, effects.rs ×6, mix.rs ×6, pipeline.rs ×8, volume.rs ×3), reached from apps/cli/src/pipeline_host.rs:51's audio limb. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p audio-engine --lib error` → 5 passed (incl. `codes_are_stable_and_lowercase`, plus the pipeline tests that consume these constructors). |

## 1. One-paragraph verdict
A clean, genuinely central typed-error mint: seven total, panic-free constructors, all at stage `Mix` with stable dotted codes, and the "one auditable home" claim is TRUE — rg proves zero `EngineError::new`/`PipelineStage::` construction anywhere else in the crate (only a doc mention in lib.rs:50), with 46 call sites across six product modules. Kinds are correctly split (Unsupported for value/shape refusals, Corrupt::Truncated for partial frames, Corrupt::InvalidStructure for ragged planes, Corrupt::Timing for cross-track chunk mismatch). Findings are hygiene: four constructors have byte-identical bodies (only docs differ), the `unsupported_format` doc is stale (it says the micro-resampler "is a later card" — resample.rs landed as card 3.B1, only the ±1000 ppm drift slice exists), and the one test's name promises a lowercase check its body never performs.

## 2. What actually works (verified)
— `cargo test -p audio-engine --lib error` → 5/5 passed; `codes_are_stable_and_lowercase` pins stage, `audio.partial_frame`, the rendered `[mix]`/`truncated` strings, the `Unsupported{format: None}` kind shape, and `audio.chunk_length_mismatch` + `timing`.
— Central-home claim: `rg "EngineError::new|PipelineStage::" crates/audio-engine/src/` → matches only in error.rs (+ lib.rs doc comment). Verified.

## 3. Findings
F-001 [S4]
Where: crates/audio-engine/src/error.rs:19-44, 73-79
Code:
```rust
pub fn bad_gain(code: &'static str) -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new(code),
    )
}
```
What is wrong: `bad_gain`, `channel_range`, `unsupported_format`, and `track_range` are four byte-identical bodies — the semantic distinction lives only in the doc comments and the caller-chosen code string. Defensible as a self-documenting surface, but it is four copies of one function.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/audio-engine/src/error.rs:36-37
Code:
```rust
/// A format the v0 audio path does not convert (e.g. a rate mismatch —
/// the micro-resampler is a later card, doc 03 §4.7 e).
```
What is wrong: stale doc — the micro-resampler EXISTS (crates/audio-engine/src/resample.rs, card 3.B1, landed; the ±1000 ppm drift-correction slice, exercised by audio_bench's drift probe at 326 MB/s). What remains true is that GENERAL rate conversion is absent (pipeline refuses `audio.rate_mismatch`), but "the micro-resampler is a later card" is false as written today.
Confidence: HIGH — Verification: read + resample.rs existence + my audio_bench release run.
F-003 [S4]
Where: crates/audio-engine/src/error.rs:101
Code:
```rust
    fn codes_are_stable_and_lowercase() {
```
What is wrong: the test name promises a lowercase check; the body asserts stability, stage, and rendered strings but never checks case (no `chars().all(char::is_lowercase)`/equivalent). A future `Audio.Partial_Frame` code would pass this test and keep the name lying.
Confidence: HIGH — Verification: read of the full test body (:100-118).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `bad_gain` | 19-25 | REAL | S4 | identical body (F-001); callers: volume.rs |
| `channel_range` | 28-34 | REAL | S4 | F-001; callers: pipeline/channel_map |
| `unsupported_format` | 38-44 | REAL | S4 | F-001 + stale doc (F-002); the `audio.rate_mismatch` site (pipeline.rs) is the AAC implicit-HE consequence path (decoder.rs F-001) |
| `partial_frame` | 50-58 | REAL | — | Corrupt::Truncated with structured expected/found; tested |
| `plane_mismatch` | 62-70 | REAL | — | Corrupt::InvalidStructure; tested via convert.rs ragged-plane lanes |
| `track_range` | 73-79 | REAL | S4 | F-001 |
| `chunk_alignment` | 86-94 | REAL | — | Corrupt::Timing; doc honestly notes lengths are NOT in the error (reported at the site); tested |
| tests | 100-118 | REAL | S4 | name-vs-body gap (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| codes_are_stable_and_lowercase | YES (for what it checks) | stage/code/rendered-string/kind pins; would fail on stage move, code rename, kind change, or Display drift |
| (name vs body) | NO | the promised lowercase invariant is unasserted (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every failure in this crate is an EngineError at stage Mix" | :3-4 | REAL | rg: no other construction site in the crate |
| "the mapping from failure to (kind, code) has one auditable home" | :5-8 | REAL | same rg; 46 external call sites |
| "the micro-resampler is a later card" | :36-37 | FALSE (stale) | resample.rs landed (F-002) |
| "The offending lengths ride in the caller's test output, not the error" | :83-85 | REAL | constructor carries no length payload (read) |

## 7. Dead / duplicated / unreachable code
No dead code (all seven constructors have callers — call-site counts in the reachability row). Duplication: F-001.

## 8. Dependencies & flags
media-types only. No unsafe, no features, no panics.

## 9. Open questions for the next agent
- `audio.rate_mismatch` (unsupported_format) is what a decoder-declared rate ≠ mixer rate produces today. Combined with the AAC implicit-HE finding (decoder.rs F-001): a half-rate-decoded HE-AAC stream either stalls its audio limb per frame on this error or negotiates a wrong mixer rate — whichever it is, the user sees no honest "HE-AAC unsupported" message. Worth a cross-file ticket.
