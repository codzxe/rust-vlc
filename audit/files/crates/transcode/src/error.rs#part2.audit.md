# AUDIT: crates/transcode/src/error.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 312 (496–807 of 807) / 3 items in table (tests mod + 2 tests) |
| Chunk boundary | part2 = 496–807 = the entire `#[cfg(test)] mod tests`. Equal split is 404; the constructor tail 404–495 (`mp4_timescale_not_native` … `mp4_sample_entry_over_budget`) moves to A01's part1 so the tests module is whole. Flagged in log; A01's part1 = 1–495. |
| File verdict | REAL (library; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO — same verdict as the crate (established in my runner.rs#part2 audit): no Cargo.toml in the workspace depends on `transcode`; these constructors are called only from the crate's own (unwired) modules and tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -c "^pub fn " crates/transcode/src/error.rs` → 42 constructors; cross-count vs the two tests (36-table + 6 in the second test = 42 — full coverage); `sed -n` verbatim checks at 121-127, 267-273, 309-317; `rg -n "pub enum ErrorKind\|pub enum AccessDenied" -A 20 crates/media-types/src/error.rs` (taxonomy: no Io/StorageFull cause exists); baseline: orchrun-test-workspace-nofailfast.log.txt:5824-5825 (both transcode error tests ok). |

## 1. One-paragraph verdict
The test half of the transcode error vocabulary is a complete, honest pin: all 42 public constructors are exercised across two tests (a 36-entry stage/kind/code table plus a six-constructor source/kind test), both pass in baseline, and any change to a kind or code fails loudly. Two diagnostic-quality defects hide behind the green: `audio_track_ambiguous(found)` and `wav_channels_out_of_range(channels)` accept a parameter and discard it (`let _ = …`) while their sibling limit constructors embed both numbers in the error — and the table test structurally cannot see the difference; and every sink write/seek/spawn I/O failure is minted as `Inaccessible { PermissionDenied }`, so a full disk reads as a permission problem (the test's own fixture is `Error::other("no space")`).

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:5824-5825` → `error::tests::constructors_mint_stage_transcode_kinds_and_codes` and `error::tests::io_and_fault_constructors_carry_sources_and_kinds` both ok.
— verified by count: 42 `pub fn` constructors (rg -c) = 36 table entries + `wav_channels_out_of_range` (793) + `sink_write_io`/`sink_seek_io`/`worker_spawn_io` (761/765/769) + `worker_panic`/`flush_undrained` (773/782) — nothing untested.
— verified by read: the table asserts stage AND kind AND code per entry (750-754); the io test walks real `source()` chains.

## 3. Findings

### F-001 [S4]
Where: crates/transcode/src/error.rs:121-127 and 267-273 (code in A01's part1); test gap in my range (569, 793)
Code:
```
pub fn audio_track_ambiguous(found: usize) -> EngineError {
    let _ = found;
```
(and)
```
pub fn wav_channels_out_of_range(channels: u32) -> EngineError {
    let _ = channels;
```
What is wrong: both constructors accept a diagnostic value and drop it; the minted errors carry no trace of the count. The caller (`runner.rs:1045 audio_track_ambiguous(found)`, `wav.rs:110-111`) computed the number specifically to report it. Sibling constructors (`filter_stages_over_budget`, `tracks_over_budget`, `mp4_samples_over_budget`) embed both `budget` and `attempted` via `ResourceLimit` — the vocabulary is inconsistent about whether numbers ride the error. The values are not secrets (track counts, channel counts), so MP §12 does not force the drop.
Why it matters: a user with three audio tracks gets "audio track ambiguous" with no count; a host with 70,000 channels gets "channels out of range" with no value — and the table test (569: `audio_track_ambiguous(2)`, 793: `wav_channels_out_of_range(0)`) passes the values in but asserts only code/kind, so the discard is invisible to the suite.
What correct looks like: embed the number (a `with_source` leaf like `FactsMismatch`, or a Display-bearing context) or remove the parameter; extend the table test to assert the value lands.
Confidence: HIGH — Verification: read of both constructors and both test sites.
Chunk note: constructor bodies are A01's part1 — cross-ref recorded in §9.

### F-002 [S4]
Where: crates/transcode/src/error.rs:309-317 (code in A01's part1); the untested kind is pinned nowhere in my range's tests
Code:
```
pub fn sink_write_io(err: std::io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
```
What is wrong: every write/seek/spawn I/O failure is minted `Inaccessible { reason: PermissionDenied }`. The taxonomy (media-types error.rs:100-111) offers no storage-full cause, so ENOSPC — the test's own fixture text is `"no space"` (761) — surfaces to any kind-driven UI as a permission problem. The second test asserts the code and the source chain but deliberately not the kind (761-763), so the mapping is unpinned.
Why it matters: wrong-kind errors mislead kind-switching UI copy; a full disk telling the user "permission denied" sends them chasing the wrong fix. Blast radius today: zero (crate unwired), hence S4 not S3.
What correct looks like: media-types gains an `AccessDenied::StorageFull` (or an Io cause); until then map ENOSPC to `ResourceLimit` and keep PermissionDenied for EACCES.
Confidence: HIGH — Verification: read of taxonomy (no fitting variant exists — the crate did the best available, but the test should at least pin the current kind).
Chunk note: code in A01's part1; recorded here because the kind is exactly what my range's test omits.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mod tests` (imports) | 496-498 | REAL | — | `use super::*` — full constructor surface in scope. |
| `constructors_mint_stage_transcode_kinds_and_codes` | 500-755 | REAL | S4 (F-001 context) | 36-entry literal-sized table; asserts stage+kind+code per entry (750-754); any kind/code drift fails. Cannot see dropped parameters (F-001). |
| `io_and_fault_constructors_carry_sources_and_kinds` | 757-805 | REAL | S4 (F-002 context) | Real `source()` walks for the three io constructors; kind asserted only for panic/flush (Decoder/BadState) and channels (Unsupported) — the io kind (PermissionDenied) deliberately unpinned; `FactsMismatch` leaf rendering asserted with both sides (798-804). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| constructors_mint… | YES | Stage, kind, and code each asserted for 36 constructors; mutating any mint's kind or code fails the exact-equality assert. |
| io_and_fault… | YES for codes/sources/kinds-asserted | Code + source-chain + panic/flush kinds asserted. NOT the io kind (F-002) and NOT dropped parameters (F-001) — two blind spots, both recorded as findings. |
| (module claim) "one table, tested in one place" | PARTIAL | 42 constructors split across two tests with an admitted exclusion (comment at 791-792) — honest in the comment, imprecise in the module doc. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the crate's whole mint vocabulary, one table, tested in one place" | error.rs:1-3 | REAL (with a caveat) | All 42 constructors covered (count verified), but across two tests; the caveat is self-documented at 791-792. |
| "Codes are part of the compatibility surface … additions are additive-only" | error.rs:7-8 | REAL (untestable here) | Table pins current codes; nothing asserts additivity — a policy claim, verified only by review discipline. |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable from shipped entry points (see header). No dead constructors: all 42 are called somewhere in-crate (runner/graph/wav/mp4mux/queue/limits/job/record per their imports; spot-verified `mp4_track_duplicate` at graph.rs:359, `flush_undrained` at runner.rs:557).

## 8. Dependencies & flags
media-types error taxonomy only; testkit not needed here (plain asserts). No features, no unsafe, no panics (the `unreachable!` in tests is test-only). 39/39 lint inheritance per baseline hygiene; `let _ = …` in constructors survives `unused_variables = deny` only because it is an explicit discard — the lint policy forces the discard to be *visible*, which is how I found F-001.

## 9. Open questions for the next agent
- A01 (part1, 1–495): owns F-001's constructor bodies — severity-call the discarded `found`/`channels` from the production side; also `sink_bad_state`'s `Corrupt/InvalidStructure` kind for a state-machine bug (not media corruption) is the same kind-misuse family as F-002.
- media-types owner (error taxonomy): the missing storage-full/Io cause is the root of F-002 — worth a wave-level ledger entry.
