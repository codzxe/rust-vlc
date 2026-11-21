# AUDIT: ui/src/components/form/SegmentedControl.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 85 / 1 (SegmentedControl) |
| File verdict | REAL (code, live) — arrow keys move selection but NOT focus (APG radiogroup violation) |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: 4 production mounts — ControlBar.tsx:537, ThemeModePicker.tsx:83, subtitles.tsx:254, OnboardingScreen.tsx:474; baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `<SegmentedControl` (4 production mounts + test); read useRadiogroupKeyboard.ts (no focus management in the handler); read SettingsPanel.tsx:295-330 (the inline copy DOES move focus — the contrast that proves the defect); rg `toHaveFocus|activeElement` in SegmentedControl.test.tsx + RadioGroup.test.tsx → 0 hits (no focus-movement coverage) |

## 1. One-paragraph verdict
A live, well-structured primitive (button-based role="radio" segments, shared roving hook, correct ARIA, four real consumers) with one genuine accessibility defect: pressing an arrow key fires `onChange` and `preventDefault()` but never moves DOM focus — the focus ring stays on the now-unchecked segment while the selection moves, so keyboard users see two highlighted segments and screen readers keep announcing the stale segment as unchecked. The codebase's own inline copy of this pattern (SettingsPanel's PanelRadioGroup, l.310) calls `radios.current[option.key]?.focus()` after selection — the correct APG behavior — making this not an oversight in the pattern but a gap in the shared hook's implementation of it. The tests assert the selection moves, never the focus, so the defect ships green.

## 2. What actually works (verified)
— verified by rg → 4 production mounts (onboarding theme pick, settings appearance + subtitles, player control bar) — this primitive, unlike RadioGroup, is alive.
— verified by reading l.52-84 → correct ARIA: div role=radiogroup + aria-label, button role=radio + aria-checked, roving tabIndex on the checked segment, disabled group + per-option disabled, aria-invalid.
— verified by reading l.75-77 → click guard `if (!checked)` prevents redundant onChange on the active segment.
— verified by reading SegmentedControl.test.tsx:16-95 → semantics/click/roving/arrows-wrap/disabled×2/id/invalid all behavioral; :97+ DOM snapshot + CSS pins.
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S3]
Where: ui/src/components/form/SegmentedControl.tsx:50 (hook wiring; hook body at useRadiogroupKeyboard.ts:30-45)
Code:
```
  const onKeyDown = useRadiogroupKeyboard(options, value, onChange);
```
with the hook's handling:
```
    const next = enabled[(currentIndex + delta + enabled.length) % enabled.length];
    if (next && next.value !== value) onChange(next.value);
    event.preventDefault();
```
What is wrong: The handler moves the SELECTION (onChange) and cancels the default, but never focuses the newly-selected segment. After ArrowRight, `document.activeElement` is still the old segment (now aria-checked=false, tabIndex=-1), while the accent lands on the new one. The APG radiogroup pattern requires arrows to move focus AND selection together. The codebase's own fourth copy — SettingsPanel.tsx:310 `radios.current[option.key]?.focus();` — implements the correct behavior, proving the pattern was known; the shared hook shipped without it.
Why it matters: Every arrow press leaves the widget in a state screen readers misreport (focused segment announced unchecked; the actually-selected segment never focused/announced). Sighted keyboard users see a split-brain highlight (focus ring on one segment, selection accent on another). This affects all four live consumers (onboarding, two settings areas, control bar).
What correct looks like: after computing `next`, focus the segment's element (the hook needs element refs or the component needs its own keydown that calls `nextEl.focus()`), matching PanelRadioGroup.
Confidence: HIGH — Verification: hook read (no focus call anywhere); contrast implementation read; no test asserts focus movement (grep 0 hits in both suites).
Mutation check (reasoned): add `nextEl.focus()` → no current test fails (nothing asserts focus); the defect is invisible to the suite by construction.

### F-002 [S4]
Where: ui/src/components/form/SegmentedControl.tsx:72-73
Code:
```
            // roving tabindex: only the selected segment is tabbable
            tabIndex={checked ? 0 : -1}
```
What is wrong: Same hole as RadioGroup F-002: if `value` matches no option, no segment is tabbable — the control has no Tab stop at all. Live consumers currently always pass a matching value (verified at mount sites), so latent; the API still permits the broken state.
Why it matters: Any future uncontrolled-start usage ships an unreachable control.
What correct looks like: first-enabled fallback stop when nothing is checked.
Confidence: HIGH — Verification: read; mount sites pass valid values.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SegmentedOption`/`SegmentedControlProps` | 6-26 | REAL | — | mirrors RadioGroup's API (swap-compatible, l.32-34) |
| `SegmentedControl` | 36-85 | REAL | S3/S4 | F-001 focus, F-002 tabindex; `value` attr on button harmless; id per segment derived from groupId+value (duplicate values ⇒ duplicate ids — caller error, unguarded) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :16-30 semantics + click onChange | YES | behavioral |
| :31-35 roving tabindex (selected tabbable) | YES | asserts tabIndex per segment — but only the matched-value case (F-002's no-match case untested) |
| :37-48 arrows move selection with wrap | YES | onChange assertions — selection only, focus NOT asserted (F-001's blind spot) |
| :50-80 disabled group / per-option skip | YES | behavioral |
| :82-95 id / invalid / DOM snapshot | YES | |
| :106-127 CSS string pins | YES (text-level) | focus-visible styles pinned as text |
Mutation check (reasoned): remove the hook's preventDefault → arrow tests still pass (jsdom fires no native radio behavior on buttons) — another blind spot; break wrap-around → :37 fails.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantics mirror RadioGroup (role=radiogroup, native-button role=radio segments, roving tabindex, arrow keys with wrap-around)" | l.32-34 | REAL (structure) / DEFECT (focus) | all present except the focus-follows-selection half of "roving" (F-001) |
| "enum settings can swap RadioGroup → SegmentedControl without rewiring" | l.33-34 | REAL | identical prop shapes; ironically RadioGroup itself is dead (its audit) |

## 7. Dead / duplicated / unreachable code
No dead code (4 live consumers). Shared correctly via useRadiogroupKeyboard (unlike SettingsPanel's copy). Unreachable in shipped product (W14).

## 8. Dependencies & flags
useRadiogroupKeyboard (shared hook — the F-001 fix belongs there so RadioGroup + SwatchGroup inherit it), form.css (`.lumen-segmented*`, `--error`). No feature flags.

## 9. Open questions for the next agent
- SwatchGroup.tsx owner (AccentColorPicker's base): same F-001 focus defect via the same hook — verify and file from your side; a single hook fix repairs all three primitives.
- SettingsPanel.tsx owner: PanelRadioGroup already does focus correctly — consider migrating it onto the fixed hook to retire the fourth copy.
