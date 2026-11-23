# AUDIT: crates/codecs/src/video/mpeg12/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Lines / functions | 60 / 0 (module manifest: 5 `pub mod` decls + honesty-boundary docs) |
| File verdict | REAL (parser tree real and tested) — with two FALSE doc claims in the manifest |
| Reachable from shipped entry point? | NO — exhaustive rg across crates/, apps/, tools/: the tree's only consumers are tests (crates/codecs/tests/mpeg12_corpus.rs + in-tree unit tests). Containers classify mpeg1video/mpeg2video with their own local peek (apps/cli/src/containers/ps.rs:60), never via this tree. `pub mod mpeg12` unconditional (codecs/src/video/mod.rs:39) so it compiles into every shipped build, uncalled |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | rg consumer scans (mpeg12::facts/headers/error/startcode/bitreader, Mpeg12Facts, descriptor_gate, decode_refusal, mpeg12 in demux/codec-api/apps); `./target/debug/deps/codecs-bae0f1756cc0a809 video::mpeg12` -> 34 passed, 0 failed, exit 0; sed verifications |

## 1. One-paragraph verdict
A 60-line honesty-boundary manifest for a real, tested, deliberately decode-free MPEG-1/2 sequence-layer parser. The module map is accurate (all five entries verified symbol-by-symbol), the "no software decode / no `Decoder` impl" boundary is true (rg: zero `impl Decoder` under the tree), the damage model is real (34 in-crate tests executed by me, incl. `fuzz_lite_scan_is_total`; W06-A08 ran mpeg12_corpus 7/7 vs live ffprobe), and the blocked-status story matches the feature matrix (feature-matrix.toml:513-524) and docs/formats/video-mpeg12.md's two recorded candidates. But two present-tense claims are FALSE: the named refusal mechanism (`error::decode_refusal` / `mpeg12.decode.blocked`) is surfaced by no shipped path — the CLI refuses mpeg2video with its own `cli.codec_unavailable` host error, and decode_refusal's only callers are its own unit tests; and the "registered `fuzz_codec_mpeg2` target" does not exist in the fuzz registry, which itself lists MPEG-1-2 as "Not yet registered". The whole tree is production-unwired: a landed parser waiting for consumers.

## 2. What actually works (verified)
- Module map: startcode (Table 6-1 classification `StartCode::from_byte` startcode.rs:38, zero-copy scanner), bitreader (MSB-first `uimsbf/bslbf` reader bitreader.rs:1-2,42), headers (sequence/GOP/picture/extension/slice + macroblock addressing, headers.rs:2-4, has_motion_vectors :121), facts (`Mpeg12Facts::scan` facts.rs:78, `descriptor_gate` :287), error (typed vocabulary + `decode_refusal` error.rs:91 with `mpeg12.decode.blocked` :109).
- Honesty boundary: "no `Decoder` impl lives under this module tree" — TRUE (rg zero matches); blocked status matches feature-matrix.toml:513-524 and docs/formats/video-mpeg12.md:21-44 ("two candidates recorded", "none admitted").
- Damage model: "Fuzz-lite pins this in `facts`" — TRUE: `video::mpeg12::facts::tests::fuzz_lite_scan_is_total` and 33 siblings all pass: `./target/debug/deps/codecs-bae0f1756cc0a809 video::mpeg12` -> `34 passed; 0 failed`, exit 0.
- Purpose fields exist: gop/picture_type_mask/frame_rate etc. (facts.rs:39,58,182).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/mpeg12/mod.rs:17-20
Code:
```
//! VLC/ffmpeg-class project queued Phase 6+. The typed refusal the
//! codec-api registry surfaces is [`error::decode_refusal`]
//! (`mpeg12.decode.blocked`); no `Decoder` impl lives under this
//! module tree.
```
What is wrong: There is no codec-api registry in the shipped build — the CLI's `HostDecoderProvider` is explicitly "the registry shim until the plugin registry card lands" (pipeline_host.rs:199-200), and it surfaces its OWN host error for mpeg1video/mpeg2video tracks: `"cli.codec_unavailable"` / "no decoder for \`{other}\` in this build (landed: ...)" (pipeline_host.rs:373-380). `error::decode_refusal` (`mpeg12.decode.blocked`) has ZERO production callers — rg across crates/, apps/, tools/ finds only error.rs's own unit tests. The documented refusal code never reaches any user. Why it matters: the honesty paragraph — the file's centerpiece — names a mechanism the product does not implement; anyone tracing the refusal path from this doc dead-ends. What correct looks like: doc says "the refusal constructor the future registry will call is error::decode_refusal; today the CLI's shim surfaces cli.codec_unavailable" — or wire decode_refusal into the shim. Confidence: HIGH — Verification: read + rg (exhaustive).
F-002 [S3]
Where: crates/codecs/src/video/mpeg12/mod.rs:53-54
Code:
```
//! a buffer with no start codes yields an empty fact set, never a
//! guess. Fuzz-lite pins this in `facts`; the registered
//! `fuzz_codec_mpeg2` target (doc 04 §8.5) formalizes it later.
```
What is wrong: "the registered `fuzz_codec_mpeg2` target" — FALSE. No such target exists: not in fuzz/fuzz_targets/ (ls: 35 targets, none mpeg), not in fuzz/Cargo.toml (grep: zero mpeg matches), not in fuzz/README.md's registry table — which instead lists MPEG-1-2 under "Not yet registered (no landed parser — doc 04's tables stay the contract): ... VP8/VP9/Theora/MPEG-1-2/MPEG-4 ASP/..." (README:92-95). The cite is also wrong: `fuzz_codec_mpeg2` appears in doc 04 at :259 — the §4 availability table's future phase-6 column — not §8.5's registry. The registry gate (scripts/fuzz_registry_check.sh) only checks internal consistency of the three sets, so a landed parser without a fuzz target fails nothing. Why it matters: a coverage auditor reading this file believes the parser has a registered fuzz lane; it has none. What correct looks like: "a fuzz_codec_mpeg2 target is doc 04's planned registry name; not yet registered". Confidence: HIGH — Verification: read + ls/grep/rg.
F-003 [S3]
Where: crates/codecs/src/video/mpeg12/mod.rs:22-30
Code:
```
//! - probe/track-info facts: resolution, frame rate,
//!   profile/level, chroma, progressive/interlaced, GOP timecode,
//!   picture-type census, motion-vector syntax presence;
//! - the hw-feed seam: ADR-001's Phase-7 adapters will hand
//!   hardware decoders exactly these parsed structures;
```
What is wrong: the purpose list names zero consumers that exist today — exhaustive rg finds no probe/track-info, hw-feed, or any other production consumer of Mpeg12Facts/headers/startcode anywhere in crates/ or apps/ (only tests). The PS container that DOES classify mpeg1video/mpeg2video does it with its own demuxer peek ("`mpeg2video`/`mpeg1video` by the sequence-extension peek" ps.rs:60 — demux::ps, no mpeg12 import), i.e. the shipped product duplicates the classification this tree was built to provide, then refuses decode without consulting it. The tree is landed-but-unwired (same posture W06-A08 flagged for h264/hw). Why it matters: 5 modules + 34 tests + a corpus lane of parser code ship in every build serving no shipped caller; the "what it is for" doc reads as current wiring. What correct looks like: a wiring status line like the h264/hevc fuzz modules' honest "Wiring status" paragraphs, or an actual consumer. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
Not required (<100 LOC, no functions — 5 `pub mod` declarations: bitreader, error, facts, headers, startcode, all unconditional; each submodule already audited by W06-A08/W06 peers).

