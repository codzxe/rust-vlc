# AUDIT: crates/plugin-api/src/filter.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 203 / 9 (FilterOutcome enum, FrameSource trait, AudioFilter x2 methods, VideoFilter x2 methods, 2 test fns + Passthrough impl + audio_frame helper) |
| File verdict | STUB-STAGING (real, compiling trait vocabulary with ZERO implementors and ZERO consumers outside its own tests — the plugin surface it stages for does not exist yet) |
| Reachable from shipped entry point? | NO for this module. plugin-api IS a shipped dependency (apps/cli/Cargo.toml:59, apps/web/Cargo.toml:61) but every shipped use is the demuxer/access vocabulary (`rg plugin_api:: apps/cli/src` -> Demuxer/SourceStream/DemuxEvent/SeekSupport/Access only). `rg FilterOutcome|FrameSource|AudioFilter|VideoFilter` outside this file + lib.rs re-exports + test -> only PluginKind/kind.rs and media-types' own PipelineStage (different types). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Read of filter.rs (full); rg trait/consumer sweep over crates/, apps/, plugins/; plugin-api Cargo.toml + apps/cli usage check; baseline citation cargo-test-workspace.txt:4821,4823. |

## 1. One-paragraph verdict
A clean, small trait vocabulary (push-outcome enum mirroring codec-api's DecodeOutcome, a frame-source seam, host-side Audio/VideoFilter traits) whose entire production footprint is zero: the only `impl AudioFilter` in the repository is the test's own `Passthrough`, nothing constructs a `FilterOutcome` outside the tests, nothing implements `FrameSource`, and no engine or host code binds these traits (the CLI's shipped plugin-api usage is exclusively the demuxer half; the engine's real filter chains use audio-engine/video-engine's own domain traits, which the doc says is deliberate). The module is honest about being staging ("[PROPOSAL: placement, flag at the doc 06 §10 reconciliation]"), the two tests do what they claim on the vocabulary, and the defects are cosmetic: a `//`-for-`///` glitch that eats the codec-api mirror note out of the rendered docs, and two intra-doc links that cannot resolve from this module's scope.

## 2. What actually works (verified)
— 2/2 tests pass (baseline cargo-test-workspace.txt:4821 `filter::tests::passthrough_negotiates_and_emits`, :4823 `filter::tests::sink_errors_propagate`). Not re-run (cargo budget).
— The trait shapes compile against real L0 types (Frame/AudioFormat/PixelFormat/EngineError) and the sink-error test constructs a genuine typed EngineError (ResourceLimit/QueueSize) and asserts propagation — the vocabulary is wireable as-is.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/filter.rs:14-18 vs repo-wide zero usage
Code:
```
//! - minimal [`AudioFilter`] / [`VideoFilter`] **host traits** for the
//!   registry's v1 builtin + light-WASM filter story (§1.4), expressed
//!   purely in L0 frame types;
```
What is wrong: the "registry's v1 builtin + light-WASM filter story" has no other half: zero non-test implementors of either trait, zero consumers of `FilterOutcome`, zero implementors of `FrameSource` (rg sweep over crates/, apps/, plugins/ — every hit outside this file is either lib.rs's re-export, the unrelated `PluginKind::AudioFilter` enum arm, or media-types' own `PipelineStage`). The plugin host that would drive these traits (plugin-host-wasm) never references them, and the whole plugin lane is feature-off (product.toml:166). Why it matters: this is vocabulary for a story that has not landed; per the audit's fakeness bar it is staging surface, not implementation — fine as long as nothing cites it as evidence of working filters. What correct looks like: either the first consumer (builtin filter in audio-engine's chain walk) or an explicit `#[doc = "staging — unimplemented"]` marker like the rest of the plugin lane uses. Confidence: HIGH — Verification: read + repo-wide rg.

F-002 [S4]
Where: crates/plugin-api/src/filter.rs:22-24
Code:
```
/// What one filter push accomplished (mirrors
// [`codec_api::DecodeOutcome`]; MP §8 bounded-recovery
/// observability).
```
What is wrong: the middle line uses `//` instead of `///` — inside a doc-comment block the middle line becomes a plain comment, so the rendered rustdoc for `FilterOutcome` silently loses the codec-api mirror note and reads "…accomplished (mirrors observability)." Confidence: HIGH — Verification: read (byte-exact).

F-003 [S4]
Where: crates/plugin-api/src/filter.rs:20 (the import list) vs doc links at 75-76, 94-95, 107-108 (`[`PipelineStage::AudioFilter`]`) and 23 (`[`codec_api::DecodeOutcome`]`)
Code:
```
use media_types::{AudioFormat, EngineError, Frame, PixelFormat};
```
What is wrong: `PipelineStage` and `codec_api` are not in this module's scope (only the four imported names are), so the `[`PipelineStage::AudioFilter`]` and `[`codec_api::DecodeOutcome`]` intra-doc links cannot resolve from these items — rustdoc emits broken-link warnings (or renders them as plain text). Actual rustdoc output UNVERIFIED (no `cargo doc` in budget), but the scope analysis is mechanical. Confidence: MEDIUM — Verification: read; rustdoc run UNVERIFIED.

F-004 [S4]
Where: crates/plugin-api/src/filter.rs:158-184
Code:
```
    #[test]
    fn passthrough_negotiates_and_emits() {
```
What is wrong: the test exercises a test-local `Passthrough` impl — it pins that the trait signature compiles and that `FilterOutcome::Emitted { frames: 1 }` round-trips, but no production code can regress because of it. Honest as a vocabulary check; near-zero as a behavior test. Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 LOC of production code — table given anyway for completeness)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FilterOutcome | 25-45 | REAL (vocabulary) | S4 | Emitted/NeedMoreInput/Drained/Recovered{skipped,reason} — never constructed in production |
| FrameSource | 52-58 | STUB-STAGING | S3 | zero implementors |
| AudioFilter::negotiate / push | 70-89 | STUB-STAGING | S3 | zero implementors outside tests |
| VideoFilter::negotiate / push | 96-114 | STUB-STAGING | S3 | zero implementors; identical shape to AudioFilter (deliberate mirror) |
| tests: Passthrough + audio_frame + 2 fns | 125-202 | REAL (vocabulary-level) | S4 | F-004 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| passthrough_negotiates_and_emits | Only if the TRAIT SHAPE breaks | tests a test-local impl; pins signature + outcome equality, nothing production |
| sink_errors_propagate | Only if the TRAIT SHAPE breaks | same — genuine typed-error construction, test-local consumer |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "post-decode audio and video chains" (doc 06 §1.2 rows 6-7) | filter.rs:1-2 | PAPER-ONLY (no chain exists) | zero consumers (F-001); engine chains use the L3 domain traits |
| "for the registry's v1 builtin + light-WASM filter story (§1.4)" | filter.rs:14-16 | PAPER-ONLY | no registry consumes these traits |
| "mirrors codec-api's DecodeOutcome shape" | filter.rs:10-13 | REAL (shape) | variant parity Emitted/NeedMoreInput/Drained/Recovered vs codec-api's outcome (read) — and the doc line itself is half-eaten by F-002 |
| "deliberately no `Send` bound (the codec-api sink precedent)" | filter.rs:50-51 | REAL | FrameSource has no Send bound; AudioFilter/VideoFilter DO require Send (the worker-thread contract) — consistent with the doc's split |

## 7. Dead / duplicated / unreachable code
The module's entire production surface is dead (F-001). Duplication is intentional and documented (host trait vs L3 domain trait; AudioFilter/VideoFilter mirror shapes).

## 8. Dependencies & flags
media-types + codec-api path deps only (plugin-api Cargo.toml:24-25); no feature gates; no unsafe. Compiled into the shipped lumen-cli and lumen-web via plugin-api, where these items are dead weight the wasm linker can strip (no instantiation sites).

## 9. Open questions for the next agent
- kind.rs/manifest.rs auditors: `PluginKind::AudioFilter/VideoFilter` are installable KINDS today (manifest category "effects", PerPlaybackSession scope) — a manifest declaring kind audio-filter installs into a host that has no filter-chain consumer for it. What does install.rs do with such a plugin — run it headless? That gap (installable kind, absent engine seam) is the live Phase-7 question this file leaves open.
- audio-engine/video-engine auditors: confirm the L3 domain traits' push shapes stay compatible with FilterOutcome so the planned bridge is not a rewrite.
