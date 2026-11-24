# AUDIT: ui/src/screens/convert/convert.css
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 299 / ~28 selectors |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ConvertScreen.tsx imports it (screen routed at /convert); honest-gate and lane-note classes emitted at ConvertScreen.tsx:246 and :331 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | class-extraction + TSX consumer scan (0 dead); rg gate/lane-note emitters |

## 1. One-paragraph verdict
Token-disciplined stylesheet for the /convert screen; zero dead selectors (all
28 classes emitted by ConvertScreen.tsx). Notably it styles two HONESTY
artifacts: `.convert__field-gate` ("naming why the action is a stub") and
`.convert__lane-note` (the "honest unavailable banner") — the convert screen
discloses its stubbed Browse and its unwired queue lane rather than faking
them. No findings.

## 2. What actually works (verified)
- 0/28 dead selectors (consumer scan; every class found in ConvertScreen.tsx).
- Progress affordance uses native <progress> styling on both engines
  (-webkit-progress-*/-moz-progress-bar, lines 257-280) — real element, not a
  div theater bar.
- Light-mode card step fallback uses a token with default
  (--color-surface-panel-raised, var fallback, 296-299) — documented pending
  token (theme lane i19).

## 3. Findings
None. Checked: dead selectors (none), token discipline (raw px only with
frame/structural comments), contrast annotations (lane-note "AA on the
panel"), media/reduced-motion (absent here — no transitions defined, so
nothing to reduce), duplication (none).

## 4. Per-function table (>100 LOC; CSS blocks as units)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| section-header pin + page column | 20-35 | REAL | 15px-class pin documented (h19 drift) |
| head/card/fields | 37-108 | REAL | |
| gate + target note | 110-131 | REAL | honesty copy classes |
| actions footer | 133-146 | REAL | |
| queue lane | 148-253 | REAL | job rows, honest partial percent, tabular nums |
| progress bar | 255-280 | REAL | native progress, both engines |
| job detail + light-mode step | 282-299 | REAL | |

## 5. Test honesty
No tests in this file. ConvertScreen.test.tsx / convertQueue.test.ts cover the
logic (other agents' lanes for the .tsx; the CSS only styles their output).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The honest Browse gate — quiet tertiary caption naming why the action is a stub" | convert.css:110-111 | REAL | emitter ConvertScreen.tsx:246; the stub disclosure claim itself is auditable in ConvertScreen.tsx (not my file) |
| "The honest unavailable banner" | convert.css:166 | REAL | emitter ConvertScreen.tsx:331 |
| "Semantic tokens only (§3.2); raw px is structural layout only" | convert.css:16 | REAL | read: all raw values carry frame/structural comments |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Design tokens. No feature gates. No backend surface.

## 9. Open questions for the next agent
- ConvertScreen.tsx auditor: confirm the gate copy actually names the reason
  (the CSS promises it; the copy lives in the TSX/i18n).
