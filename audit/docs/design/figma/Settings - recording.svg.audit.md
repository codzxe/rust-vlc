# DATA AUDIT: design/figma/Settings - recording.svg
| Field | Value |
|---|---|
| Auditor | W19-A10 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 409 lines, ASCII SVG; 1048x865 canvas (Settings floating-window family) |
| File verdict | REAL (Figma export) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (family bloat class) |
| Commands run | head/rg; render dims; consumer chain checks |

## 1. What it depicts
The Settings floating-window "Recording" screen — "Recording, Casting & Streaming Out"
per its shipped consumer's comment. Same window grammar as the family: 1048x865,
backdrop-filter:blur foreignObject, ids "Settings - recording"/"Search"/"Ellipse 5".
Render twin: design/figma-renders/Settings_-_recording.png (900x743, IHDR-measured).

## 2. Consumer / fidelity
- Consumers: w26 ratings h12.md:24/:67 (scored the screen, found the pre-fix
  0/10-rows apology state), w26.i15 closing fix (staged-inert pattern), logs b01/b02/
  i12/i13/i14, analysis tools-settings.md §C12.
- Shipped consumer verified: ui/src/screens/settings/areas/recording.tsx (159 lines)
  renders the frame's 10 rows visible-but-disabled, gated by product.toml:164
  `recording = false  # Phase 7` (rule F2: no phantom features, no dead controls that
  look alive). The SVG's rows are therefore reproduced as staged UI, not implemented
  recording functionality.
- Fidelity: clean vector export consistent with the 1048x865 family grammar.

## 3. Findings
F-001 [S4] The asset's functional plane is PAPER-ONLY by design: no recording feature
exists behind the frame (product.toml Phase-7 gate false); the design-to-code chain is
honest about it (feature-gated, tooltip-marked). Confidence: HIGH — Verification: read.

## 4. Verdict
REAL design asset; its consumer is an honest staged-inert renderer, not a hidden stub.
