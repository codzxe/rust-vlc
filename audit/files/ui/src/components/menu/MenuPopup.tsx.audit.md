# AUDIT: ui/src/components/menu/MenuPopup.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 363 / 1 component + 12 inner fns/helpers |
| File verdict | REAL (keyboard-complete ARIA menu popup with measured viewport clamping; one unreachable-in-practice off-by-one) |
| Reachable from shipped entry point? | YES — mounted by MenuBar.tsx (app menubar), ContextMenu.tsx (right-click), ProfilesFlyout.tsx, Surface.tsx; those mount from AppShell/PlayerScreen → App → main.tsx. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg callers (4 hosts + tests); cat timings.ts (150/100ms constants exist); baseline ui-test.txt:107 `✓ src/components/menu/MenuPopup.test.tsx (8 tests) 204ms`, :59 MenuBar 19 tests |

## 1. One-paragraph verdict
Real, accessibility-serious menu popup: roving focus with separator skipping, ArrowRight/Left submenu semantics with depth awareness, Escape with close-to-parent, Tab trap, hover-intent timers (150ms open / 100ms close, constants in timings.ts — verified), and a useLayoutEffect viewport clamp that flips pointer-anchored popups across the pointer and submenus left/up before paint. Timers are cleaned up on unmount; submenu popups render inside their host row so pointer travel into them does not trip the close timer. Two minor notes: an ArrowUp-from-no-focus off-by-one (lands on n-2, not last) that shipped hosts cannot reach because autofocus always sets an index, and top-level CSS-anchored (menubar) popups get no clamping at all.

## 2. What actually works (verified)
— verified by rg: hosts = MenuBar.tsx, ContextMenu.tsx, ProfilesFlyout.tsx, Surface.tsx; `autoFocusLastItem` wired from MenuBar.tsx:159 (`openFocus === "last"`), matching the APG ArrowUp-open behavior the comment claims.
— verified by cat: `SUBMENU_OPEN_DELAY_MS = 150`, `SUBMENU_CLOSE_DELAY_MS = 100` exist in timings.ts.
— verified by baseline: MenuPopup 8 tests, MenuBar 19 tests, MenuItem 4 tests — all PASS (ui-test.txt:107, :59, :154).

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/MenuPopup.tsx:192-200
Code:
```
  const move = (delta: number): void => {
    if (entries.length === 0) return;
    let index = activeIndex;
    for (let step = 0; step < entries.length; step += 1) {
      index = (index + delta + entries.length) % entries.length;
```
What is wrong: With `activeIndex === -1` (no item focused) and ArrowUp, `(−1−1+n)%n = n−2` — focus lands on the second-to-last item instead of the last. Unreachable in shipped hosts (autofocus on mount always sets an index; no host passes `autoFocusFirstItem={false}` — rg verified). What correct looks like: `index = activeIndex < 0 && delta < 0 ? entries.length : activeIndex`. Confidence: HIGH on arithmetic — Verification: read; reachability: rg.
F-002 [S4]
Where: ui/src/components/menu/MenuPopup.tsx:106-149 — the clamp handles `depth === 0 && pointer coords` and `depth > 0` (any anchor). A top-level CSS-anchored popup (menubar drop-down, depth 0, no px style) gets no clamping; a long menu near the viewport bottom would clip. Menubar sits at the window top, so blast radius ~zero today; the doc comment (L14-24) is accurate about this split. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| pointerCoord | 37-44 | REAL | — | px-number/string parse, percent/calc → null (deliberate) |
| MenuPopup (component) | 81-362 | REAL | — | |
| ↳ useLayoutEffect clamp | 106-149 | REAL | S4 | F-002 top-level CSS-anchored unclamped |
| ↳ unmount timer cleanup | 151-157 | REAL | — | both timers cleared |
| ↳ autofocus effect | 163-180 | REAL | — | last-item path for APG ArrowUp-open; eslint-disable scoped + justified |
| ↳ clearHoverTimers/focusIndex | 182-190 | REAL | — | |
| ↳ move | 192-200 | REAL | S4 | F-001 off-by-one from -1 |
| ↳ firstIndex/lastIndex | 202-208 | REAL | — | separator-skipping |
| ↳ activate | 210-220 | REAL | — | submenu opens w/o dismiss; leaf fires onSelect then dismissAll |
| ↳ handleKeyDown | 222-279 | REAL | — | all keys terminal (preventDefault+stopPropagation); documented why |
| ↳ hoverEnter/hoverLeave | 281-307 | REAL | — | hover-intent with cancel-on-reenter; submenu nested in host row keeps close from arming |
| ↳ render | 309-361 | REAL | — | role=menu, aria-label, recursive submenu with focus restore |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| MenuPopup.test.tsx (8, baseline PASS) | YES | keyboard nav + submenu semantics asserted with real timers advanced (timings.ts imported by tests per its header) |
| MenuBar.test.tsx (19) | YES | exercises this component through its primary host incl. focus restore |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every popup measures itself on mount … and repositions before paint" | MenuPopup.tsx:14-16 | TRUE | useLayoutEffect L106; idempotent via style-prop origin (L104-105) |
| "hover for submenus" with delays | MenuPopup.tsx:9 | TRUE | timings.ts 150/100ms (cat) |

## 7. Dead / duplicated / unreachable code
None. All props consumed by real hosts (autoFocusLastItem ← MenuBar; density ← hosts; style ← ContextMenu pointer coords).

## 8. Dependencies & flags
MenuItem, types, timings. No flags.

## 9. Open questions for the next agent
- ContextMenu.tsx (not mine): confirm it passes pointer px coords so the depth-0 clamp branch actually runs in production.
