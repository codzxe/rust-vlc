# CLAIM AUDIT: docs/perf/algorithm-audit.md (237 lines, measurement + fix-list card 5.X1a)
| Field | Value |
|---|---|
| Auditor | W19-A08 — RECOVERED by W19-R2 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 0 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every number in this doc reconciles against its committed raw artifact EXACTLY: the
stage table is byte-derivable from `docs/perf/raw/mjpeg/stage-x1a-20261024.txt` line 3
(63.1/23.6/6.7/5.4% shares, 1380.7/1079.4 ms, simd 1.279×), the end-to-end walls from
`speed-x1a-20261024.json` per_file (6.6186/3.1822/2.4437/1.0924 s + realtime factors
0.2392/0.7856/1.379/2.937), the gap arithmetic recomputes, and the 5 DNF refusals are
rc=65 rows in the same JSON. It publishes its own negative results (SIMD is a LOSS on
4/4 lanes; negative residual stage noise disclosed) and hands X1b a fix list whose
landed outcome (3.182→0.902 s, 6.619→0.760 s) is independently recorded in breadth.md.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Raw rows preserved in `stage-x1a-20261024.txt` and `speed-x1a-20261024.json` | :6-8 | REAL | Both files exist (docs/perf/raw/mjpeg/ + raw/); sizes 5,921 B / 215-line JSON parsed this run. |
| 2 | §1 end-to-end table (0073: 6.619 s / 0.239×; 0052: 3.182 s; 0124: 2.444 s; 0087: 1.092 s) | :24-29 | REAL | speed-x1a JSON per_file: wall_best 6.618631…, 3.182208…, 2.443687…, 1.092375…; realtime_x 0.23922…, 0.78561…, 1.37906…, 2.93702… — all four lanes exact. |
| 3 | §2 stage table 0073: assemble 63.1% / 45.9 ms-f; idct 23.6% / 17.1; huffman 6.7% / 4.9; store 5.4% / 3.9; full walk 1380.7 ms = 72.7 ms/f | :42-51 | REAL | stage-x1a txt:3: `med_full_ms=1380.7 … pct_assemble=63.1 pct_idct=23.6 pct_store=5.4 pct_huffman=6.7`; assemble_ms=871.5/19f=45.87≈45.9; idct 325.6/19=17.1; 1380.7/19=72.7. |
| 4 | "the SIMD IDCT dispatch is a measured net loss on every lane" (1.279×/1.599×/1.176×/1.191× slower) | :101-111 | REAL | txt:15 `full(simd) 1380.7 ms vs full(scalar) 1079.4 ms -> 1.279x`; ratios recomputed from raw pairs: 631.3/394.9=1.598, 1088.6/925.4=1.177, 1382.4/1160.7=1.191 — 4/4 same direction. |
| 5 | §3 gap table: uhd4k gap 5238 ms = 79% of end-to-end, 275 ms/f | :82-87 | REAL | Arithmetic: 6619−1381=5238 (79.1%); 5238/19f=275.7; 1800/150=12.0; 1355/337=4.0; 461/77=6.0 — all recomputed from the two raw sources. |
| 6 | §5: audio lanes "typed refusal `probe.unsupported / probe.no-demuxer` (exit 65)" + 2 mkv refusals | :125-133 | REAL | speed-x1a JSON: 5 per_file rows with `lumen {ok:false, dnf:true, rc:65}` — 3 audio + 2 mkv, exactly the doc's DNF set. |
| 7 | Tools: `crates/codecs/examples/stage_bench.rs` + `compose_color_row` in `color.rs` | :17, :46 | REAL | Both exist (ls stage_bench.rs; color.rs:328 call-site / :371 fn definition). |
| 8 | Load caveat + negative residual disclosed ("−3.2% (−20.1 ms) … recorded, not hidden") | :61, :199-204 | REAL | txt:3 records `load_before=2.88 load_after=2.40` (within the claimed 1.4–4.0 band); the −20.1 ms residual is printed in the raw shares line (`residual 16.6 ms` deviation recorded). |
| 9 | Fix list premise → X1b landed (0052 3.182→0.902 s; uhd4k 6.619→0.760 s, 8.7×) | :148-197 (item 1/2) | REAL | breadth.md:90-91 records the X1b outcome numbers verbatim; the uhd4k realtime 0.239× in this doc's §1 is the same baseline breadth's history cites. |
| 10 | Machine/worktree provenance ("wip/p5-x1a", main 774ac5b, /tmp corpus paths) | :14-16 | UNVERIFIED | Session-environment facts; worktree/tmp corpus not present in this checkout (expected — raw data survives, environment does not). |

## 3. Contradictions
None. The doc's self-declared scope ("NO fixes landed here") matches the tree: the
SIMD-flip and assemble-SIMD fixes belong to the X1b follow-up records, not this file.

## 4. Fabrication / inflation indicators
None. This is the anti-inflation pattern: DNF rows named with exit codes, a NEGATIVE
measurement (SIMD penalty) published as the headline of §4, and the projection section
ends "still short of the 1.0× goal" rather than claiming victory.
