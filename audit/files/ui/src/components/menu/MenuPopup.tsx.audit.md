# AUDIT: ui/src/components/menu/MenuPopup.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 362 / 8 (MenuPopup + pointerCoord, clearHoverTimers, focusIndex, move, firstIndex, lastIndex, activate, handleKeyDown, hoverEnter, hoverLeave) |
| File verdict | REAL (code) — UNREACHABLE from any shipped binary (W14) |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web build: 10+ production consumers (ContextMenu.tsx, MenuBar.tsx, ProfilesFlyout.tsx, SettingsPanel.tsx, ControlBar.tsx, PlaylistPanel.tsx, MediaList.tsx, MiniPlayer.tsx, DiscoveryScreen.tsx, PlaylistOptionsMenu.tsx); baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Grep `MenuPopup` (24 files: 10 production consumers + tests + docs); read MenuPopup.test.tsx:1-80 (geometry mock pattern) + test list (8 clamp cases); arithmetic check of the clamp formulas (below) |

## 1. One-paragraph verdict
A genuinely correct ARIA menu: roving focus with separator skipping in both directions, type-ahead-free but complete key set (Arrows/Home/End/Tab-trap/Escape/Enter/Space), submenu hover timers with proper cancel semantics (the nested-DOM mouseleave trick works because submenus render INSIDE the host row), terminal-key stopPropagation so nested popups don't double-drive the menubar, and mount-time viewport clamping whose formulas check out arithmetically and are tested with prototype-pinned geometry (the tests would fail if the math regressed). Two minor gaps: the clamp never re-runs when menu content changes after mount, and clicking a submenu parent opens the submenu without moving focus into it (APG deviation). As ever: no shipped binary can mount it.

