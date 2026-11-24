# AUDIT: ui/e2e/web/site/index.html
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 21 lines / static QA page |
| File verdict | REAL (web conformance harness page; consumer + run lane verified) |
| Reachable from shipped entry point? | NO — test tooling page (card 7.I1 browser matrix) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | file read; harness.js existence + seam check (exit 0); baseline rc-smoke cite |

## 1. One-paragraph verdict
A deliberately bare QA surface for the Playwright-driven web conformance matrix: honest
self-label ("not the product UI — this page is test tooling"), a status element the driver
overwrites, and a module script loading harness.js — which EXISTS (ui/e2e/web/site/harness.js)
and exposes `window.__lumenWebMatrix` at its line 738 (verified rg). The lane it serves ran
in the committed baseline: rc-smoke.txt:205-210 `=== 8. web-ui-browser-load (vite preview +
Playwright chromium) ===` / `web-ui-browser-load: PASS`.

## 2. What actually works (verified)
- rg `__lumenWebMatrix` in ui/e2e/web/site/harness.js -> line 27 (comment) and 738
  (`window.__lumenWebMatrix = {`), matching index.html's `#status` promise
  `window.__lumenWebMatrix.run()` (verified read of both files).
- Baseline: rc-smoke.txt:205-210 — browser-load PASS.

## 3. Findings
F-001 [S4] The page's `<p id="status">idle — ...` copy is the only human surface; nothing
guards the harness.js contract from the HTML side (if the script 404s, the page silently
stays idle). The Playwright spec would fail on run() absence, so the gap is cosmetic.
Confidence: HIGH — read.

## 4. Consumers
ui/e2e/web/web-matrix.spec.ts + matrix.config.json (the browser matrix driver).
Baseline lane: web-ui-build + web-ui-browser-load both PASS (rc-smoke.txt).

## 5. Open questions
- Does the web-matrix spec run per-browser? (matrix.config.json — W-next for that file.)
