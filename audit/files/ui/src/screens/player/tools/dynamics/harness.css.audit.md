# AUDIT: ui/src/screens/player/tools/dynamics/harness.css (generated/dev-asset-adjacent — short audit)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 45 (4 class rules: w26c2-stage/cell/caption/frame) |
| File verdict | REAL (dev-only vision-harness styles, correctly scoped) |
| Reachable from shipped entry point? | NO (by design) — imported only by dynamics/harness.tsx; the harness lane is the visual-harness entry (main.tsx:46 query-param override point, "production builds ignore" per App.tsx:153), never an app route |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln harness.css ui/src` -> only the sibling harness.tsx files (visualizer/video/dynamics pattern); `rg harness ui/src/App.tsx ui/src/main.tsx` -> dev-shell references only |

## 1. Blunt verdict
Does what the header says: a 400px-cell presentation stage for frame
comparison, dev-only, consumed exclusively by the colocated harness
component. All values are either semantic tokens (--color-surface-deep,
--font-family-sans, --font-size-caption, --color-content-secondary — all
exist in tokens.css, verified in that audit) or documented frame
dimensions. The one honesty nicety that survives checking: the stage
background rationale (:14-18) explains WHY the canvas sits between surface
and panel (so both card states stay evaluable) — real reasoning, not
decoration. No app-route import exists (rg).

## 2. Findings
None. Checked: token existence (all present), dead selectors (all four
classes used by harness.tsx — sibling file), production leakage (none
found; UNVERIFIED beyond rg — bundle not inspected).

## 3. Cross-references
- The harness components (dynamics/harness.tsx) are the dev-shell surface
  another agent should audit for the "mock data presented as real"
  question — this file is only its styling.
