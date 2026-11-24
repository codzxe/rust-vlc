# AUDIT: tools/vta/dnx/evidence/hostile-results.txt
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 27 / n.a. (evidence data: 3 hostile batteries + sweeps: L1, L4, L5) |
| File verdict | REAL (data audit: internally consistent; numbers match the admission record's H section exactly) |
| Reachable from shipped entry point? | n.a. (evidence artifact of the VTA spike tool; the audited crate filmcraft-dnx 0.9.0 was later admitted — root lock + codec-dnx feature) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | arithmetic check (Ok+Err = 256 per sweep, 3/3); cross-grep into records/filmcraft-dnx-0.9.0.md (CIDs, per-lane Ok/Err, panics); citation grep (README:53, worklog:10281) |

## 1. One-paragraph verdict
A small, verifiable evidence file. Every section's sweep arithmetic closes (208+48=256, 201+55=256, 206+50=256 — sampled at step 5376/2688/5376 over ~0.9 MB frames), the battery answers are typed (Truncated / Invalid("unknown header version") / Row(0)), and the three CIDs (1238, 1251, 1235) plus depths (8/8/10) match the admission record's lane table exactly — as do the per-lane Ok/Err splits (208/48, 201/55, 206/50) and "0 panics". The one defect is cosmetic-but-real: the first section (L1) lost its `=== L1 ===` header, so the file only identifies two of its three sections internally.

## 2. What actually works (verified)
— verified by arithmetic: L1 sweep "208 Ok-frames, 48 typed Err" = 256 sampled points; L4 201+55=256; L5 206+50=256 — all three sections self-consistent with "256 sampled truncation points".
— verified by cross-check vs docs/dependencies/records/filmcraft-dnx-0.9.0.md:236 "0 panics (208 Ok / 48 typed Err on L1; 201/55 L4; 206/50 L5)" — every count matches the record; :235 "256 cut points"; :237 "Zero panics anywhere — 3 lanes × (6 cases + 256 sweep points)".
— verified by CID cross-check: record :192/:195/:196 lanes L1 CID 1238, L4 CID 1251, L5 CID 1235 (yuv422p10le 10-bit) — evidence shows `cid 1238 depth 8`, `cid 1251 depth 8`, `cid 1235 depth 10` in the same lane order.
— citation: tools/vta/dnx/README.md:53 "hostile-results.txt — battery + sweep output for L1/L4/L5"; worklog archive-2026-10.md:10281 (spike commit listing).

## 3. Findings
F-001 [S4]
Where: tools/vta/dnx/evidence/hostile-results.txt:1-8
Code:
```
hostile zero-length              -> Err(Truncated)
hostile truncated-frame(1/2)     -> Ok(1920x1080 cid 1238 depth 8)
...
=== L4 ===
```
What is wrong: the L1 block has no `=== L1 ===` header (only L4 and L5 carry markers) — the section is identifiable only via the README and the record's CID table. A data file whose first section is anonymous invites misreading (e.g. as a duplicate of L5, since both are 1920x1080 step 5376). Confidence: HIGH — Verification: read + grep (2 `=== L` markers for 3 sections).
Notable non-defect: `truncated-frame(1/2) -> Ok(...)` (a half-truncated frame still yields a frame) is faithfully recorded rather than hidden, and the record's §H lists the sampled-sweep limitation (:252-253 "Full per-byte truncation sweep not run") — the honesty chain is intact.

## 4. Per-function table
Not applicable (data file). Sections: L1 (no header, CID 1238 8-bit), L4 (CID 1251 8-bit), L5 (CID 1235 10-bit); each: 6 battery cases + 256-point sweep + "TOTAL PANICS: 0".

## 5. Test honesty
This file IS the no-panic-contract evidence the admission record cites. Mutation reasoning: if the crate panicked on hostile input, the sweep line would read differently (panics counted); the record pins the same numbers, so the two artifacts corroborate each other. Re-run on the current tree: not possible without building the spike tool (cargo-build-budget) — UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hostile battery panic-free" (spike outcome) | records/filmcraft-dnx-0.9.0.md:152, 224-237 | REAL (as data) | evidence file numbers match record exactly (see §2) |
| "hostile-results.txt — battery + sweep output for L1/L4/L5" | tools/vta/dnx/README.md:53 | REAL | 3 sections present; L1 headerless (F-001) |

## 7. Dead / duplicated / unreachable code
n.a. The spike tool is quarantined under tools/vta/dnx (same non-workspace pattern as vta/mpeg4); the admitted crate came through the record chain.

## 8. Dependencies & flags
n.a. (plain text evidence).

## 9. Open questions for the next agent
- Re-run the battery against the admitted in-tree path (crates/codecs dnx integration) — the evidence here tested the SPIKE harness, not the integrated decoder; whether the integrated lane re-ran the hostile battery: check the codecs dnx test suite (out of this assignment).
