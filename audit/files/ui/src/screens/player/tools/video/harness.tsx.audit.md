# AUDIT: ui/src/screens/player/tools/video/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 83 / 5 (CardFrameStates + 4 harness mounts) |
| File verdict | REAL (dev-only vision harness — never on app routes) |
| Reachable from shipped entry point? | NO by design — sole consumer harness.test.tsx (rg) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers (harness.test.tsx only) |

## 1. One-paragraph verdict
Vision harness mounting the four REAL video tool components (Colors, Crop,
Essential, HDR) in their frame states for the screenshot comparison loop.
Same pattern as the subtitles harness: production components, no
re-implementation, no app-route leakage. Note it mounts the staged tools —
so the vision loop compares staged cards against frames, which is exactly
what the frames show (unwired controls). No findings.

## 2. What actually works (verified)
- Sole importer: harness.test.tsx — "never imported by app routes" holds.
- Mounts the production tools (imports 14-17), collapsed + expanded states
  per frame.

## 3. Findings
None. Checked: production leakage (none), duplication (shared
CardFrameStates helper), copy discipline (captions are frame names, prop
pass-through noted for the lint rule).

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| CardFrameStates | 22-34 | REAL | shared stage helper |
| ColorsHarness/CropHarness/EssentialHarness | 37-64 | REAL | stacked states |
| HdrHarness | 71-83 | REAL | list opened via public Dropdown API in the test |

## 5. Test honesty
harness.test.tsx renders + screenshots; the HDR open-state uses the public
Dropdown API (documented 66-68) rather than internal poking — honest
harnessing.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" | harness.tsx:1 | REAL | rg: sole consumer is harness.test.tsx |

## 7. Dead / duplicated / unreachable code
None within its dev-only purpose.

## 8. Dependencies & flags
Production tool components + harness.css. None.

## 9. Open questions for the next agent
- None.
