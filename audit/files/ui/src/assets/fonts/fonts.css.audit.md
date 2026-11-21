# AUDIT: ui/src/assets/fonts/fonts.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 34 / 1 @font-face rule (26 lines of header comment) |
| File verdict | REAL (one latent portability defect) |
| Reachable from shipped entry point? | YES (web build): main.tsx:5 `import "./assets/fonts/fonts.css";` — the FIRST import in the app entry, ahead of React. Baseline #22 `npm run build` OK (vite bundles the css+woff2). No GUI binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `Read` fonts.css (complete) + `LS` the fonts dir (woff2 + LICENSE.txt present); `stat` InterVariable.woff2 → 352,240 bytes; `head` LICENSE.txt → SIL OFL 1.1, "(c) 2016 The Inter Project Authors"; **python3 fontTools** (available in sandbox) → name table `Inter Variable / Version 4.001;git-9221beed3`, fvar axes `opsz 14..32 / wght 100..900`, cmap spot-check Ǎ(0x01CD)/ə(0x0259)/ğ(0x011F) all present (latin-ext verified); `rg -n -- "--font-family-sans" src/tokens/generated/tokens.css` (:64-65, "Inter" first in sans AND time stacks); `rg -n "fonts.css" src` (sole import: main.tsx:5, with its own "Bundled type FIRST" rationale); `rg -n "repo-bundled type"` → ui/e2e/visual/MATRIX-20261109.md:43 verbatim; `sed -n '95,125p' ui/e2e/visual/MATRIX-20261113.md` (CI run 43378741271, 13/50 list, the fix record); `rg -n "italic" src --glob '*.css'` → zero (normal-only face is complete for this app); `rg -n "browserName" ui/playwright.visual.config.ts` → chromium only. |

## 1. One-paragraph verdict
The rare UI file whose every factual claim survives hostile checking: the token stack really does put "Inter" first (tokens.css:64-65), the purge-failure story is real and quoted faithfully from ui/e2e/visual/MATRIX-20261113.md (CI run 43378741271, 13/50 failed, exact screen list), the bundled face really is Inter Variable with the claimed axes (fontTools: opsz 14..32, wght 100..900, matching `font-weight: 100 900`), latin-ext coverage spot-checks pass, the license file really is OFL 1.1 alongside, and main.tsx really imports it first. The one defect is technical, not documentary: `format("woff2-variations")` is a legacy editor's-draft keyword that the current CSS Fonts spec replaced with `format(woff2) tech(variations)`; the src list has no modern fallback entry, so any user agent that does not recognize the legacy keyword skips the only src and silently falls back to the host font — the exact regression this file exists to kill — and the repository's own visual evidence can never catch that, because the visual lane runs Chromium only.

