# AUDIT: ui/src/screens/player/tools/equalizer/equalizer.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 461 / 20 selector groups (+1 media block) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as UI code — imported by EqualizerTool.tsx:43 (`import "./equalizer.css"`), mounted via ToolsDialogHost on /player (baseline #22 vite build OK; #23 vitest green incl. GraphicEQ/EqualizerTool/harness tests). Desktop host binary cannot compile at HEAD (W14-A01 F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "className|data-preset|eq-tool|eq-panel" GraphicEQ.tsx EqualizerTool.tsx` (all selectors consumed, both directions) · `rg -n "id:" presets.ts` (preset ids none/flat/classical/club/dance/full-bass/full-bass-treble/full-treble — match the 6 data-preset rules; none/flat absence documented in-file) · `rg -n "eq-tool--open|tools-dialog__staged-tool" dialogs.css ToolsDialog.tsx` (:378 :has() rule + :118 wrapper exist — the transitional pair is consistent) · per-token `rg -c "\-\-$t:" ui/src/tokens/generated/tokens.css` (all 18 custom properties used are defined) · `rg -n "lumen-dropdown__trigger|__value|__list|__option" components/menu/Dropdown.tsx` (class names match) · `rg -n "eq-panel__toggle-label" ui/src -g '*.css'` (ZERO hits — dead class) · `rg -n "inset: auto 0 2px" components/slider/slider.css` (:132-135 vertical fill is bottom-anchored → 0 dB tick geometry correct by construction) · `rg -n "PENDING_COMMAND" EqualizerTool.tsx` (:49 "audio.set-eq" — command absent from generated/commands.ts) · baseline: ui-vitest.log.txt (1705 passed), ui-build.log.txt; NO lint:css run exists in baseline (see UNVERIFIED) |

