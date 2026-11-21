# AUDIT: crates/codecs/src/video/mpeg12/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 60 / 6 items (module docs + 5 module declarations; no functions, no tests in this file) |
| File verdict | UNREACHABLE (the parse-only mpeg12 tree is real and tested, but no production code consumes it) |
| Reachable from shipped entry point? | NO — rg "mpeg12\|Mpeg12" across crates/ + apps/ production sources (excluding tests and comments) finds ZERO consumers: media-probe/src, demux/src, playback-engine, apps/cli all clean. The only references are `crates/codecs/tests/mpeg12_corpus.rs:29` (test), `crates/media-probe/tests/registry_roundtrip.rs:130-133` (test + fixture path), and comments (apps/web/Cargo.toml:94, subtitles/cc_data.rs:20,325). MPEG-2 video is not in the CLI's landed-decoder list (pipeline_host.rs:377 error string). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg "mpeg12\|Mpeg12" repo-wide production vs test split (consumers listed above); rg "impl.*Decoder\|decode_refusal" mpeg12/*.rs (no Decoder impl — honesty claim verified); rg "Mpeg12Facts\|descriptor_gate" outside the module (tests only); ls fuzz/fuzz_targets/ + baseline full-gate.log.txt:47 ("35 targets") — fuzz_codec_mpeg2 NOT among them; rg "fuzz_codec_mpeg2" worklog ("deferred to admission"); rg mpeg12 baseline test log (35 test lines ok :742-775) |

## 1. One-paragraph verdict
This is the module root of the MPEG-1/2 sequence-layer parser: 54 lines of unusually honest documentation plus five module declarations. Every checkable honesty claim in the docs verifies TRUE — there is no Decoder impl under the tree (rg-verified), software decode IS `blocked` in the feature matrix with a recorded reason (docs/feature-matrix.toml:524), the two sub-admission-bar crates.io candidates are recorded (worklog archive-2026-10.md:9612: oxideav-mpeg12video 0.0.13, na_mpeg2_decoder 0.1.0), and the referenced record doc exists. The submodule code is real and well-tested (35 mpeg12 test lines green in baseline :742-775). The two defects: the entire tree is production-dead — the docs' stated consumers ("probe/track-info facts", "the hw-feed seam") exist in no shipped or even production code path, only in tests — and the docs call `fuzz_codec_mpeg2` "the registered" target when the fuzz registry's 35 targets do not include it (the worklog says "deferred to admission").

## 2. What actually works (verified)
— `rg -n "mpeg12" audit/runs/baseline/test-workspace.log.txt` → 35 test lines all `ok` (:742-775): bitreader (5), error (4), facts, headers, startcode suites — the declared submodule map (mod.rs:34-44) matches the files that exist and the tests that ran.
— Honesty-claim verification (all TRUE): no `impl Decoder` under mpeg12 (rg); `decode_refusal` exists at error.rs:91 with tests at :143/:156; matrix row 515-524 `blocked_reason` matches the module docs' story word-for-word in substance; `docs/formats/video-mpeg12.md` exists (rg hit).
— Baseline `bash scripts/fuzz_registry_check.sh` → "35 targets, diff empty" (full-gate.log.txt:47) — used to falsify the "registered" fuzz claim (F-002).

## 3. Findings