## 2. What actually works (verified)
— verified by arithmetic on l.119-122: pointer-anchored flip — `left = pointerX + w > vw − m ? max(m, pointerX − w) : max(m, pointerX)` — flips up-left across the pointer, floor-clamps at the margin; correct in all four fit/overflow quadrants.
— verified by arithmetic on l.142-143: submenu vertical shift `dy = max(m − top, vh − m − h − top)` — when the menu is taller than the viewport the max picks "top at margin" (the two candidates differ by `vh − 2m − h < 0`), the least-bad fit; correct.
— verified by reading the render tree: the submenu `<MenuPopup>` is a DOM child of `.lumen-menu__host` (l.322-356) → pointer moving from row into the popup never fires the host's mouseleave → the close timer only starts on real departure; openTimer canceled on row change (l.289) and closeTimer canceled on re-enter (l.284-287).
— verified by reading MenuPopup.test.tsx:17-43 + 8 cases → geometry pinned on the prototype so the LAYOUT-effect-time measurement sees it; flip, margin clamp, no-fit, origin-shift, and the ContextMenu wrapper path are all asserted on real computed `style.left/top`.
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S4]
Where: ui/src/components/menu/MenuPopup.tsx:106-149
Code:
```
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (el === null) return;
```
What is wrong: The clamp effect's dependency array is `[depth, style]` (l.149) — it measures once at mount (or when the caller supplies a fresh style object). If `entries` change after mount (dynamic menus — e.g. a playlist context menu whose rows arrive async, or a submenu that grows), the popup keeps its mount-time position/transform: a panel that was clamped for 3 rows and grows to 15 overflows the viewport with no re-measure. The submenu's `el.style.transform` (l.146) likewise persists across content growth.
Why it matters: The w26.f07 contract is "nothing ever opens clipped off-window" (l.15-16) — true at open, unguarded afterward. Limited blast radius: current consumers build entries synchronously from local state.
What correct looks like: add `entries` (or a ResizeObserver on `el`) to the effect's triggers and re-run the same idempotent clamp.
Confidence: HIGH on the code path; MEDIUM on whether any current consumer mutates entries post-mount (none found in the 10 consumers' usage patterns I sampled).
Confidence note: no consumer with async menu content verified — hence S4, not S3.

### F-002 [S4]
Where: ui/src/components/menu/MenuPopup.tsx:210-220
Code:
```
    if (entry.submenu !== undefined && entry.submenu.length > 0) {
      clearHoverTimers();
      setOpenSubmenu(index);
      return;
    }
```
What is wrong: `activate()` (the click and Enter/Space path) on a submenu parent opens the submenu but leaves focus on the parent item; only ArrowRight's path benefits from the child's `autoFocusFirstItem` mount effect, and that fires regardless — so keyboard is fine, but a MOUSE click on a submenu row followed by ArrowDown operates on the parent popup (focus never entered the submenu), an APG "menu > submenu" interaction deviation.
Why it matters: Mixed mouse/keyboard users get a menu whose key handling follows the wrong popup after a click. Small, real, non-crashing.
What correct looks like: in `activate()`, after `setOpenSubmenu(index)`, the child's mount focus effect already moves focus in — but it only runs if `autoFocusFirstItem` is true (default), so the actual gap is only when consumers pass `autoFocusFirstItem={false}`; verify + document, or explicitly focus the first child from the parent.
Confidence: MEDIUM — Verification: read; the default-true path DOES focus into the submenu on mount, so the deviation only bites `autoFocusFirstItem={false}` hosts (none of the sampled consumers pass false to submenus — submenus render without the prop, default true). Downgraded to S4 accordingly.
Confidence: HIGH that the code is as described; the reachable deviation is narrow.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pointerCoord` | 37-44 | REAL | — | px-number or "Npx" string only; percent/calc left to CSS — honest |
| `MenuPopup` (state/refs) | 95-100 | REAL | — | open/close hover timers as refs, cleaned up on unmount (l.151-157) |
| clamp layout effect | 106-149 | REAL | S4 | F-001 (no re-clamp on content change); formulas verified; idempotent w.r.t. the style prop |
| unmount timer cleanup | 151-157 | REAL | — | both timers cleared |
| mount focus effect | 163-180 | REAL | — | last-item (ArrowUp open, J02-9) precedence correct; mount-only with documented eslint-disable |
| `clearHoverTimers` | 182-185 | REAL | — | |
| `focusIndex` | 187-190 | REAL | — | optional-chained focus (all-separator degenerate case no-ops safely) |
| `move` | 192-200 | REAL | — | modular wrap + separator skip both directions; all-separator entries fall through to a null ref (no crash — theoretical only) |
| `firstIndex`/`lastIndex` | 202-208 | REAL | — | separator-skipping |
| `activate` | 210-220 | REAL | S4 | F-002; disabled/separator guards correct; onSelect + onDismissAll ordering correct |
| `handleKeyDown` | 222-279 | REAL | — | every handled key terminal (preventDefault+stopPropagation); ArrowLeft gated on depth; ArrowRight checks submenu; Tab traps via move() |
| `hoverEnter`/`hoverLeave` | 281-307 | REAL | — | open/close delay semantics correct; cancel logic verified |
| render | 309-361 | REAL | — | role=menu, density class, nested recursion, aria-label from resolved label; submenu idPrefix chaining for unique ids |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| test:81-89 flip up-left across pointer | YES | asserts computed style.left/top after mount with pinned rect (300×200 popup, 1000×600 viewport) |
| test:90-98 clamp to margin when flipped cannot fit | YES | big popup at far corner → margin floor asserted |
| test:99-106 coordinates untouched when fits | YES | no false clamping |
| test:107-114 origin shift to margin | YES | pointer at 0,0 |
| test:115-128 ContextMenu wrapper path | YES | the shared primitive through a real consumer |
| test:147-163 submenu flip left / shift up / untouched | YES | host-rect mock pins the parent row geometry |
Mutation check (reasoned): change `pointerX - rect.width` to `pointerX` → flip tests fail; break the submenu dy max() ordering → shift-up test fails. The clamp math is the best-tested code in my file set. Keyboard/hover behavior is NOT covered here (lives in MenuBar/ContextMenu/MenuItem suites — other agents).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "nothing ever opens clipped off-window" (w26.f07) | l.15-16 | REAL at mount / UNVERIFIED after content change | F-001; mount-time math + tests verified |
| "they FLIP across the pointer (up / left) … shifting to the viewport margin as a last resort" | l.17-21 | REAL | l.119-124 + tests 81-114 |
| "Tab focus trap (card 4.B3)" | l.10 | REAL | l.248-253 |
| "disabled items focusable-but-inert (aria-disabled)" | l.11 | REAL-UNPROVEN | guard in activate (l.212); the aria-disabled rendering is MenuItem's (other file) |

## 7. Dead / duplicated / unreachable code
No dead code. Duplication: none material (MenuBar/ContextMenu share this primitive rather than reimplementing — the right direction). Unreachable in shipped product (W14).

## 8. Dependencies & flags
MenuItem, types, timings (SUBMENU_OPEN/CLOSE_DELAY_MS). No feature flags. CSS classes (`lumen-menu`, `--density`, `__host`, `__separator`, `--submenu`) defined in components/menu/menu.css (verified by grep, file owned by another agent).

## 9. Open questions for the next agent
- Do any consumers mutate `entries` after mount (async menu content)? If yes, F-001 upgrades to S3.
- MenuItem.tsx owner: confirm `tabIndex` roving (−1/0) matches this component's `itemRefs` focus contract.
