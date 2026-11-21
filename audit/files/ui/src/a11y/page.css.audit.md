# AUDIT: ui/src/a11y/page.css
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 14 / 1 rule (body floor) |
| File verdict | REAL — wired at app entry, tokens-only, both modes resolve; the axe suite it cites exists, covers the named routes in both modes, and is CI-wired (but has no committed baseline run) |
| Reachable from shipped entry point? | YES: main.tsx:37 `import "./a11y/page.css"` — the app entry, ahead of every screen render. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "a11y/page" src/` (imported by main.tsx:37, cited by lumen.css:59 + appshell.css:26); `rg -n "axe|contrast" e2e/a11y.spec.ts` (AxeBuilder wcag2a/2aa, /playlist + /media-info routes :44-45, both theme modes :207-217, accepted-violations ledger :109-130); `rg -n "color-surface:|color-content-primary:" tokens/generated/tokens.css` (defined per mode: dark :39/:54, light :123/:130); `rg -n "test:e2e" .github/workflows/ui.yml` (step 5 runs playwright) |

## 1. One-paragraph verdict
Fourteen honest lines: the body-level paint floor that stops any screen forgetting its own background from rendering dark-theme ink on raw browser white. It is imported at the app entry (main.tsx:37), uses only the two semantic tokens that resolve in both theme modes (verified in the generated token sheet), and the 4.F4 axe suite it cites genuinely exists and audits the two named routes (/playlist, /media-info) in both modes with wcag2a/2aa tags including color-contrast. The single blemish: the identical body paint is declared in TWO always-loaded sheets (this one and theme/lumen.css:59-64) — a documented, deliberate duplication that still leaves two truths for "the body floor" to drift apart.

## 2. What actually works (verified)
— verified by `rg` → sole rule imported once at app entry; no other sheet depends on its absence.
— verified by tokens/generated/tokens.css:39/54 (dark) + :123/130 (light) → `--color-surface`/`--color-content-primary` resolve per mode; light-mode pair #161616 on #c2c2c2 ≈ 10:1 — the floor itself passes AA in both modes.
— verified by e2e/a11y.spec.ts:44-45,207-217 → the "4.F4 axe suite" is real: AxeBuilder with wcag2a/2aa tags over /playlist and /media-info in both theme modes, with a stale-entry-surfacing accepted-violations ledger (:86-130); CI ui.yml step 5 runs `npm run test:e2e`.
— verified by read appshell.css:26-31 + lumen.css:59-70 → the body-MARGIN reset lives in lumen.css/appshell.css (with cross-referencing comments); this file paints but does not reset — no scope creep.

## 3. Findings

### F-001 [S4]
Where: ui/src/a11y/page.css:11-14
Code:
```
body {
  background: var(--color-surface);
  color: var(--color-content-primary);
}
```
What is wrong: the same two declarations exist verbatim in theme/lumen.css:59-64 (`body { background: var(--color-surface); color: var(--color-content-primary); … }`), and both sheets are always loaded (main.tsx imports lumen.css:34 then page.css:37 — page.css wins only by import order). The comments cross-document the duplication ("kept here so the base sheet alone never shows raw white"), but two always-live declarations of "the body floor" can drift (e.g. someone retints one sheet's copy); the same pattern already needed a coordination comment for body margin (appshell.css duplicate pending owner drop, k05 P3-4).
Why it matters: two truths for one floor; the a11y-critical invariant should live in exactly one sheet.
What correct looks like: keep the floor here (the a11y sheet is the semantically right owner) and reduce lumen.css's copy to a comment pointing at it, or vice versa.
Confidence: HIGH — Verification: read of both sheets + main.tsx import chain.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `body` floor rule | 11-14 | REAL | S4 | tokens-only, both modes resolve (≈10:1 light); duplicate with lumen.css (F-001) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| e2e/a11y.spec.ts "axe: zero wcag2a/2aa violations on {route} ({mode})" (:207) | YES | deleting the floor would put dark-theme ink on white and trip color-contrast on /playlist, /media-info in dark mode — the suite's accepted ledger names only specific marker-scoped hits, so a whole-page contrast regression would fail the assertion (it prints hits so stale accepted entries also surface) |
| — gap | The suite's green state has NO committed run in audit/runs/baseline/ (INDEX #21-24 covers npm ci/build/vitest/checks only); its passing status rests on CI config (ui.yml step 5) — UNVERIFIED at HEAD by a baseline artifact. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "/playlist and /media-info routes painted no background … caught by the 4.F4 axe suite" | page.css:3-6 | REAL-UNPROVEN | the suite exists, covers exactly those routes in both modes (e2e/a11y.spec.ts:44-45,207-217) and is CI-wired; the historical catch event itself is not re-runnable from the baseline (no playwright run there) |
| "Tokens only, both theme modes resolve" | page.css:9 | REAL | tokens.css per-mode definitions verified |
| "guarantees the page behind them is NEVER the raw browser white … any future screen that forgets its paint" | page.css:7-9 | REAL | body-level paint loads before any screen; screens that paint their own surfaces (player deep, settings surface) layer on top |

## 7. Dead / duplicated / unreachable code
Duplicated: the body floor (F-001). Nothing dead.

## 8. Dependencies & flags
Semantic tokens only (verified in generated tokens.css). No flags. Sits after lumen.css in main.tsx import order — order-dependence noted in F-001.

## 9. Open questions for the next agent
- Whoever owns theme/lumen.css: resolve the body-floor duplication (F-001) and the pending appshell.css body-margin duplicate (k05 P3-4) in one pass — three sheets currently touch `body`.
- REQUEST-ORCH-RUN (recorded, non-blocking): `cd ui && npm run test:e2e -- a11y.spec.ts` would convert the axe suite's status from CI-config-wired to executed-evidence.
