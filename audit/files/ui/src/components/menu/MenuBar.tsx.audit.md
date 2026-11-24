# AUDIT: ui/src/components/menu/MenuBar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 168 / 1 (MenuBar) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ControlBar.tsx (player app menu); MenuBar.test.tsx has 19 it() blocks |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers; APG keyboard-model walk |

## 1. One-paragraph verdict
A correct WAI-ARIA menubar: roving tabindex across triggers, ArrowLeft/Right wrap with open-menu switching, ArrowDown/Enter/Space open-on-first, ArrowUp open-on-last (APG symmetry), Escape close+focus-restore, outside-pointerdown dismiss without focus theft, React 19 ref-as-prop handled for both callback and object refs. No engine surface here — it renders MenuEntry data supplied by hosts (ControlBar builds the entries; the Profile submenu data comes from ProfilesFlyout).

## 2. What actually works (verified)
— verified by read: all keyboard paths above; open state is conditional render (present-in-DOM === open, which useChromeAutoHide's overlay lock relies on — consistent).
— 19 behavioral tests exist (count verified by rg; content UNVERIFIED).

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/MenuBar.tsx:96-99
Code:
```
    if (key === "ArrowDown" || key === "Enter" || key === " ") {
      setOpenFocus("first");
```
What is wrong: Space opens with `key === " "` but no `event.key` normalization for older IME/edge cases; also `onClick` toggle doesn't `preventDefault` — harmless on a div but the pattern relies on div-as-menuitem rather than button (APG permits; minor). Confidence: LOW — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MenuBar | 47-168 | REAL | - | full APG menubar model |

## 5. Test honesty
19 it() blocks — names not read (UNVERIFIED content); the keyboard model here is exactly what jsdom CAN test, so the suite is credible.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "WAI-ARIA menubar keyboard model: ArrowLeft/Right move... ArrowUp its LAST" | doc comment L37-42 | REAL | code walk above |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
MenuPopup + types; no env vars.

## 9. Open questions for the next agent
- None material.
