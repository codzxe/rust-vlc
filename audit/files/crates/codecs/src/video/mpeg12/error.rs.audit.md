# AUDIT: crates/codecs/src/video/mpeg12/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 185 / 4 fns + 2 impls + 4 tests |
| File verdict | UNREACHABLE (real, tested code — but the doc's "engine-facing" claim is false: nothing outside the mpeg12 module and its test suite calls `decode_refusal`/`accepts`/`descriptor_gate`/`Mpeg12Facts::scan`) |
| Reachable from shipped entry point? | NO. `pub mod mpeg12;` (video/mod.rs:39, unconditional) compiles it into the codecs crate, but repo-wide rg (all crates/*/src excluding mpeg12/**) finds ZERO consumers: the only references are mpeg12's own module files, mpeg12_corpus.rs (integration test), and a subtitles comment. The codec-api registry never receives this refusal — no dispatch exists. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg decode_refusal/descriptor_gate/Mpeg12Facts::scan repo-wide; rg mpeg1video/mpeg2video across codecs/src outside mpeg12 (zero hits); baseline :845-848; sed quote checks |

## 1. One-paragraph verdict
The error module itself is exactly what it should be — four typed parse-failure variants with stable rule codes, a Display that names the rule, a descriptor gate over exactly `mpeg1video`/`mpeg2video`, and a `decode_refusal` that builds a typed `Unsupported` with the honest "software decode is blocked" detail; its four unit tests pass in baseline. But the module doc's central claim — that this refusal is "the typed Unsupported that the codec-api registry receives" (error.rs:6-11) — is false in the current tree: the mpeg12 module is compiled unconditionally yet nothing outside it (no registry, no demuxer, no media-probe, no engine) calls any of its entry points. The refusal exists, is tested, and reaches no engine. The feature matrix is honest about the lane (codec-video-mpeg1-2 = blocked ×4, empty evidence, blocked_reason says "sequence-layer parser + typed refusal LANDED" — landed as code, not as wiring), and video/mod.rs:36-38 says the module is "unconditional until the registration card's per-codec gates" — i.e., the dispatch is explicitly pending, which error.rs's doc overstates.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:845-848 — all 4 `mpeg12::error::tests::*` ok (descriptor gate exact-two-ids incl. case-sensitivity `MPEG2VIDEO` rejected, both descriptors → Unsupported + code `mpeg12.decode.blocked`, foreign-descriptor detail via source downcast, Display strings pinned).
- The enum is genuinely consumed by the parser: headers.rs:13 imports `Mpeg12Error`; parse paths at headers.rs:158-535 return Truncated/Marker/OutOfRange/Invalid variants (rg-verified).
- `accepts` is re-exported as `facts::descriptor_gate` (facts.rs:285-289) — same single spelling, but see F-001: that gate has no external consumer either.

## 3. Findings
F-001 [S3]
Where: error.rs:6-11
Code:
```
//! The one engine-facing conversion is the **decode-refusal gate**:
//! [`Mpeg12Error::decode_refusal`] builds the typed
//! [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
//! that the codec-api registry receives for `mpeg1video` /
//! `mpeg2video` descriptors while software decode is `blocked` —
```
What is wrong: no registry receives it. Repo-wide search (rg over all crates/*/src, excluding mpeg12/**): `decode_refusal` is called only by facts.rs:260-261 (`Mpeg12Facts::decode_refusal`, itself called only by facts.rs:418 — a test) and by mpeg12/mod.rs's doc link; `descriptor_gate`/`accepts` likewise have zero non-test consumers; `Mpeg12Facts::scan` is called only by tests/mpeg12_corpus.rs. "Engine-facing" and "the codec-api registry receives" describe wiring that does not exist; the module doc in video/mod.rs:36-38 admits the wiring is pending ("unconditional until the registration card's per-codec gates"). Why it matters: a reader (or the matrix evidence chain) would believe playing an MPEG-2 file surfaces this typed refusal; in reality nothing dispatches into the module — the engine's generic no-decoder path (the mjpeg/pcm first-vertical registry) is what answers today. What correct looks like: either wire the gate into the registry or soften the doc to "the refusal the registry WILL receive once the dispatch card lands". Confidence: HIGH — Verification: read + rg (zero external callers, enumerated).
Severity note: not S2 — the feature-matrix row is honestly blocked with empty evidence, so no user-facing claim is contradicted; the defect is the module doc describing future wiring as present.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Mpeg12Error (enum) + Display | 17-66 | REAL | — | consumed by headers.rs parse paths (rg-verified); Display pinned by test |
| std::error::Error impl | 68 | REAL | — | |
| DESCRIPTORS / accepts | 73-79 | REAL | S3* | exact-two gate; tested; but see F-001 — no external consumer |
| decode_refusal | 91-112 | REAL | S3* | typed Unsupported + `mpeg12.decode.blocked` + Detail source; tested; UNREACHABLE (F-001) |
| Detail + impls | 117-126 | REAL | — | 2.B4 pattern: reason in source, not Display |
| tests ×4 | 133-184 | REAL | — | baseline :845-848 all ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_exactly_the_two_ids | YES | exact-set incl. case + empty string |
| decode_refusal_is_typed_unsupported_for_both_descriptors | YES | kind + code pinned |
| decode_refusal_names_foreign_descriptors | YES | source-downcast detail asserted |
| parse_errors_render_their_rule | YES | exact Display strings |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the typed Unsupported that the codec-api registry receives for mpeg1video/mpeg2video descriptors" | error.rs:6-11 | FALSE (wiring absent) | F-001: zero non-test callers repo-wide |
| "an honest, observable refusal, never a silent no-op" | :11 | REAL (as a value) / UNREACHABLE (as behavior) | the EngineError is correctly built and tested; nothing observes it in the shipped path |
| "sequence-layer parser + typed refusal LANDED" | docs/feature-matrix.toml:524 (blocked_reason) | REAL (code) / UNREACHABLE (wiring) | module + tests exist and pass; no dispatch |
| matrix row status blocked ×4, evidence "" | docs/feature-matrix.toml:513-522 | REAL (honest) | matches the no-Decoder-impl fact (mpeg12/mod.rs:18-19 "no Decoder impl lives under this module tree") |

## 7. Dead / duplicated / unreachable code
The entire module's public surface is currently library-internal-only (compiled, tested, never dispatched) — F-001. This is a deliberate, documented holding pattern (Phase-6 admission), not rot; but per the audit's reachability rule it is UNREACHABLE from the shipped entry points.

## 8. Dependencies & flags
Depends only on media_types::error. Unconditional `pub mod mpeg12` (no cargo feature gate — video/mod.rs:39), so it IS in the shipped binary's codegen but on no executed path.

## 9. Open questions for the next agent
- mpeg12/{mod,headers,facts,startcode,bitreader}.rs auditors: same reachability verdict applies to the whole module — coordinate one module-level call.
- Whoever owns the registration card: wiring `descriptor_gate`+`decode_refusal` into the codec-api registry is the missing step that would flip this file's doc claim true.
