# CLAIM AUDIT: docs/reviews/w24-mq-fixes-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL 7 / TRUE-at-time-since-fixed 1 / REAL-UNPROVEN 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A double-lane fix-audit whose fix claims all still verify in-tree, and whose own
confirmed follow-up defect (F-1) was subsequently fixed exactly as recommended.
The three new regression tests exist, the corrected spans/arithmetic recompute
exactly, the schema re-pointings landed, the dynamic guard code exists, and
build-msi.ps1 now escapes basenames with an in-code credit to this very audit's F-1.
The re-run batteries are historical/UNVERIFIED, as the doc itself scopes honestly.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | P2-2 fixed with a real regression test (worst_slope_headline...) | :17-18,:42 | REAL (fix + test present) | report.rs:156-159 folds `f64::NEG_INFINITY`; regression test at report.rs:505 `fn worst_slope_headline_prints_the_largest_finite_slope()` — both present today. The empirical revert-proof (:18) is a historical run — UNVERIFIED here, but the test exists and asserts a real headline render. |
| 2 | P2-3: synthesis now 1,699,754 / 127,467 with stride arithmetic inline | :43,:22 | REAL | docs/soak/soak-report-20261110.md:62-63 carries both spans; recomputation: 614x489=300,246 → 2,000,000-300,246=1,699,754 ✓; 609x37=22,533 → 150,000-22,533=127,467 ✓; 613x245=150,185 → 1,000,000-150,185=849,815 ✓; profile 3x480.0+216.4+200.1=1856.5 s=30.94 min ✓. |
| 3 | P2-4: schema $comment now points at soak-report §6.2; no regression-guards.md reference remains | :44 | REAL | tests/soak/scenarios.schema.json:5 $comment ends "(the regression-guard wiring gap is recorded in docs/soak/soak-report-20261110.md §6.2)"; rg regression-guards in tests/soak + soak_scenarios -> only that gap mention, no dangling file ref. |
| 4 | P3-5: seeks_per_reopen now an optional scenario key (loader default 16, range 1..1024) + regression test | :49 | REAL | scenarios.schema.json:110 `"seeks_per_reopen"` key present; manifest.rs:802 `fn seeks_per_reopen_overrides_and_range_checks()`. |
| 5 | Three new regression tests: worst_slope..., count_baseline_and_span..., seeks_per_reopen... | :17 | REAL | All three exist: report.rs:505, runner.rs:1353, manifest.rs:802 (rg on fn names). |
| 6 | P4-1: guards derive the expected secret set from packaging.toml [signing.*] at guard time via inline python tomllib | :57 | REAL | release.yml:267-273 and :528-534 `import sys, tomllib ... signing = tomllib.load(fh)["signing"] ... print(f"PACKAGING_SIGNING_{section.upper()}_{key.upper()}")` — the dynamic derivation the doc describes is in the workflow today. |
| 7 | P4-2: gen-wxs.sh --self-test exercises the REAL generation path with hostile basenames | :58 | REAL | scripts/release/gen-wxs.sh:5 comment "doc-basename XML escaping + hostile-fixture self-test added in W24-Q"; --self-test referenced at :86. |
| 8 | F-1: build-msi.ps1:161,164 still had unescaped $base in File/@Source at w24/q HEAD (for the Lead) | :66 | TRUE-at-time-since-fixed | The defect was real then; since fixed by W24-U with in-code credit: build-msi.ps1:97-99 "since W24-U (W24-T audit F-1) the doc BASENAMES in File/@Source do too", :143 `$escBase = XmlEscape $base`, :145/:148 `Source=`"$escBase`"`, hostile assert :437 `Source="a&amp;b&lt;c&gt;&quot;d&quot;.md"` — the exact one-line fix + mirrored hostile-fixture check this audit recommended. |
| 9 | M battery: 32/32 in 39.32-39.38 s; clippy CLEAN; fmt CLEAN; lint-literals 0/452 | :17,:20 | REAL-UNPROVEN (historical) | In-worktree runs on branches not present here; structure corroborated (the 32 = 29 inherited + 3 named tests, all 3 existing). Note: today's baseline fmt --check FAILED — post-review drift elsewhere, not a contradiction of this report's then-state. |
| 10 | Q battery: gate.sh PASS, 42/42 run-blocks bash -n, YAML/JSON valid, lint-literals 0/452 | :31,:33,:62 | REAL-UNPROVEN (historical) / structure consistent | My own census today: release.yml 6 jobs / 55 run-blocks (grown from 42 by later waves — consistent history); schema JSON machine-valid (loaded during this audit). |

## 3. Contradictions
None. OBS-1 (27.3-min mixed derivation) and OBS-2 (off-by-one commit-count label) are
the doc's own nits against its lanes — self-critical, not inflated.

## 4. Fabrication / inflation indicators
None. The verdict is earned by enumeration (9/9 + 4/4 per-finding verdicts with commit
SHAs), the honest-sampling statement (:75) separates run vs recomputed vs not-run, and
the one confirmed follow-up defect is escalated rather than hidden.

Commands run: rg NEG_INFINITY/worst_slope in report.rs; rg 3 test fn names; sed soak-report :62-63; python3 arithmetic (spans + profile total); rg $comment + seeks_per_reopen in schema; rg tomllib in release.yml; sed build-msi.ps1 :95-110/:143-148; rg self-test gen-wxs.sh; yaml census (6 jobs/55 blocks).
