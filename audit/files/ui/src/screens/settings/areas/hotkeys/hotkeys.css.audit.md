# AUDIT: ui/src/screens/settings/areas/hotkeys/hotkeys.css
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 173 / 9 rule groups (≈20 selectors) |
| File verdict | UNREACHABLE from shipped binaries; REAL as a stylesheet (token-only, all selectors emitted, 2 disclosed specificity disables) |
| Reachable from shipped entry point? | NO at HEAD (host crate unbuildable, W14-A01 E0425). Imported by the hotkeys lane (HotkeysEditor.tsx renders inside the controls-area dialog); compiled + tested in baseline #22/#23. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | class census `rg -l "<class>" src --glob '!*.css'` for 11 class families → all emitted (HotkeysEditor.tsx / controls dialog); `rg -n "data-changed\|data-conflict" HotkeysEditor.tsx` → :112-113 (the state attributes the row rules key on are real); `rg -c "id: \"" generated/bindings.ts` → 150 (vs the sheet's "141-binding" comment); token spot-checks in tokens.css; sed quote verification. |

## 1. One-paragraph verdict
A clean, small sheet: semantic tokens only (the header claim is TRUE here), every selector matches a component emission, the changed/conflict row states key on real data attributes (HotkeysEditor.tsx:112-113), the visually-hidden utility is properly table-scoped after a documented global-collision fix, and the caption-size floor ("no text below 11px", with a measured 4.4:1 contrast rationale for the mode column) shows real a11y discipline. The two `no-descending-specificity` disables carry justification comments. One stale number: the body-scroll comment says "the 141-binding table" while the generated bindings now carry 150 rows — the fix the comment describes (table never grows the page) still holds, but the count drifted. The sheet also skins the surface where my PlayerScreen#part1 F-001 lives: the 150-row hotkeys table advertises bindings (hide-ui Ctrl+\, quit Ctrl+Q, record Shift+R, play-bookmark F1–F10) whose uiActions have no executor.

## 2. What actually works (verified)
— verified by class census: zero orphan selectors.
— verified by `rg -n "data-changed|data-conflict" HotkeysEditor.tsx:112-113`: both row-state hooks are emitted from real editor state.
— verified by token scan: no hex, no color functions; the two disables are `no-descending-specificity` only (no color-no-hex escapes in this sheet).
— executed: baseline #22 (compile), #23 (HotkeysEditor.test.tsx + hotkeysData.test.ts in the 1705-pass set).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/hotkeys/hotkeys.css:66-67 vs ui/src/screens/settings/areas/hotkeys/generated/bindings.ts
Code:
```
/* The table body — the only scroller (the 141-binding table never grows
 * the page: the w26.i15 fix for the 5780px inline slab). */
```
What is wrong: The generated bindings file now contains 150 entries (`rg -c "id: \""` → 150); the comment's count is stale by 9. The structural claim (body is the only scroller) remains true, but the pin rots silently as the keymap grows.
Why it matters: Comment counters that nobody updates are the same drift class as the stale CI-proposal and schema-enum comments found elsewhere this wave; here it also undersells the surface where unimplemented uiAction bindings are advertised (cross-ref my PlayerScreen#part1 F-001: hide-ui/quit/record/play-bookmark render as live rows in this table with no executor).
What correct looks like: drop the number from the comment, or assert it (hotkeysData.test.ts pins structure — a count pin would keep it honest).
Confidence: HIGH — Verification: rg count vs comment.

## 4. Per-function table (rule groups)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dialog.hotkeys-dialog` | 11-20 | REAL | — | 1048×800 frame pins; r14 clipping |
| topbar/title/close (+hover/focus) | 22-64 | REAL | — | 28px hit; token inks |
| `.hotkeys-editor__body` | 68-73 | REAL | S4 | F-001 stale count |
| applybar neutralization | 77-87 | REAL | — | static footer overriding the settings sticky rule — dual selector (with and without the settings-applybar compound) |
| table th/td | 91-107 | REAL | — | collapse + hairlines |
| changed/conflict row states | 109-120 | REAL | — | keyed on real data attributes (HotkeysEditor.tsx:112-113); accent vs accent-strong distinction |
| input base + disabled | 123-136 | REAL | — | 2 disclosed no-descending-specificity disables with reasons |
| conflict/command/mode/reset cells | 138-160 | REAL | — | mode contrast rationale (measured 4.4:1 → secondary, not tertiary) |
| table-scoped visually-hidden | 166-173 | REAL | — | scoped after a documented global-collision fix (w26.k13); clip-path modern form |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| HotkeysEditor.test.tsx (baseline #23) | partial | row rendering/edit/conflict behavior is mutation-sensitive; geometry untested (jsdom) |
| hotkeysData.test.ts / conflicts.ts tests | YES | pin the generated data shape and conflict detection — the table's truth source |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "semantic tokens only (doc 08 §3.2)" | hotkeys.css:1 | REAL | zero hex/color literals in sheet |
| "the 141-binding table never grows the page" | hotkeys.css:66-67 | STALE count / REAL structure | 150 bindings now; body-overflow rule verified |
| "v2 §2.9: no text below 11px" | hotkeys.css:142-143 | REAL | caption token used for the smallest text |

## 7. Dead / duplicated / unreachable code
None in-sheet. Unreachable from shipped binaries (W14-A01). The visually-hidden block duplicates the sr pattern shared with onboarding.css and setting-field.css — disclosed and scoped, acceptable.

## 8. Dependencies & flags
Design tokens (all present), Dialog primitive (`lumen-dialog` compound — verified Surface.tsx:245), settings-applybar override. No flags.

## 9. Open questions for the next agent
- HotkeysEditor auditor: does the editor mark uiAction-only bindings (no command/recipe) any differently from command bindings? If not, the table advertises ~9+ dead keys as live (cross-ref PlayerScreen#part1 F-001).