### F-001 [S3] The stated consumers of the mpeg12 tree do not exist: production-dead parser
Where: crates/codecs/src/video/mpeg12/mod.rs:22-30; consumer evidence rg repo-wide
Code:
```
//! # What the sequence layer is for
//!
//! - probe/track-info facts: resolution, frame rate,
//!   profile/level, chroma, progressive/interlaced, GOP timecode,
//!   picture-type census, motion-vector syntax presence;
```
What is wrong: Nothing in production consumes these facts. `Mpeg12Facts::scan` / `descriptor_gate` are called only from `crates/codecs/tests/mpeg12_corpus.rs:29` and referenced by `crates/media-probe/tests/registry_roundtrip.rs:130-133` (a test, citing a fixture path). media-probe/src, demux/src, playback-engine, and apps/cli contain zero mpeg12 references; the CLI's decoder match has no mpeg2 arm and its error string lists the landed decoders without it (pipeline_host.rs:377). The third stated purpose ("the hw-feed seam: ADR-001's Phase-7 adapters will hand hardware decoders exactly these parsed structures", :27-28) is future tense for adapters that do not exist (hw tree audit: unreachable, MF lane absent).
Why it matters: ~5 modules of maintained, fuzz-lite'd parser code (headers/bitreader/startcode/facts/error — other agents' files) whose only execution is the test suite, while the docs describe present-tense purposes ("the one-pass total stream scan probe/track-info consume", mod.rs:40-41). A reader would believe MPEG-TS/PS probing goes through this scan; it does not.
What correct looks like: Either wire media-probe's TS/PS probing to `Mpeg12Facts::scan` (the stated purpose) or change the docs to "for the future probe/hw consumers; today test-corpus-only" — and consider a `codec-mpeg12-parse` feature gate like the external engines have.
Confidence: HIGH — Verification: read + exhaustive production-vs-test rg split.
### F-002 [S4] "the registered fuzz_codec_mpeg2 target" is not registered
Where: crates/codecs/src/video/mpeg12/mod.rs:52-54
Code:
```
//! a buffer with no start codes yields an empty fact set, never a
//! guess. Fuzz-lite pins this in `facts`; the registered
//! `fuzz_codec_mpeg2` target (doc 04 §8.5) formalizes it later.
```
What is wrong: The fuzz registry contains 35 targets (baseline full-gate.log.txt:47, "diff empty" — registry consistent with implementation) and `fuzz_codec_mpeg2` is not one of them (ls fuzz/fuzz_targets/: aac, adpcm, alac, av1, flac, h264, hevc, mjpeg, mp2, mp3, pcm, vorbis — no mpeg2). The worklog records the truth: "fuzz_codec_mpeg2 deferred to admission" (archive-2026-10.md:9612). Calling it "the registered" target inverts the fact.
Why it matters: Small, but it is exactly the "names promising more than the body" class — a doc asserting test infrastructure that does not exist, in a repo whose credibility rests on honest evidence labels.
What correct looks like: "a future `fuzz_codec_mpeg2` target (deferred to the Phase-6 admission, per the worklog)".
Confidence: HIGH — Verification: ls of fuzz/fuzz_targets/ + baseline full-gate log + worklog grep.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-54 | REAL | S3/S4 | Honesty claims verified TRUE (no decode, blocked matrix, recorded candidates); consumer claims describe nonexistent production users (F-001); "registered" fuzz target false (F-002). |
| `pub mod bitreader` | 56 | REAL | — | File exists; 5 tests green in baseline (:742-747). A04's assignment. |
| `pub mod error` | 57 | REAL | — | File exists; decode_refusal + 4 tests green (:748-751). A04's assignment. |
| `pub mod facts` | 58 | REAL | — | File exists; facts tests green (:752-753+). A05's assignment. |
| `pub mod headers` | 59 | REAL | — | File exists; headers tests green (part of the 35 lines). A04/A06 assignments (2 parts). |
| `pub mod startcode` | 60 | REAL | — | File exists; startcode tests green. A10's assignment. |

(No functions, impls, macros, or config keys exist in this file — the six rows above are the complete item inventory. Checked: no `fn`, no `impl`, no `const`, no `#[cfg]` gates, no tests.)

## 5. Test honesty
No tests in this file. The tree's tests live in the five submodules and `tests/mpeg12_corpus.rs`; all green in baseline. The corpus test is the real evidence class (ffmpeg-extracted TS/PS/AVI/MP4 lanes per the worklog quote) — its execution is what the matrix row's "10/10 differential" claim rests on.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "There is no software MPEG-1/2 decode here" | mod.rs:9-13 | REAL | rg: no Decoder impl under mpeg12; matrix :524 blocked. |
| "no Decoder impl lives under this module tree" | mod.rs:19-20 | REAL | rg "impl.*Decoder" mpeg12/*.rs → zero. |
| "Software decode remains `blocked` in the feature matrix" | mod.rs:13-14 | REAL | docs/feature-matrix.toml:515-524 row status + blocked_reason. |
| "two pre-1.0 candidates below the admission bar, both recorded" | mod.rs:14-16 | REAL | worklog archive-2026-10.md:9612 names both (oxideav-mpeg12video 0.0.13, na_mpeg2_decoder 0.1.0). |
| "the one-pass total stream scan probe/track-info consume" | mod.rs:40-41 | FALSE | No production consumer (F-001); tests only. |
| "the registered fuzz_codec_mpeg2 target … formalizes it later" | mod.rs:53-54 | FALSE | 35-target registry lacks it; worklog: "deferred to admission" (F-002). |

## 7. Dead / duplicated / unreachable code
The whole mpeg12 tree is production-unreachable (F-001) — compiled unconditionally (video/mod.rs:39 `pub mod mpeg12;`), executed only by tests. No duplication found at this level (the h264 parser precedent is cited as the design template, not copied).

## 8. Dependencies & flags
No deps, no unsafe, no features at this level (submodule-level posture is those agents' scope). Wave-6 hostile question: mpeg12 contributes nothing to shipped decode capability — and unlike h264, it honestly refuses at the vocabulary level (error.rs:91 `decode_refusal`), consistent with the matrix's blocked row.

## 9. Open questions for the next agent
- facts.rs (A05): does `Mpeg12Facts::scan` handle the AVI/MP4-extracted corpus lanes through the real demuxers (per the worklog's note that the landed AVI demuxer was MJPG-only v0)? The corpus evidence's lane provenance matters for the matrix row's "10/10 differential" claim.
- If any wave owns media-probe's TS/PS probing: is MPEG-video track info today guessed from container fields rather than `Mpeg12Facts::scan`? That would be the F-001 wiring gap's user-visible symptom (wrong fps/profile in track info).
