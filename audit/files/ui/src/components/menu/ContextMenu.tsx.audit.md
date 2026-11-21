# AUDIT: ui/src/components/menu/ContextMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 100 / 1 component (+1 interface) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — 6 production consumers (PlayerScreen.tsx, MiniPlayer.tsx, PlaylistScreen.tsx, PlaylistPanel.tsx, MediaList.tsx, DiscoveryScreen.tsx — the canvas/right-click and file-row context menus). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep clamp/flip in MenuPopup.tsx + menu.css (viewport clamping IS handled downstream — w26.f07); grep -rln ContextMenu consumers (6 production files). |

## 1. One-paragraph verdict
A correct right-click menu host: pointer-coord positioning with the viewport clamping delegated to MenuPopup (verified present, w26.f07), full keyboard model inside the popup, per-instance id prefix (the documented j02 duplicate-id fix), ref forwarding composed with the internal root ref, and focus capture/restore on open/Escape. One inconsistency: the outside-pointer dismiss path closes without restoring focus, while Escape does — harmless while focus never entered the menu, but a focus loss if the user opened the menu, tabbed into it, then clicked outside.

## 2. What actually works (verified)
— MenuPopup clamping verified (MenuPopup.tsx:14-31,102-131) — raw pointer coords are safe. Consumers real. Baseline: menu suites + the 6 consumer suites pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/menu/ContextMenu.tsx:55-58 vs 64-67
Code:
```
      if (rootRef.current !== null && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
```
```
  const closeAndRestore = (): void => {
    setOpen(false);
    previousFocus.current?.focus();
  };
```
What is wrong: two close paths with different focus behavior. Escape/submenu-dismiss call closeAndRestore (focus returns to the pre-open element); the outside pointerdown path calls bare setOpen(false). If the user opened the menu, moved focus INTO it (arrow keys), then clicked outside, the popup unmounts with focus inside it → focus drops to <body>; previousFocus is not restored.
Why it matters: keyboard+pointer mixed users lose their place; the "focus is not stolen" comment (line 52) is true at OPEN time, not at this close path.
What correct looks like: outside dismiss should also restore when the popup (not the outside target) holds focus.
Confidence: MEDIUM — DOM focus semantics traced, not executed in jsdom.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ContextMenuProps` (interface) | 8-23 | REAL | — | generic K, React 19 ref-as-prop |
| `ContextMenu` | 33-100 | REAL | S4 (F-001) | id prefix per instance; ref compose; contextmenu handler captures focus + coords |
| — outside-dismiss effect (inner) | 53-62 | REAL | S4 (F-001) | listener lifecycle correct (open-gated, cleanup) |

## 5. Test honesty
ContextMenu tests live in the menu suite (another agent). Mutation: remove the outside listener → outside-dismiss test fails; remove closeAndRestore → Escape-focus test fails. The F-001 mixed-path likely untested (would need focus-in-popup + outside click).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "right-click opens a role=menu popup at the pointer coordinates (fixed positioning, portal-free)" | ContextMenu.tsx:26-31 | REAL | handler 77-82; MenuPopup fixed |
| "Escape closes and restores focus to the previously-focused element; outside pointer dismisses" | ContextMenu.tsx:30-31 | REAL (Escape) / PARTIAL (outside: dismisses without restore — F-001) | 64-67 vs 55-58 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
MenuPopup, menu.css, types (identityResolveLabel). All exist.

## 9. Open questions for the next agent
- MenuPopup.tsx owner: does the clamping measure run on WINDOW RESIZE while open (a menu opened near the edge, then the window shrinks)? Only mount-measure was visible in the grep.
- Does any consumer rely on the outside-dismiss-not-restoring behavior (e.g. MediaList keeping selection focus)?
