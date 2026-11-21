# AUDIT: crates/codecs/src/audio/opus/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Lines / functions | 184 / 7 (enum + 3 impls/fns + 2 constructors + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.ogg` → containers "ogg" → pipeline_host.rs:276-283 `"opus"` arm → `OpusDecoder::configure/decode` → `OpusError::{Unsupported,Corrupt,BadState}` via `into_engine_error()` → the CLI's error surface). Executed: baseline test-workspace.log.txt:451 `audio::opus::error::tests::taxonomy_maps_each_variant_to_its_public_kind ... ok`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "OpusError\|unsupported\|corrupt" crates/codecs/src/audio/opus` (used by decoder.rs:79,351-369,432-436 and head.rs via super::error); `wc -l */error.rs` under audio/ (7 sibling files, 1347 lines total — the duplication family); `rg -n "struct Detail\|fn unsupported\|fn corrupt" flac/error.rs vorbis/error.rs mp3/error.rs` (identical pattern confirmed); `sed -n '151,175p' docs/dependencies/records/ruopus-0.1.2.md` (binding-condition ground truth); Read on the full file. |

## 1. One-paragraph verdict
A small, correct, total error taxonomy: three variants, one-to-one mapping onto public `EngineError` kinds, structured `CorruptDetail` carried through, human-readable reason in the error chain — and the module's "nothing panics; every constructor is total" claim is verified by inspection (no unwrap/expect/index/panic in the file). The one test asserts exact rendered strings, stage tags, and chain-source text for all three variants — it would fail under any mapping drift. Both findings are hygiene: this is the seventh near-verbatim copy of the same ~185-line pattern across the audio codec family (1347 lines total), and its doc's binding-condition citation disagrees with decoder.rs's own section numbering (error.rs matches the admission record; decoder.rs's header deviates — the defect lives there, flagged for the part2 owner).

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:451` → the taxonomy test `ok` (baseline run #3).
— Verified by read: every constructor (`unsupported`, `corrupt`) is total; `into_engine_error` covers all three variants exhaustively; `Detail` implements `Display` + `Error` (chain-root); no panics, no ignored parameters, no `unsafe`.
— Verified by `rg` in decoder.rs (my part1 audit): every `OpusError` construction site passes a stable dotted code + structured detail (`opus.head.missing`, `opus.packet.decode`, `opus.packet.channel_multiple`, `opus.output.slot_size`, `opus.not_configured`) — the taxonomy is actually used, not decorative.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/opus/error.rs:110-142 (family-wide pattern)
Code:
```
/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);
```
What is wrong: This file is one of seven near-verbatim copies of the same taxonomy skeleton across `crates/codecs/src/audio/{flac,mp3,aac,vorbis,alac,opus,pcm}/error.rs` (1347 lines total): same `Detail(String)` carrier, same `unsupported`/`corrupt` constructors, same enum shape, same mapping table — differing only in the codec name and code strings. The pattern is self-acknowledged ("the 5.B2 Vorbis pattern", "the 2.B4 FeatureDetail pattern verbatim") but nothing prevents the copies from drifting (they already differ in test coverage: only vorbis/flac carry an extra `corrupt_variant_carries_the_structured_class`-style test; opus's single test is the thinnest).
Why it matters: Seven copies of a mapping contract means seven places to fix when `EngineError`'s shape changes; a missed copy compiles fine and silently diverges in error reporting.
What correct looks like: A small generic `CodecError` (or macro) parameterized by codec name + code constants, with per-codec code lists kept local — the structure is identical, only the strings differ.
Confidence: HIGH — Verification: read + wc/rg across the seven files.

### F-002 [S4]
Where: crates/codecs/src/audio/opus/error.rs:12-16 (vs decoder.rs:19-27 and docs/dependencies/records/ruopus-0.1.2.md:151-166)
Code:
```
//!   these are **pre-constructor validations** (the ruopus
//!   constructors panic on channels∉{1,2}/rate∉{8..48}k — the
//!   admission record's binding condition 2: validate from OpusHead
//!   BEFORE the engine is ever built, so the panic contract is
//!   unreachable through this wrapper).
```
What is wrong: Cross-file doc inconsistency about the admission record's "two binding conditions". The record (ruopus-0.1.2.md:158-166) defines: condition 1 = the Cargo feature pin (default-features off, std only — non-negotiable); condition 2 = validate channels/rate from OpusHead before the constructor AND implement pre-skip + end-trim + channel-order mapping. This file cites the pre-constructor gate as "binding condition 2" — correct per the record. But decoder.rs:19-27 (part1, my previous audit) renumbers: a section titled "The two binding conditions (admission record, honored here)" lists "1. The pre-constructor panic gate" and "2. The valid-timeline window", while the actual condition 1 (the feature pin) is discussed separately in the engine section. A reader following either file's numbering into the record finds the other numbering contradicted.
Why it matters: Admission-record conditions are the compliance chain for external-crate admission (doc 11 §3.7 re-triggers); misnumbered citations make audit-by-citation unreliable — exactly the provenance chain a Devil Audit must be able to follow.
What correct looks like: decoder.rs's section header should say "the record's condition 2, in its two halves" (or renumber against the record); error.rs is already right.
Confidence: HIGH — Verification: read of all three sites (record quoted verbatim above in commands).
Affected sibling: crates/codecs/src/audio/opus/decoder.rs:19-27 — flagged for the part2 owner (W07-A07); my part1 audit's doc-claims table quoted the section without catching the numbering conflict.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `OpusError` enum | 41-65 | REAL | — | 3 variants; doc table (35-39) matches the mapping code exactly (verified line by line). |
| `into_engine_error` | 69-101 | REAL | — | Exhaustive match; Unsupported carries `FormatInfo { name: Some("opus") }` (one small allocation per error — error path, fine); Corrupt passes the structured `CorruptDetail` through; BadState → `ErrorKind::Decoder{BadState}` + code `opus.not_configured` — accurate (BadState fires only pre-configure in decoder.rs:321-331, 491-493). |
| `From<OpusError>` | 104-108 | REAL | — | Delegates to into_engine_error. |
| `Detail` + `Display` + `Error` | 112-121 | REAL | F-001 | Chain-root carrier; verbatim family copy. |
| `unsupported` | 124-129 | REAL | — | Total constructor. |
| `corrupt` | 132-142 | REAL | — | Total constructor; structured detail required by type. |
| test `taxonomy_maps_each_variant_to_its_public_kind` | 148-183 | REAL | — | Exact-string assertions on `to_string()`, stage, and chain source for all 3 variants — would fail under any mapping/renaming drift; executed green (baseline :451). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind (148) | YES | Asserts the rendered public string byte-for-byte (`"[decode] unsupported in opus (opus.head.magic)"` etc.), the stage, and the chain-source text — any variant→kind remap, stage change, or dropped source breaks it. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | module docs 25-26 | REAL | Read: zero panic-capable constructs in the file. |
| "the ruopus constructors panic on channels∉{1,2}/rate∉{8..48}k — the admission record's binding condition 2" | module docs 12-16 | REAL (vs record) / INCONSISTENT with decoder.rs | Record 158-166 confirms condition 2 covers the gate; decoder.rs:19-27 renumbers — F-002. |
| "Isolation is packet-scoped (doc 03 §4.5(d)): a damaged packet poisons at most itself" | module docs 19-23 | REAL | decoder.rs decode has no cross-packet state besides counters (my part1 trace); the 6-case hostile measurement is the admission record's claim (external, cited not re-run). |
| Variant→kind table (37-39) | module docs 35-39 | REAL | Matches into_engine_error 72-100 exactly. |

## 7. Dead / duplicated / unreachable code
No dead code. Duplication: the 7-file family (F-001). `OpusError`'s `PartialEq, Eq` derives are used only by tests — harmless.

## 8. Dependencies & flags
Depends only on media-types error vocabulary. Rides `codec-opus` (default-on). No unsafe, no external deps.

## 9. Open questions for the next agent
- W07-A07 (decoder.rs part2): fix the "two binding conditions" section header numbering per F-002.
- Whoever audits the sibling error.rs files: same F-001 duplication applies; consider a wave-level consolidation recommendation.
