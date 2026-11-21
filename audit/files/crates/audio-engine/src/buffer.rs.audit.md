# AUDIT: crates/audio-engine/src/buffer.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 95 / 3 prod items (AudioBuffer, frames, AudioSink trait) + 1 test + 1 helper |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → `apps/cli/src/pipeline_host.rs:661-662` `pipeline.mix()`/`take_output()` → `crates/audio-engine/src/pipeline.rs:398` returns this `AudioBuffer` → sink `write`; `pipeline_host.rs:619` `AudioSink::latency`. Implementations shipped: `audio-output/src/null.rs:244,408` (NullSink/NullBackend — the shipped `audio.null-device` lane) and `stub.rs:77` (UnsupportedBackend). Executed in baseline: AVI first-vertical play session (rc-smoke 6/6) pumps the audio limb end to end.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln 'AudioBuffer'` (users: audio-engine pipeline, audio-output null/stub/pump, 5 test files); `rg -n 'impl AudioSink'` (3 implementors); `rg -n 'fn take_output\|pts' pipeline.rs` (pts provenance :306,:376-380); `sed -n` quote verification; hand-check of `frames_in` delegation against media-types frame.rs:74-81 (verified during the convert.rs audit). |

## 1. One-paragraph verdict
A 95-line interface module that does exactly what it claims: an interleaved-bytes buffer with honest frame math (whole-instants or `None`, no rounding — delegated to media-types' zero-guarded `frames_in`), and a two-method sink trait whose contract comments match both shipped implementors. Both types are live in the shipped null-device play lane and were exercised by the baseline AVI session. Zero findings; the negative result is backed by the itemized checks in §2.

## 2. What actually works (verified) — exhaustive check list for the zero-finding claim
— verified `frames()` (:35-37) delegates to `AudioFormat::frames_in` (media-types frame.rs:74-81), which returns `None` for `frame_bytes == 0` and for non-whole lengths — no divide-by-zero, no rounding; the doc's "data corruption … never a guess" matches the implementation, and the test pins both sides (:86, :92).
— verified the "bytes are always interleaved" doc (:17-19) against the producing path: `pipeline.rs` stage D fills slots via `planes_to_f32_interleaved_into` (convert.rs:256-278, exact-length checked) — the pipeline cannot emit a partial frame, so the doc's "the pipeline never emits that shape" (:32-33) holds structurally.
— verified `pts` provenance: pipeline.rs:306 stores the first staged frame's pts, :376-380 forwards it into the `AudioBuffer`; test `pipeline.rs:509` asserts `buffer.pts == frame.pts` — the field doc (:26-27) is honored, not aspirational.
— verified the `AudioSink` doc claims against implementors: null backend's `latency` answers `None` (the "discard sink" case, :61), errors are typed EngineErrors (MP §8 claim matches null.rs's typed refusals); the SPSC/no-shared-lock guidance (:46-48) is a constraint on implementors, and the shipped null path has no callback lock to violate.
— verified derives (`Debug, Clone, PartialEq, Eq` on AudioBuffer) are supported by `SharedBytes`/`AudioFormat`/`Stamp` (all implement them — compiles in the baseline build, run #1).
— verified no dead code: every item has shipped callers (AudioBuffer: pipeline.rs:101/398; AudioSink::write/latency: pipeline_host.rs:619/662 + audio-output pump.rs); no duplication (audio-output's types reference, not re-define, these).

## 3. Findings
None. (Checked for: rounding in frame math, zero-channel division, panics, doc-vs-code drift on pts/interleaving/latency, dead trait methods, misuse of `Send` bound — the bound is required by the pump thread model and satisfied by all three implementors.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AudioBuffer (struct) | 20-28 | REAL | — | Plain data; doc claims verified (§2). |
| AudioBuffer::frames | 30-38 | REAL | — | Delegates to the guarded `frames_in`; total, no panic. |
| AudioSink (trait) | 40-63 | REAL | — | 2 methods, both implemented by 3 shipped/test types; error taxonomy documented and matched. |
| tests: stereo_f32 helper | 70-77 | helper | — | — |
| tests: buffer_frame_math | 79-93 | REAL test | — | Both outcomes asserted. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| buffer_frame_math | YES | Exact `Some(2)` and `None` assertions — a rounding implementation (len/frame_bytes integer division) fails the 7-byte case. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The bytes are always interleaved in format's layout" | buffer.rs:17-19 | REAL | Stage-D producer verified (convert.rs:256-278 via pipeline.rs:425/434). |
| "the pipeline never emits that shape [partial frames]" | buffer.rs:32-33 | REAL | Exact-length interleave (`out.len() != needed` typed error) — structurally impossible from the pipeline. |
| "`None` is an honest 'not measurable', never a guess" (latency) | buffer.rs:60-62 | REAL | NullSink implements exactly this (null.rs latency → None for the discard path). |
| "Implementations: audio-output backends (null, and the platform adapters behind ADR-005)" | buffer.rs:41-43 | REAL-UNPROVEN | null + stub exist; the ADR-005 platform adapters (cpal) are NOT in any shipped build (baseline: `audio.null-device` only) — the phrase is future-tense, not a false present-tense claim. |

## 7. Dead / duplicated / unreachable code
None. (`UnsupportedBackend` in audio-output/src/stub.rs is an intentional honest-refusal implementor, not dead weight.)

## 8. Dependencies & flags
- media-types only (frame, packet, time). No unsafe, no allows, std-only. Compiled unconditionally in audio-engine (a direct dep of lumen-cli and apps/web).

## 9. Open questions for the next agent
- audio-output/pump.rs auditor: does the pump uphold the "write may block, must not deadlock against the callback" constraint (:44-48) — the trait can only document it, the pump must prove it (SPSC ring claim, doc 03 §4.9 c).
- If a real (cpal) backend lands per ADR-005, revisit the s16 seam finding from my convert.rs audit (F-001 there): the sink format is f32-only today.
