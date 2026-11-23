# AUDIT: crates/demux/src/mp4/error.rs
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2025-06-27 |
| Lines / functions | 170 / 13 (constructors only; tests live in mp4/tests.rs) |
| File verdict | REAL (12/13 variants wired; 1 dead variant) |
| Reachable from shipped entry point? | YES — apps/cli containers/mod.rs:67 `"mp4" => mp4::open` -> containers/mp4.rs:100 `Mp4Demuxer::new` -> every wired constructor is minted inside the open/next_event/seek paths (call sites below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "error::<name>"` per constructor (crates+apps); `cargo test -p demux --lib mp4::` (exit 0, 38 passed); `sed -n` quote verification |

## 1. One-paragraph verdict
Twelve of thirteen error constructors are genuinely wired into the MP4
demuxer's open/parse/seek paths (verified call-site by call-site). The
thirteenth — `depth_exceeded` — is dead: zero callers in the entire repo, and
its companion guard `MAX_BOX_DEPTH` (mp4/mod.rs:126) is a public constant
with zero uses, while mp4/mod.rs:31 *documents* a "depth guard
([`MAX_BOX_DEPTH`])" that does not exist in the walk. The walk is safe only
by accident of structure (fixed-depth call chains, no data-driven
recursion), not by the documented guard. This is the hostile-focus "dead/
unwired MP4 error variant" — found, verified, and it drags a false doc
claim with it.

## 2. What actually works (verified)
— verified by `cargo test -p demux --lib mp4::` -> 38 passed, 0 failed
(error-code pins for not-mp4/no-moov/truncated/not-started/seek.out-of-range/
box-overruns-parent/no-sample-table/no-tracks at tests.rs:444,478,490,512,
618,696,698,759). — Wiring census (rg over crates+apps, non-test callers):
not_mp4 mod.rs:701; truncated mod.rs:351,357,398,405,743,773,1593,1605,1787,
1800,1828; bad_box mod.rs:359,369,375,411,414,421,426,879; box_overruns_parent
mod.rs:372; oversized_box mod.rs:768; no_moov mod.rs:761; no_tracks mod.rs:777;
no_sample_table mod.rs:795,1146,1152; not_started mod.rs:2011,2021,2052;
seek_out_of_range mod.rs:2060,2062,2066,2069,2075,2116,2213;
scan_budget_exhausted mod.rs:718,1856,2210. Only `depth_exceeded`: zero.

## 3. Findings
F-001 [S3] — the hostile-focus hit: dead error variant + false guard claim
Where: crates/demux/src/mp4/error.rs:78-90; crates/demux/src/mp4/mod.rs:31-32, 124-126
Code (error.rs):
```
/// The box-descent depth exceeded the walk's guard (`moov → trak →
/// mdia → minf → stbl → …` is six; anything deeper than the guard is
/// a cycle-shaped lie, not a file).
pub fn depth_exceeded(offset: u64) -> EngineError {
```
and (mod.rs doc):
```
//!   [`MAX_MOOV_BYTES`], each `moof` ≤ [`MAX_MOOF_BYTES`] — then
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
//!   child>parent check on every level. Sample payloads are *never*
```
and (mod.rs constant):
```
/// Box-descent depth guard (`moov→trak→mdia→minf→stbl` is five;
/// `moov→udta→meta→ilst→©nam→data` is six).
pub const MAX_BOX_DEPTH: u32 = 8;
```
What is wrong: `error::depth_exceeded` has zero callers repo-wide (rg over
crates+apps: the only hit is its own definition); `MAX_BOX_DEPTH` has zero
uses (rg: only the doc link and the definition); yet mod.rs:31 documents a
"depth guard" on the structural walk. No such guard exists — the walk's
safety comes from the fixed call-chain shape (parse_moov -> trak -> mdia ->
minf -> stbl, udta -> meta -> ilst, moof -> traf, all via flat `iter_boxes`
calls at mod.rs:345-384 with strictly-advancing `pos`), not from any depth
counter. Why it matters: the doc claims a protection mechanism the code does
not have; if anyone ever adds a data-driven descent (e.g. a generic
container recursion), the documented guard will not stop it — and the dead
constructor advertises a typed refusal (`demux.mp4.depth-exceeded`) that no
input can ever produce. Dead public API + FALSE doc clause. What correct
looks like: either implement the depth counter and refuse past 8 with this
constructor, or delete the constructor + constant + doc clause.
Confidence: HIGH — Verification: read + rg (no build needed).

F-002 [S4]
Where: crates/demux/src/mp4/error.rs:92-95
Code:
```
/// A structural payload (`moov`, `moof`) exceeds the demuxer's
/// in-memory ceiling — refused as a typed resource limit rather than
/// trusted into an allocation.
pub fn oversized_box(_kind: &[u8; 4], budget: u64, attempted: u64) -> EngineError {
```
What is wrong: `_kind` is accepted and discarded — the error can never say
which box kind (`moov` vs `moof`) tripped the ceiling; the sole caller
(mod.rs:768) dutifully passes `b"moov"` into a parameter that goes nowhere.
Checklist 6.1 (ignored parameters). What correct looks like: fold the kind
into the context (e.g. `demux.mp4.oversized-box.moov` or a detail field).
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/src/mp4/error.rs:27-33 vs crates/demux/src/ts/psi.rs:113-119
Code:
```
pub fn not_mp4(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("mp4".to_string()),
                sniffed: sniffed.to_vec(),
```
What is wrong: `sniffed.to_vec()` is uncapped — the module doc's "Sniffed
magic bytes are capped by the taxonomy's own Display" (error.rs:12-13)
caps *rendering*, not the copy: any future caller passing a large slice
copies it in full into the error. The PSI twin (`not_psi`) caps inside the
constructor (`sniffed[..sniffed.len().min(16)]`); the mkv tag zone's
budget errors take numbers only. Currently harmless — the sole caller
(mod.rs:699-701) passes a `read_prefix_at(32, 0)` slice — but the guard is
caller discipline, not the constructor. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/mp4/error.rs:53-62 vs mod.rs:359 etc.
Code:
```
pub fn bad_box(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
```
What is wrong: `demux.mp4.bad-box` is minted at 8 call sites but pinned by
zero tests (rg over mp4/tests.rs + tests/*.rs: no `bad-box` hit; same for
`oversized-box` and `scan-budget-exhausted`). Three live error variants
have no test proving they fire. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 21-23 | REAL | - | shared ctor helper |
| not_mp4 | 27-37 | REAL | S4 | wired (mod.rs:701); uncapped sniff copy (F-003) |
| truncated | 41-48 | REAL | - | 11 call sites; test-pinned |
| bad_box | 53-62 | REAL | S4 | 8 call sites; unpinned by tests (F-004) |
| box_overruns_parent | 67-76 | REAL | - | 1 call site (mod.rs:372); test-pinned |
| depth_exceeded | 81-90 | UNREACHABLE (dead) | S3 | zero callers (F-001) |
| oversized_box | 95-104 | REAL | S4 | wired (mod.rs:768); `_kind` ignored (F-002); unpinned |
| no_moov | 107-114 | REAL | - | wired; test-pinned |
| no_tracks | 119-126 | REAL | - | wired; test-pinned |
| no_sample_table | 130-139 | REAL | - | 3 call sites; test-pinned |
| not_started | 142-147 | REAL | - | 3 call sites; test-pinned |
| seek_out_of_range | 151-156 | REAL | - | 7 call sites; test-pinned |
| scan_budget_exhausted | 161-170 | REAL | S4 | 3 call sites; unpinned by tests |

## 5. Test honesty (tests in mp4/tests.rs)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| not-mp4/no-moov/truncated/not-started/seek pins (444,478,490,512,696,698,759) | YES | code strings asserted on real fixtures |
| box-overruns-parent pin (618) | YES | child>parent fixture |
| no-sample-table/no-tracks OR-pins (647,592) | PARTIAL | OR-arms weaken the pin (either code passes) |
| (none for bad-box/oversized-box/scan-budget/depth) | NO | no test fires these codes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "walked in memory with a depth guard ([MAX_BOX_DEPTH])" | mp4/mod.rs:29-32 | FALSE | MAX_BOX_DEPTH zero uses; depth_exceeded zero callers (F-001) |
| "moov ≤ MAX_MOOV_BYTES, each moof ≤ MAX_MOOF_BYTES" | mp4/mod.rs:29-30 | REAL | mod.rs:767, 1790, 2177 |
| "Constructors take byte counts, offsets and box kinds only" | error.rs:10-12 | FALSE (letter) | oversized_box takes a box kind and drops it (F-002) |
| "Sniffed magic bytes are capped" | error.rs:12-13 | REAL-UNPROVEN | capped at Display (media-types), not at copy; sole caller passes ≤32 bytes |

## 7. Dead / duplicated / unreachable code
- `depth_exceeded` (error.rs:81-90): dead — the file's only unwired variant.
- `MAX_BOX_DEPTH` (mp4/mod.rs:126): dead public constant documenting a
  nonexistent guard.
- No todo!/unimplemented!/panic! in this file (rg-verified).

## 8. Dependencies & flags
`media_types` only; no feature gates; part of the always-compiled `demux`
crate reachable from apps/cli. No #[allow].

## 9. Open questions for the next agent
- mp4/fuzz.rs's module doc lists moov/moof ceilings but is silent on depth —
  consistent with the guard never existing; if a future card adds generic
  recursion, F-001 becomes S1 (stack overflow). Worth a guard-note in the
  fuzz ledger.
- Why do tests 592/647 use OR-arms? If the fixture's outcome is genuinely
  ambiguous, the pin proves less than it claims.
