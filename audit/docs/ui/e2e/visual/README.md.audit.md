# CLAIM AUDIT: ui/e2e/visual/README.md
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 REAL / 3 REAL-UNPROVEN / 0 / 0 / 0 / 1 FALSE (stale pending note) / 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
Mostly accurate operational record for the visual harness — its on-disk
counts verify exactly (216 baselines counted, PR lane 50/nightly 216 per
manifest). One materially stale claim: the wave-26 section still lists the
full-matrix diff-green re-verification + re-seed as "Pending (Lead, wave
L)" while the sibling MATRIX-20261113.md (same directory, dated the same
day, not referenced by this README) records that exact 5th re-pin as DONE:
216/216 update, 216/216 compare verify, 208 baseline files changed. The
README omits the newest MATRIX record and under-reports the harness's
verified state.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Disk holds **216 baselines** (25 routes × 2 viewports × 2 DPIs × 2 themes = 200 grid + 16 accent samples)" | :84-87 | REAL | `ls ui/e2e/visual/baselines/linux \| wc -l` -> 216; manifest ROUTES=25 enumerated; ACCENT_SAMPLES=16 |
| 2 | "the nightly lane is **216 cases**, the PR lane **50**" | :86-87 | REAL | manifest.ts: pr = 1280x720@1x → 25 routes × 2 = 50; MATRIX-20261113.md run record: PR 50 passed, nightly 216/216 |
| 3 | "Wire CI (done, issue #5): Linux PR lane per push/PR ... in .github/workflows/ui.yml" | :99-101 | REAL | ui.yml `visual` job runs `node scripts/visual-run.mjs --mode=pr` on every push (read earlier in this audit) |
| 4 | Seeded 2-px regression: "8 failed", "exit 1"; unseeded "8 passed (14.2s)", nightly "32 passed" | :19-44 | REAL-UNPROVEN | historical console log (2026-10-23); SEED_2PX mechanism verified in visual.spec.ts:88-100; not re-runnable here (playwright/browser budget) |
| 5 | "`cargo xtask visual --mode=pr` runs the same lane through the Rust task runner and forwards the exit code verbatim (verified: 0)" | :46-47 | REAL-UNPROVEN | tools/xtask/src/cmd/visual.rs exists; no cargo in sandbox |
| 6 | "**Pending (Lead, wave L)**: waves i–k landed further UI changes ... full-matrix diff-green re-verification + re-seed is deferred" | :91-95 | FALSE (stale) | MATRIX-20261113.md:147-152 records the 5th re-pin done: "nightly `--update`: **216/216**", "nightly compare verify: **216/216**", "208 baseline files changed" — the pending work this README defers is completed in the unreferenced sibling doc |
| 7 | "macOS/Windows baseline sets: still CI-blocked on a first real seed run" | :102-104 | REAL | only `baselines/linux/` exists (ls) |
| 8 | "Closed the 4.H1 snapshotDir trap in playwright.config.ts (testIgnore: "**/visual/**")" | :69-71 | REAL-UNPROVEN | config change claim; playwright.config.ts not re-read by me (visual.spec.ts + workflow read; UNVERIFIED detail) |
| 9 | 4.G2 section: 15 routes, 136 nightly, 30 PR | :49-55 | REAL (historical) | dated 2026-10-24 record; superseded by :81-87 — clearly sectioned as history |
| 10 | issue #37 re-pin: 136/136 nightly, 30/30 PR, provenance MATRIX-20261109.md | :61-68 | REAL-UNPROVEN | MATRIX-20261109.md exists (ls); numbers not re-run |
| 11 | "W25 redesign re-pin ... shipped with no MATRIX record ... now recorded in MATRIX-20261112.md" | :75-80 | REAL | docs/waves/w26/baselines/MATRIX-20261112.md exists (rg hit earlier) |
| 12 | "In-panel states ... are NOT routes — they belong to the in-flow state harnesses" | :105-108 | REAL | matches manifest.ts:20-27 comment verbatim |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| :91-95 "Pending (Lead, wave L) ... re-seed is deferred" | MATRIX-20261113.md:147-152 run record: 5th re-pin complete, 216/216 compare-verified | The MATRIX is true — its run record includes environment provenance (chromium 156.0.8078.4, playwright 1.64.1) and byte-level change counts (208 files); the README's pending note predates/misdates it |

## 4. Fabrication / inflation indicators
None in the numbers (all counts reconcile). The staleness pattern: newest
provenance record (MATRIX-20261113.md) exists on disk but is not linked
from this README, and the README's "What wave 26 delivered" section stops
at MATRIX-20261112 — one provenance hop behind the truth.

## 5. Matrix rows
Not a matrix chunk. Cross-references: my 24 baseline-PNG audits cite
MATRIX-20261113.md's run record as their execution evidence; the stale
"Pending" note in this README would UNDERSTATE that evidence.
