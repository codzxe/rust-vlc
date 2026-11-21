# AUDIT: crates/demux/src/mkv/error.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-21 |
| Lines / functions | 284 / 17 items (STAGE, mint, 13 constructors, 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (every constructor has live callers on the mkv demux path — rg counts: truncated 98, bad_structure 98, not_started mod.rs:919/1005/1028, seek_out_of_range mod.rs:847-893, scan_too_slow mod.rs:867, cue_table_too_large cues.rs, timecode_overflow mod.rs/cluster.rs, no_tracks/no_clusters/unknown_doctype/unsupported_ebml_limits/not_matroska header.rs+mod.rs; chain: `lumen open` → containers/mod.rs:57 → MkvDemuxer → this module) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | rg caller sweeps per constructor (13/13 live); rg `not-started` in demux/src/error.rs (AVI twin); sed -n quote verification (checked for the `[h` display-artifact trigger — none of the quoted lines contain it); baseline cites below |

## 1. One-paragraph verdict
A small, honest constructor module: all 13 constructors are live, the taxonomy is correctly reused (never redefined), codes are stable dotted strings, the no-secrets discipline holds, and the three tests are real mutation-killers executed in the baseline. All four findings are doc/contract drift, not behavior: `timecode_overflow` takes an `_offset` it silently discards, the module doc's "every constructor … `demux.mkv.*`" claim is false for `not_started` (deliberately the AVI twin's `demux.not-started`), `bad_vint`'s doc lists a trigger no caller ever mints, and `truncated`'s documented `expected`/`found` contract is violated by three call-site shapes — including two sites in ebml.rs that mint inverted pairs (`expected ≤ found`).

