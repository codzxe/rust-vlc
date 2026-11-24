# DATA AUDIT: design/figma/Settings - controls.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 401 lines, ASCII SVG; 1048x865 canvas (Settings floating-window family) |
| File verdict | REAL (Figma export) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | head/rg; renders ls; consumer rg |

## 1. What it depicts
The Settings floating-window "Mouse, Keyboard & Remote Controls" screen (§C8 of the
tools-settings anatomy spec): 1048x865 family grammar (backdrop-filter foreignObject,
Search, Ellipse 5), the 13 control rows (mouse dropdowns, skip amounts, keyboard
shortcuts action, staged Remotes section). Render twin: design/figma-renders/
Settings_-_controls.png (900-wide family scale), plus the separate control-part sheet
Controls.png (900x5115) for the player control bar — a different artifact, not this frame.

## 2. Consumer / fidelity
- Consumers: w26 ratings h11.md §1 (rates /settings/controls against this frame —
  "All 13 frame rows render with frame-verbatim labels and values" after g13/gfix-B
  fixes, with measured vertical drift: rows pitch 46 vs frame 48, Mouse block 8-10px
  high, Remotes section 42-47px above frame position); state-map w26.g13.md; findings
  w26.e08.md; logs w26.f02.md; analysis tools-settings.md §C8-C10/§D2.
- The shipped screen exists and renders the frame's rows (implemented with measured
  px-level drift — the h11 rating is the drift record).
- Fidelity: clean vector export; the frame's honest-empty pattern for Remotes is
  reproduced as staged-inert UI downstream.

## 3. Verdict
REAL asset with an implemented consumer; downstream drift is measured and documented in
the w26 rating lane, not hidden.
