# AUDIT: crates/plugin-api/src/filter.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-21 |
| Lines / functions | 203 / 9 items (1 enum, 3 traits, 2 test types + 1 test helper, 2 tests) |
| File verdict | UNREACHABLE (real trait definitions with zero production implementors or consumers; the only implementor is a test-local mock) |
| Reachable from shipped entry point? | NO. `rg "impl (AudioFilter\|VideoFilter) for"` repo-wide → filter.rs:127 (the test `Passthrough`) ONLY. `FilterOutcome`/`FrameSource` appear nowhere outside plugin-api (lib.rs re-export + this file). No registry, no engine wiring, no plugin references (`plugin-host-wasm` has zero filter mentions). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:1 |
| Commands run | rg `FilterOutcome\|FrameSource` over crates/ + apps/ (2 files: plugin-api lib.rs + filter.rs); rg `plugin_api::filter` (0 matches); rg `impl …AudioFilter/VideoFilter for` (test-only); rg `AudioFilter\|VideoFilter` in crates/audio-engine (0 matches) and crates/video-engine (one doc comment); sed video-engine/src/lib.rs 1–60 ("the video filter chain … a later card; the pipeline seam is reserved"); sed docs/06-plugin-architecture.md 80–91 (the honest staging record) + 59–60 (the kind-table rows); sed audio-engine/src/effects.rs 1–40 (the LIVE filter chain is a different trait). |

