# AUDIT: crates/audio-engine/src/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 119 / 8 (7 constructors + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open/play` → apps/cli/src/pipeline_host.rs:51 `audio_engine::{AudioPipeline, PipelineSpec}` → pipeline.rs:115,264,286,289,293,332 + mix.rs:37,59,97 + channel_map.rs:42,97,102,147,151 + convert.rs:46,50,152,264 construct these errors on every shipped audio-path validation failure; `bad_gain` is on the Volume path audited in my volume.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg for each constructor name over crates/audio-engine/src (call-site census: all 7 constructors have live callers — zero dead); sed -n mix.rs:84-106 (doc-vs-code code mismatch found, cross-file); sed -n pipeline.rs:270-349 (rate-mismatch gate + micro-resampler wiring for the hostile question); head audit/files/crates/audio-engine/src/resample.rs.audit.md (W07-A02's verdict on the resampler); grep baseline log (error test ok, :83) |

## 1. One-paragraph verdict
A tiny, fully-wired error-constructor module: every one of the seven constructors has live call sites on the shipped audio path, all produce `EngineError` at stage `Mix` with stable dotted codes, and the three Corrupt-shape constructors carry structured details (`Truncated`, `InvalidStructure`, `Timing`). The single test executed in the baseline run and pins stage, code, and rendered strings for three of the seven; the other four are pinned by their consumers' tests (volume.rs, mix.rs, channel_map.rs). Defects are hygiene-level only: a stale comment describing the micro-resampler as a future card when it has landed and is wired; four byte-identical `Unsupported` constructors inflating the "one auditable home" claim; and `chunk_alignment` carrying no lengths in its structured detail. This file also settles one arm of the wave-7 hostile question: general sample-rate conversion does not exist — the pipeline refuses any frame whose rate ≠ the negotiated mixer rate through this file's `unsupported_format("audio.rate_mismatch")` (pipeline.rs:288-289).

## 2. What actually works (verified)
— All 7 constructors wired: `bad_gain` (volume.rs:46,49,64), `channel_range` (channel_map.rs:42,97,102; pipeline.rs:259,293), `unsupported_format` (pipeline.rs:115,289; effects/eq.rs:135,138), `partial_frame` (convert.rs:152,264), `plane_mismatch` (channel_map.rs:147,151; convert.rs:46,50), `track_range` (mix.rs:37,59; pipeline.rs:264,286), `chunk_alignment` (mix.rs:97,103; pipeline.rs:332). Zero dead constructors.
— Kind/discipline split verified: value problems → `Unsupported`, data-shape problems → `Corrupt` with structured `CorruptDetail` — consistent with the doc taxonomy (error.rs:15-49).
— `error::tests::codes_are_stable_and_lowercase ... ok` in baseline (test-workspace.log.txt:83) — pins stage Mix, `audio.partial_frame` code, "[mix]" and "truncated" renderings, the `Unsupported { format: None }` shape, and the Timing rendering.
— Hostile-question arm verified: pipeline.rs:288-289 refuses `frame.format.rate != spec.rate` with this file's `audio.rate_mismatch` — there is NO general SRC in the shipped audio path; the only resampler is the drift micro-resampler, whose arithmetic W07-A02 hand-verified as correct but which is unreachable in shipped builds (governor armed only by tests — audit/files/crates/audio-engine/src/resample.rs.audit.md).

