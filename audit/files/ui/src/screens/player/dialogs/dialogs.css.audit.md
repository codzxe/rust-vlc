# AUDIT: ui/src/screens/player/dialogs/dialogs.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 544 / 38 rule blocks (+1 @media) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle: dialogs are mounted from PlayerScreen (rg: GotoTimeDialog/BookmarksDialog/ToolsDialog imported via screens/player/dialogs/index.ts → PlayerScreen.tsx); importers of this sheet: GotoTimeDialog.tsx:43, BookmarksDialog.tsx:52, ToolsDialog.tsx:42, ToolsDialogShell.tsx:43. NO shipped binary mounts it (W14: desktop-tauri E0425 at HEAD — no lumen-tauri binary exists). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | rg class census (35 hooks × TSX — all used); token value verification in tokens/generated/tokens.css; DOM-order verification (sed GotoTimeDialog/ToolsDialogShell/BookmarksDialog); hex scan (comments only); light-theme Δ math check (#c2c2c2 vs #c6c6c6 = Δ4; #bababa vs #c6c6c6 = Δ12). |

## 1. One-paragraph verdict
Styling sheet for the player's Go-to-Time / Bookmarks dialogs and the Audio/Video tools shell + card skeleton. Every selector is consumed by reachable TSX (35/35 census), every declaration is token-based (the only raw px are structural sizes with frame citations; the only hex strings live in comments), and the two non-obvious engineering claims were verified against the markup: the `order: -1` focus-order trick really is matched by body-before-header DOM order in all three hosts (GotoTimeDialog.tsx:99/:129, BookmarksDialog.tsx:210/:286, ToolsDialogShell.tsx:96/:100), and the `:has(> .player-dialog--tools)` 400px override really matches a direct child (ToolsDialogShell.tsx:88-91). The dialog submit paths are wired to the real player-controls seam (`actions.seek` at GotoTimeDialog.tsx:84), not dead buttons. One S4: a comment calls the value-chip ink "mono" while the applied `--font-family-time` resolves to the plain Inter stack — identical to `--font-family-sans` (tokens.css:64-65) — so nothing mono exists; tabular alignment comes solely from `font-variant-numeric`.

