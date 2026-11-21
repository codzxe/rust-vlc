# AUDIT: ui/stylelint.config.js
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 111 / 2 (resolveConfigStandardPath, default config object) + 3 rule groups + 1 override |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as tooling: `.github/workflows/ui.yml` step "2. lint" runs `npm run lint:css` (ui.yml:102-110, chained with eslint, `if: always()`) → scripts/lint-css.mjs → this config over `src/**/*.css`. Not shipped in the bundle (build-time gate only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file + scripts/lint-css.mjs + ui/package.json + ui.yml:102-116 · `rg -n 'lint:css' .github/ scripts/ docs/` (CI wiring found in ui.yml:110; docs/waves logs carry its history) · `rg -c -g '*.css' 'stylelint-disable' ui/src` (17 files, 55 disable comments) · `rg -o 'stylelint-disable(-line\|-next-line)? [a-z-]+' ui/src --no-heading | sort | uniq -c` (44 color-no-hex, 8 no-descending-specificity (blocks), 1 function-disallowed-list, 1 color-no-hex block, 3 line-form) · `rg -n '"node_modules/stylelint"' package-lock.json` (18.0.0 / 41.0.0 pinned — matches the comment) · `sed -n` on w26.gfix-E.md:162, w26.i04.md:96 (historical repo-wide FAILURES), w26.k20.md:147 (claimed exit 0) · baseline INDEX (lint:css absent from all baseline runs) |

## 1. One-paragraph verdict
The machinery is real and careful: three token-discipline rules layered on stylelint-config-standard, a BEM-calibrated selector-class pattern with a documented [PROPOSAL] to tighten, a genuinely clever dual-path resolution of the extended config (local install first, sibling-of-running-CLI fallback), a scoped exemption for the one emitter-output file that exists precisely so nobody hand-edits a disable into generated code, and CI wiring that a wave-log says was earned by a real slip (fonts.css committed unlinted). The problem is the header's central honesty claim: "Hand-written CSS carries ZERO inline disables — every raw color in src/**/*.css is a lint error" is FALSE at HEAD by two orders of magnitude — 55 `stylelint-disable` comments live in 17 hand-written CSS files, 44 of them `color-no-hex` exemptions covering raw hex values (nine accent swatches in form.css alone under a block disable, eighteen in states.css, six in playlist.css…), plus one `function-disallowed-list` exemption in surface.css. The tree's real policy is "raw colors allowed via justified inline disable pending the token lane" (every disable carries a frame measurement + handoff note) — a defensible policy the config comment denies exists. Second: the gate's pass status at HEAD is UNPROVEN — the audit baseline contains no lint:css run, its wave history includes documented repo-wide failures, and the lone "exit 0, zero violations" claim is a wave-log entry, not reproducible evidence.

## 2. What actually works (verified)
— The rules are correctly composed: `color-no-hex`, `function-disallowed-list` with regexes for the rgb/hsl/hwb/lab/lch/oklab/oklch families plus exact `"color"` (which, as a non-regex entry, matches only the `color()` function — `color-mix()` passes, exactly as the comment intends), `color-named: "never"` — and `transparent` used ~50× across the CSS survives because it is not a `<named-color>` (consistent with w26.k20's logged exit 0).
— The selector-class-pattern regex is sound and honestly calibrated: it accepts the codebase's real BEM (`lumen-menu__item--selected`) while rejecting camelCase/snake/uppercase/leading digits, with the strict-BEM tightening deferred as [PROPOSAL].
— The tokens.css override is correctly scoped (`files: ["src/tokens/generated/tokens.css"]`) and its rationale verified: that file is emitter output, byte-for-byte drift-checked by `npm run tokens:check` — PASS in baseline ui-tokens log (baseline #24).
— resolveConfigStandardPath: primary `require.resolve` from the config's own location (works for real installs — stylelint-config-standard@41.0.0 is a devDependency, pinned in the lockfile at exactly the version the comment names); fallback resolves as a sibling of the running stylelint CLI (argv[1]) and passes the resolved FILE to `extends` so nested extends resolve correctly. Both paths traced; no silent-failure mode (a double miss throws at config load, failing loudly).
— scripts/lint-css.mjs matches the comment's contract: local bin first (offline), npx fallback with a loud notice; spawn failures exit 2, not 0 — the degraded path cannot silently pass.
— CI wiring exists: ui.yml:102-110 runs `npm run lint` then `npm run lint:css` with a comment recording why the CSS gate was promoted from local-only.

## 3. Findings

### F-001 [S3]
Where: ui/stylelint.config.js:15-23
Code:
```
// Exception policy (W25 reality, w26.k20 refresh of a Solstice-era note that
// still pitched "veil gradients, tape shadows" — exemptions that no longer
// exist): the ONLY exemption is the scoped tokens.css override below, and
// it exists because that file is emitter output. Hand-written CSS carries
// ZERO inline disables — every raw color in src/**/*.css is a lint error,
```
What is wrong: the claim is false at HEAD. `rg -c -g '*.css' 'stylelint-disable' ui/src` counts 55 inline disable comments across 17 hand-written CSS files: 44 `color-no-hex` exemptions (states.css ×18, playlist.css ×6, variants.css ×5, onboarding.css ×3 block, form.css ×3 line + ×1 block covering nine raw-hex accent swatches at form.css:503-535, player.css ×2, setting-field.css, subtitles.css, options.css, equalizer.css, customize.css, controls.css ×1 each), 8 `no-descending-specificity` (blocks in icon.css ×3, appshell.css, list.css, onboarding.css + next-line ×2 hotkeys.css, ×1 playlist.css, ×1 customize.css), and 1 `function-disallowed-list` (surface.css:39, raw-black scrim ink). "The ONLY exemption is the scoped tokens.css override" is contradicted by every one of these.
Why it matters: the comment is the policy statement contributors and auditors read to learn the enforcement model. The tree's actual model — raw colors permitted via a justified inline disable carrying a frame measurement and a token-lane handoff note (e.g. options.css:310 "no token — token-lane handoff, w26.g18 P3-3") — is a materially different (and defensible) regime; describing it as "ZERO inline disables" hides 44 known token-lane debts behind a claim of zero, and the stated remedy ("do NOT get inline disables: they get semantic tokens") is the opposite of what 17 files actually did.
What correct looks like: rewrite the exception policy to match reality: "raw colors in hand-written CSS require an inline disable with a frame citation and a token-lane handoff marker; the token lane retires them"; optionally add a counting check (`rg -c 'stylelint-disable'` non-regression) so the debt is visible instead of denied.
Confidence: HIGH — Verification: read + exhaustive grep with per-rule breakdown (commands above).

### F-002 [S4]
Where: ui/stylelint.config.js:20-21 (the gate claim), audit/runs/baseline/ (absence)
Code:
```
// and `npm run lint:css` (scripts/lint-css.mjs → local stylelint 18.0.0)
// is the gate.
```
What is wrong: the gate's pass status at HEAD is unproven. The audit baseline (audit/runs/baseline/INDEX.md, runs #21-24) covers npm ci / build / test / tokens / settings / hotkeys / l10n — but NOT `lint:css` (nor `lint`). The only "exit 0, zero violations" evidence is docs/waves/w26/logs/w26.k20.md:147,162 — a wave-log claim, not a reproducible run — while other wave logs record the repo-wide gate FAILING mid-wave (w26.gfix-E.md:162: fails in settings.css; w26.i04.md:96: "9 errors — all in OTHER lanes' files").
Why it matters: a gate whose current state nobody has run is a claim, not a gate; the wave history proves this gate regresses between lanes (settings.css, dynamics.css slips) and was only caught by sandbox runs.
What correct looks like: add `npm run lint:css` (and `npm run lint`) to the baseline run set — REQUEST-ORCH-RUN already filed (see log entry 4).
Confidence: HIGH (on the evidence gap) — the actual HEAD pass/fail is UNVERIFIED.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| resolveConfigStandardPath | 39-46 | REAL | — | dual-path resolution traced; loud failure if both miss |
| rules.color-no-hex / function-disallowed-list / color-named | 52-63 | REAL | — | "color" exact-match excludes color-mix (verified semantics); transparent not a named color |
| rules.selector-class-pattern | 75-82 | REAL | — | regex accepts real BEM, rejects case violations; [PROPOSAL] documented |
| overrides[tokens.css] | 84-110 | REAL | — | scoped to the emitter-output file; nulls exactly the 8 rules the output shape trips; drift guard (tokens:check) PASS in baseline #24 |
| header comment block | 1-32 | FALSE (F-001) / UNVERIFIED (F-002) | S3/S4 | "ZERO inline disables" false; gate status unproven |

## 5. Test honesty (if tests exist in/for this file)
No test exercises this config directly (List.test.tsx's `cssRule` helper greps list.css source — it is not stylelint). The config's verification surface is the CI gate itself (ui.yml step 2), which has no baseline run — see F-002. Mutation reasoning: flipping `color-no-hex` to `null` would let 44+ documented exemptions AND any new raw hex through silently; the CI gate (when it runs) is the only detector.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Hand-written CSS carries ZERO inline disables — every raw color in src/**/*.css is a lint error" | :18-19 | FALSE | 55 disables in 17 files, 44 color-no-hex (F-001) |
| "the ONLY exemption is the scoped tokens.css override below" | :17-18 | FALSE | F-001 (plus inline exemptions) |
| "New genuinely-unexpressible values do NOT get inline disables: they get semantic tokens" | :21-23 | FALSE as practice | every disable carries a "no token — token-lane handoff" justification |
| "color-mix() is deliberately ALLOWED: composing var(--token)s … is still token discipline" | :9-11 | REAL | "color" is an exact-name entry, not a regex — color-mix not matched (dynamics.css:239-243 relies on it) |
| "transparent is exempt by the rule itself" | :12-13 | REAL-UNPROVEN | transparent/currentcolor are not <named-color> productions; ~50 usages survive w26.k20's logged exit 0; not executed in this sandbox |
| "npm run lint:css … is the gate" | :20-21 | REAL-UNPROVEN | wired in ui.yml:110 ✓; no baseline execution (F-002) |
| "stylelint 18.0.0 + config-standard 41.0.0" | lint-css.mjs comment / w26.k20 | REAL | lockfile pins both exactly (package-lock.json:3626-3628, 3693-3695) |

## 7. Dead / duplicated / unreachable code
- None. The npx fallback path is degraded-mode tooling, deliberately kept with a loud notice.

## 8. Dependencies & flags
- stylelint ^18.0.0 (lockfile: 18.0.0), stylelint-config-standard 41.0.0, node:module createRequire, node:process. ESM config (`type: module`). No feature flags.

## 9. Open questions for the next agent
- Who owns retiring the 44 color-no-hex disables — the "token lane" is cited by every one; a theme-tokens audit should count them against tokens.json coverage.
- ui.yml's "2. lint" step runs eslint then stylelint in one step with `if: always()` — a stylelint failure marks the step red but eslint results already ran; acceptable, noted for the CI auditor.