## 3. Findings
### F-001 [S4] Stale comment: "the micro-resampler is a later card" — it has landed
Where: crates/audio-engine/src/error.rs:36-37
Code:
```rust
/// A format the v0 audio path does not convert (e.g. a rate mismatch —
/// the micro-resampler is a later card, doc 03 §4.7 e).
```
What is wrong: The micro-resampler exists in-tree (`resample.rs`), is constructed by the pipeline (`MicroResampler::new` at pipeline.rs:130), and is wired at the drift seam (pipeline.rs:347-353, `set_correction` at pipeline.rs:164). What remains future work is *general* sample-rate conversion (48 kHz→44.1 kHz) — which resample.rs's own docs also disclaim. The comment conflates the two and describes shipped code as a later card.
Why it matters: A reader auditing rate-conversion capability gets the wrong tree state from this comment; the honest statement is "general SRC is not supported; the drift micro-resampler exists (pipeline.rs:86)".
What correct looks like: Update the parenthetical to name the drift micro-resampler as landed and general SRC as the actual gap.
Confidence: HIGH — Verification: read (pipeline.rs:86,130,164,347-353; resample.rs exists; W07-A02's audit confirms wiring).
### F-002 [S4] Four byte-identical `Unsupported` constructors — the "one auditable home" is seven near-clones
Where: crates/audio-engine/src/error.rs:19-44 and 73-79
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
What is wrong: `bad_gain`, `channel_range`, `unsupported_format`, and `track_range` have literally identical bodies (stage Mix, `Unsupported { format: None }`, caller's code). The module doc (error.rs:5-8) sells "the mapping from failure to (kind, code) has one auditable home" — the actual distinct mappings are three (Unsupported+code, Corrupt/Truncated+fixed code, Corrupt/InvalidStructure+code, Corrupt/Timing+fixed code); the four Unsupported clones add names, not mappings.
Why it matters: Mild duplication; a new constructor is copy-paste bait, and the doc overstates the consolidation.
What correct looks like: One `pub fn unsupported(code) -> EngineError` plus the three corrupt-shape constructors; or keep the names but say honestly that they are readability aliases.
Confidence: HIGH — Verification: read (bodies compared).
### F-003 [S4] `chunk_alignment` carries no lengths — the most likely wiring bug is the least observable error
Where: crates/audio-engine/src/error.rs:81-94
Code:
```rust
pub fn chunk_alignment() -> EngineError {
    EngineError::new(
        STAGE,
        ErrorKind::Corrupt {
            detail: CorruptDetail::Timing,
        },
        ErrorContext::new("audio.chunk_length_mismatch"),
    )
}
```
What is wrong: `CorruptDetail::Timing` has no payload; the doc admits "the offending lengths ride in the caller's test output, not the error" (error.rs:83-85). A cross-track frame-count disagreement in production reports *which code* fired but not *what lengths disagreed* — the operator cannot diagnose the misaligned producer from the error alone.
Why it matters: Observability hole on the documented "most likely" failure shape of a multi-track mix; the vorbis/pcm taxonomies this project ships carry structured `Truncated { expected, found }` for exactly this reason.
What correct looks like: Either extend `CorruptDetail::Timing` with lengths (media-types change) or log the lengths at the mix.rs/pipeline.rs call sites before returning.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `bad_gain` | 19-25 | REAL | — | Unsupported+code; callers volume.rs:46,49,64; F-002 clone |
| `channel_range` | 28-34 | REAL | — | F-002 clone; callers channel_map/pipeline |
| `unsupported_format` | 38-44 | REAL | — | F-001 stale doc; callers pipeline.rs:115,289 (the rate-mismatch refusal), eq.rs |
| `partial_frame` | 50-58 | REAL | — | Corrupt/Truncated with expected/found ✓ (the structured shape F-003 wants for Timing) |
| `plane_mismatch` | 62-70 | REAL | — | Corrupt/InvalidStructure; callers channel_map/convert |
| `track_range` | 73-79 | REAL | — | F-002 clone; callers mix/pipeline |
| `chunk_alignment` | 86-94 | REAL | — | Corrupt/Timing, no lengths (F-003); callers mix.rs:97,103, pipeline.rs:332 |
| test `codes_are_stable_and_lowercase` | 100-118 | REAL | — | pins 3 of 7 constructors' shapes/renderings; name promises a lowercase check the body does not perform (nit); the other 4 constructors are pinned by consumer tests (volume.rs:162, mix.rs:198, channel_map.rs:249-254) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| codes_are_stable_and_lowercase | YES (for what it covers) | exact stage/code/rendering assertions for partial_frame, unsupported_format, chunk_alignment; pattern-match on the Unsupported kind. Not covered *here*: bad_gain, channel_range, plane_mismatch, track_range — each pinned at its call-site tests instead (verified by rg). Name/body mismatch: no lowercase assertion exists. |

Executed ok in baseline (test-workspace.log.txt:83).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every failure in this crate is an EngineError at stage Mix ... stable dotted context code" | error.rs:3-8 | REAL | all 7 constructors + consumers verified; test pins stage |
| "the micro-resampler is a later card" | error.rs:36-37 | FALSE (stale) | F-001 — resample.rs exists, pipeline.rs:130,164 wires it; only *general* SRC is future work |
| "The offending lengths ride in the caller's test output, not the error" | error.rs:83-85 | REAL (honest admission) | CorruptDetail::Timing has no payload — F-003 |

## 7. Dead / duplicated / unreachable code
- Zero dead constructors (all call-sited — census in §2).
- Duplication: the four Unsupported clones (F-002).

## 8. Dependencies & flags
- `media_types::error::*` only; no external deps, no `unsafe`, no feature gate; part of the unconditional audio-engine crate.

## 9. Open questions for the next agent
- mix.rs owner (no audit file yet in audit/files/crates/audio-engine/src/): mix()'s doc (mix.rs:85-88) promises `audio.planar_plane_mismatch` for ragged planes, but the code returns `error::chunk_alignment()` (`audio.chunk_length_mismatch`) for BOTH the ragged-plane and frame-count cases (mix.rs:101-104, verified by sed). Doc/code mismatch in the consumer, plus the ragged case arguably belongs to `plane_mismatch`.
- W07-A02 (resample.rs owner): your F-001 panic-path finding (in_frames > plane len) is reachable only through pipeline-internal calls today; the pipeline passes staged geometry so it cannot trip — agreed with your unreachable verdict.
