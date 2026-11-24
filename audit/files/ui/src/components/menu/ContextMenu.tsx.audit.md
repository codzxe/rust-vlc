# AUDIT: ui/src/components/menu/ContextMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 100 / 1 (ContextMenu) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — DiscoveryScreen, PlaylistPanel/PlaylistScreen, MediaList (routed screens) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln "ContextMenu"` (3+ production consumers); `rg -c "it\(" ContextMenu.test.tsx` (7) |

## 1. One-paragraph verdict
Real right-click menu host: pointer-positioned popup, focus restore on close, outside-pointer dismiss, per-instance id prefix (duplicate-id fix cited), React 19 ref-as-prop forwarding. The only nit: the popup is placed at raw `clientX/clientY` with no viewport clamping — near the right/bottom screen edge the 265×296 popup can overflow the viewport (CSS may or may not clamp; not visible in this file).

## 2. What actually works (verified)
— verified by consumer sweep + ContextMenu.test.tsx (7 cases; baseline vitest 1705 passed).

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/ContextMenu.tsx:92
Code:
```
          style={{ left: position.x, top: position.y }}
```
What is wrong: no viewport-edge clamping/flip at this layer. If menu.css does not clamp, a right-click near the bottom-right corner renders the popup off-screen (content unreachable). UNVERIFIED whether menu.css clamps (max-width/position strategy not checked — the popup is `position: fixed` per docstring). Confidence: LOW-MEDIUM — Verification: read; CSS clamp not verified.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ContextMenu | 33-100 | REAL | S4 | F-001 |

## 5. Test honesty
7 tests (open/close/restore/outside-dismiss patterns; would fail on regressions).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "right-click opens a `role=\"menu\"` popup at the pointer coordinates (fixed positioning, portal-free)" (L28-30) | docstring | REAL | L77-96 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
MenuPopup + types. No backend coupling (entries supplied by hosts).

## 9. Open questions for the next agent
- Verify menu.css `.lumen-context-menu__popup` clamping behavior (F-001) — visual/browser check needed.