## 1. One-paragraph verdict
This is the styling half of the equalizer tool, and it is unusually rigorous: every raw px carries a frame citation, every color is a semantic token (the single hex — the 0 dB tick #1A1611 — carries a documented stylelint-disable and a token-lane handoff note), every selector is consumed by real components, the six per-preset curve-glyph rules key on structural `data-preset` markers that match presets.ts ids exactly, and the cross-file transitional contract (dialogs.css `:has(.eq-tool--open)`, ToolsDialog's `tools-dialog__staged-tool` wrapper, Dropdown's class names) all verify. The geometry math is correct by construction: the vertical slider fill is bottom-anchored (slider.css:132-135), so the 0 dB tick's fixed `bottom: 157px` on the fill lands at a constant 160 px (= mid of the 320 px track, half the ±20 dB range) regardless of gain. What this stylesheet dresses, however, is an honest ghost: the tool it styles carries `PENDING_COMMAND = "audio.set-eq"` (EqualizerTool.tsx:49) — a command that does not exist in the generated command catalog — and self-stages with the `.eq-tool__staged` note this very file styles (W8's finding: the EQ awaits a nonexistent backend). The only defects are hygiene: one dead class name and the documented transitional dual-staged-note machinery.

## 2. What actually works (verified)
— Selector↔component closure, both directions: every class in this sheet is applied by GraphicEQ.tsx (:152-258) or EqualizerTool.tsx (:126-158) or the shared Dropdown; conversely GraphicEQ/EqualizerTool apply only `eq-panel__toggle-label` without a rule (F-001).
— Preset glyph wiring is structural: EqualizerTool.tsx:120 `data: { preset: spec.id }` feeds the `data-preset` attribute; presets.ts ids match the six wedge rules one-to-one (none/flat intentionally ruleless, documented at :154-155, :183-184).
— 0 dB tick geometry verified by construction: slider.css:132-135 `.lumen-slider--vertical .lumen-slider__fill { inset: auto 0 2px; height: calc((100% - 4px) * p / 100) }` — fill bottom pinned 2 px above track bottom; tick `bottom: 157px` + 2 px height → center 160 px above track bottom = exact mid-track for the ±20 dB range (rule comment :411-412 math checks out).
— All 18 CSS custom properties used are defined in ui/src/tokens/generated/tokens.css (per-token rg).
— The 10-band/preamp distinction is test-pinned: GraphicEQ.test.tsx:121-127 and EqualizerTool.test.tsx:93 assert `.eq-panel__fader--zero` on the 10 band columns and NOT on preamp — green in baseline ui-vitest.log.txt.
- The rail shrink fix (max-width:748px + overflow-x:auto, :366-376) prevents the documented w26.g15 under-scrim overflow in the 398px tools dialog.

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/player/tools/equalizer/equalizer.css (absence); the class is applied at GraphicEQ.tsx:168
Code:
```
        <span className="eq-panel__toggle-label">
```
What is wrong: `.eq-panel__toggle-label` is applied by GraphicEQ.tsx:168 but has no rule in equalizer.css, harness.css, or any other stylesheet (repo-wide rg over *.css: zero hits). Every sibling part of the toggle (`.eq-panel__toggle`, `.eq-panel__chevron`) is styled; the label span is a naked hook.
Why it matters: either a style is missing (the label relies entirely on inherited toggle styles — currently indistinguishable from intended, since the toggle sets font/size/color on itself) or the class is dead weight that misleads the next maintainer into expecting a rule.
What correct looks like: delete the class from GraphicEQ.tsx, or add the (possibly empty-but-documented) rule if it exists as a styling seam for the harness.
Confidence: HIGH — Verification: read + repo-wide rg.

### F-002 [S4]
Where: ui/src/screens/player/tools/equalizer/equalizer.css:57-59 (with dialogs.css:374-378 and ToolsDialog.tsx:118)
Code:
```
.tools-dialog__staged-tool .eq-tool__staged {
  display: none;
}
```
What is wrong: documented TRANSITIONAL debt, still live: two staged-note rendering paths exist — the tool's own in-body note (:46-50) and the tools-dialog wrapper's `:has(.eq-tool--open)` note (dialogs.css:378) — with this suppression rule arbitrating. The comment says "Delete this rule together with the wrapper + its :has() rule" (w26.k10, j10 P3-4); the deletion has not happened.
Why it matters: three files must stay in sync for one caption line; any drift (e.g. the wrapper class renamed) silently double-renders or loses the honest-unbound note — the one marker telling users the EQ is not engine-bound.
What correct looks like: land the deletion (tool owns its staged note; wrapper + :has + suppression rule removed).
Confidence: HIGH — Verification: read + rg confirming all three pieces exist and are mutually consistent today.

## 4. Per-selector table
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .eq-tool | 37-42 | REAL | — | wrapper; `eq-tool--open` state class applied by EqualizerTool.tsx:126 |
| .eq-tool__staged | 46-50 | REAL | — | styles the honest-unbound note (audio.set-eq pending, W8) |
| .tools-dialog__staged-tool .eq-tool__staged | 57-59 | REAL | S4 | F-002 transitional suppression |
| .eq-tool__presets (+trigger/value/list/option scopes) | 65-141 | REAL | — | frame-measured dropdown grammar; class names match Dropdown.tsx |
| option ::before baseline | 156-178 | REAL | — | painted glyph, shared DOM untouched; selected row hides glyph (frame) |
| data-preset wedges ×6 | 194-287 | REAL | — | classical/club/dance/full-bass/full-bass-treble/full-treble; ids match presets.ts exactly |
| .eq-panel / --open | 292-305 | REAL | — | morph surface; rx2 documented no-token structural |
| .eq-panel__toggle (+hover/focus-visible) | 310-339 | REAL | — | a11y focus ring present |
| .eq-panel__chevron | 344-346 | REAL | — | glyph does not rotate (frame) |
| .eq-panel__body (+[hidden]) | 352-361 | REAL | — | display:none wins over flex (a18 pattern) |
| .eq-panel__rail | 366-376 | REAL | — | max-width shrink fix (w26.g15) |
| .eq-panel__col | 378-384 | REAL | — | 68px pitch |
| .eq-panel__fader | 391-394 | REAL | — | 328px = 320 track + overhangs |
| .lumen-slider--vertical track inset | 398-401 | REAL | — | composes over slider.css (higher specificity, verified) |
| .eq-panel__fader--zero fill::after | 408-420 | REAL | — | 0 dB tick; geometry verified correct (bottom-anchored fill) |
| .eq-panel__label | 428-435 | REAL | — | Lead WCAG ruling overrides frame ink (documented) |
| .eq-panel__chip | 442-452 | REAL | — | ValueChip composition |
| @media prefers-reduced-motion | 456-461 | REAL | — | transitions disabled |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| GraphicEQ.test.tsx:121-127 (--zero on 10 band cols, absent on preamp) | YES (DOM-level) | class assertions; catches wiring drift, not visual drift |
| EqualizerTool.test.tsx:93 (10 --zero faders) | YES (DOM-level) | same |
| harness.test.tsx (loads tokens.css…equalizer.css for visual states) | YES (existence-level) | a deleted/renamed CSS file fails the import; visual correctness itself needs the `visual` harness (not in baseline) |
No test asserts the geometry (157px tick, 748px rail) — visual-only, covered at most by the non-baseline playwright `visual` lane.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only; raw px below are frame-measured … each carries its frame citation" | file header :25-28 | REAL | verified per-selector (citations present; single documented hex exception :417-418) |
| "Colors are var(--…) everywhere except the 0 dB tick" | header :27-28 | REAL | only #1a1611 present, with disable comment |
| EQ tool staged/unbound ("no `audio.set-eq` command exists yet") | EqualizerTool.tsx:47-49 | REAL | command absent from generated/commands.ts CommandRequest union (verified full read of the union) |

## 7. Dead / duplicated / unreachable code
- F-001: `.eq-panel__toggle-label` — class applied, never styled.
- F-002: transitional staged-note trio (equalizer.css:57-59 + dialogs.css:374-378 + ToolsDialog wrapper).
- No orphan selectors: every selector in this sheet has an applying component (verified both directions).

## 8. Dependencies & flags
- Depends on tokens.css custom properties (all 18 verified defined), slider.css primitives (EQSlider class names), Dropdown class names, ValueChip (form lane). No feature flags. stylelint compliance UNVERIFIED — `npm run lint:css` has no baseline run (INDEX #21-24 cover build/test/tokens/settings/hotkeys/l10n only); the file's own stylelint-disable comment implies the gate is expected to run somewhere (REQUEST-ORCH-RUN candidate, see log).

## 9. Open questions for the next agent
- ui/stylelint.config.js (my next files) — confirm whether the config would accept this file; the 0 dB tick disable-next-line syntax only matters if lint:css actually runs in CI.
- Whoever owns ToolsDialog/dialogs.css: F-002's deletion spans your files.
- The `visual` playwright lane (not run in baseline) is the only check that could validate the frame-measured geometry; UNVERIFIED here.
