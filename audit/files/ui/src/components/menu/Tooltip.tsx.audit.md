# AUDIT: ui/src/components/menu/Tooltip.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 127 / 1 (Tooltip + TooltipProps) |
| File verdict | UNREACHABLE (correct, well-tested component with ZERO production consumers — dead code plus 10 passing tests for it) |
| Reachable from shipped entry point? | NO on both levels: (a) no GUI binary at HEAD (ui-wide); (b) NO SPA-internal consumer either — `rg -rln "Tooltip" ui/` (excluding its own .tsx/.test.tsx/node_modules) finds ONLY: states.md (doc), timings.ts (its delay constants), menu/index.ts (barrel re-export), menu.css (its styles), SettingField.tsx:114 (a COMMENT citing "k02's Tooltip pattern" — not an import). No screen, component, or e2e file mounts `<Tooltip>`. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read complete; exhaustive consumer search (`rg -rln "Tooltip" ui/` with exclusions; `rg -n "<Tooltip"` ui/src -g '!*.test.*' → zero hits; e2e checked → zero); timings.ts read (500ms show / 150ms hide [PROPOSAL — states.md]); menu.css:515-542 read (absolute-positioned bubble, relative wrapper); Tooltip.test.tsx read head (10 tests, fake timers). |

## 1. One-paragraph verdict
A correctly built tooltip primitive — hover-intent timers with proper cancel-on-reenter, immediate keyboard show, describedby merged onto the child trigger (the j02 J02-1 fix is real and well-reasoned), Escape consumed only while open — that NOTHING in the shipped UI renders. Ten tests pass in the baseline suite proving a component no screen mounts; the barrel exports it, the design doc lists it (doc 08 §259 primitives), the CSS styles it, and the only "consumer" reference in the tree is a comment in SettingField.tsx that borrows its pattern. Dead code with test theater.

## 2. What actually works (verified)
— Timer state machine traced: enter cancels pending hide then restarts show; leave cancels pending show; early-leave before show fires nothing (`if (!open) return` :93); unmount clears both timers (:52).
— describedby composition: child's existing aria-describedby merged while open, preserved untouched when closed (:62-72) — the comment's SR rationale (focus lands on the child, not the wrapper) is correct APG reasoning.
— Escape consumed + stopPropagation only when open (:102-110) — the dialog-host fix (J02-13).
— 10/10 tests pass (baseline run #23) — delays asserted against the shared constants, keyboard/focus paths covered.

## 3. Findings

### F-001 [S3]
Where: ui/src/components/menu/Tooltip.tsx:31 (component) with ui/src/components/menu/index.ts:8 (export) and Tooltip.test.tsx (10 tests)
Code:
```
export function Tooltip({
  content,
  children,
```
What is wrong: The component has zero production consumers. Every reference in the tree is self-referential (its tests, its barrel export, its CSS block, its doc entry, its timing constants) or a comment (SettingField.tsx:114 "k02's Tooltip pattern"). Ten unit tests exercise it faithfully — tests for a subject no user can ever reach.
Why it matters: Dead primitive presented as shipped infrastructure (doc 08 §259 lists it among the primitives; states.md documents its states; the barrel exports it for anyone to assume it's live). It also carries maintenance cost with zero user value — the exact "tested dead code" pattern this audit hunts. (If a consumer lands later the component itself is fine — the finding is the current dead state, not the implementation.)
What correct looks like: Either mount it where the design frames put tooltips (the frames' row tooltips the settings/info lanes reference) or delete it until a consumer exists (with its tests, CSS block, and timing constants).
Confidence: HIGH — Verification: exhaustive rg across ui/ (src + e2e, production + tests separated).
(Severity S3, not S4, because the design docs present it as a shipped primitive and it inflates the tested-surface numbers.)

### F-002 [S4]
Where: ui/src/components/menu/Tooltip.tsx:100-101
Code:
```
      onFocus={() => setOpen(true)}
      onBlur={hide}
```
What is wrong: React's onBlur behaves like focusout (bubbles): if the trigger element ever contains MULTIPLE focusables, moving focus between them fires blur→hide (immediate setOpen(false)) then focus→show — the bubble unmounts and remounts, flickering. The standard guard (ignore blur when relatedTarget stays inside the wrapper) is absent.
Why it matters: Latent — doubly so, since the component has no consumers at all (F-001); a single-button trigger never triggers it.
What correct looks like: `onBlur={(e) => { if (!e.currentTarget.contains(e.relatedTarget)) hide(); }}`.
Confidence: HIGH — Verification: read; React focus-event semantics.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TooltipProps` | 6-17 | REAL | — | Documented delays/side; content i18n at caller. |
| `Tooltip` | 31-127 | REAL (implementation) / UNREACHABLE (mount) | F-001, F-002 | Timer machine correct; describedby merge correct; Escape semantics correct; no portal (absolute bubble inside wrapper — clips in overflow-hidden ancestors; another latent concern for any future consumer inside VirtualList rows). |

## 5. Test honesty (Tooltip.test.tsx, 10 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "shows the bubble after the show delay (hover intent)" | YES | Fake-timer boundary asserted (delay−1 → absent, delay → present). |
| Keyboard/focus/describedby/Escape tests (per header) | YES | Behavior assertions on the trigger's attributes and bubble role. |
Irony recorded: all 10 tests are honest ABOUT THE SUBJECT — the subject itself is dead (F-001). They would all still pass if every screen in the app were deleted.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Tooltip (doc 08 §259 primitives list)" | :20 | PAPER-ONLY as shipped | Component exists and works; no shipped surface renders it (F-001). |
| states.md menu-family list including Tooltip | states.md | PAPER-ONLY (same) | Same evidence. |
| "keyboard focus shows immediately, WAI-ARIA APG: never delay for keyboard users" | timings.ts:13-14 | REAL | Code :100 (onFocus → setOpen(true), no timer). |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable (F-001). Its CSS block (menu.css:517-542) and timing constants (timings.ts:23-26) are dead weight alongside it.

## 8. Dependencies & flags
No external deps beyond React; menu.css + timings.ts (both co-dead with it). No feature flags.

## 9. Open questions for the next agent
- Is there a design frame that REQUIRES tooltips (the settings ⓘ lane borrowed "k02's Tooltip pattern" — SettingField.tsx:114)? If yes, why did that lane re-implement the pattern instead of mounting this component?
- MenuBar/Dropdown owners (A10 has MenuBar): do their frames' row tooltips exist? If the menu family never uses tooltips, the component's home crate should shed it.
