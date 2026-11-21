# AUDIT: crates/plugin-api/src/subtitle.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-21 |
| Lines / functions | 231 / 9 items (2 traits, 1 enum, 1 fn, module docs, 2 test doubles, 3 tests) |
| File verdict | UNREACHABLE (real, well-designed contract; ZERO implementations and ZERO callers tree-wide — the claimed impl crate has not adopted the seam) |
| Reachable from shipped entry point? | NO. `rg "impl SubtitleParser\|impl SubtitleSink\|dyn SubtitleParser\|dyn SubtitleSink" crates apps` → only this file's own test doubles (LineParser :106, Collector :164). No engine code feeds chunks, no sink consumes cues through this trait. The claimed impl crate does not even depend on plugin-api: `crates/subtitles/Cargo.toml:11-13` — "media-types only — the registry row also lists plugin-api, which joins when the SubtitleParser seam lands (5.M1)". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "SubtitleParser\|SubtitleSink\|ParseStatus\|not_this_format" crates apps --type rust` (census: plugin-api self-references only); `rg -n "impl.*SubtitleParser\|dyn Subtitle" crates apps` (only test doubles); `rg -n "plugin-api\|plugin_api" crates/subtitles/Cargo.toml crates/subtitles/src` (absent — seam pending 5.M1); Read of subtitles/Cargo.toml:1-30 + subtitles/src/lib.rs:90-119 (real parser inventory: ass, vtt, dvb, vobsub — none via this trait); `sed -n '99p;104,116p' crates/plugin-api/src/lib.rs` (prelude asymmetry); `rg -n "subtitle::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (:4466-4468 all 3 green). |

## 1. One-paragraph verdict
The contract itself is good work: a chunk-fed, bounded-per-call parser trait with an honest three-state outcome (`Accepted`/`Drained`/`Recovered{skipped, reason}` — damage as data, not silence), a sink whose errors abort cleanly, and a panic-free typed refusal helper (`not_this_format` caps its sniff window at 16 bytes with `min`, no slicing hazard). But the trait is a contract with no counterparty: the only `impl SubtitleParser` and `impl SubtitleSink` in the entire workspace are the toy doubles in this file's own test module, no engine code ever calls `next_cues`, and the module doc's pointer — "Impl crate: `subtitles` (L2)" — is contradicted by that crate itself, which does not depend on plugin-api and says the seam "joins when … lands (5.M1)" (Cargo.toml:12) with "Future 5.M1 unification: the `SubtitleParser` seam" (lib.rs:101). Real subtitle parsing exists — ASS, WebVTT, DVB bitmap, VobSub parsers all live in crates/subtitles on their own seam — so the plugin-facing contract lags the shipped capability it claims to describe.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4466-4468` → `subtitle::tests::{damage_is_data_not_silence, format_mismatch_error_shape, parses_in_chunks_and_drains}` all ok.
— Verified by read: `not_this_format` (80-92) builds a typed `EngineError` (stage `Demux`, kind `Unsupported` with sniffed-prefix cap, dotted context `plugin.subtitle.not-this-format`) with no panic path — `sniffed[..sniffed.len().min(16)]` cannot over-slice; the shape is asserted by `format_mismatch_error_shape` (224-229).
— Verified by read: the contract semantics the docs promise are demonstrated end-to-end by the toy double — mid-line chunk buffering (179-190: `hello sub|` + `9000` across two calls yields exactly one cue), no early emission of incomplete cues (184), flush-by-`None` (192-196), and damage reported as `Recovered{skipped: 1}` with the cue counted, not silently dropped (207-219).
— Verified consistency: "Instance scope: per playback session (§1.3)" (line 4) matches `kind.rs` `instance_scope(SubtitleParser) → PerPlaybackSession` (kind.rs:130-131, test-pinned).

## 3. Findings

### F-001 [S3]
Where: crates/plugin-api/src/subtitle.rs:1-2 (with the whole trait, 16-77)
Code:
```
//! The subtitle-parser contract (doc 06 §1.2 kind 10): subtitle
//! format → timed [`SubtitleCue`]s. Impl crate: `subtitles` (L2).
```
What is wrong: "Impl crate: `subtitles` (L2)" is false in the current tree. `crates/subtitles` does not depend on plugin-api (Cargo.toml deps: media-types only) and documents the adoption as future work — Cargo.toml:11-13: "the registry row also lists plugin-api, which joins when the SubtitleParser seam lands (5.M1)"; lib.rs:101: "Future 5.M1 unification: the `SubtitleParser` seam." The trait therefore has zero implementations (only this file's test doubles) and zero callers — no engine path constructs a `dyn SubtitleParser` or implements `SubtitleSink`. The real subtitle feature runs on the subtitles crate's own APIs (its ASS/VTT/DVB/VobSub modules), which this contract neither wraps nor is wrapped by.
Why it matters: Doc 06's kind 10 ("subtitle-parser") is presented as a live plugin kind; a reader tracing `PluginKind::SubtitleParser` to its "impl crate" finds a contract nobody implements. The plugin taxonomy and the real subtitle pipeline are two disjoint worlds; the seam meant to join them (5.M1) is pending, and nothing in this file flags that — the pointer reads as present tense.
What correct looks like: `//! Impl crate: `subtitles` (L2) — [PENDING: seam adoption is card 5.M1; the crate's parsers do not implement this trait yet]`, or land 5.M1.
Confidence: HIGH — Verification: read + rg impl/caller census (zero outside tests) + subtitles crate's own manifests quoted verbatim.

