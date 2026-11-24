# AUDIT: ui/src/components/appshell/appshell.css
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 (CORRECTED same day — see §10) |
| Lines / functions | 554 / ~90 rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by AppShell.tsx (persistent chrome around every route except /onboarding and /dev/tokens, header L2-3) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg token/corruption sweeps; sanitizer-proof fixed-string counts (`rg -c '.app-shell__sidebar[hidden]'` → 1); `npx stylelint` attempt (failed: no node_modules, exit 2) |

## 1. One-paragraph verdict
A disciplined, token-driven stylesheet (80 `var(--)` uses; every raw hex is inside frame-provenance comments) with a correct `hidden`-beats-flex mechanism: `.app-shell__sidebar[hidden] { display: none; }` (L272) re-asserts display:none so the HTML `hidden` attribute wins over `.app-shell__sidebar{display:flex}` — verified intact by fixed-string count (1). The sidebar toggle's visual collapse works. Remaining nits: the global `body` margin reset lives in a component sheet, and the sheet is comment-heavy.

## 2. What actually works (verified)
— verified by `rg -c --fixed-strings '.app-shell__sidebar[hidden]'` → 1 (count-based proof, immune to display mangling); `.app-shell__sidebar{display:flex}` at L257-258 read directly.
— verified by `rg -c 'var\(--'` → 80 token usages; hex scan → comments only.
— NOT verified: stylelint pass (no node_modules; orchestrator baseline lists UI lint passing).

## 3. Findings
F-001 [S3]
Where: ui/src/components/appshell/appshell.css:21-27 (body margin reset placement)
Code (bracket-safe): `body { margin: 0; /* shell owns the full viewport */ }`
What is wrong: a global `body` reset living in a component sheet — routes mounting WITHOUT AppShell (/onboarding, /dev/tokens per header L2-3) inherit an unreset 8px body margin → scrollbar on those screens. The comment admits no global sheet zeroes it. Confidence: MEDIUM — Verification: read; not visually executed.
F-002 [S4]
Where: file-wide comment density (~40% frame-provenance commentary, measurements duplicated across appshell/player/variants sheets — re-pin maintenance hazard). Confidence: LOW — Verification: read.

## 4. Per-function table
Rule groups audited: banner/wordmark/window-controls (L30-160), rail (L195-230), sidebar head/search/meta/modes/list (L255-330) incl. the `[hidden]` collapse rule (L271-274, INTACT), playlist row/grid (L338-360), main region (L363-375), reduced-motion blocks ×2 (L378+L417, duplicated — minor), chrome-fade (`html[data-lumen-chrome='hidden']` blocks). All REAL; `.app-shell__strip:empty` (L61) defensive only.

## 5. Test honesty
No CSS tests (jsdom applies no CSS) — structural blind spot, noted honestly by the lane's own tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`hidden` must beat the class display (the toggle collapses the rail)" | comment L271 | REAL | fixed-string count of `.app-shell__sidebar[hidden]` = 1; rule verified present |
| "Semantic tokens only; raw px below is structural geometry" | header L21-22 | REAL | 80 var() uses; hex only in comments |
| docs/waves/w26/findings/w26.g04.md:23 "verified: shell sidebar [hidden] + data-sidebar='closed' → rail display:none, canvas 1388px" | wave-26 finding | REAL-UNPROVEN | player.css `:has(> .app-shell__sidebar[hidden])` rule intact (fixed-string count 1); measurement not re-run |

## 7. Dead / duplicated / unreachable code
`.app-shell__strip:empty` (L61) — defensive; no current empty-strip state found (UNVERIFIED exhaustive).

## 8. Dependencies & flags
Token layer. No env vars. stylelint expects stylelint-config-standard (config:44) — uninstallable in-tree (no node_modules).

## 9. Open questions for the next agent
- Does any /onboarding screenshot show the 8px body scroll (F-001)?

## 10. CORRECTION RECORD (mandatory disclosure)
The first version of this audit reported an S1 "tree-wide `[h`-deletion corruption" (`idden]` selectors, `const idden, setHidden]` in useChromeAutoHide.ts, `el.closest("idden]")` in Surface.tsx). That finding was a DISPLAY ARTIFACT: the Bash tool's output sanitizer strips the two-character sequences `[h` and `[m` from displayed text (verified: `printf 'a[h] b[m] c[d]'` displays as `a] b] c[d]`), which made intact `[hidden]`/`[must_use]` text LOOK corrupted in sed/python/rg line output. Sanitizer-proof verification (`rg -c --fixed-strings '[hidden]'` and `'[hidden, setHidden] = useState'` — numeric counts) proves every implicated file is INTACT: appshell.css:272 `.app-shell__sidebar[hidden]` (1), dynamics.css `.dynamics-tool__body[hidden]` (1), equalizer.css `.eq-panel__body[hidden]` (1), visualizer.css `.viz-tool__body[hidden]` (1), dialogs.css `.tools-dialog__panel[hidden]` + `.tools-card__body[hidden]` (1+1), player.css `:has(> .app-shell__sidebar[hidden])` (1), useChromeAutoHide.ts:121 `const [hidden, setHidden] = useState(false);` (1), Surface.tsx:174 `el.closest("[hidden]")` (1), Surface.test.tsx (2). No corruption exists. The retraction is recorded in audit/runs/W15-A08.log.md; sibling audits (useChromeAutoHide.ts, GraphicEQ.tsx, DynamicsCard.tsx, states.md, stylelint.config.js, VideoSliderRow.tsx) were corrected the same day.
