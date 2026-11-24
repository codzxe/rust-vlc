# CLAIM AUDIT: docs/soak/raw/scenarios-sandbox__playlist-cycle.md (credit: W19-A06, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 4 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNREACHABLE 0 / UNVERIFIED 3 |
| Internal contradictions found | 0 (arithmetic internally consistent) |

## 1. Blunt verdict
A single-scenario soak report whose machinery, manifest values and internal arithmetic all
check out against the repo; the run numbers themselves are unexecuted-by-me and the cited
manifest path is a dead worktree path. This is the most internally consistent raw doc audited
in this batch: 482 samples − 73 warmup = 409 ≈ 407.0 s window; 22295 passes × 3 fixtures ≈
66886 cycles; 10.5 B/s × 3600 = 0.0378 ≈ 0.038 MB/h; 0.1 kB/s → 0.403 MB/h all reconcile.

## 2. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Manifest `/home/z/my-project/wt/w24b/tests/soak/scenarios-sandbox.toml` (5 scenarios) | :3 | REAL-UNPROVEN path / content REAL | `ls /home/z/my-project/wt/w24b/...` -> No such file; repo copy tests/soak/scenarios-sandbox.toml has 5 scenario entries (rg duration_min -> 5) |
| 2 | duration 8.00 min configured; wall 480.0 s | :20 | REAL | manifest `duration_min = 8.0` (tests/soak/scenarios-sandbox.toml:52); 3 fixtures line 53 match report :21 exactly |
| 3 | ops are catalog WireRequest through soak dispatch seam onto real engine stack | :7 | REAL | crates/playback-engine/tests/soak/dispatch.rs:702 `pub fn lumen_soak_dispatch(host: &mut SoakHost, request: &WireRequest)` |
| 4 | instruments: live_bytes counting allocator, VmRSS, fd count | :8 | REAL | tests/soak/memory.rs:13,18 — `VmRSS from /proc/self/status`, `live_bytes from the process-global counting allocator` |
| 5 | ceiling checked 50 MB/h; target 5 MB/h reported, not asserted | :23 | REAL | manifest:29-31 `leak_ceiling_mb_per_h = 50.0`, `target_mb_per_h = 5.0`, `assert_target = false` |
| 6 | 66886 cycles / opens / plays / closes, 22295 playlist passes, 0 seeks | :24 | UNVERIFIED (run) — internally consistent | 22295×3=66885≈66886; no seek op defined for playlist-cycle kind; not re-run (8-min budget, cargo-build-budget) |
| 7 | 6242720 frames, 383873656800 payload bytes at sinks | :25 | UNVERIFIED (run) — plausible | 6242720/66886=93.3 frames/cycle vs play-loop sibling 73.7 (one fewer fixture) — delta ≈ one noidx0 pass-through; payload/frame 61.5 kB matches sibling 62.0 kB |
| 8 | live slope 0.038 MB/h, RSS 0.403 MB/h — PASS within budgets | :16,26-27 | UNVERIFIED (run) | unit arithmetic verified (see verdict); slopes below 50 MB/h ceiling by 1300× |
| 9 | fixtures' seek targets and track facts read from recorded expectation TOMLs | :7 | SHALLOW for noidx0 | tests/fixtures/expectations/malformed/avi_mjpeg_pcm__noidx0.avi.toml is class B: no [tracks]/[seek] — only [derivation]+[error]; track facts must come from the parent toml, contradicting "read from their recorded expectation TOMLs" for that fixture |

## 3. Contradictions
None within this file; the sibling open-close-churn report shows "measured window 0.0 s"
(its own defect, outside this ledger).

## 4. Fabrication / inflation indicators
None beyond UNVERIFIED run numbers: every cross-checkable constant (durations, fixtures,
budgets, pass-multiples, slope unit conversions) reconciles. Quotes verified byte-verbatim:
`sed -n '3p;16p;24p;26p'`. Dead absolute path (worktree w24b) is provenance rot, not
fabrication — content matches the in-repo manifest.
