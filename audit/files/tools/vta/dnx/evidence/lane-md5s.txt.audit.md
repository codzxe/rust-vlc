# AUDIT: tools/vta/dnx/evidence/lane-md5s.txt (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines | 11 (header + 10 hash rows: 5 lanes x ref/mine) |
| File verdict | REAL — the .ref column is LIVE-VERIFIED by the workspace test suite (baseline) |
| Reachable from shipped entry point? | N/A — VTA evidence artifact; its REF hashes are pinned by crates/codecs/tests/dnx_differential.rs which runs in the workspace suite (baseline: 6/6 pass) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | format/uniqueness validation -> 10/10 rows OK; byte-compare of the 5 REF hashes vs test constants :174/:190/:206/:221/:246 -> identical; baseline cargo-test-workspace.txt:1032-1041 cross-check; README evidence-inventory read; record date/tooling read |

## 1. One-paragraph verdict
The strongest kind of evidence artifact in this family: five of its ten hashes are not just committed text — they are pinned assertions in the live workspace test suite, which regenerates the lanes with ffmpeg and asserts the exact md5s (baseline: dnx_differential 6/6 passed), and the five constants in the test are byte-identical to this file's `.ref.raw` column. The `.mine.raw` column hashes the (now-vanished) harness output and is referenced by nothing. Two hygiene gaps: the file's contents are duplicated as hardcoded test constants with no reconciliation mechanism, and the README's committed-evidence inventory omits this file.

## 2. What actually works (verified)
— verified by python validation: 10 rows, every hash is 32 lowercase hex chars, all 10 hashes unique, 5 lane names (`L1_1080p_175M` ... `L5_1080p10_185M`) matching compare.py's hardcoded lanes and the README's five ffmpeg generation commands exactly.
— verified header consistency: `2026-10-26, ffmpeg 7.1.5 vs filmcraft-dnx 0.9.0` == record filmcraft-dnx-0.9.0.md:7 "Evidence gathered 2026-10-26 ... five-lane differential against ffmpeg 7.1.5".
— verified LIVE pinning: crates/codecs/tests/dnx/mod.rs:124-125 documents the file as the `.ref.raw` MD5 source; dnx_differential.rs:174/190/206/221/246 hardcode the five REF hashes — byte-identical to this file's column (f555a591…, ee452375…, 8e924a4f…, f6d1e56c…, 76cb5352…). Baseline run (audit/runs/baseline/cargo-test-workspace.txt:1032-1041): `Running tests/dnx_differential.rs ... 6 passed; 0 failed ... 6.08s`, including lane_l1..l5 — i.e. the orchestrator's suite regenerated the lanes and reproduced these exact reference hashes.
— record §D consistency: the record's lane table (:190-196) confirms "3 = 3" frames and L2's static bars — consistent with this file's per-lane granularity.

## 3. Findings
F-001 [S4]
Where: tools/vta/dnx/evidence/lane-md5s.txt:3,5,7,9,11 (the .mine.raw column)
Code: `f4e1f9b9a00817e5c20a854b9a97892f L1_1080p_175M.mine.raw` (and the four sibling mine rows)
What is wrong: the five MINE hashes are referenced nowhere — the test pins only the REF column, compare.py recomputes its own per-frame md5s, the record's lane table uses per-frame MD5s, and the README's committed-evidence inventory (:49-53) lists only ffprobe-facts/compare-results/hostile-results. The .dnxhd lanes are regenerable (the test does it) but the harness's raw outputs are gone, so the mine column is assertion-only.
Confidence: HIGH — Verification: rg (no consumers) + README inventory quoted.

F-002 [S4]
Where: tools/vta/dnx/evidence/lane-md5s.txt:2,4,6,8,10 vs crates/codecs/tests/dnx_differential.rs:174
Code: `f555a591a0bc733e18ba7937c212e543 L1_1080p_175M.ref.raw` / test: `        "f555a591a0bc733e18ba7937c212e543",`
What is wrong: the REF hashes exist in two places — this file and hardcoded string constants in the test — with no reconciliation mechanism (the test does not read the file). They agree today (verified byte-exact, 5/5) but can drift silently; the file is the documented source of truth ("the record committed these per lane in ... lane-md5s.txt", mod.rs:124) while the test is the enforced one.
Confidence: HIGH — Verification: byte-compare ran.

## 4. Test honesty
The consumer test is honest and strong: it REGENERATES the lanes (deterministic lavfi sources + ffmpeg dnxhd encoder), decodes with ffmpeg, hashes, and asserts equality with the committed pin — a live oracle, not a mock; baseline proves it executed and passed (6/6, 6.08s).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "per-lane whole-stream MD5s (ref vs mine) — 2026-10-26, ffmpeg 7.1.5 vs filmcraft-dnx 0.9.0" | :1 | REAL (ref half live-verified; mine half unverifiable — raws gone) | baseline run + test constants |
| "the record committed these per lane in ... lane-md5s.txt as the .ref.raw MD5s" | dnx/mod.rs:124 | REAL | 5/5 byte-identical constants in dnx_differential.rs |

## 6. Dead / duplicated / unreachable code
The mine column (F-001); the duplicated ref constants (F-002).

## 7. Dependencies & flags
N/A — flat data; consumers: dnx_differential.rs (via duplicated constants), documentation cross-refs.

## 8. Open questions for the next agent
- None material. The five mine hashes could be re-derived by rebuilding the vta-dnx harness and regenerating the lanes (ffmpeg present) — blocked by cargo-build-budget; UNVERIFIED.