## 2. What actually works (verified)
— Verified by fontTools (run in sandbox): family "Inter Variable", `fvar` axes `('opsz', 14.0, 14.0, 32.0)` and `('wght', 100.0, 400.0, 900.0)` — the comment's "opsz+wght 100..900" and the rule's `font-weight: 100 900` are both exact. Latin-ext glyphs Ǎ/ə/ğ present in cmap (the "latin+latin-ext coverage" claim, sampled).
— Verified by `stat` + `head`: 352,240-byte woff2 (sane for Inter variable latin+latin-ext), LICENSE.txt is the genuine SIL OFL 1.1 with the Inter Project copyright — the "copy requirement is satisfied by this repository" claim (:20-22) holds on disk. (Whether LICENSE.txt is git-tracked is UNVERIFIABLE under this session's no-git deviation; it is referenced by MATRIX-20261113.md:110.)
— Verified by rg: the "repo-bundled type — no host font dependency" quote exists verbatim at ui/e2e/visual/MATRIX-20261109.md:43; the CI failure narrative (run 43378741271, 13/50, screen list, settings-stayed-exact) matches MATRIX-20261113.md:97-104 word for word in every checkable detail.
— Verified by rg: no `italic` anywhere in ui CSS — the normal-only face covers the app's full usage; no synthetic-italic risk.
— Verified by main.tsx:1-5: first-import ordering claim is real, and `font-display: block` matches its documented purpose (hold first paint; a bundled asset cannot 404 in practice).

## 3. Findings

### F-001 [S3]
Where: ui/src/assets/fonts/fonts.css:33
Code:
```css
  src: url("./InterVariable.woff2") format("woff2-variations");
```
What is wrong: `format("woff2-variations")` is a legacy CSS-Fonts-4-editor's-draft keyword, superseded in the current spec by `format(woff2) tech(variations)` (or plain `format("woff2")` — a variable font IS a woff2). It is the ONLY src entry: per spec, a user agent that does not support the named format skips the src and moves on — there is nothing to move on to, so such a UA unloads the face and renders the token stack's fallback (`-apple-system/Segoe UI/Roboto/sans-serif`), i.e. host-dependent type — the precise regression the header documents this file was created to fix (CI run 43378741271). Chromium-family browsers accept the legacy keyword; whether Firefox/WebKit do is UNVERIFIED in this sandbox (no browser available; no network).
Why it matters: the blast radius is exactly the parity guarantee the file promises ("the face travels with the app on every OS", :15) — on a skipping UA the promise silently fails, text-heavy screens drift per-host, and nothing in the repository notices: `playwright.visual.config.ts:44` pins the visual lane to `browserName: "chromium"`, so the repo's own evidence pipeline is structurally blind to the failure mode.
What correct looks like: dual src — `src: url(...) format("woff2-variations"), url(...) format("woff2");` — or the spec-current `format(woff2) tech(variations)` with a plain `format("woff2")` fallback; both keep Chromium behavior identical and stop spec-strict UAs from skipping.
Confidence: HIGH (that the keyword is legacy-spec and single-src is fragile by construction), MEDIUM (that a real shipped UA actually skips it — UNVERIFIED, no browser in sandbox; flag for a REQUEST-ORCH-RUN-style visual check on firefox/webkit if the visual lane ever grows a second browser).
Confidence: HIGH — Verification: read + spec knowledge; UA behavior UNVERIFIED.

## 4. Per-function (rule) table
| Rule | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `@font-face { Inter }` | 28-34 | REAL | F-001 | family/style/weight-range/display all correct for the verified face (axes confirmed by fontTools); src URL resolves (woff2 in same dir); the format keyword is the single defect. |

## 5. Test honesty
No test file targets fonts.css directly. Indirect: the visual lane (ui/e2e/visual/, playwright chromium) re-pins PNG baselines against the rendered face — MATRIX-20261113.md's post-fix record claims PR 50/50 and nightly 216/216 passed. That record is the doc's own claim, not a baseline-executed run (baseline INDEX #21-24 covers npm ci/build/vitest/tokens only — no playwright execution), and it is Chromium-only — so the parity guarantee is REAL-UNPROVEN from this audit's evidence, and structurally unable to catch F-001.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the token stack declares 'Inter' first (tokens.css --font-family-sans)" (:3-4) | this file | REAL | tokens.css:64-65 — both sans and time stacks. |
| "CI run 43378741271, visual PR-lite lane: 13/50 failed (…screen list…)" (:8-11) | this file | REAL | MATRIX-20261113.md:97-99 — identical run id, count, and screen list. |
| "MATRIX-20261109.md: 'repo-bundled type — no host font dependency'" (:13-14) | this file | REAL | MATRIX-20261109.md:43 verbatim (the cited filename lacks only its path, ui/e2e/visual/). |
| "Inter v4.1 variable (opsz+wght 100..900, latin+latin-ext coverage), SIL OFL 1.1 — LICENSE.txt alongside" (:19-21) | this file | REAL (axes/coverage/license) / UNVERIFIED (the "v4.1" label) | fontTools: axes + latin-ext + OFL verified; name table says `Version 4.001;git-9221beed3` — whether Inter's "4.1" release ships as name-table 4.001 cannot be checked offline (label traced to MATRIX-20261113.md:109, same wording). |
| "font-display: block on purpose: first paint never shows a fallback face" (:24-26) | this file | REAL (with the standard 3s-block caveat) | Read; correct for a bundled asset; if the woff2 ever failed to load, text would be invisible up to the ~3s block period — acceptable documented tradeoff. |
| "baselines are portable again … the face travels with the app on every OS" (:15-16) | this file | REAL-UNPROVEN / at risk | Post-fix visual passes are the matrix's own record (not baseline-run); and F-001 makes the "every OS" claim UA-conditional, not OS-conditional only. |

## 7. Dead / duplicated / unreachable code
- None: single rule, single import (main.tsx:5), asset + license present. The woff2 binary itself belongs to asset-group w15-g003 (another agent's lane) — this audit verified only what fonts.css claims about it.

## 8. Dependencies & flags
- None beyond the woff2 asset. No tokens, no flags. Import order dependency on main.tsx (documented and honored).

## 9. Open questions for the next agent
- w15-g003's owner audits InterVariable.woff2 as an asset — cross-check my fontTools facts (axes 14..32/100..900, version 4.001, 352,240 bytes) against that group's expectations and confirm LICENSE.txt is tracked (I cannot, no-git deviation).
- If anyone adds a second browser to the visual lane, F-001 becomes immediately testable; until then it stays latent.
