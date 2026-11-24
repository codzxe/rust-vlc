# CLAIM AUDIT: docs/formats/streaming-dash.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R5) |
| Claims extracted | 7 |
| REAL 7 (2 re-counted EXECUTED-equivalent) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Fully verified record. The matrix row (partial ×3 + web not_started) matches; the doc's
"quoted from the matrix notes" gaps section is byte-faithful to the row's actual notes; all
four self-built files exist; the fuzz target exists; dash-mpd-core is absent from every
Cargo.toml (never linked, as claimed); and the doc's re-countable test arithmetic
reproduces EXACTLY today: 12+4+4+7 = 27 dash `#[test]` fns. The record is also
self-correcting (it documents its own W24-R2 P3-6 correction of the stale 191 figure).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row network-dash stays partial on every platform per the 6.C2 flip | :8-12 | REAL | feature-matrix.toml:1444-1452 `status = { windows="partial", linux="partial", macos="partial", web="not_started" }` (rg). |
| 2 | Honest gaps quote = matrix notes verbatim (reload loop, timeShiftBufferDepth, multi-Period, CENC, mockhttp e2e, DASH-IF vectors pending) | :61-67 | REAL | matrix :1454 notes contain the exact quoted sentence "Partial per 6.C2 flip: ... MPD reload loop (minimumUpdatePeriod), timeShiftBufferDepth trimming, multi-Period stitching, CENC decrypt, mockhttp socket e2e + DASH-IF vectors pending" + the [PROPOSAL] ABR-class sentence — byte-compared, identical. |
| 3 | self_built = playlists/src/mpd.rs + streaming/src/dash/{resolve,sched,abr,fuzz}.rs | :27-28 | REAL | ls: crates/playlists/src/mpd.rs present; crates/streaming/src/dash/ -> abr.rs fuzz.rs mod.rs resolve.rs sched.rs (all five). |
| 4 | 27 dash #[test] fns (12 resolve + 4 fuzz + 4 abr + 7 sched), re-countable | :41-43 | REAL (re-counted) | rg -c "#\[test\]" per file: resolve.rs 12, fuzz.rs 4, abr.rs 4, sched.rs 7 → 27 exactly, unchanged since authoring. |
| 5 | Crate-wide 191 (2 ignored) at commit 93b7614; 206 as of 2026-11-10 (post W24-R2 P3-6 correction) | :44-50 | REAL | current census crates/streaming/src = 214 `#[test]` fns (crate kept growing post-2026-11-10, direction consistent; commit-id unverifiable under no-git). |
| 6 | fuzz_target = fuzz/fuzz_targets/fuzz_demux_mpd.rs + in-crate deterministic sweep | :22-23 | REAL | `ls fuzz/fuzz_targets | grep mpd` -> fuzz_demux_mpd.rs; crates/streaming/src/dash/fuzz.rs present (4 tests). |
| 7 | crates = [] — zero external deps; dash-mpd-core used as parse candidate-oracle only, never linked | :24-26 | REAL | rg -c "dash-mpd" across crates/streaming/Cargo.toml, crates/playlists/Cargo.toml, root Cargo.toml -> 0 matches; matrix notes :1454 "dash-mpd-core 0.1.0 as parse candidate-oracle" — consistent. |

## 3. Contradictions
None — record and matrix agree by construction (the record was authored FROM the matrix
row and upgrades nothing, as stated).

## 4. Fabrication / inflation indicators
None — the record explicitly says "no code was re-run or re-claimed" and its one numeric
correction (191→206) is disclosed with the review id that forced it.

Commands run: rg network-dash + notes in matrix (byte-compare with doc quote); ls playlists/src/mpd.rs + streaming/src/dash + fuzz/fuzz_targets (fuzz_demux_mpd.rs); rg -c "#\[test\]" per dash file (12/4/4/7=27) and crate-wide (214); rg dash-mpd in 3 Cargo.tomls (0).
