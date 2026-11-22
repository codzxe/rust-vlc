# AUDIT: crates/demux/src/mkv/error.rs

| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-13 |
| Lines / functions | 284 / 14 (13 constructors + `mint`) + 3 tests |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — every constructor has in-tree callers on the open/start/seek/next_event paths: mod.rs:345 (`unknown_doctype`), :369 (`not_matroska`), :435/438 (`no_clusters`/`no_tracks`), :818 (`timecode_overflow`), :847/850/883/893 (`seek_out_of_range`), :867 (`scan_too_slow`), :919/1005/1028 (`not_started`); header.rs:228/280/330/341; cluster.rs:431/433; cues.rs:92; ebml.rs:467/479; tags.rs (bad_structure/truncated). Chain to CLI: apps/cli/src/containers/mkv.rs → demux::mkv::MkvDemuxer. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | `Read` full file; `sed -n '116,127p;168,175p' error.rs` (quote verification, exit 0); `rg` for all 13 constructor names across `crates/demux/src/mkv` (exit 0); `rg 'not-started' crates/demux/src/error.rs` (exit 0). No cargo (adaptation). |

## 1. One-paragraph verdict

A pure typed-error constructor module: 13 thin functions, each minting a stage-`Demux` `EngineError` with a stable dotted code, on top of the shared media-types taxonomy — the file contains no parsing, no arithmetic, no state, and no panicking construct. Every constructor is reachable from the shipped demux path, and the three in-file tests genuinely pin stage, code, kind, and Display shape (they would fail on regression). The defects are all documentation-attribution drift: the header claims every code is `demux.mkv.*` while `not_started` mints `demux.not-started` (the identical code the crate-level `crate::error::not_started` already mints, making the two refusal sources indistinguishable); `timecode_overflow` accepts an offset parameter it discards; and `bad_vint`'s doc lists two trigger cases that no call site implements (unknown-size-on-non-master and size-vint-overrun are minted elsewhere as `bad_structure`/`truncated`). REAL-UNPROVEN only because I could not execute the tests (no cargo); nothing in the file is capable of crashing.

## 2. What actually works (verified)

- All 13 constructors return correctly-shaped `EngineError`s — verified by reading each body (every one is a 3-10 line `mint(...)` call) and by the in-file tests at 204-283, which assert stage, `context.code`, `ErrorKind` payload equality, and `Display` prefix/suffix for all 13.
- No secrets: no constructor takes a path, URI, or payload; the only byte content is `not_matroska`'s `sniffed` (≤ 4 bytes at its only real call site, header.rs:277+280) and `unknown_doctype`'s doctype string; test 3 (277-283) asserts the 64-byte probe input does not survive Display. Verified.
- No `unwrap`, no `expect`, no indexing, no arithmetic — grep over the file (read) confirms the only operations are struct construction and `to_vec()` on bounded sniffed input.

## 3. Findings

### ER-001 [S4] `timecode_overflow` accepts an offset it silently discards
Where: crates/demux/src/mkv/error.rs:120-127
Code:
```rust
pub fn timecode_overflow(_offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Timing,
        },
```
What is wrong: the parameter is `_offset` — underscore-ignored. Callers pass real diagnostic data: mod.rs:818 `error::timecode_overflow(block.payload_offset)` and cluster.rs:431/433 `error::timecode_overflow(offset)`. The `CorruptDetail::Timing` taxonomy variant carries no offset field, so the position of the corrupt block is dropped from every "timecode overflow" diagnostic.
Why it matters: §3.1 hit (parameter accepted but ignored). For a user debugging a corrupt file, the one number that locates the failure is collected and thrown away; every other constructor in this file carries its offset.
What correct looks like: drop the parameter, or extend the taxonomy variant.
Confidence: HIGH — Verification: read + grep (call sites listed above).
(Note: the root cause is the taxonomy variant's shape — media_types' file, not this one — the defect here is keeping the dead parameter to fake uniformity.)

