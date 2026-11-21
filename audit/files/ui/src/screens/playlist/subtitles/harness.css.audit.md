# AUDIT: ui/src/screens/playlist/subtitles/harness.css
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / rule groups | 64 / 7 (.w26b11-stage + modifiers, -cell, -caption, -frame--filename, -frame--rows, the match-control width pin) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — by design and verified: imported ONLY by harness.tsx (rg: zero other importers), which is a DEV-ONLY vision harness ("never imported by app routes", harness.tsx:1; App.tsx has no harness route; snapshot export env-gated W26B11_HARNESS_OUT). Not part of any production bundle |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg "harness.css" importers (harness.tsx only); rg harness in App.tsx (none); sed harness.tsx:1-19 (dev-only header + env gate); token check: 4/4 var(--…) defined; class-consumption check vs harness.tsx |

## 1. One-paragraph verdict
Exactly what it claims to be: dev-only vision-harness styling, imported by
exactly one dev-only module, never reachable from app routes, with every
class consumed by harness.tsx and every token resolving. The single finding
is the non-standard `zoom` property — acceptable in a Chromium-targeted dev
tool, but worth recording since `zoom` is not in the standard cascade and
Firefox only recently implemented it.

## 2. What actually works (verified)
— verified by rg: importers of harness.css = harness.tsx only; App.tsx
route table contains no harness route; harness.tsx header declares dev-only
and the snapshot export is env-gated (W26B11_HARNESS_OUT) — the file cannot
reach a production bundle (vite tree: harness.tsx itself is only reachable
from harness.test.tsx).
— verified by class grep: all 7 selector groups (w26b11-stage, --rows,
--filename, -cell, -caption, -frame--filename, -frame--rows) rendered by
harness.tsx (lines 111-114+).
— verified by token grep: --color-surface-deep, --font-family-sans,
--font-size-caption, --color-content-tertiary all defined (generated tokens
+ lumen.css).
— verified by baseline ui-vitest.log.txt (1705 passed): harness.test.tsx
("renders all frame states without crashing" + snapshot export) executes
with this sheet loaded.

## 3. Findings
### F-001 [S4] Non-standard `zoom` property for the reference-scale trick
Where: ui/src/screens/playlist/subtitles/harness.css:27-33
Code:
```
.w26b11-stage--rows {
  zoom: 1.73; /* frame: subtitle_one.png 900/519 */
}
```
(likewise `zoom: 2.22` at 31-33)
What is wrong: `zoom` is a non-standard (formerly IE/Chromium) property —
not part of the standard cascade; Firefox support landed only recently and
differs in interaction with fixed positioning. The comment documents the
rationale (reference-scale comparison) but the portability caveat is
unstated.
Why it matters: Dev-only blast radius (if a non-Chromium browser renders the
harness, closeups mis-scale silently). Zero production impact.
What correct looks like: `transform: scale()` + `transform-origin` (with
layout compensation) or a documented Chromium-only note in the header.
Confidence: HIGH — Verification: read.

## 4. Per-rule-group table
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .w26b11-stage | 6-16 | REAL | — | token-clean surface-deep stage |
| .w26b11-cell | 18-22 | REAL | — | consumed |
| .w26b11-stage--rows / --filename | 27-33 | REAL | S4 (F-001) | zoom trick |
| .w26b11-caption | 35-40 | REAL | — | token fonts |
| .w26b11-frame--filename | 43-47 | REAL | — | frame-measured 406px + x22 padding, marked |
| .lumen-dropdown width pin | 53-55 | REAL | — | the e09 F6 fix (362px content box) — documented rationale |

## 5. Test honesty (harness.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders all frame states without crashing (55-62) | YES (crash-level) | smoke over the harness mount incl. this sheet |
| exports standalone HTML when env-gated (63+) | NO (in normal CI) | env-gated by design — honest dev-only label |
No CSS assertions (jsdom does not compute layout) — acceptable for a vision
harness; the visual comparison is a human/dev-workflow step, UNVERIFIED in
baseline (no playwright run).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness styling (never imported by app routes)" | header 1-2 | REAL | rg importer proof + App.tsx route check |

## 7. Dead / duplicated / unreachable code
The whole sheet is unreachable from production — intentionally, honestly,
and labeled. Not counted as dead code (its purpose is dev tooling, and its
consumer exists).

## 8. Dependencies & flags
Generated tokens only. No feature flags. Gated by nothing (its importer is
the gate).

## 9. Open questions for the next agent
- None for this file. Sibling harness.css files (options, video, dynamics,
  equalizer, visualizer lanes) presumably share the pattern — their agents
  should verify the same import-gate proof.