### F-002 [S4]
Where: crates/plugin-api/src/lib.rs:99 (affects this module's surface)
Code:
```
pub use subtitle::{SubtitleParser, SubtitleSink};
```
What is wrong: The prelude exports the two traits but NOT `ParseStatus` — the return type of the trait's only method — nor `not_this_format`, the helper written for implementors. Any future implementor importing from the prelude must still path-qualify `plugin_api::subtitle::ParseStatus` on every signature.
Why it matters: An asymmetric export surface on a contract meant to be implemented by other crates; the moment 5.M1 lands, every implementor hits the inconsistency.
What correct looks like: `pub use subtitle::{not_this_format, ParseStatus, SubtitleParser, SubtitleSink};` (the prelude already re-exports richer sets for other modules, e.g. manifest's seven types at lib.rs:110-112).
Confidence: HIGH — Verification: read of lib.rs:99,108-116 (prelude block) + this file's pub items.

## 4. Per-function table (231 lines, whole file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs | 1-10 | MISLEADING | F-001 | "Impl crate: subtitles (L2)" false today; scope note (per-playback-session) accurate; "Phase 5 vocabulary extensions" honest. |
| `SubtitleParser` trait | 16-45 | REAL (contract) / unimplemented | F-001 | Chunk-fed, `None`=flush, early returns documented, errors typed via EngineError(Demux stage). Zero impls, zero callers. |
| `ParseStatus` enum | 47-63 | REAL | — | Accepted/Drained/Recovered{skipped: u32, reason: &'static str} — bounded-recovery vocabulary; recovered-as-data pinned by test. |
| `SubtitleSink` trait | 65-77 | REAL (contract) / unimplemented | F-001 | No Send bound (worker-local) — documented precedent; Err aborts in-flight call, parser stays usable. |
| `not_this_format` | 79-92 | REAL (dead) | — | Panic-free 16-byte sniff cap; typed stage/kind/context; only caller is its test. |
| tests: `LineParser` double | 100-158 | REAL (test code) | — | Toy line format; buffering + recovery semantics exercised. |
| tests: `Collector` double | 160-169 | REAL (test code) | — | Vec sink. |
| tests: `parses_in_chunks_and_drains` | 171-198 | REAL | — | See §5. |
| tests: `damage_is_data_not_silence` | 200-220 | REAL | — | See §5. |
| tests: `format_mismatch_error_shape` | 222-230 | REAL | — | See §5 — the only test touching production code (`not_this_format`). |

## 5. Test honesty (all 3 green: orchrun run :4466-4468)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `parses_in_chunks_and_drains` | NO for shipped code / YES for contract semantics | The subject is a trait definition (cannot misbehave at runtime); the code under test is the in-file toy double. It proves the DOCUMENTED semantics (chunk buffering, no early emission, flush) are implementable — semantic documentation, not regression protection. Breaking the double's buffering (mutation: drop the pending re-attach at 126) fails 183-190. |
| `damage_is_data_not_silence` | Same class | Pins `Recovered{skipped: 1}` + zero cues emitted for an unparsable stamp — the MP §8 bounded-recovery promise, demonstrated on the double. |
| `format_mismatch_error_shape` | YES | Exercises the production fn `not_this_format` (dead but real): asserts stage == Demux and kind == Unsupported. Mutation: wrong stage/kind fails 225-229. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Impl crate: `subtitles` (L2)" | subtitle.rs:2 | FALSE (today) | F-001 — subtitles crate lacks the plugin-api dep; seam is card 5.M1 (Cargo.toml:12, lib.rs:101). |
| "Instance scope: per playback session (§1.3)" | subtitle.rs:4 | REAL | Matches kind.rs:130-131 (test-pinned there). |
| "panics are contract violations … never an internal parse-the-whole-file loop" | subtitle.rs:20-23 | UNVERIFIABLE | Contract-level claim; zero implementations exist to violate or honor it. |
| "Cancellation is generation staleness: cue batches carry the demux generation" | subtitle.rs:23-25 | UNVERIFIABLE | The trait carries no generation parameter; no engine caller exists to check. |
| "Styled and bitmap payloads are Phase 5 … the L0 `SubtitleCue` is plain text in v1" | subtitle.rs:8-10 | REAL (for this seam) / STALE (for the product) | The seam is plain-text; but bitmap subtitle decoding ALREADY EXISTS in crates/subtitles (DVB ETSI EN 300 743, VobSub — lib.rs:104-119, wave 9) on its own seam. The plugin contract lags the real capability it describes. |
| "recoverable damage skips the cue and is reported as data" | subtitle.rs:37-39 | REAL (contract) | `Recovered` variant + test 207-219. |

## 7. Dead / duplicated / unreachable code
- DEAD: the entire module from the integration standpoint — zero impls, zero callers (F-001); `not_this_format` has no non-test caller.
- Not prelude-exported: `ParseStatus`, `not_this_format` (F-002).
- No duplication: `subtitles` crate's own parser APIs are a different design (their own audit's scope) — the duplication risk arrives when 5.M1 bridges them.

## 8. Dependencies & flags
- Deps: media-types only (SubtitleCue, SharedBytes, EngineError, PipelineStage) — matches the crate's L1 registry row; no feature gates; no unsafe; `[lints] workspace = true`.
- Test deps via dev-dependency testkit (`Must` not used here — tests use match/unreachable instead; consistent style).

## 9. Open questions for the next agent
- Card 5.M1 owner: this file is the landing zone — the subtitles crate's ASS/VTT/DVB/VobSub parsers must either implement `SubtitleParser` (bitmap kinds will need the "Phase 5 vocabulary extensions" the doc defers) or the kind-10 row needs a status change.
- The `subtitles` crate auditor (wave 5/9 agents may have covered it): confirm none of its parsers quietly re-derive this contract's semantics under different names (damage-ledger tolerance in vtt/mod.rs sounds like `Recovered` re-invented).
