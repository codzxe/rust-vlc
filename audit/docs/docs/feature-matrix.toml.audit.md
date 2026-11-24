# CLAIM AUDIT: docs/feature-matrix.toml (whole file — W19-R1 recovery, entries 26-34)
| Field | Value |
|---|---|
| Auditor | W19-A02, W19-A03, W19-A04, W19-A05, W19-A06, W19-A07, W19-A08, W19-A09, W19-A10 (9 ledger entries, one file) — recovered by W19-R1 |
| Date | 2026-11-24 |
| Claims extracted | 11 (file-level; 252 row-level verdicts live in the 21 chunk audits audit/docs/feature-matrix.toml.rows-*.audit.md) |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 1 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNREACHABLE 0 | |
| Internal contradictions found | 3 (last_updated vs applied transitions; header "tool OWED" vs landed tool; 2 dangling evidence paths) |

## 1. Blunt verdict
The parity ledger's structure is fully real and machine-verifiable: tomllib parses all
3,421 lines, 252 unique-id rows, and every census figure I recomputed (status cells
129/193/503/147/36, per-phase 10/5/93/49/18/25/52, 21 categories, platform implemented
66/61/66) matches the orchestrator's feature-audit baseline EXACTLY. The compat-matrix
demotions are honestly applied and traceable. The rot is at the edges: two evidence paths
dangle (the flagship H.264 row points at a directory-shaped path that does not exist),
meta.last_updated predates the last edit, and the header still calls the feature-audit
tool "OWED and unassigned debt" that landed long ago.
Display note (byte-hygiene): raw shell reads of this file render "[meta]" as "eta]" —
the known display-pipeline artifact (W19-A09 CORRECTION); tomllib parse proves the file
is structurally clean. No corruption.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "SCHEMA FROZEN 2026-10-19" + schema_version 1 | :19,35-36 | REAL | meta.schema_frozen="2026-10-19", schema_version=1 (tomllib) |
| 2 | baseline "VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | :36 | REAL | identical hash across dumps 04/13/14/16/17 + compat-matrix json |
| 3 | 252 rows, unique ids, 21 categories | structure | REAL | python3 tomllib census: 252 rows, ids unique, 21 categories — equals baseline feature-audit output |
| 4 | status vocabulary {blocked, implemented, not_started, partial, unsupported} | rows | REAL | census re-derivation: 129/193/503/147/36 = 1008 = 252x4 cells, EXACT match to baseline |
| 5 | platform implemented counts windows 66 / linux 61 / macos 66 | census | REAL | my per-platform census = 66/61/66 — matches live-matrix claims in W19-A10's PARITY-REPORT audit |
| 6 | compat-matrix demotions applied (asf/aac-lc/alac/vp9/pcm linux → partial) | rows | REAL | live rows all show linux="partial"; applied jsonl (6 lines, applied_at 2026-11-10T20:07:51Z) cites rows+reasons verbatim |
| 7 | "CI validates it TODAY via the inline python3 structural check (card 1.E2)" | :8-9 | REAL | .github/workflows/ci.yml:255 "schema-validation (G1/G6 — … + feature-matrix …)" |
| 8 | "The full tools/feature-audit … is OWED and unassigned debt" | :10-12 | SHALLOW (stale, safe direction) | tools/xtask/src/cmd/feature_audit.rs exists; baseline ran it (rows 252 + R4 violations printed) |
| 9 | "Parity percentages are computed by the tool, never stored" | :23 | REAL | no parity field in any row (tomllib key scan); baseline parity-report computes 0/252 |
| 10 | meta.last_updated = "2026-11-09" | :38 | FALSE (stale) | matrix edited 2026-11-10T20:07:51Z by apply-transitions (F-002) |
| 11 | evidence strings resolvable | rows | SHALLOW | feature-audit baseline: FAILURE, 2 R4 violations (rows 23, 163) — see F-001 |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| last_updated 2026-11-09 | transitions applied 2026-11-10T20:07:51Z | matrix content post-dates its own meta timestamp |
| "feature-audit … OWED and unassigned debt" | tools/xtask/src/cmd/feature_audit.rs + baseline run | tool landed; header stale (understates — safe direction) |
| row 23 evidence "crates/codecs/src/video/h264/sw/**" | sw.rs is a FILE, no sw/ directory | evidence path dangling though the code exists |

## 4. Findings
F-001 [S3] Where: rows codec-video-h264-software (evidence "crates/codecs/src/video/
h264/sw/**") and config-advanced-folding (evidence cites ui/src/screens/settings/areas/
advanced.tsx).
What is wrong: both evidence paths do not resolve — the feature-audit gate exits FAILURE
(2 violations, baseline xtask-feature-audit.txt). The flagship H.264 row's proof pointer
is broken while the actual code (sw.rs, rust_h264 =0.6.1) exists; a reader following the
evidence hits a missing file. What correct looks like: fix the glob to `sw.rs` (or the
actual dir), fix/remove the advanced.tsx pointer. Confidence: HIGH — Verification: ran
(tomllib + baseline + ls).
F-002 [S4] meta.last_updated not bumped by `cargo xtask compat-matrix
apply-transitions` — the ledger's change-control discipline (R4 evidence + git audit)
skips its own freshness field. Confidence: HIGH — Verification: read + applied-jsonl.
F-003 [S4] Header calls the feature-audit tool unassigned debt; it is in-tree and
running. Confidence: HIGH.

## 5. Matrix-row coverage note
Row-level verdicts are owned by the 21 chunk audits (rows-001-252, waves 13-18 + W19
agents); this file-level ledger adds the cross-cutting facts: census equality with the
orchestrator baseline, transition application, the two dangling evidence paths, and the
staleness set. W18's "status layer drifts both directions" is confirmed at both edges
here: demotions applied 2026-11-10 (5 rows), promotion recorded 2026-11-15 (rust_h264,
per W19-A02's w24-h audit of row 427).
