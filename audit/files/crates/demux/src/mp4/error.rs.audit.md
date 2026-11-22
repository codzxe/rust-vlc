# AUDIT: crates/demux/src/mp4/error.rs

| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-22 |
| Lines / functions | 170 / 13 (1 private `mint` + 12 public constructors) |
| File verdict | REAL (11 of 12 constructors live and wired; 1 dead) |
| Reachable from shipped entry point? | YES. Chain: apps/cli demux host → `demux::mp4::Mp4Demuxer::{new,start,next_event,seek}` → `error::*` call sites throughout mp4/mod.rs (not_mp4 mod.rs:701, truncated :351-405/…, no_moov :761, oversized_box :768, no_tracks :777, no_sample_table :795/1146/1152, not_started :2011-2052, seek_out_of_range :2060-2069, scan_budget_exhausted :718/1856/2210). W00 baseline corroborates the path executes: h264+aac MP4 opens (2 tracks) in the CLI build. Exception: `depth_exceeded` — zero callers repo-wide (see F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg per-constructor call-site census in mp4/mod.rs + fuzz.rs; repo-wide rg `depth_exceeded\|MAX_BOX_DEPTH` (1 unrelated test name in playlists/src/mpd.rs:2531); rg code-string assertions across `src/mp4/tests.rs` + `tests/*.rs`; sed verification of all quotes; read of `media_types::error::ErrorKind` enum and mod.rs call-site regions (330-420, 690-772, 1786-1800, 2173-2185). No cargo (adaptation) → no execution. |

## 1. One-paragraph verdict

This is the answer to "are MP4 errors stringly-typed or swallowed?": they are neither. Every constructor mints a typed `EngineError` at stage `Demux` with a structured payload (`CorruptDetail::Truncated{expected,found}`, `InvalidStructure{offset}`, `ResourceLimit{limit,budget,attempted}`) plus a stable dotted code; all live call sites return `Err(...)` or route through the sticky-replay `self.fail(...)` (mod.rs:690-693) — no `.ok()`, no `let _ =`, no catch-all in the minting or call paths spot-checked. The defects are at the edges: one constructor (`depth_exceeded`) documents and mints a box-depth guard that does not exist anywhere in the parser (its constant `MAX_BOX_DEPTH` is referenced only by a doc comment), and four of the twelve codes — including both memory-refusal codes that implement the MP §12 ceilings — have zero test witnesses in the entire demux test tree, so the resource-limit guarantees are unverifiable-by-suite.

## 2. What actually works (verified)

- Verified statically: 12 typed constructors, zero panics/`unwrap`/indexing in the file; payloads carry numbers and offsets only.
- Verified statically: `not_mp4`'s `sniffed.to_vec()` is bounded at its only call site — `let head = self.reader.read_prefix_at(32, 0)?; ... error::not_mp4(&head)` (mod.rs:699-701) — so ≤32 bytes, honoring the "No secrets / capped sniff" doc.
- Verified statically: `oversized_box`'s `attempted` is the real declared length at the live call site (`error::oversized_box(b"moov", MAX_MOOV_BYTES, moov_len)` mod.rs:768), and `scan_budget_exhausted`'s `attempted == budget` is semantically exact for budget exhaustion.
- 8 of 12 codes are pinned by test assertions (rg counts across src/mp4/tests.rs + tests/): not-mp4 1, truncated 4, box-overruns-parent 2, no-moov 1, no-tracks 2, no-sample-table 1, not-started 2, seek.out-of-range 2. W00 baseline: suite expected green (1751 pass / 2 UI-golden fail / 25 ignored); per-test status UNVERIFIED here.

## 3. Findings

### F-001 [S3] `depth_exceeded` is a dead constructor minting a guard that was never built; `MAX_BOX_DEPTH` is documentation-only
Where: crates/demux/src/mp4/error.rs:78-90
Code:
```rust
/// The box-descent depth exceeded the walk's guard (`moov → trak →
/// mdia → minf → stbl → …` is six; anything deeper than the guard is
/// a cycle-shaped lie, not a file).
pub fn depth_exceeded(offset: u64) -> EngineError {
```
What is wrong: Repo-wide rg for `depth_exceeded` finds only this definition plus an unrelated test name in `crates/playlists/src/mpd.rs:2531` — zero callers. Its companion constant `MAX_BOX_DEPTH` (mod.rs:126, value 8) is referenced only by the module doc "walked in memory with a depth guard ([`MAX_BOX_DEPTH`])" (mod.rs:31) — no code reads it. The walk needs no runtime guard as built: box descent is a fixed ladder of `iter_boxes` calls at hard-coded levels (mod.rs:934, 1072, 1099, 1127, 1148, 1929-1941, 1964 — moov→trak→mdia→minf→stbl and udta→meta→ilst→covr→data), not input-driven recursion, so there is no stack-overflow path today. The defect is the claim, not the safety: the docs assert a guard, the code contains an unused constant and an unreachable error for it.
Why it matters: A maintainer reading mod.rs:31 or this constructor believes deep-nesting input is refused typed; in fact a "cycle-shaped lie" file just falls off the fixed ladder into unknown-box skipping. If anyone later generalizes the descent into real recursion, the guard they'd assume exists is absent.
What correct looks like: Delete `depth_exceeded` + `MAX_BOX_DEPTH`, or wire an actual depth parameter through the descent ladder; fix mod.rs:31's claim.
Confidence: HIGH — Verification: read + repo-wide rg.

### F-002 [S3] Four of twelve error codes have zero test witnesses in the whole demux test tree — both memory-refusal codes are unpinned
Where: crates/demux/src/mp4/error.rs:95-104, 161-170, 53-62 (codes with 0 assertions)
Code:
```rust
pub fn oversized_box(_kind: &[u8; 4], budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
```
What is wrong: rg across `src/mp4/tests.rs` and all of `crates/demux/tests/*.rs` for the code strings: `demux.mp4.bad-box` 0, `demux.mp4.oversized-box` 0, `demux.mp4.scan-budget-exhausted` 0, `demux.mp4.depth-exceeded` 0. The MP4 unit suite (file 1 of this agent) pins 8 codes but never the moov-ceiling refusal (live at mod.rs:767-769), never the fragment-scan budget (live at mod.rs:718, 1856, 2210), never `bad_box` (live at mod.rs:359, 369, 375, 411 — the 64-bit-size-below-floor / non-advancing-extent / zero-timescale refusals). The fuzz harness swallows errors by contract (mp4/fuzz.rs:6-10), so it witnesses nothing about codes either.
Why it matters: The resource-limit guarantees (MP §12) are exactly the protections this project's docs lean on against the "allocate from file header" failure class; deleting the `moov_len > MAX_MOOV_BYTES` check or the scan-budget counter would pass every test in the tree. Untested ≠ unimplemented — the checks exist and are reachable — but the suite provides zero regression protection for them.
What correct looks like: One test per refusal: an 17 MiB declared moov → `demux.mp4.oversized-box`; a moof ladder past 65,536 boxes → `demux.mp4.scan-budget-exhausted`; a size=1/largesize<16 header → `demux.mp4.bad-box`.
Confidence: HIGH — Verification: read + rg code-string census (counts above).
(Note: `depth-exceeded`'s 0 count is F-001, not a coverage gap.)

### F-003 [S4] `oversized_box` accepts a box kind it throws away
Where: crates/demux/src/mp4/error.rs:95
Code:
```rust
pub fn oversized_box(_kind: &[u8; 4], budget: u64, attempted: u64) -> EngineError {
```
What is wrong: `_kind` is an ignored parameter. The single live caller passes `b"moov"` (mod.rs:768) and the error payload carries no kind and no offset — a consumer of `ResourceLimit` cannot tell which box class was refused without the code string alone. The module doc advertises "Constructors take byte counts, offsets and box kinds only" (error.rs:11-12) as the payload discipline; the one constructor that takes a kind discards it. The parameter also anticipates a `moof` use that does not exist: the moof-over-ceiling paths damage-note and skip (`self.note(moof_len, "moof-over-ceiling")`, mod.rs:1790-1792; silent skip at mod.rs:2177-2179) rather than refuse typed — an inconsistency owned by mod.rs (see open questions).
Why it matters: Diagnostic quality; signature lies about payload content.
What correct looks like: Either fold the kind into the code/context or drop the parameter.
Confidence: HIGH — Verification: read + rg call sites.

### F-004 [S4] `not_mp4` doc understates the probe's accept set (omits `skip`)
Where: crates/demux/src/mp4/error.rs:25-27
Code:
```rust
/// The bytes are not an ISO-BMFF/QuickTime container (no `ftyp` with
/// a known brand, and no leading `moov`/`wide`/`free` either).
pub fn not_mp4(sniffed: &[u8]) -> EngineError {
```
What is wrong: The probe also accepts a leading `skip` box — `matches!(&kind, b"moov" | b"wide" | b"free" | b"skip")` (mod.rs:173). The constructor doc (and probe's own doc at mod.rs:150-152, same omission) lists three of the four. A reader auditing "when is not_mp4 raised" gets an incomplete predicate.
Why it matters: Doc-vs-code drift in the error contract; the same omission replicated in two files.
What correct looks like: Add `skip` to both doc lists.
Confidence: HIGH — Verification: read of both files.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `mint` (private) | 21-23 | REAL | — | Uniform ctor; no panics. |
| `not_mp4` | 27-37 | REAL | — | `Unsupported{FormatInfo{name,sniffed}}`; sniffed bounded ≤32B at caller (mod.rs:699-701). Pinned by 1 test. Doc drift F-004. |
| `truncated` | 41-48 | REAL | — | `Corrupt::Truncated{expected,found}` — real byte counts from callers. 4 test pins. Live at 6+ call sites. |
| `bad_box` | 53-62 | REAL | — | `Corrupt::InvalidStructure{offset}`. Live (mod.rs:359/369/375/411) but ZERO test pins (F-002). |
| `box_overruns_parent` | 67-76 | REAL | — | `InvalidStructure{offset: parent_end}`. 2 test pins (incl. exact pin in tests.rs child_beyond_parent second half). |
| `depth_exceeded` | 81-90 | STUB-adjacent dead code | S3 | F-001: zero callers; documents a nonexistent guard. UNREACHABLE. |
| `oversized_box` | 95-104 | REAL | S4 | F-003 `_kind` ignored; `attempted` real at caller. ZERO test pins (F-002). |
| `no_moov` | 107-114 | REAL | — | 1 test pin. Live mod.rs:761. |
| `no_tracks` | 119-126 | REAL | — | 2 test pins. Live mod.rs:777. |
| `no_sample_table` | 130-139 | REAL | — | 1 test pin. Live mod.rs:795/1146/1152. |
| `not_started` | 142-147 | REAL | — | `Unsupported{format:None}` — best fit in the 7-kind taxonomy (no state kind exists; verified media-types/src/error.rs:190-214). 2 pins. |
| `seek_out_of_range` | 151-156 | REAL | — | Same taxonomy note. 2 pins. 3-segment code `demux.mp4.seek.out-of-range` — consistent with editions' 3-segment scheme, no defect. |
| `scan_budget_exhausted` | 161-170 | REAL | — | `ResourceLimit{RecoveryAttempts, budget, attempted}`; attempted==budget is exact for exhaustion. Live mod.rs:718/1856/2210. ZERO test pins (F-002). |

## 5. Test honesty

This file contains no tests. Its honesty is measured by how its codes are pinned elsewhere (census in F-002 / §2): 8 of 12 codes pinned by `src/mp4/tests.rs` string-equality assertions (`err.context.code.as_str()`); 4 codes pinned nowhere in the repo. Code strings are declared part of the compatibility surface (media-types/src/error.rs:216-220), so string-equality pinning is the intended contract, not a smell.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The error *taxonomy* is media-types' seven kinds — never redefined here" | error.rs:3-4 | REAL | All 12 ctors use `ErrorKind`/`CorruptDetail`/`LimitKind` from media-types (read); no local error enum. |
| "Constructors take byte counts, offsets and box kinds only — never paths, URIs or payload content" | error.rs:10-12 | REAL (with one discard) | Signatures verified; F-003: the only kind parameter is ignored. |
| "Sniffed magic bytes are capped by the taxonomy's own `Display`" | error.rs:12-13 | REAL-UNPROVEN | Bounded at call site (32B, mod.rs:699-701) — verified; the taxonomy Display cap not audited by this agent (media-types is another wave's file). |
| "walked in memory with a depth guard ([`MAX_BOX_DEPTH`])" (touches `depth_exceeded`) | mod.rs:31 | FALSE | F-001: no code reads MAX_BOX_DEPTH; no caller of depth_exceeded. |
| "moof ≤ [`MAX_MOOF_BYTES`]" ceiling as typed refusal | mod.rs doc :30 + error.rs:92-94 implication | PARTIAL (mod.rs's finding) | moof-over-ceiling is damage-note/skip (mod.rs:1790-1792, 2177-2179), not `oversized_box`; only moov refuses typed (mod.rs:768). |

## 7. Dead / duplicated / unreachable code

- `depth_exceeded` (error.rs:81-90): dead, repo-wide.
- `MAX_BOX_DEPTH` (mod.rs:126): dead constant, doc-only reference — recorded here because its only consumer would have been this file's dead constructor.
- No duplication: the wav/ogg constructor-mirroring claimed at error.rs:6-7 is pattern reuse, not copy-paste of logic.

## 8. Dependencies & flags

- No feature flags; `pub mod error;` unconditional (mp4/mod.rs:99) — compiled into every demux build; the 11 live constructors are reachable from shipped binaries via the MP4 demuxer (W00 baseline: MP4 open path executes in CLI build).
- Depends only on `media_types` error vocabulary. No new external deps.

## 9. Open questions for the next agent

- mp4/mod.rs owner: why is an oversized `moov` a typed refusal (mod.rs:768) but an oversized `moof` a damage-note skip (mod.rs:1790-1792) — silent skip in the seek-scan path (mod.rs:2177-2179) means a >1 MiB moof is skipped without any note there; is that intended?
- Whoever owns media-types `ErrorContext`/`Display`: confirm the sniffed-bytes Display cap claimed at error.rs:12-13 (REAL-UNPROVEN here).
- Test-suite owners (W16): F-002's four unpinned codes are the cheapest high-value additions to src/mp4/tests.rs.
