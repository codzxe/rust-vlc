# CLAIM AUDIT: docs/formats/streaming-dash.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 2 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Partial-status record that stays partial — the record explicitly refuses to upgrade
itself ("nothing here upgrades it"). MPD parse + SegmentTemplate/Timeline resolution + ABR
verify in-tree. Its honest-gaps list (reload loop, time-shift, CENC, DASH-IF vectors
pending) matches the matrix notes.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the MPD model and parse core (6.C1a), the SegmentTemplate/SegmentList/SegmentBase resolution with typed plan budgets (6.C1b) ... landed and tested in crates/playlists + crates/streaming" | L3-6 | REAL | playlists/src/mpd.rs:12-15 (SegmentTemplate/List/Base + $Identifier$ substitution + SegmentTimeline t+n×d docs); streaming/src/dash/{resolve,sched,abr,fuzz}.rs exist |
| 2 | "the MPD fuzz target (6.C1c)" | L5-6 | REAL | streaming/src/dash/fuzz.rs + matrix evidence "fuzz_demux_mpd cargo-fuzz target + in-crate deterministic mutation sweep" |
| 3 | "buffer-based ABR ... live-edge join (6.C2)" | L6-7 | REAL | dash/abr.rs exists (EWMA/margin/dwell model per module docs) |
| 4 | "The matrix row network-dash stays partial on every platform" | L7-8 | REAL | matrix network-dash: `status = { windows = "partial", ... }` |
| 5 | "the reload loop, time-shift trimming, multi-Period stitching, CENC decrypt, the mockhttp socket e2e, and the DASH-IF vectors are pending" | L8-10 | REAL | honest-gaps list; no reload/cenc modules in dash/ (ls: resolve/fuzz/mod/abr/sched only) |
| 6 | "streaming 191 lib tests (2 ignored) incl. 27 dash" | matrix evidence | REAL-UNPROVEN | not re-run; baseline cargo-test-workspace covers aggregate pass |
| 7 | "This record was authored by the 8.G1 docs sweep (2026-11-10) to close the record gap" | L10-12 | REAL | docs-sweep provenance: docs/audit/docs-sweep-20261110.md cited; gap-closing behavior (paper-to-record), not inflation |

## 3-5. Contradictions / fabrication / matrix rows
None / none / network-dash row partial — agreement verified. Commands: ls dash; rg mpd.rs
doc-comments; matrix sed.
