# AUDIT: tools/xtask/src/cmd/visual.rs
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-23 |
| Lines / functions | 115 / 3 non-test fns, 2 tests |
| File verdict | REAL (thin wrapper, does exactly what its doc says; not exercised by any automated path — see F-001) |
| Reachable from shipped entry point? | NO (dev tooling). Reachable via `cargo xtask visual` (main.rs:124-125 -> cmd/mod.rs:134 `run: visual::run`). CI runs the harness DIRECTLY (`node scripts/visual-run.mjs --mode=pr`, ui.yml:173-174), bypassing this wrapper; not in full_gate.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (harness artifacts, CI wiring, registration, mjs arg validation; exit 0); sed quote checks (exit 0); baseline test citations |

## 1. One-paragraph verdict
An honest 30-line shell-out: it resolves ui/ from CARGO_MANIFEST_DIR (with a hard error if package.json is missing), picks npm.cmd on Windows (std's Command::new does not try PATHEXT), forwards every argument verbatim to `npm run visual --`, and propagates the child's exit code 1:1 (0/1..255 verbatim, >255 → 2, signal → 1). Every doc claim in the header checks out against the code and the harness artifacts (ui/package.json:17 `"visual": "node scripts/visual-run.mjs"`, visual-run.mjs exists and validates unknown args with usage + exit, playwright.visual.config.ts committed, e2e/visual/baselines committed). The one gap: no automation ever runs THIS file — CI invokes visual-run.mjs directly, so the wrapper (and its Windows-shim logic specifically) is local-dev-only and can silently rot.

## 2. What actually works (verified)
— Both unit tests pass in the baseline (cargo-test-workspace.txt:6723-6724) and the environment test is real (asserts ui/package.json + playwright.visual.config.ts exist — it fails if the harness is deleted).
— verified by rg: harness artifacts all exist (ui/scripts/visual-run.mjs, ui/e2e/visual/baselines + manifest.ts, ui/playwright.visual.config.ts); visual-run.mjs:30-39 rejects unknown args with usage — the module doc's "scripts/visual-run.mjs owns validation (unknown args exit 2 with usage)" claim is TRUE.
— Registration real: cmd/mod.rs:130-134.

## 3. Findings
F-001 [S4]
Where: tools/xtask/src/cmd/visual.rs:56-83 vs .github/workflows/ui.yml:173-174
Code:
```
    let status = Command::new(npm_program())
        .arg("run")
        .arg("visual")
```
```yaml
      - name: "visual: PR-lite lane (compare vs committed baselines)"
        run: node scripts/visual-run.mjs --mode=pr
```
What is wrong: the CI visual job (and full_gate.sh — zero hits) never go through this subcommand; CI calls the .mjs directly. The wrapper's entire value-add — Windows npm.cmd resolution, ui-dir resolution, exit-code normalization — is exercised only by humans running `cargo xtask visual` locally, so a regression in it (e.g. the Windows shim) would break local dev on Windows while CI stays green. Why it matters: drift between the documented dev entry point and the actual automation path; the Windows branch of npm_program() is untested on Linux CI and unexercised by any bot. What correct looks like: CI runs `cargo xtask visual --mode=pr` (it exists precisely to be the driver), or the doc says "local driver only — CI calls the mjs directly". Confidence: HIGH — Verification: rg (ui.yml + full_gate.sh + scripts/).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ui_dir | 34-48 | REAL | — | CARGO_MANIFEST_DIR + ancestors().nth(2) = workspace root (tools/xtask -> root); package.json guard errors loudly |
| npm_program | 52-54 | REAL | — | cfg!(windows) npm.cmd shim — correct, but only ever exercised locally (F-001) |
| run | 56-83 | REAL | — | verbatim arg forwarding; exit mapping 0/1..255 passthrough, >255->2, signal->1 (documented); help via include_str! (file exists) |
| tests (2) | 89-113 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ui_dir_resolves_to_the_workspace_ui_package | YES | asserts real files exist at the resolved path; catches a moved/renamed harness |
| npm_program_uses_the_cmd_shim_on_windows | WEAK on Linux | asserts the cfg branch — on Linux it can only ever compare "npm"=="npm"; the Windows arm is unverifiable in this lane (would not catch a broken npm.cmd string on Linux CI) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "thin, honest shell-out with verbatim exit-code propagation" | visual.rs:4-5 | REAL | run() L67-82: forwards args, maps exit codes 1:1 |
| "harness itself lives in ui/e2e/visual/ + ui/scripts/visual-run.mjs (Playwright Chromium, zero new npm deps)" | visual.rs:2-3 | REAL | all artifacts exist (rg/ls); Playwright usage per ui.yml chromium install step |
| "scripts/visual-run.mjs owns validation (unknown args exit 2 with usage)" | visual.rs:65-66 | REAL | visual-run.mjs:38-39 prints unknown-argument + usage |
| "--seed ... the harness must FAIL (acceptance proof, never in CI)" | visual.rs:16-17 | REAL-UNPROVEN | seed flag exists in mjs usage text; the must-FAIL proof not executed by me (would need a Playwright run — outside load discipline) |
| "--mode=pr ... the fast, strict per-PR gate (doc 08 §9.3)" | visual.rs:11-12 | REAL | ui.yml visual job runs the pr lane on every push/PR (ui.yml:148+) — though NOT through this wrapper (F-001) |

## 7. Dead / duplicated / unreachable code
- No dead code in-file. The whole subcommand is unreachable from automation (F-001) — dev-entry only.
- The visual lane's CURRENT pass/fail state is UNVERIFIED in the baseline (orchestrator ran vitest only, not the Playwright visual lane; ui.yml comments claim "proven green 2026-11-09: 30 passed 57.8s" — a workflow-comment claim I could not re-run).

## 8. Dependencies & flags
- std only; spawns `npm` (npm.cmd on Windows) with cwd=ui/. No env vars read or printed. Zero xtask-side feature flags.

## 9. Open questions for the next agent
- Who owns deciding whether CI should invoke `cargo xtask visual` instead of the raw mjs (F-001)? The UI-domain agent should also verify the "30 passed" claim in ui.yml:143-144 against a real run.
- visual_help.txt (include_str!) content not audited beyond existence — if its usage text drifts from visual-run.mjs's own usage string, the two can disagree silently.
