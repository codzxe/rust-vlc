# CLAIM AUDIT: docs/reviews/w24-e-docs-findings.md (324 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 12 (core re-run claims + finding fix statuses) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 REAL / 0 / 0 / 0 / 0 / 0 / 3 UNVERIFIED (git-inspection claims) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This review doc (W24-R2, ~2026-11-10) is real verification work: its core recompute
claims reproduce exactly today where the underlying tree has not moved, and its 9 findings
were subsequently FIXED in-tree — several with comments explicitly crediting this review.
The "73 format rows = 21+16+19+17" recompute matches my independent parse exactly; the
"37 claimed" figure is 38 today solely because codec-video-h264-software flipped
blocked→implemented on 2026-11-15 (rust_h264 Lead admission, matrix :419-425) — the review's
P2-1 premise (blocked ×4) was true at its date. Where the tree HAS moved (settings schema
32→121 `[[setting]]`, ui=true 21→24), the review's numbers are honest snapshots, not lies.
The strongest evidence of quality: P2-2's dangling evidence pointer (rc-smoke-2026-11-10.md,
never committed) is still absent from docs/audit/ and both format records were repointed to
release-checklist §2 (container-avi.md:71, audio-pcm.md:60) — the review's exact suggested
fix, applied; P2-3's bash -n multi-file defect fix appears verbatim at release-checklist.md:51
citing "W24-R2 finding P2-3"; P3-7's GNU stat portability fix landed in rc-smoke.sh:131-134
(`wc -c <` POSIX + explanatory comment); P3-4's anchored pattern fix landed at
docs-sweep:110.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Matrix recompute ... 73 format rows = 21+16+19+17; 37 claimed desktop; 0 web" | :205-206 | REAL (73 exact; 37→38 post-admission) | My parse: 73 rows (container 21, video 16, audio 19, subtitle 17); claimed 38 today; the delta is h264-software's 2026-11-15 rust_h264 admission (matrix:419-425 evidence "closed 2026-11-15 on the Lead's explicit instruction"). |
| 2 | "`ls docs/formats/*.md | wc -l` → 53" | :207-208 | REAL | Re-run: 53. |
| 3 | "ls docs/audit/ shows only docs-sweep-20261110.md" (rc-smoke-2026-11-10.md never committed) | :56-57 | REAL | docs/audit/ today = docs-sweep-20261110.md + panic-baseline.json (no rc-smoke file) — P2-2 stands as a correct finding, and its fix landed (repointed refs at container-avi.md:71, audio-pcm.md:60). |
| 4 | "`stat -c %s` ... would fail check 6 on macOS" (P3-7) | :131-134 | REAL finding; FIXED since | scripts/rc-smoke.sh:131-134 today: `wc -c <` POSIX port with comment "GNU `stat -c %s` reads 0 on macOS (BSD stat wants -f %z)". |
| 5 | "`bash -n` ... only checks the first file argument" (P2-3) | :71-89 | REAL (bash semantics); FIXED since | release-checklist.md:51 now has the loop `for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" \|\| exit 1; done` + "(W24-R2 finding P2-3)". |
| 6 | "keymaps/default.toml → 150 bindings" | :217 | REAL | My count: 141 base + 9 macOS overrides = 150 (see rows-157-169 audit, row 161). |
| 7 | "`rg -c '\[\[setting\]\]'` → 32 actually prints 33; anchored gives 32" (P3-4) | :94-99 | REAL at date; tree grew since | Today anchored count = 121 (schemas/settings.toml 1627 lines — the w26.b02 settings seed landed post-review); docs-sweep:110 now records the anchored pattern citing "anchored to exclude the schema's explanatory comment on line 13" — the fix applied. |
| 8 | "The rc-smoke harness is real — no stubs, no fake passes, exit code is the gate, BLOCKED counts as FAIL" | :16-19, 239-249 | REAL | Baseline audit/runs/baseline/rc-smoke.txt: overall FAIL recorded honestly; AGENT-SPEC §13 corroborates engine-build/headless-decode/command-layer-play/browser-load PASS. |
| 9 | "config-validator → FAILURE, 4 violations, all missing assets/brand/*" | :198-200 | REAL | audit/runs/baseline/xtask-config-validator.txt (AGENT-SPEC §13: "config-validator: FAILURE — 4 missing brand assets"). |
| 10 | "full_gate.sh has exactly the 9 steps the checklist lists and does not run config-validator" | :202-203 | STALE detail (superseded) | full_gate.sh today has 11 sections (adds packaging gate + one more) and still does not run config-validator; claim was true at date. |
| 11 | git-inspection claims (merge markers, origin/w24/c pushed, commit hashes 45c5c6f/70ed69c/ff001e5, git tag 0) | :250-285 | UNVERIFIED | No git in sandbox by rule. |
| 12 | "Worklog ... single W24-E entry (r4)" (P3-9) | :164-174 | REAL | `rg -c "Task ID: W24-E" worklog.md` -> 1 (entry at :841, consolidated r4 closeout; lines shifted from review's :778). |

## 3. Contradictions
None. All divergences are dated snapshots superseded by later engine growth (settings
schema 32→121 entries; h264-software blocked→implemented 2026-11-15; full_gate 9→11 steps),
each traceable to a recorded event.

## 4. Fabrication / inflation indicators
None. This document is the opposite of inflation: it reduced two green claims to
accurate-but-defective (P2-1, P2-3) and its findings were accepted and fixed by the
reviewed lane.

## 5. Fix-uptake verification (independent, 2026-11-24)
| Review finding | Fixed in tree? | Evidence |
|---|---|---|
| P2-1 (wrong row classification) | Superseded (row flipped 2026-11-15) | matrix:419-425 rust_h264 admission |
| P2-2 (dangling rc-smoke doc) | YES | container-avi.md:71, audio-pcm.md:60 repointed to release-checklist §2 |
| P2-3 (bash -n first-file-only) | YES | release-checklist.md:51 loop + credit line |
| P3-4 (unanchored rg count) | YES | docs-sweep:110 anchored pattern note |
| P3-7 (GNU stat) | YES | rc-smoke.sh:131-134 wc -c port |
| P3-8/P3-9 | Deferred/accepted-as-is | worklog.md:925 records P3-9 deferral |
