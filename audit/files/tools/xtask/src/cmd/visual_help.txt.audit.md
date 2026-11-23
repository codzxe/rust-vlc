# AUDIT: tools/xtask/src/cmd/visual_help.txt
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-23 |
| Lines / functions | 18 / 0 (data: help text, `include_str!`-embedded) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (xtask dev tooling); reachable from `cargo xtask visual --help` — PROVEN by execution |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo run -q -p xtask -- visual --help` → exit 0, output byte-identical to the file (`diff` clean); rg verification of every flag/artifact-path claim against `ui/scripts/visual-run.mjs`, `ui/e2e/visual/visual.spec.ts`, `ui/playwright.visual.config.ts`, `.github/workflows/ui.yml`, and the baselines directory. Harness itself NOT run (npm/playwright lane — outside load discipline). |

## 1. One-paragraph verdict
Every claim in this help text is true against the code that implements it, and I proved the text is live: `cargo xtask visual --help` prints it byte-identical (diff clean, exit 0) via `include_str!` at visual.rs:58. The three flags match the harness exactly (`--mode=pr` default at visual-run.mjs:26 `let mode = "pr"`; `--mode=nightly` full matrix; `--update` → `--update-snapshots` at mjs:49; `--seed` → `SEED_2PX=1` at mjs:55, consumed by visual.spec.ts:86-91 to inject `transform: translateX(2px)`), the baseline layout matches the Playwright `snapshotPathTemplate: "e2e/visual/baselines/{platform}/{arg}{ext}"`, the artifact paths match the config (reporter `outputFolder: "playwright-report/visual"`; Playwright default `test-results/`), and the exit-code contract matches both the mjs (`process.exit(child.status ?? 2)`, unknown-arg usage → 2) and visual.rs's verbatim forwarding. The only soft spot: "per-OS sets" — only the Linux set is committed (216 PNGs under `baselines/linux/`); the macos/windows sets do not exist yet, and the CI lane that would capture them is `[BLOCKED on first real macOS/Windows run]` with `continue-on-error: true` — a documented, honest gap (ui.yml:35-41: "fails by design (missing baselines) — that is honest, not fake green"), which the help's own "linux is canonical" phrase discloses but understates.

## 2. What actually works (verified)
- `cargo run -q -p xtask -- visual --help` → exit 0; `diff` against tools/xtask/src/cmd/visual_help.txt → byte-identical.
- Flag claims ≡ implementation: visual-run.mjs:26-49 (mode default pr, --update → --update-snapshots), mjs:55 (`SEED_2PX: seed ? "1" : "0"`), visual.spec.ts:30-31 (VISUAL_MODE/SEED_2PX readers), spec:86-91 (the 2-px `translateX` proof shift).
- Artifact-path claims ≡ playwright.visual.config.ts: reporter `["html", { outputFolder: "playwright-report/visual" }]` → `ui/playwright-report-visual/`; Playwright's default `test-results/` output dir not overridden → `ui/test-results/`.
- Exit-code claims ≡ visual-run.mjs:39-40 (unknown arg → usage + exit 2), mjs:63 (`process.exit(child.status ?? 2)`), visual.rs:70-77 (verbatim 0/1/2 forwarding; signals → failure).
- "never in CI" (--seed): no CI lane passes `--seed` — ui.yml's `visual` (PR-lite) and `visual-nightly` (full matrix) lanes and the blocked `visual-baseline-seed` lane (`--update-snapshots` capture, not `--seed`) all verified; the acceptance-proof flag appears in no workflow step.
- Baselines: `ui/e2e/visual/baselines/linux/` = 216 PNGs (convert-1280x720-{1x,2x}-{dark,light}, 1920x1080 set, …) — layout matches the help's `<platform>/<case>.png` exactly.

## 3. Findings
F-001 [S4]
Where: tools/xtask/src/cmd/visual_help.txt:14-15
Code:
```
Baselines: ui/e2e/visual/baselines/<platform>/<case>.png (per-OS sets, doc
08 §6.3; linux is canonical).
```
What is wrong: "per-OS sets" (plural) describes the layout convention, but only ONE OS set exists — `ls ui/e2e/visual/baselines/` → `linux` only (216 files). The macos/windows sets are absent, the CI lane that would capture them is `[BLOCKED on first real macOS/Windows run]` + `continue-on-error: true` (ui.yml:253-256), and compare-mode runs on those OSes "fail by design (missing baselines)". The help's "linux is canonical" hints at this but a reader would reasonably expect two more sets to exist. Honest-but-understated; the CI comment is more explicit than the help text. What correct looks like: one clause — "(only the linux set is committed; macos/windows pending first real runner)".
Confidence: HIGH — Verification: read + ls.

## 4. Per-function table
N/A (data file, zero functions; consumer `visual::run` at visual.rs:56-60 prints it on `--help`/`-h` — executed).

## 5. Test honesty
N/A (no tests in this file). Consumer-side: visual.rs's own tests (ui_dir resolution, npm shim) do not exercise the help path — I executed it manually instead.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Usage: cargo xtask visual [--mode=pr\|nightly] [--update] [--seed]" | visual_help.txt:3 | REAL | mjs:31-38 parses exactly these; unknown args → usage + exit 2 |
| "--mode=pr (default): 1280x720 @1x, dark + light on the current screens" | visual_help.txt:5-6 | REAL | mjs:26 default `pr`; spec:11 + ui.yml:29 "PR-lite lane (1280x720 @1x, dark+light — 15 routes × 2)" |
| "--mode=nightly full matrix: {1280x720, 1920x1080} x DPI {1,2} x {dark,light}" | visual_help.txt:7-8 | REAL | spec:12 "full matrix: viewports × DPI × modes"; ui.yml:34 "full matrix (136 cases)"; baselines contain the 2×2×2 case ids |
| "--update re-record baselines (doc 08 §6.5 policy …)" | visual_help.txt:9-10 | REAL | mjs:49 `--update-snapshots` |
| "--seed inject the 2-px regression proof shift; the harness must FAIL (… never in CI)" | visual_help.txt:11-12 | REAL | spec:86-91 (`translateX(2px) !important` injection); no CI lane passes --seed (ui.yml verified) |
| "Baselines: … (per-OS sets …; linux is canonical)" | visual_help.txt:14-15 | SHALLOW (understated) | only linux set committed (216 PNGs); macos/windows missing, capture lane BLOCKED (F-001) |
| "Diff artifacts land in ui/test-results/ and ui/playwright-report-visual/" | visual_help.txt:15-16 | REAL | config reporter outputFolder `playwright-report/visual`; Playwright default `test-results/` |
| "Exit codes: 0 pass \| 1 visual diff/failure \| 2 hard error" | visual_help.txt:18 | REAL | mjs:63 `process.exit(child.status ?? 2)`; mjs:40 unknown-arg exit 2; visual.rs:70-77 verbatim forwarding |

## 7. Dead / duplicated / unreachable code
None. The file is included by exactly one consumer (visual.rs:58) and printed on --help (executed).

## 8. Dependencies & flags
None (plain text data). Its accuracy depends on the ui/ harness (visual-run.mjs, visual.spec.ts, playwright.visual.config.ts) — all cross-checked; the harness itself was not executed (npm/playwright lane outside this audit's load discipline; UI lane results cited in baseline: npm ci/typecheck/test/build pass, visual lane not in the baseline runs).

## 9. Open questions for the next agent
- The visual harness end-to-end (`cargo xtask visual` PR lane, and the `--seed` must-FAIL acceptance proof) has not been executed in this audit nor in the orchestrator baseline — worth one real run by whoever audits ui/e2e/visual/* to confirm the 216-baseline lane actually passes on linux.
- The blocked macos/windows baseline capture (continue-on-error lane) means the "per-OS" story is one real runner away from completion — track whether the [BLOCKED] label ever clears.