## 2. What actually works (verified)
- Class census: `rg -l` per hook → all 35 hooks used by the four dialog TSX hosts (+ `eq-tool--open` cross-file hook from EqualizerTool, staged-note rule :378-380).
- Token integrity: all `var()` names resolve in ui/src/tokens/generated/tokens.css; verified values behind the claims: `--font-size-section: 15px` (:72 — the "15px step" claim), `--radius-thumbnail: 8px` (:102 — the r8 card pin), `--radius-window: 14px` (:103), `--color-record: #e15032` (:51), `--color-surface-deep` #0e0e0e/#bababa (:56/:132 — the light-well Δ math checks out).
- Raw-hex scan: zero hex values in declarations; all hex in comments are frame citations.
- Focus-order contract: body-before-header DOM order confirmed in all three hosts; `order: -1` on `.player-dialog__header` (:76-77) paints it first — the comment and code agree.
- Backend wiring (the wave's hostile question): the dialogs this sheet styles send real commands through the shared seam — `actions.seek(seconds)` (GotoTimeDialog.tsx:84, the same funnel as the seekbar), Bookmarks add/delete at BookmarksDialog.tsx:215-266; the sheet's own `.player-dialog__error` (:207-211) is the honest inline failure surface for those sends (BookmarksDialog.tsx:232, role="alert").
- Reduced-motion block (:538-544) guards only transitions this sheet itself declares (:238, :410, :430) — not a dead duplicate (contrast playlist.css F-003).
- Baseline: vite build compiles the sheet (INDEX.md row 22); vitest dialog suites passed (row 23: GotoTimeDialog.test.tsx, BookmarksDialog.test.tsx, ToolsDialog*.test.tsx assert on these class hooks).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/dialogs/dialogs.css:499-500 (applied at :501-521, esp. :510)
Code:
```
/* ToolsValueChip — the dark read-out at a row's right end (frame: EQ /
 * Compressor rows' 138×34 rx5 #090909 value fields, mono ink). */
```
What is wrong: "mono ink" — the chip renders `font-family: var(--font-family-time)` (:510), and `--font-family-time` is `"Inter", "-apple-system", "Segoe UI", "Roboto", sans-serif` (tokens/generated/tokens.css:65), byte-identical to `--font-family-sans` (:64). Inter is a proportional face; nothing monospaced renders here. Same false belief at GotoTimeDialog.tsx:20-21 ("tabular mono input" — A04's file). The design-token source is explicit that the time face is the SAME stack ("timecode face — same UI stack; apply font-variant-numeric: tabular-nums", themes/tokens.json:376).
Why it matters: Comments are the only traceability to the frames; "mono" misleads any future auditor or dev into believing a monospace face (or a distinct font) is at work, and hides that `--font-family-time` is currently a no-op alias of the sans stack (every use could be `-sans` with zero visual change).
What correct looks like: Comment should say "tabular ink (Inter + font-variant-numeric: tabular-nums; the time face IS the UI stack by W25 §1)".
Confidence: HIGH on the fact (token values read verbatim) — Verification: read; MEDIUM that the authors meant "monospace" (ambiguous prose).

## 4. Per-function table (rule blocks)
| Rule(s) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dialog.player-dialog__surface` | 38-43 | REAL | — | width min(440,92vw)+max-width 92vw (redundant second cap, harmless); overflow:hidden pins header/footer while body scrolls |
| `:has(> .player-dialog--tools)` 400px | 55-57 | REAL | — | Direct-child match verified (ToolsDialogShell.tsx:88-91); :has() modern-browser dependency disclosed |
| `.player-dialog` | 59-67 | REAL | — | Used by all hosts |
| `.player-dialog__header` (+light hold) | 76-98 | REAL | — | order:-1 focus trick verified against DOM order; light re-recess Δ math verified (#c2c2c2→#bababa on #c6c6c6) |
| `.player-dialog__title` | 100-116 | REAL | — | 15px semibold via --font-size-section (verified 15px); ellipsis guards overflow |
| `.player-dialog__close` / `__body` / `__subtitle` | 118-139 | REAL | — | Hooks at GotoTime:136/:99/:100 |
| `.player-dialog__form` row flip (+label/control/error) | 147-171 | REAL | — | flex-flow:row wrap resets the primitive's column (comment explains the height-basis failure mode); error wraps to own line (flex-basis:100%) |
| `--bookmarks` form + `__form-action` | 174-182 | REAL | — | BookmarksDialog.tsx:215-226 |
| `__hint` / `__empty` / `__error` | 186-211 | REAL | — | Engine-truth hint (GotoTime:124-127); error = inline command-failure surface (Bookmarks:232) |
| Bookmarks list `__list`/`__row`/`__row-time`/`__row-label` | 216-257 | REAL | — | 6px/8px padding ≈40px band (comment math checks: 8×0.75=6px); overflow-wrap:anywhere |
| `__row-main` / `__row-delete` (+focus/hover reveal) | 260-286 | REAL | — | Full-row quiet jump button; delete at --opacity-muted 0.3 restored on focus-visible/hover (keyboard-safe) |
| `__footer` (+min-width 100px, only-child right) | 294-311 | REAL | — | Frame 100×32 grammar; only-child margin-left:auto keeps action side |
| Tools shell `__cards`/`__tabs`/`__tablist`/`__panel` (+[hidden]) | 315-356 | REAL | — | display:none beats flex for hidden panels; tab radius scoped to r6 (no pill tabs in any frame — verified claim style) |
| `__staged-tool`/`__staged` (+ `:has(.eq-tool--open)`) | 362-380 | REAL | — | EQ staged note shown only when expanded — the "honest-unbound" EQ seam (cross-ref W8: audio.set-eq nonexistent) |
| `tools-card__toggle` (+hover/active/focus/chevron/aria-expanded) | 388-437 | REAL | — | 360×48 frame bar; chevron rotate on aria-expanded; focus-visible outline present |
| `tools-card__body` (+[hidden]) | 439-461 | REAL | — | surface-raised + r8 pin both disclosed as logged deltas vs frame (#1B1B1B rx6) |
| `tools-row` family | 465-497 | REAL | — | Label-left/control-right grammar shared by all tool cards |
| `tools-row__value` (+light hold) | 501-534 | REAL | — | F-001 comment; light well verified |
| `@media (prefers-reduced-motion)` | 538-544 | REAL | — | Guards only this sheet's own transitions — not redundant |

## 5. Test honesty
No CSS tests. Indirect: GotoTimeDialog.test.tsx / BookmarksDialog.test.tsx / ToolsDialog.test.tsx / ToolsDialogShell.test.tsx assert roles and class hooks this sheet styles (jsdom — no computed styles). harness.test.tsx renders the mounted ToolsDialog with both tabs (smoke: `innerHTML.length > 0` + 2 tabs) — crash-only proof; pixel fidelity UNVERIFIED (visual harness not in baseline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Escape + scrim-click close" owned by surface primitive | dialogs.css:3-4, 12-14 | cross-ref | surface.css (my file 7) — verified there |
| "title 15px semibold" | dialogs.css:17, 108-112 | REAL | --font-size-section: 15px (tokens.css:72), --font-weight-semibold: 600 (:80) |
| "Semantic tokens only; raw px solely structural" | dialogs.css:25-27 | REAL | hex scan: declarations contain zero hex; raw px all carry frame citations |
| "mono ink" value chip | dialogs.css:500 | FALSE (comment) | F-001 — --font-family-time ≡ --font-family-sans ≡ Inter stack |
| r8 card "frame rect reads rx6 — 2px delta, logged" | dialogs.css:455-456 | REAL (disclosed drift) | --radius-thumbnail: 8px; deviation documented, not hidden |
| Body-before-header DOM + order:-1 | dialogs.css:72-77 | REAL | GotoTimeDialog.tsx:99/:129; BookmarksDialog.tsx:210/:286; ToolsDialogShell.tsx:96/:100 |

## 7. Dead / duplicated / unreachable code
None. All 35 hooks consumed; the reduced-motion block is not a duplicate (guards own transitions only).

## 8. Dependencies & flags
tokens/generated/tokens.css (all vars resolve), surface.css (.lumen-dialog/.lumen-scrim chrome), button.css (secondary/tinted footer grammar), form.css (.lumen-field row flip), list.css (.lumen-tabs), equalizer.css (.eq-tool--open :has hook). No feature flags. `:has()` requires a modern webview (Tauri webkit2gtk / vite targets — fine in dev browsers; UNVERIFIED on the shipped webview since no GUI binary exists).

## 9. Open questions for the next agent
- GotoTimeDialog.tsx:20 "tabular mono input" (A04's file) — same "mono" falsehood as F-001.
- ToolsDialogShell.tsx (A07) owns the focus trap; verify the trap's first-focusable truly is the body input when auditing it.
- The r6→r8 card-radius drift and the #1B1B1B→surface-raised fill drift are logged to the "token lane" (w26.i09) — the themes/tokens.json audit (A01 parts 1/2) should confirm these handoffs exist there.