## 5. Test honesty
No tests in this file. The tree's suites are real and green: 34 in-crate unit tests executed by me this run (exit 0); mpeg12_corpus.rs executed by W06-A08 (7/7 lanes vs live ffprobe 7.1.5, log audit/runs/W06-A08.log.md File 6).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no software MPEG-1/2 decode here ... no `Decoder` impl lives under this module tree" | mod.rs:9-20 | REAL | rg: zero `impl Decoder` in mpeg12/; feature-matrix.toml:513-524 blocked |
| "The typed refusal the codec-api registry surfaces is error::decode_refusal (mpeg12.decode.blocked)" | mod.rs:17-19 | FALSE | No registry exists (pipeline_host.rs:199-200 "registry shim"); shipped refusal = cli.codec_unavailable (pipeline_host.rs:373-380); decode_refusal zero prod callers (F-001) |
| "blocked ... no validated pure-Rust engine exists (5.H1 survey — two pre-1.0 candidates, both recorded)" | mod.rs:13-16 | REAL | docs/formats/video-mpeg12.md:21-44 "crates = [] — none admitted; two candidates recorded" |
| "Fuzz-lite pins this in `facts`" | mod.rs:53 | REAL | fuzz_lite_scan_is_total executed this run (34/34 mpeg12 tests, exit 0) |
| "the registered `fuzz_codec_mpeg2` target (doc 04 §8.5) formalizes it later" | mod.rs:53-54 | FALSE | No target in fuzz/ (F-002); README:92-95 lists MPEG-1-2 "Not yet registered"; doc04 cite is §4:259 not §8.5 |
| "a buffer with no start codes yields an empty fact set, never a guess" | mod.rs:52-53 | REAL | facts.rs scan damage-tolerance tests (empty_and_garbage_inputs_are_total, executed 34/34) |
| "probe/track-info facts ... hw-feed seam" as purpose | mod.rs:22-30 | SHALLOW | machinery real; zero shipped consumers (F-003) |

## 7. Dead / duplicated / unreachable code
The entire tree is production-unreachable (F-003) though compiled unconditionally (video/mod.rs:39) and honestly documented as decode-free. Not dead by the test-only-consumer standard (corpus + 34 unit tests exercise it), but no shipped entry point reaches it.

## 8. Dependencies & flags
None. No feature gates; unconditional in every build including the release lane.

## 9. Open questions for the next agent
- Who owns wiring Mpeg12Facts::scan into the CLI probe (or the Phase-7 hw-feed seam)? Until then this is 5 modules of ballast in every shipped binary.
- fuzz/README.md's claim "Each grows a row here the day its parser lands (the registry gate below fails the build otherwise)" is itself inaccurate — the gate only diffs three internally-consistent sets and cannot detect a landed-but-unregistered parser (mpeg12 is the live counterexample). Flag to the fuzz/README owner.
- The feature-matrix row's "8k fuzz rounds" evidence (feature-matrix.toml:524) — which run does that cite? In-crate fuzz-lite or a campaign? No campaign target exists (F-002) — matrix auditor should verify.
