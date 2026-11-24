# AUDIT: ui/src/screens/player/controls/SettingsPanel.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 402 / 7 exported components (+2 interfaces, 1 const) |
| File verdict | REAL — pure presentational primitives, callbacks in/out, honest inert/reason rows |
| Reachable from shipped entry point? | YES (App -> PlayerScreen -> ControlBar.tsx:70-79 imports SettingsPanel/PanelRadioGroup/QuietButton) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `SettingsPanel|PanelRadioGroup|QuietButton` src (exit 0 — consumer ControlBar.tsx); rg `tracks.set-audio-track` src (exit 0 — dispatch site context.tsx:398-400) |

## 1. One-paragraph verdict
Clean primitive layer: the player's settings popover chrome (dialog + tablist + focus trap + radiogroups + toggle/value rows + quiet footer button). No backend wiring of its own — every interaction is a caller callback, and the file ships honest faces for absent seams (PanelReasonRow; inert trailing icon documented at L176-180). The deadness lives one layer up: the radio rows it renders are fed by ControlBar whose track selection dispatches `tracks.set-video-track`/`tracks.set-audio-track` (context.tsx:398-400) — commands the shipped desktop host refuses (`unsupported-for-host`, SERVED list apps/cli/src/dispatch.rs:41-56). So in the product build these panels LIST real tracks (tracks.list IS served) but SELECTING any row is a silent no-op. That defect is charged to context.tsx/ControlBar, not this file.

## 2. What actually works (verified)
— verified by rg: consumed by ControlBar.tsx:70-79 (the frame's Controls.svg variant panels); presentational only — no fabricated data anywhere in the file (checked every render path).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/controls/SettingsPanel.tsx:224-252 (PanelRadioRow) via consumers
Code:
```
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      disabled={disabled}
```
What is wrong: nothing IN this file — but every production mount of these radio rows selects tracks via `tracks.set-*-track` commands the shipped host refuses (SERVED list has no tracks.set-*). The panel renders real, populated, keyboard-accessible track rows whose selection silently fails in the product build (the engine-true `checked` never moves because the engine never applied it — the "never optimistic" rule at L271-273 makes the failure visible as "the check never lands", with no error surface). Confidence: HIGH — Verification: read + rg (context.tsx:398-400 dispatch; SERVED list).
F-002 [S4]
Where: ui/src/screens/player/controls/SettingsPanel.tsx:284-291
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
What is wrong: when every option is disabled, `findIndex` returns -1 and `Math.max(0, -1)` = 0 — the tab stop is assigned to a disabled (unfocusable) button, so the whole radiogroup is skipped by Tab. Edge case; blast radius one group. Confidence: HIGH — Verification: read (mutation reasoning; UNVERIFIED by execution).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SettingsPanelTab (iface) | 28-33 | REAL | — | pre-resolved labels |
| FOCUSABLE_SELECTOR | 36-37 | REAL | — | tab-trap query |
| SettingsPanel | 60-174 | REAL | — | dialog + tablist + arrow keys + Esc + focus trap |
| PanelSection | 181-206 | REAL | — | inert trailing icon honestly aria-hidden |
| PanelRadioRow | 224-252 | REAL | S3 | presentation of refused-by-host actions (F-001) |
| PanelRadioOption (iface) | 254-263 | REAL | — | |
| PanelRadioGroup | 275-334 | REAL | S4 | roving tabindex; all-disabled stop bug (F-002); check is engine-truth, never optimistic |
| PanelReasonRow | 338-340 | REAL | — | the honest no-seam face |
| PanelValueRow | 343-350 | REAL | — | label/value |
| PanelToggleRow | 353-370 | REAL | — | wraps shared Toggle |
| QuietButton | 380-401 | REAL | — | bare/footer variants |

## 5. Test honesty
No tests in this file. ControlBar/panel tests (lane) mount fake callbacks — they prove the presentation only. None assert behavior under host refusal (same blind spot as PlaylistPanel). UNVERIFIED beyond baseline (vitest 1705 passed, orchestrator §13).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the same engine-truth rows the menu carried" (L17) | module header | SHALLOW downstream | rows are engine-truth; selection is refused by the shipped host (F-001) |
| "an arrow select dispatches onSelect and moves FOCUS, the check itself lands only when the engine reports it" (L271-273) | doc comment | REAL as written | and in the product shell the engine never reports it — the honest rule turns refusals into silent non-motion |

## 7. Dead / duplicated / unreachable code
None in-file. (Downstream deadness documented in F-001 — charged to ControlBar/context.tsx owners.)

## 8. Dependencies & flags
components barrel (Icon, Toggle, IconName). No client/transport import — deliberately presentation-only. No feature flags.

## 9. Open questions for the next agent
- ControlBar.tsx owner: does any track-selection UI surface the `unsupported-for-host` refusal (toast/reason row), or is it silent? If silent, the settings panels are dead-button surfaces in the product build.
- Should PanelRadioGroup render PanelReasonRow automatically when every option is disabled (today F-002 makes the group vanish from the tab ring)?
