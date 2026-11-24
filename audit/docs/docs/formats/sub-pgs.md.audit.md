# CLAIM AUDIT: docs/formats/sub-pgs.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A06) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 0 / 0 / 0 / 0 / 0 / 1 |

## 1. Blunt verdict
Clean consolidation record: matrix row, module files, test counts (32 lib /
2 survival) and the open-gap disclosure (R-27 overlay contract missing) all
reconcile against the tree and the orchestrator baseline exactly. Unlike
audio-pcm.md, its matrix status quote is still current.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "row status `implemented` on win/linux/macos, `not_started` on web, with evidence `crates/subtitles/src/pgs/**`" | :8-11 | REAL | feature-matrix row: `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }`; evidence string starts `crates/subtitles/src/pgs/**` |
| 2 | self_built = segment/palette/rle + epoch machine in mod.rs, zero deps | :20-21 | REAL | `ls crates/subtitles/src/pgs/` -> fuzz.rs mod.rs palette.rs rle.rs segment.rs |
| 3 | fuzz_target `fuzz_sub_pgs` + integration survival suite | :16-18 | REAL | pgs/fuzz.rs + crates/subtitles/tests/pgs_fuzz_survival.rs on disk |
| 4 | "`cargo test ... --lib pgs` — 32 passed / 0 failed" | :52-53 | REAL | baseline cargo-test-workspace.txt: 32 `test pgs::` lines, all ok, 0 FAILED |
| 5 | "`--test pgs_fuzz_survival` — 2/0" | :54-55 | REAL | baseline:6078-6084 "running 2 tests / test result: ok. 2 passed; 0 failed" |
| 6 | "The renderer overlay contract is still missing (R-27)" | :36-38 | REAL | docs/KNOWN-LIMITS.md:44 "Bitmap subtitles: renderer overlay contract missing ... R-27; ADR-012" |
| 7 | "Linux-only evidence ... web column `not_started`" | :61-62 | REAL | KNOWN-LIMITS §1 program-wide limit row corroborates; web status not_started in matrix |
| 8 | "worktree `wt/w24e` @ `c339ee8`" gates context | :50 | UNVERIFIED | no git in sandbox (rulebook §2); worktree path absent |

## 3. Contradictions
None — the doc's status quote matches the live matrix row byte-for-byte
(the audio-pcm.md linux drift does not recur here).

## 4. Fabrication / inflation indicators
None. The gap section leads with what is NOT done (overlay proof), the
honest direction. Findings: none.
