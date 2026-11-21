# AUDIT: ui/src/components/menu/MenuBar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 168 / 5 (MenuBar, focusTrigger, closeMenu, outside-dismiss effect, handleKeyDown) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — `<MenuBar` is mounted ONLY by its own test file (rg across ui/src: zero production JSX sites). The app shell removed the menubar: AppShell.tsx:35 "The Solstice menubar is GONE (no W25 frame carries one)". Barrel-exported (components/menu/index.ts:2-3) but no screen imports it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg "<MenuBar" (only MenuBar.test.tsx); rg "MenuBar" across ui/src (MenuPopup/ContextMenu/ControlBar hits are COMMENTS citing it as a pattern source); sed AppShell.tsx:33-38; MenuPopup stopPropagation check (MenuPopup.tsx:224-229); MenuBar.test.tsx survey:68-250 |

## 1. One-paragraph verdict
A correctly implemented WAI-ARIA menubar — APG keyboard model (ArrowUp opens
on last item, menu switching resets to first, Escape restores focus,
roving tabindex, wrap-around), popup keys terminal via MenuPopup's
stopPropagation, and a 16-test suite that would fail on regression. It is
also dead code: the W25 frame redesign deleted the menubar from the product
(AppShell.tsx:35), and nothing in production mounts this component — the
app menu became ControlBar's AppMenuButton + MenuPopup-at-pointer pattern.
The component survives only through its barrel export and its tests.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): MenuBar.test.tsx
executes labels-via-seam, ARIA states, ArrowDown-first/ArrowUp-last open,
navigation skipping separators, Home/End, Enter activation + focus return,
Escape close + restore, submenu arrows + Escape-scope, hover-intent
submenus, trigger-to-trigger switching, Tab trap, disabled items,
accelerator rendering.
— verified by read: the bubbling hazard (popup is a DOM child of the
menubar container, line 151-162) is real but correctly neutralized —
MenuPopup.tsx:224-229 stops propagation for every handled key, so popup
arrows never drive the menubar handler.
— verified by read: modulo wrap-around (86) is safe for menus.length ≥ 1;
outside-pointer dismiss effect subscribes/cleans correctly per openIndex.

## 3. Findings
### F-001 [S3] Dead component: exported, fully tested, mounted by nothing
Where: ui/src/components/menu/MenuBar.tsx:40-47 (vs AppShell.tsx:35)
Code:
```
export function MenuBar<K extends string>({
  menus,
  resolveLabel = identityResolveLabel,
  density = "comfortable",
  id,
  className = "",
  ref,
}: MenuBarProps<K>): ReactElement {
```
(and AppShell.tsx:35:)
```
// The Solstice menubar is GONE (no W25 frame carries one): navigation
// lives in the rail + breadcrumb; open-media stays shell-owned (the ONE
```
What is wrong: No production JSX mounts MenuBar. rg over ui/src: the only
`<MenuBar` site is MenuBar.test.tsx. ControlBar.tsx:882 references it in a
comment as a possible FUTURE relocation ("f08/AppShell may relocate the
trigger into the shell title bar (MenuBar)") — a maybe, not a caller.
Why it matters: 168 lines of dead UI code plus a test suite keeping it
green — maintenance cost with zero shipped value; the barrel export invites
the false impression that the app has a menubar. Dead code that tests green
is exactly how drift hides.
What correct looks like: Either delete MenuBar (+ its tests + barrel lines)
until the f08 relocation materializes, or mount it somewhere real.
Confidence: HIGH — Verification: rg exhaustively (all hits traced).
### F-002 [S4] Function refs never receive null on unmount
Where: ui/src/components/menu/MenuBar.tsx:121-125
Code:
```
      ref={(node) => {
        rootRef.current = node;
        if (typeof ref === "function") ref(node);
        else if (ref !== null && ref !== undefined) ref.current = node;
      }}
```
What is wrong: React calls this outer callback with null at unmount; the
code sets rootRef.current = null but does NOT forward null to a function
`ref` — user callback refs miss their cleanup notification. (React 19's
ref-cleanup-function return is also not used.)
Why it matters: Consumers doing bookkeeping in callback refs leak an entry;
today no production consumer exists (F-001), so the blast radius is zero.
What correct looks like: `if (typeof ref === "function") { ref(node); return
() => ref(null); }` or forward null in the null branch.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MenuBar | 40-167 | REAL (code) / UNREACHABLE (mount) | S3 (F-001) | APG-correct; ref forwarding S4 |
| focusTrigger | 61-63 | REAL | — | optional-chained |
| closeMenu | 65-68 | REAL | — | close + focus restore |
| outside-dismiss effect | 71-80 | REAL | — | correct subscribe/cleanup per openIndex; focus not stolen (documented) |
| handleKeyDown | 82-117 | REAL | — | all APG branches; preventDefault on handled keys; popup keys stopped upstream (MenuPopup:229) |

## 5. Test honesty (MenuBar.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| labels via seam (74-81) | YES | no hardcoded text |
| ARIA haspopup/expanded (83-91) | YES | state wiring |
| ArrowDown first / ArrowUp last (92-124) | YES | the J02-9 APG pin |
| navigation/Home/End/skip separators (125-144) | YES | real key events |
| Enter activates + closes + refocus (145-153) | YES | end-to-end behavior |
| Escape close/restore; submenu scope (154-191) | YES | nested behavior pinned |
| hover-intent submenus (192-214) | YES | fake timers |
| trigger switching, Tab trap, disabled, accelerators (215-250) | YES | full model |
Mutation check: delete the setOpenFocus("last") in the ArrowUp branch →
test 103 fails. Genuinely strong suite — for a component nothing uses.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| doc 08 §267 overlays: "MenuBar (app menu)" | file header 31 | FALSE (stale) | no app menu menubar exists; AppShell.tsx:35 documents removal |
| "WAI-ARIA menubar keyboard model" | header 32-35 | REAL | code + tests |
| "Portal-free absolute positioning (doc 08 specifies no portals)" | header 37-38 | REAL | popup rendered inline (151-162) |

## 7. Dead / duplicated / unreachable code
The entire component (F-001). Cross-references in ControlBar.tsx:882/905/1058
are comments only.

## 8. Dependencies & flags
MenuPopup, types, menu.css — all live (MenuPopup has real production
consumers). No feature flags.

## 9. Open questions for the next agent
- ControlBar.tsx:882's "f08/AppShell may relocate the trigger into the shell
  title bar (MenuBar)" — is f08 still a live plan? If not, delete this file.