## 1. One-paragraph verdict
Dead vocabulary with a false cover story. The traits are competently written (precise contracts, MP §8 discipline, honest `Send` reasoning), but nothing in the tree implements them except the file's own test mock, nothing consumes `FilterOutcome` or `FrameSource`, and the filter functionality that DOES exist — audio-engine's effect chain (`AudioEffect`/`EffectChain`, issue #33/D-012, gain/balance/mute) — uses a different trait shape entirely. Worse, the module doc asserts the domain traits `audio_engine::AudioFilter`/`video_engine::VideoFilter` "live in their L3 domain crates"; neither exists anywhere — doc 06:84–89 records the truth (staged in plugin-api as a `[PROPOSAL]` deviation, reconciliation pending a later card), and this file's header contradicts it. Compiled into every workspace build (plugin-api is a CLI dependency) for zero reach.

## 2. What actually works (verified)
- The file compiles and its two tests pass (baseline #3 workspace run; plugin-api compiled at test-workspace.log.txt).
- The trait contracts themselves are well-specified: bounded-work-per-call calling contexts, negotiation-before-frames, typed errors with the right stages, `Send` reasoning for the worker-thread model, no-`Send` reasoning for `FrameSource` (codec-api sink precedent).
- `sink_errors_propagate` (186–202) pins that a downstream sink error surfaces, not swallows — the right discipline for the push model.
- Nothing verified beyond that: no production path exercises any of it.

## 3. Findings

### F-001 [S3] — Module doc's central claim is false: the domain traits do not exist
Where: crates/plugin-api/src/filter.rs:4-6
Code:
```
//! The **domain traits** `audio_engine::AudioFilter` and
//! `video_engine::VideoFilter` live in their L3 domain crates (doc 06
//! §1.2 trait-ownership note: keeps L1 free of L3/L4 deps).
```
What is wrong: `audio_engine::AudioFilter` does not exist (rg over crates/audio-engine: zero `AudioFilter` matches). `video_engine::VideoFilter` does not exist (video-engine/src/lib.rs:18–22: "The video filter chain (`VideoFilter` — color adjust, crop, deinterlace …): **a later card; the pipeline seam is reserved**"). Doc 06's own §1.2 record (lines 84–89) states the domain crates "do not yet re-export the plugin-api filter/playlist/discovery traits — the filter-chain card … owns the reconciliation" — i.e., doc 06 tells the truth and this header narrates a reconciliation that has not happened. Doc 06's kind table rows 6/7 (lines 59–60) repeat the same nonexistent- trait attribution.
Why it matters: A reader implementing a filter plugin would target the wrong crate map; the file presents an architecture fact ("domain traits live in L3") that the tree contradicts. This is the FALSE-claim pattern (docs assert what code contradicts), planted in a header that ships in every build.
What correct looks like: "The domain traits are NOT yet defined — audio-engine ships `AudioEffect` (effects.rs) and video-engine reserves the seam; these host traits are the staged `[PROPOSAL]` vocabulary (doc 06 §1.2 note) awaiting the filter-chain card's reconciliation."
Confidence: HIGH — Verification: repo-wide rg + sed of both crates + doc 06.

### F-002 [S3] — The whole trait set is dead: no implementor, no consumer, no registry; the live filter chain uses a different ABI
Where: crates/plugin-api/src/filter.rs:70-114 (traits) and 14-15 (the claimed registry story)
Code:
```
//! - minimal [`AudioFilter`] / [`VideoFilter`] **host traits** for the
//!   registry's v1 builtin + light-WASM filter story (§1.4), expressed
```
What is wrong: (a) The only implementor of either trait in the repository is the test-local `Passthrough` (filter.rs:127). (b) `FilterOutcome` and `FrameSource` are referenced by no file outside plugin-api (rg: lib.rs re-export + this file). (c) No filter registry exists (doc 06:132–136's registry rule is itself `[PROPOSAL]`; plugin-host-wasm contains zero filter surface). (d) The actual filter functionality that ships — audio-engine's effect chain, `AudioEffect`/`EffectChain` (effects.rs, issue #33/D-012, gain/balance/mute with pan-law tests) — is a per-track in-place planar-f32 trait, a different ABI from this push/`FilterOutcome` model. The tree therefore carries TWO filter vocabularies: one live (audio-engine), one dead (this file).
Why it matters: Dead code compiled into every workspace lane; a second, divergent filter ABI that the promised reconciliation now has to either adopt or delete; and the "light-WASM filter story" (§1.4) has no runtime surface at all — relevant to the wave's hostile question (no real plugin is loaded anywhere; none of these kinds has a guest).
What correct looks like: Either wire the effect chain behind these traits (make `AudioEffect` adapters implement `AudioFilter` and have the engine's chain walk consume `FilterOutcome`) or delete the module until the filter-chain card, and fix lib.rs's re-exports.
Confidence: HIGH — Verification: rg implementors/consumers + effects.rs read.

### F-003 [S4] — Broken doc comment splits the `FilterOutcome` docs
Where: crates/plugin-api/src/filter.rs:22-24
Code:
```
/// What one filter push accomplished (mirrors
// [`codec_api::DecodeOutcome`]; MP §8 bounded-recovery
/// observability).
```
What is wrong: Line 23 opens with `//` (a plain comment), not `///` — the doc comment is severed mid-sentence. rustdoc renders "What one filter push accomplished (mirrors observability)." and the intra-doc link to `codec_api::DecodeOutcome` never resolves (it is not part of the doc).
Why it matters: Garbled public API docs for the module's central enum; the mirror-shape claim becomes invisible to doc readers.
What correct looks like: `///` on all three lines.
Confidence: HIGH — Verification: read (Read tool shows the raw `//`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-18 | FALSE | F-001/F-002 | Domain-traits claim contradicted by tree; registry story paper. |
| FilterOutcome (enum) | 25-45 | REAL (dead) | F-003 | Emitted{frames}/NeedMoreInput/Drained/Recovered{skipped,reason}; mirrors codec-api DecodeOutcome + recovery arm; broken doc line 23. |
| FrameSource (trait) | 52-58 | UNREACHABLE | — | Zero implementors repo-wide. |
| AudioFilter::negotiate | 77 | UNREACHABLE | — | Chain-build-time format negotiation; sound contract. |
| AudioFilter::push | 84-88 | UNREACHABLE | — | Push + FnMut sink + FilterOutcome; sound contract. |
| VideoFilter::negotiate/push | 102/109-113 | UNREACHABLE | — | PixelFormat twin of the audio shape. |
| tests: Passthrough | 125-141 | REAL (mock) | — | The subject under test is the test's own impl. |
| tests: audio_frame helper | 143-156 | REAL | — | S16 stereo 48k, SharedBytes::from_static. |
| tests: passthrough_negotiates_and_emits | 158-184 | REAL (weak) | — | Pins identity negotiation + Emitted/sink agreement — compile-shape smoke, no production subject. |
| tests: sink_errors_propagate | 186-202 | REAL (weak) | — | Sink error surfaces — right discipline, same mock-subject caveat. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| passthrough_negotiates_and_emits | Only structurally | The subject is the test-local mock; production code under test = trait definitions + an enum. Removing `PartialEq` from FilterOutcome fails it at compile time anyway. It proves implementability, nothing more. |
| sink_errors_propagate | Only structurally | Same: exercises the mock's error path; asserts `is_err()` on a hand-built error. |

Both tests are honest about what they are (no inflated names), but they cannot fail for any production-behavior reason — there is no production behavior.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The domain traits `audio_engine::AudioFilter` and `video_engine::VideoFilter` live in their L3 domain crates" | filter.rs:4-5 | FALSE | Neither trait exists (rg; video-engine lib.rs:18–22 defers to "a later card"). |
| "host traits for the registry's v1 builtin + light-WASM filter story (§1.4)" | filter.rs:14-15 | PAPER-ONLY | No registry code; plugin-host-wasm has no filter surface; doc 06's registry rule is `[PROPOSAL]`. |
| "[PROPOSAL: placement, flag at the doc 06 §10 reconciliation]" | filter.rs:7-8 | REAL | The proposal status is accurately recorded in doc 06:80–91. |
| "mirrors codec-api's `DecodeOutcome` shape" | filter.rs:11-13 | REAL-UNPROVEN | DecodeOutcome exists (used in apps/web host.rs:534–541); variant-for-variant diff not performed. |
| doc 06 kind rows 6/7 attribute `audio-filter`/`video-filter` to audio-engine/video-engine L3 traits | docs/06:59-60 | FALSE | Same proof as F-001. |
| "filters … must accept its claimed `formats_in` and emit its claimed `formats_out`" (conformance) | docs/06:846 | PAPER-ONLY | ConformanceRunner does not exist (see my access.rs audit F-002). |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable production-wise (F-002). Duplication: two filter ABIs in the tree — plugin-api's push/FilterOutcome model (dead) vs audio-engine's in-place `AudioEffect` chain (live, effects.rs). lib.rs re-exports the dead set at lines 91 and 107.

## 8. Dependencies & flags
`media-types` only (AudioFormat, EngineError, Frame, PixelFormat). No features, no unsafe, no L3 deps (the L1-purity constraint that caused the staging in the first place). Baseline clippy/fmt clean for this file (baseline #5, #7 — note #7's one dirty file was plugin-api's testkit_wiring.rs, not this).

## 9. Open questions for the next agent
- Whoever owns the filter-chain card decision: adopt-or-delete this module (F-002) — and sync filter.rs's header with doc 06:80–91 (F-001) the moment either happens.
- A03 (plugin-api/demux.rs, kind.rs owner A07): the same "staged [PROPOSAL] deviation" family (playlist/discovery traits) — check whether those are equally dead; my rg showed `FrameSource`-style isolation for filter only, playlist/discovery not checked.
- Wave gate: this file is direct evidence for the plugin-domain headline — filter kinds 6/7 have no engine consumer, no builtin, no WASM guest, and a false trait-ownership claim in the shipped header.
