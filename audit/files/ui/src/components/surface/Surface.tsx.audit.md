# AUDIT: ui/src/components/surface/Surface.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 473 / 10 (popupOwnsFirstEscape, warnUnnamedSurface, useSurfaceFocusTrap, useSurfaceA11y, Dialog, Sheet, Panel, Card, Well, Separator, SectionHeader, HeroPill) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by 12+ routed screens (ConvertScreen, PlaylistScreen, SettingsScreen, NetworkStreamScreen, OpenMediaDialog, MediaInfoPanel, CustomizePanels, ...), all reached from App.tsx routes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -l "components/surface"` (12 producer files); `rg -n "describe\(" Surface.test.tsx` (8 suites, 33 it/test); full read |

## 1. One-paragraph verdict
Clean, real primitive library: modal Dialog/Sheet with a serious focus trap (opener restore, deterministic Tab cycling, hidden-subtree filtering, nested-surface stopPropagation, IME-composition guard, popup-Escape ownership by open state rather than bare role presence — each behavior carries a fix-citation comment naming the defect it replaced), plus 6 presentational primitives. 33 tests across 8 describe blocks cover trap/escape/edge cases. The only defect is selector-level: custom `[tabindex]` widgets exposing only `aria-disabled` are trapped as focusable while semantically disabled.

## 2. What actually works (verified)
— verified by `rg`: 12 non-test producer files import these primitives; Surface.test.tsx has 33 test cases in 8 suites ("Dialog (focus trap...)", "Dialog Escape ownership (w26.g19 root fix...)", "Dialog trap edge cases", "Sheet", "tones", "Panel", "Card", "Well"). Baseline: vitest 1705 passed (§13).

## 3. Findings
F-001 [S4]
Where: ui/src/components/surface/Surface.tsx:31-42
Code:
```
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  ...
  '[tabindex]:not([tabindex="-1"])',
```
What is wrong: only native `disabled` excludes form controls; a custom `[tabindex]` row that conveys disabled state via `aria-disabled="true"` (the pattern this UI uses for honest-inert rows, e.g. ControlBar's disabled-with-reason entries use real `disabled` but menu rows use aria-disabled) remains in the trap cycle and receives focus while inert. Blast radius: focus lands on a control that does nothing. Confidence: MEDIUM — Verification: read; no failing case demonstrated (would need an aria-disabled tabindex host inside a Dialog — UNVERIFIED which hosts do this).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FOCUSABLE_SELECTOR + popupOwnsFirstEscape | 31-83 | REAL | S4 | F-001; open-state detection documented against the Folder-2/3 wedge it fixed |
| warnUnnamedSurface | 90-96 | REAL | — | dev-only console warning |
| useSurfaceFocusTrap | 122-198 | REAL | — | opener restore, Tab takeover, hidden filter, isComposing guard, ref-synced onClose |
| useSurfaceA11y | 201-207 | REAL | — | useId fallback title |
| Dialog | 219-257 | REAL | — | role=dialog, aria-modal, scrim click gated by closeOnOutsideClick |
| Sheet | 266-305 | REAL | — | same trap; side variants |
| Panel | 319-342 | REAL | — | `as` polymorphic; emits .lumen-pane (name collision rationale documented) |
| Card | 356-376 | REAL | — | 4 variants |
| Well / Separator | 384-410 | REAL | — | presentational |
| SectionHeader | 419-445 | REAL | — | host-chosen heading level |
| HeroPill | 453-473 | REAL | — | surface only; buttons stay in button lane |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Dialog focus trap suite (26-) | YES | asserts focus moves/restores/cycles |
| Escape ownership suite (178-) | YES | regression tests for the popup-wedge defects named in comments |
| trap edge cases (272-) | YES | empty-surface, hidden subtrees |
| Sheet/tones/Panel/Card/Well (345-471) | YES | attribute/class assertions |
Not executed individually — baseline vitest 1705 passed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "On close: focus is restored to the opener" (L119-120) | docstring | REAL | L195 cleanup; test suite covers |
| "an OPEN overlay popup owns the FIRST Escape" (L45-47) | comment | REAL | L157-159 yield + test suite 178- |

## 7. Dead / duplicated / unreachable code
None. All exports consumed (12 producer files).

## 8. Dependencies & flags
React only + `import.meta.env.DEV` (Vite). No backend coupling — pure UI primitives (correct: nothing here pretends to touch a backend).

## 9. Open questions for the next agent
- Inventory which Dialog/Sheet hosts place `aria-disabled` (not `disabled`) custom rows inside surfaces to size F-001; none found in my assignment's files.
