# AUDIT: crates/subtitles/src/model/cue.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 148 / 6 (CueId::{new,get}, CueText::{total_len,is_plain}, Cue::new, 2 tests) |
| File verdict | UNREACHABLE (clean data model, consumed crate-wide; no shipped entry point) |
| Reachable from shipped entry point? | NO — crate-level wall (only fuzz/Cargo.toml:50 links `subtitles`); inside the crate `Cue`/`CueText`/`CueId` are the universal currency (srt/parser.rs:54,673; vtt/mod.rs:69; ass/parser.rs:683; srt/render.rs:46) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 (the S3-class WebVTT-id defect is folded into F-001, graded at the higher level per spec §4.6) |
| Commands run | Grep `CueId|identifier` in vtt (vtt/mod.rs:231-249 handling found); Grep `auto_id` in vtt/mod.rs (:143 init 0); sed quote verifications (exit 0); baseline test citation |

## 1. One-paragraph verdict
A small, correct, well-typed data model — the crate's universal Cue/CueText/CueId vocabulary — with one false doc claim at its heart: CueId is documented as carrying "WebVTT cue identifiers", but WebVTT ids are arbitrary strings that do not fit a u64 and are silently discarded by the vtt parser (replaced by auto-assigned counters with no damage event, in violation of the crate's own damage-is-data doctrine, and with collisions possible against real numeric ids). Everything else (half-open interval composition, plain/styled split with saturating total_len) is sound.

## 2. What actually works (verified)
— Baseline (audit/runs/baseline/cargo-test-workspace.txt:5889-5890): both `model::cue::tests` pass. `CueInterval::new(10,9)` typed-invalidity, `total_len` on plain/styled/empty payloads, and const-ness of `Cue::new` all verified by read; `usize::saturating_add` fold is overflow-safe by construction.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/model/cue.rs:8-11
Code:
```
/// A cue's identity within a track — opaque, stable, comparable.
/// Newtype over `u64` (SRT numbering, Matroska block UIDs and WebVTT
/// cue identifiers all fit). Copy + `Ord` so documents can be sorted
/// or deduplicated without borrowing.
```
What is wrong: the claim is false for WebVTT and the "stable identity" contract is not upheld by the vtt path. WebVTT cue identifiers are arbitrary strings (the spec default is a text line); vtt/mod.rs:245-249 replaces every non-numeric id with an auto counter — `*auto_id = auto_id.saturating_add(1); CueId::new(*auto_id)` — with NO damage event recorded (unlike every other tolerated malformation in the crate's damage-ledger design), so a differential test cannot even observe the loss. Worse, `auto_id` starts at 0 and counts 1,2,3… (vtt/mod.rs:143), so a file mixing numeric ids and id-less cues (e.g. numeric id "2" plus two id-less cues) produces DUPLICATE CueIds in one document, contradicting "stable identity within the track" and breaking the documented sort/dedup use. Why it matters: identity is this type's entire job. What correct looks like: either an `Id::Text(String)` variant or a guaranteed-unique auto id namespace (e.g. high-bit offset), plus a damage event for discarded string ids. Confidence: HIGH — Verification: read (vtt/mod.rs:143,231-249).
Note: this same crate-level unreachability as every other file in this assignment applies (no shipped app links the crate); the modeling defect is real regardless.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CueId::new`/`get` | 15-26 | REAL | S3 (via F-001) | Sound newtype; the doc contract above it is what's false. |
| `StyledSegment` | 30-36 | REAL | — | text + TextStyle; empty text documented legal. |
| `CueText::{Plain,Styled}` | 44-50 | REAL | — | D-1 memory split is sensible. |
| `CueText::total_len` | 52-64 | REAL | — | Saturating fold, correct. |
| `CueText::is_plain` | 66-70 | REAL | — | matches!. |
| `Cue::new` | 87-104 | REAL | — | Const, non-fallible; components carry validity (CueInterval::new rejects reversed). |
| 2 tests | 115-147 | REAL | — | Honest for a data type. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| cue_assembles_typed_parts | YES | Field-by-field equality + total_len. |
| typed_invalidity_stays_in_components | YES | Exact ModelError variant; empty-payload lens. |
No test covers id uniqueness across a document (F-001's failure mode) — that lives in the vtt parser, which has no such test either (UNVERIFIED beyond grep; vtt/mod.rs auditor should confirm).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SRT numbering, Matroska block UIDs and WebVTT cue identifiers all fit" | cue.rs:9-10 | FALSE (WebVTT) | vtt/mod.rs:245-249 discards non-numeric ids (F-001). |
| "zero-duration is legal" | cue.rs:79 | REAL | [start,start) never active — consistent half-open semantics (compositing timing tests pass in baseline). |
| "a constructed `Cue` is always well-formed" | cue.rs:88-90 | REAL | All components validate in their own constructors (read). |

## 7. Dead / duplicated / unreachable code
None inside the file. Crate-level unreachability per pattern (F-001 note).

## 8. Dependencies & flags
model::position, model::time, model::style. No unsafe, no I/O, no features. Derive set (Hash/Ord) matches the documented sort/dedup purpose.

## 9. Open questions for the next agent
- vtt/mod.rs auditor: confirm no test pins auto-id uniqueness vs numeric ids; propose the damage-event addition for discarded string ids.
- srt/parser.rs: does the SRT path use the file's own index numbers or `doc.cues.len()+1` when indexes are missing/duplicated (duplicate SRT indexes are common in the wild)?
