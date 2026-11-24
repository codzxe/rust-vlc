# AUDIT: tools/vta/dnx/evidence/compare-results.txt (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines | 80 (5 lanes x 3 frames, read in full) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — VTA evidence artifact (the dnx lane data behind 6.VTA-DNX-RATIFY) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read (full); sibling evidence read (ffprobe-facts.txt, hostile-results.txt, lane-md5s.txt); rg into crates/codecs/tests/dnx/mod.rs and filmcraft-dnx-0.9.0.md; baseline dnx_differential result check |

## 1. One-paragraph verdict
The honest tolerance-class evidence for the DNxHD admission: five lanes,
every frame's MD5 DIFFERS from ffmpeg 7.1.5, per-plane maxΔ 1-2 (LSB),
differing-sample ratios 0.040%-6.644%. This is the data behind the
"ratified tolerance class" (NOT bit-exact) verdict, and the in-tree
dnx_differential test pins these very numbers (mod.rs:284 quotes the L1
line verbatim) and passes today (baseline 6/6). One data observation: the
L2 lane's three frames are byte-identical duplicates.

## 2. What actually works (verified)
— Arithmetic: L1 frame 0 Y differing=26038/2073600 = 1.256% ✓ (matches
in-tree pinned constant at crates/codecs/tests/dnx/mod.rs:284).
— lane-md5s.txt gives per-lane whole-stream ref-vs-mine MD5s, all
different — consistent with every frame differing here.
— Baseline cargo-test-workspace.txt:1032-1040: `dnx_differential` 6/6 pass
today (lane_l1..l5 + truncated_refuses_typed) — the lane assertions are
live-verified against the local ffmpeg oracle.
— ffprobe-facts.txt corroborates lane geometry (1920x1080 yuv422p etc.);
hostile-results.txt: 0 panics, typed errors — honest sibling data.

## 3. Findings
F-001 [S3]
Where: tools/vta/dnx/evidence/compare-results.txt:18-32
Code:
```
 frame 0:
  md5 differs (a0f09fa72731)
  Y: maxΔ=1 differing=825/2073600 (0.040%)
```
(the same `md5 differs (a0f09fa72731)` and identical 825/1513/2126 counts
repeat for frames 1 and 2)
What is wrong: L2's three "frames" are byte-identical duplicates (same
per-frame MD5 and identical per-plane delta counts) — the lane contributes
ONE unique frame of evidence, not three. The evidence table presents 3
frames per lane (15 comparisons) but the effective unique-frame count is
13 (L2 = 1). Nothing marks the duplication.
Why it matters: the lane's measured divergence class rests on a single
unique image; a reader counting "15 frames measured" overstates the
coverage.
What correct looks like: an annotation (duplicate-frame lane) or a fourth
distinct frame.
Confidence: HIGH — Verification: read (identical md5 + identical counts).

## 4. Data summary
Lanes: L1_1080p_175M (1.256-1.426% differing, maxΔ2), L2_1080p_36M
(0.040-0.205%, maxΔ1, 3 identical frames), L3_1080p_145M (4.958-6.610%,
maxΔ2 — worst), L4_720p_90M (1.451-1.806%, maxΔ2), L5_1080p10_185M
(1.156-1.419%, maxΔ2). All frames md5-differ. This is tolerance-class
differential data, honestly negative on bit-exactness.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "0.04–6.6 % of samples differing, content-dependent" | filmcraft-dnx-0.9.0.md:154 | REAL | this file: 0.040% (L2) to 6.610% (L3 Cr) |
| "Not bit-exact vs ffmpeg — tolerance class only; the ±2 LSB" | filmcraft-dnx-0.9.0.md:241 | REAL | every frame "md5 differs" here |
| "L3 ... 4.96–6.61 % differing | all 3 differ" | filmcraft-dnx-0.9.0.md:194 | REAL | L3 block :33-48: 4.958-6.644% |
| dnx_differential 6/6 (in-tree, live oracle) | worklog/matrix | REAL | baseline run today: 6 passed / 0 failed |

## 6. Dead / duplicated code
N/A (data). See F-001 for the duplicated-frame lane.

## 7. Dependencies & flags
N/A. Sibling artifacts: compare.py (the comparison driver), lane-md5s.txt
(whole-stream MD5s), hostile-results.txt (0 panics).

## 8. Open questions for the next agent
The "md5 differs (xxxx)" per-frame value is 12 hex chars — which md5 is it
(of mine? of the diff?) — compare.py should answer; audit compare.py's
formatting if assigned.