## 2. What actually works (verified)
- Executed: baseline test-workspace.log.txt:1790-1792 — `mkv::error::tests::{errors_carry_demux_stage_and_stable_codes, kinds_map_to_the_documented_taxonomy, not_matroska_sniff_capped_at_sixteen_bytes_by_display}` all `ok`. The codes are additionally pinned end-to-end by integration tests that ran in the same baseline: mkv_fixtures.rs:425/431 (`demux.mkv.not-matroska`), :445 (`demux.mkv.truncated`), :420 (`demux.mkv.unknown-doctype`).
- All 13 constructors live (rg sweep, counts in the header table); none dead.
- No-secrets: verified by reading every signature — byte counts, offsets, format names; the only bytes carried are `not_matroska`'s sniffed prefix, whose callers pass ≤ 4 bytes (header.rs:277-280 `read_prefix_at(4, …)`) or empty (mod.rs:369) — the 16-byte Display cap tested at line 277-283 never binds in production but is correctly tested anyway.
- Panic/alloc audit: `to_vec`/`to_string` on caller-bounded inputs (sniffed ≤4 B in practice; doctype ≤ MAX_TEXT_FIELD 4 KiB at the caller); no unwrap/expect/index/unsafe.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/mkv/error.rs:116-120
Code:
```
/// A block's timecode arithmetic overflows the presentation timeline
/// (cluster timecode + relative timecode outside `i64` scale ticks, or
/// a negative absolute stamp). Wrong timestamps are corruption, not a
/// rounding choice — the typed `Corrupt::Timing` flavor.
pub fn timecode_overflow(_offset: u64) -> EngineError {
```
What is wrong: the parameter is `_offset` — accepted and discarded. `CorruptDetail::Timing` carries no offset, so the error names no position, while every caller passes a real one (e.g. mod.rs's emit path passes `block.payload_offset`).
Why it matters: the signature advertises positional context that the error will not carry; a maintainer debugging a timecode-overflow file gets "corrupt timing" with no offset while every sibling constructor (`bad_vint`, `bad_structure`) points at the byte. Misleading API shape ("ignored parameters" — checklist §Fakeness).
What correct looks like: drop the parameter, or extend the taxonomy's `CorruptDetail::Timing` with an offset field — either way the signature stops lying.
Confidence: HIGH — Verification: read (signature + CorruptDetail usage; callers pass offsets that vanish).

### F-002 [S4]
Where: crates/demux/src/mkv/error.rs:10-11 vs 173-175
Code:
```
//! Every constructor mints stage [`PipelineStage::Demux`] with a stable
//! `demux.mkv.*` context code.
```
…
```
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: `not_started`'s code is `demux.not-started` — not in the `demux.mkv.*` zone the doc promises for "every constructor". It deliberately duplicates the AVI vertical's constructor (demux/src/error.rs:182-183 mints the identical code), which is a defensible operational choice (one code, one meaning, zone-agnostic) — but the module doc's universal claim is false.
Why it matters: code-stability contracts are what consumers (and this audit) grep against; a doc that says "every constructor is `demux.mkv.*`" while one is `demux.not-started` invites either a wrong grep or a "fix" that renames a stable code.
What correct looks like: doc: "every constructor except the shared `not_started` twin … whose code is the zone-agnostic `demux.not-started`".
Confidence: HIGH — Verification: read (both sites; AVI twin at demux/src/error.rs:182-183 verified identical).

### F-003 [S4]
Where: crates/demux/src/mkv/error.rs:86-89
Code:
```
/// An EBML variable-length integer is structurally invalid: a zero
/// length byte (leading zero), an element ID longer than 4 octets, an
/// unknown-size marker on a non-master element, or a size vint that
/// overruns its read window.
pub fn bad_vint(offset: u64) -> EngineError {
```
What is wrong: the third listed trigger — "an unknown-size marker on a non-master element" — is minted by no caller, ever. Every unknown-size-non-master site uses `bad_structure` (header.rs:410-411 and 751-753, attachments.rs:163-167 and 210-212, cluster.rs's parse_timecode). `bad_vint`'s live triggers are only the vint-grammar refusals in ebml.rs:467/479.
Why it matters: the doc invents a mapping that the code deliberately contradicts (ebml.rs:15-17's module doc makes the same false promise from the other side — see my ebml.rs audit F-005). Two docs, one wrong rule, zero code enforcing it.
What correct looks like: delete the third clause here, and align ebml.rs:15-17 with the `bad_structure` reality.
Confidence: HIGH — Verification: rg all `bad_vint(` call sites (4, all in ebml.rs parse paths) + read of every unknown-size refusal site.

### F-004 [S4]
Where: crates/demux/src/mkv/error.rs:73-77 (contract) vs ebml.rs:465/477 and attachments.rs:172 (violating callers)
Code:
```
/// The container declares more structure than the stream carries
/// (truncated EBML header, truncated master element, truncated block
/// payload at a structure boundary). `expected` is the absolute end the
/// structure needed; `found` is the stream length actually available.
pub fn truncated(expected: u64, found: u64) -> EngineError {
```
What is wrong: the documented invariant (expected = needed end > found = available stream length) is broken by three caller shapes: (a) ebml.rs:465 `error::truncated(pos + 1, pos + bytes.len() as u64)` — with `bytes.len() ≥ 1` this mints expected ≤ found ("needed 101, found 102"), an inverted pair; (b) ebml.rs:477 same shape (`after_id + 1` vs `pos + bytes.len()`, inverted whenever any size-vint bytes were read); (c) attachments.rs:172 `error::truncated(child_end, end)` — `found` is the parent master's declared end, not "the stream length actually available" (the stream can be longer).
Why it matters: `CorruptDetail::Truncated{expected, found}` renders those numbers to users/logs; inverted or misattributed pairs produce nonsense messages ("expected fewer bytes than found") that erode trust in the typed-error story. No functional impact — kind and code are still correct, and the mkv suite pins the code string, not the pair.
What correct looks like: ebml.rs should mint `expected = pos + bytes.len() + 1` (the first byte beyond the stream) at the two mid-vint sites; attachments' contained-lie should either pass the stream length or the doc should say "found = the enclosing bound the read was clamped to".
Confidence: HIGH — Verification: read of the three call sites + arithmetic on each branch condition.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| STAGE const | 23-24 | REAL | — | PipelineStage::Demux, asserted by test 1 for all 13. |
| mint | 26-29 | REAL | — | The single construction seam; no logic to get wrong. |
| not_matroska | 35-45 | REAL | — | Carries sniffed prefix (≤4 B from live callers); Display cap tested. |
| unknown_doctype | 51-61 | REAL | — | Name-only; doctype bounded by caller's text ceiling. |
| unsupported_ebml_limits | 66-71 | REAL | — | Used by header.rs limits checks (2 sites). |
| truncated | 77-84 | REAL | S4 (F-004) | Contract drift at 3 of 98 call sites. |
| bad_vint | 90-99 | REAL | S4 (F-003) | Doc lists a trigger no caller mints. |
| bad_structure | 105-114 | REAL | — | The workhorse (98 sites); carries offset. |
| timecode_overflow | 120-127 | REAL | S4 (F-001) | `_offset` discarded. |
| no_tracks | 131-141 | REAL | — | Unsupported + format name. |
| no_clusters | 146-153 | REAL | — | Corrupt/InvalidStructure, offset None (no position exists — honest). |
| cue_table_too_large | 158-167 | REAL | — | ResourceLimit/Memory with budget+attempted; consumed by cues.rs ceiling. |
| not_started | 173-175 | REAL | S4 (F-002) | Code outside the documented mkv zone; AVI-twin duplicate by design. |
| seek_out_of_range | 179-184 | REAL | — | Unsupported; 5+ live sites in mod.rs seek paths. |
| scan_too_slow | 189-198 | REAL | — | ResourceLimit/CpuTime carrying byte budgets (units documented in-field). |
| tests (3) | 200-284 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_stable_codes (204-233) | YES | All 13 constructors × stage + exact code string + Display prefix/suffix shape; any code rename, stage change, or Display-format change fails. |
| kinds_map_to_the_documented_taxonomy (235-274) | YES | Full ErrorKind equality for truncated/bad_vind/timecode/cue-table shapes; a kind swap fails. |
| not_matroska_sniff_capped_at_sixteen_bytes_by_display (276-283) | YES | 64 input bytes must not survive rendering; a Display-cap regression fails the count. Mutation check: removing the cap in media-types' Display doubles the "ab" matches → fail. |
| Coverage gap | — | Nothing pins the expected>found invariant (would have caught F-004) or that timecode_overflow's offset is used (impossible today — see F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every constructor mints … a stable `demux.mkv.*` context code" | error.rs:10-11 | FALSE (one exception) | F-002: not_started → `demux.not-started`. |
| "error.rs belongs to 5.D1; the parallel D2a agent owns its own additions" | container-matroska.md:72-74 | REAL | tags.rs/editions.rs/chapters.rs/attachments.rs mint zone codes locally (verified in my attachments.rs audit; chapters/tags own files). |
| "Error paths (all typed, no panics …)" | container-matroska.md:63-68 | REAL | This module + the mkv parsers hold to it (no panic path found in ebml/attachments/error; baseline clippy -D warnings clean #5/#6). |
| "Constructors take byte counts, offsets and format names only — never paths, URIs or file content" | error.rs:15-16 | SHALLOW (one nuance) | not_matroska carries sniffed file-content bytes (first ≤4) — acknowledged two lines later; bounded + Display-capped. |

## 7. Dead / duplicated / unreachable code
- None dead: all 13 constructors have live callers.
- Duplication: `not_started` duplicates demux/src/error.rs:182-183 verbatim (same code string, same kind) — deliberate zone self-containment per the module doc; the R3 registry merge is the recorded hoist plan (error.rs:5-8).

## 8. Dependencies & flags
- Only `media_types` (the taxonomy — correctly not redefined). No feature gates, no unsafe, no external deps. Matches the C5 claim.

## 9. Open questions for the next agent
1. media-types owner: `CorruptDetail::Timing` has no offset variant — F-001's discard is taxonomy-forced; decide extend-or-drop.
2. ebml.rs owner: the two inverted `truncated` pairs (F-004 a/b) live in parse_header_at — one-line fixes there.
3. Whoever consolidates zone error modules (R3 merge): the `demux.not-started` code collision across two modules is fine today; keep it deliberate when hoisting.