### ER-002 [S4] `not_started` breaks the file's own `demux.mkv.*` code claim and duplicates the crate-level constructor's code
Where: crates/demux/src/mkv/error.rs:10-11 vs 173-175
Code:
```rust
//! Every constructor mints stage [`PipelineStage::Demux`] with a stable
//! `demux.mkv.*` context code.
```
and
```rust
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: `not_started`'s code is `demux.not-started` — not `demux.mkv.*`. It is byte-identical to the crate-level constructor's code (`crates/demux/src/error.rs:183`, same string), which the body comment admits is deliberate ("mirrored here so the mkv zone stays self-contained").
Why it matters: two different refusal sites (any non-mkv demuxer's not-started vs the mkv demuxer's) mint indistinguishable errors — attribution is impossible from the code alone, and the header's stated invariant is false within its own file.
What correct looks like: `demux.mkv.not-started`, accepting the one-line divergence from the crate-level twin.
Confidence: HIGH — Verification: read + grep (`rg 'not-started' crates/demux/src/error.rs` → line 183).

### ER-003 [S4] `bad_vint`'s doc lists two trigger cases that no call site implements
Where: crates/demux/src/mkv/error.rs:86-89
Code:
```rust
/// An EBML variable-length integer is structurally invalid: a zero
/// length byte (leading zero), an element ID longer than 4 octets, an
/// unknown-size marker on a non-master element, or a size vint that
/// overruns its read window.
```
What is wrong: repo-wide grep for `bad_vint` shows exactly four mint sites: ebml.rs:467 and :479 (vint-grammar `Invalid` — the first two doc cases, real) and header.rs:328/:339 (a fixed-width uint field wider than 8 bytes — a case the doc does not list, and which `bad_structure`'s own doc at 101-104 claims for itself: "a fixed-value field out of grammar"). The two remaining doc cases are dead: unknown-size-on-non-master is minted as `bad_structure` (mod.rs:567, 632, 728), and a size vint cut by the window is `truncated` (ebml.rs:465/477) or a clean `Ok(None)` (ebml.rs:461-463).
Why it matters: the error taxonomy is this project's diagnostic surface; docs that name trigger cases that live under a different constructor (and vice versa) misdirect anyone debugging from an error string. Also note this contradicts ebml.rs:15-17, which promises `bad_vint` for the unknown-size-on-non-master case (recorded in the ebml.rs audit, E-006/doc table).
What correct looks like: docs that match the four real mint sites.
Confidence: HIGH — Verification: read + grep (all `bad_vint` sites enumerated).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` (private) | 27-29 | REAL | — | Single construction seam; correct. |
| `not_matroska` | 35-45 | REAL | — | Copies `sniffed` (≤4 B at the real call site); Unsupported + FormatInfo; tested 207, 238-240, 277-283. |
| `unknown_doctype` | 51-61 | REAL | — | Unsupported with the rejected doctype name; tested 208, 241-244. |
| `unsupported_ebml_limits` | 66-71 | REAL | — | Typed refusal for MaxIDLength/MaxSizeLength outside 1..=8; called header.rs:330/341. |
| `truncated` | 77-84 | REAL | — | Corrupt::Truncated{expected,found}; ~20 call sites; tested 210, 245-253. |
| `bad_vint` | 90-99 | REAL | S4 | Doc lists nonexistent trigger cases (ER-003); kind correct; tested 211, 254-259. |
| `bad_structure` | 105-114 | REAL | — | Corrupt::InvalidStructure{offset}; the mkv zone's workhorse refusal; tested 212. |
| `timecode_overflow` | 120-127 | REAL | S4 | `_offset` discarded (ER-001); tested 213, 260-265. |
| `no_tracks` | 131-141 | REAL | — | Unsupported (not Corrupt) — defensible: a track-less file is out of scope, not damaged; called mod.rs:438, header.rs:228. |
| `no_clusters` | 146-153 | REAL | — | Corrupt::InvalidStructure{offset:None}; called mod.rs:435. |
| `cue_table_too_large` | 158-167 | REAL | — | ResourceLimit::Memory; the cue-bomb refusal; called cues.rs:92; tested 216-219, 266-273. |
| `not_started` | 173-175 | REAL | S4 | Code `demux.not-started` breaks the header's `demux.mkv.*` claim and duplicates src/error.rs:183 (ER-002); tested 220. |
| `seek_out_of_range` | 179-184 | REAL | — | Unsupported; called mod.rs:847/850/883/893. |
| `scan_too_slow` | 189-198 | REAL | — | ResourceLimit::CpuTime with byte budget; called mod.rs:867; tested 222. |
| `tests::errors_carry_demux_stage_and_stable_codes` | 204-233 | REAL | — | Pins all 13 codes + Display shape; would fail on any code/kind change. |
| `tests::kinds_map_to_the_documented_taxonomy` | 235-274 | REAL | — | Payload equality on 6 kinds; would fail on kind drift. |
| `tests::not_matroska_sniff_capped_at_sixteen_bytes_by_display` | 276-283 | REAL | — | Proves the no-secrets Display cap with a 64-byte probe. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_stable_codes (204) | YES | Exact equality on stage + code for all 13; Display prefix/suffix asserted. |
| kinds_map_to_the_documented_taxonomy (235) | YES | `assert_eq!` on full `ErrorKind` payloads. |
| not_matroska_sniff_capped_at_sixteen_bytes_by_display (276) | YES | Counts hex pairs in rendered text; 64 in, ≤16 out. |
| Coverage gap | — | No test can catch a *call site* choosing the wrong constructor (that is a per-walker concern); the three ER findings are doc/attribution issues the tests structurally cannot see. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every constructor mints stage Demux with a stable `demux.mkv.*` context code" | error.rs:10-11 | FALSE | `not_started` mints `demux.not-started` (line 174). ER-002. |
| bad_vint covers "an unknown-size marker on a non-master element, or a size vint that overruns its read window" | error.rs:86-89 | FALSE | No mint site implements either case; both live under `bad_structure`/`truncated` (grep, all sites listed in ER-003). |
| "Constructors take byte counts, offsets and format names only — never paths, URIs or file content" | error.rs:15-16 | REAL | Verified by reading all 13 signatures + bodies; sniffed bytes ≤ 4 at the real call site; Display cap proven by test 276-283. |
| "the mkv zone stays self-contained ... the R3 registry merge is expected to hoist a shared set later" | error.rs:5-8 | REAL-UNPROVEN | The self-containment is real (only media_types imports); whether any "R3 registry merge" is planned is a doc-plan claim outside this file (UNVERIFIED). |

## 7. Dead / duplicated / unreachable code

- `not_started` duplicates `crate::error::not_started` (src/error.rs:182-184) including its code string (ER-002) — deliberate, commented, still a duplication.
- `bad_vint`'s doc-dead cases (ER-003) are documentation, not dead code; all 13 functions are called (grep proof in the reachability row).
- No unreachable constructors.

## 8. Dependencies & flags

- `media_types` only (`EngineError`, `ErrorKind`, `PipelineStage`, error payload types). No feature flags, no `unsafe`, no I/O. The taxonomy's limits leak into this file twice (ER-001's discarded offset; the sniffed-bytes Display cap living in media_types).

## 9. Open questions for the next agent

- media_types owner: `CorruptDetail::Timing` carries no offset — should it? (Two mkv call sites currently discard position data, ER-001.)
- mod.rs/header.rs owners: `bad_vint` vs `bad_structure` choice for width violations is inconsistent with both constructors' docs (ER-003) — pick one taxonomy rule and fix the docs or the call sites.
- src/error.rs owner: is the deliberate code duplication of `not_started` (ER-002) load-bearing anywhere (e.g., a consumer matching on the string)?
