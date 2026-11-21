# AUDIT: ui/src/screens/player/controls/SettingsPanel.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 401 / 8 (exported) |
| File verdict | REAL (a11y-correct popover panel primitives; vitest-proven; one false seam claim in a comment) |
| Reachable from shipped entry point? | NO (ui-wide: no GUI binary at HEAD). SPA-internal: ControlBar.tsx:70-79 imports it; PanelPopover (ControlBar part2 :786+) mounts it; PlayerScreen → /player App.tsx:303. Executed under vitest (baseline run #23). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read complete; `rg -n "SettingsPanel\b" ui/src -l` (sole consumer ControlBar); SettingsPanel.test.tsx inventory (4 tests); cross-checked ControlBar.test.tsx:1068 (radiogroup coverage through the host); FOCUSABLE_SELECTOR vs DOM consumers (tab buttons + radio rows are <button> — covered); `audio.list-devices` seam evidence carried over from ControlBar audit (generated/commands.ts:24; catalog command.rs:195-201). |

## 1. One-paragraph verdict
A tight, WAI-ARIA-correct popover primitive set: dialog with mount focus + Escape + Tab wrap, real tablist with arrow-key focus-follows-selection, radiogroups with roving tabindex where the checked row is the Tab stop and checking is engine-truth (never optimistic), honest inert reason rows. The keyboard model is genuinely tested. Two real complaints: the PanelSection header comment asserts "no engine seam exists" for the inert rescan glyph — the same false claim as ControlBar's Output-Device section (the `audio.list-devices`/`set-device` seam exists in the transport and engine catalog), and PanelRadioGroup is a close reimplementation of the shared RadioGroup primitive rather than a reuse.

## 2. What actually works (verified)
— Keyboard model verified by tests: focus-on-open (:40), Tab wrap (:49), arrow tab switching (:67), single tab stop (:100) — SettingsPanel.test.tsx, passed in baseline run #23; radiogroup roving + arrows covered via ControlBar.test.tsx:1068.
— APG conformance by read: radiogroup arrow keys move focus AND select (APG "selection follows focus" for radio) with disabled-row skip and full-ring wrap (:305-313); tablist arrows wrap modulo length (:118); tabpanel labelled by active tab (:166).
— Honest faces: PanelReasonRow is a plain div (never a fake control); QuietButton is a real button with a real handler at every call site (ControlBar footers → openToolsDialog).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/controls/SettingsPanel.tsx:176-180
Code:
```
/** Section header (Controls.svg "Embedded Tracks" / "Defaults" rows). The
 * optional trailing icon is the frame's right-aligned header affordance —
 * e.g. the "Output Device" rescan glyph (audio variant, x1870–1890): it
 * renders INERT (aria-hidden, no handler) because no engine seam exists
 * for it — the same honesty rule as the app menu's inert rows. */
```
What is wrong: The justification "no engine seam exists for it" is FALSE. The rescan affordance's seam is device re-enumeration — `audio.list-devices` / `audio.set-device` are typed in this very transport (`ui/src/generated/commands.ts:24`), defined in the engine catalog (`crates/commands/src/catalog/command.rs:195-201`), and accepted by the real engine (variant list in docs/perf/raw/stats-probe.jsonl:1). The component ships an affordance-shaped, click-dead glyph (aria-hidden, no handler) on the grounds that the backend doesn't exist — it does.
Why it matters: The false premise is baked into a reusable component's contract, so every future host inherits a dead affordance "by rule". Sighted users see a refresh glyph that does nothing (the SR face is honest — aria-hidden). Same falsehood as ControlBar F-001 (S2 there, where a whole section renders permanently "unavailable").
What correct looks like: PanelSection taking an optional `onTrailingIconClick`; hosts wire `audio.list-devices` rescan; the honesty rule covers hosts WITHOUT a seam.
Confidence: HIGH — Verification: read + seam evidence from ControlBar audit (same repo facts).

### F-002 [S4]
Where: ui/src/screens/player/controls/SettingsPanel.tsx:275-334
Code:
```
export function PanelRadioGroup({
  label,
  options,
}: {
```
What is wrong: PanelRadioGroup reimplements the shared RadioGroup primitive's roving-tabindex + arrow model (its own header :270-271 says "the same pattern as the shared RadioGroup primitive (doc 08 §4.5)") instead of extending it — the delta is only the right-aligned `value` column and section-title naming.
Why it matters: Two radiogroup implementations to keep in sync; the next a11y fix must land twice (this is exactly how the four divergent lane-msg resolvers happened — cited in StageOverlays' own cleanup note).
What correct looks like: RadioGroup growing an optional trailing-value slot, or PanelRadioGroup composing it.
Confidence: HIGH — Verification: read (shared primitive not read line-by-line — UNVERIFIED detail: whether RadioGroup already supports a value slot; from the menu barrel it does not).

### F-003 [S4]
Where: ui/src/screens/player/controls/SettingsPanel.tsx:285-291
Code:
```
  const stopIndex =
    checkedIndex >= 0
      ? checkedIndex
      : Math.max(
          0,
          options.findIndex((option) => !option.disabled),
        );
```
What is wrong: When every option is disabled, `findIndex` returns −1 and `Math.max(0, −1)` pins the roving tabindex (0) on row 0 — a disabled row. Harmless in practice (disabled `<button>`s are not Tab-focusable, so the browser skips it; the group then has NO Tab stop), but the code claims row 0 is the stop while no stop exists.
Why it matters: Misleading invariant; a future swap of `<button>` for a focusable-on-disabled element (e.g. div with tabindex) would expose it.
What correct looks like: `const stopIndex = checkedIndex >= 0 ? checkedIndex : options.findIndex(o => !o.disabled);` with `tabIndex={index === stopIndex ? 0 : -1}` naturally assigning −1 when stopIndex is −1.
Confidence: HIGH — Verification: read + trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SettingsPanelTab` (type) | 28-33 | REAL | — | Pre-resolved labels (i18n at call sites). |
| `FOCUSABLE_SELECTOR` | 36-37 | REAL | — | Covers buttons/links/inputs/tabindex; tab + radio rows are <button>. |
| `SettingsPanel` | 60-174 | REAL | — | Mount focus; Escape (stopPropagation); Tab wrap first↔last; tablist arrows wrap + focus follows; tab/tabpanel ids wired; footer optional. |
| `PanelSection` | 181-206 | REAL | F-001 | Inert trailingIcon with false "no seam" justification. |
| `PanelRadioRow` | 224-252 | REAL | — | role=radio, data-checked, optional value; roving props from group. |
| `PanelRadioOption` (type) | 254-263 | REAL | — | Descriptor. |
| `PanelRadioGroup` | 275-334 | REAL | F-002, F-003 | APG-correct arrows (focus+select, skip disabled, wrap); engine-truth checked; duplicate of shared primitive; all-disabled stopIndex quirk. |
| `PanelReasonRow` | 338-340 | REAL | — | Honest inert reason. |
| `PanelValueRow` | 343-350 | REAL | — | Label/value. |
| `PanelToggleRow` | 353-370 | REAL | — | Label + Toggle with aria-label. |
| `QuietButton` | 380-401 | REAL | — | Real button; bare face for settings variants; icon hardcoded "info" (frame's ⓘ). |

## 5. Test honesty (SettingsPanel.test.tsx, 4 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "focus lands on the panel itself on open" (:40) | YES | Asserts document.activeElement === panel. |
| "Tab wraps inside the panel" (:49) | YES | Drives Tab/Shift+Tab at edges, asserts wrap targets. |
| "ArrowRight/ArrowLeft switch tabs with focus following selection" (:67) | YES | Key events + assert activeTab + focus. |
| "roving tabindex: only the active tab is a Tab stop" (:100) | YES | Asserts tabIndex values per tab. |
Gap: no direct test of PanelRadioGroup here — covered only via ControlBar.test.tsx:1068 (host-level). Acceptable but indirect; a mutation of the disabled-skip ring (:305-313) would need ControlBar's test to catch it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "it renders INERT … because no engine seam exists for it" | SettingsPanel.tsx:178-180 | FALSE | audio.list-devices/set-device exist (commands.ts:24, catalog command.rs:195-201, stats-probe variant list). |
| "the same pattern as the shared RadioGroup primitive (doc 08 §4.5)" | SettingsPanel.tsx:270-271 | REAL | Pattern verified in this file; duplication noted (F-002). |
| "Escape closes and restores trigger focus (e12 F8)" | header :18 | REAL (panel half) | Escape → onRequestClose verified here; trigger-focus restore lives in ControlBar's PanelPopover (part2, A09 to verify). |

## 7. Dead / duplicated / unreachable code
- No dead exports (all consumed by ControlBar; PanelReasonRow/PanelValueRow/PanelToggleRow used at ControlBar :529-549/:584-601/:615/:622/:686).
- Duplication: PanelRadioGroup vs shared RadioGroup (F-002).

## 8. Dependencies & flags
Icon, Toggle from components barrel; no client/transport imports (pure presentational — good layering). No feature flags. Styling in controls.css (A03's part2? — controls.css#part2 owned by A03).

## 9. Open questions for the next agent
- A09 (ControlBar part2): verify PanelPopover's outside-pointer dismiss restores trigger focus as the header promises.
- Does the shared RadioGroup primitive (components/menu or form) already accept a trailing value slot? If yes, F-002 upgrades from duplication to dead code.
