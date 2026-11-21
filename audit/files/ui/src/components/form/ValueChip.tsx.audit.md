# AUDIT: ui/src/components/form/ValueChip.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 21 / 1 component (+1 interface) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — consumed by the equalizer's GraphicEQ (GraphicEQ.tsx:35,258 `<ValueChip className="eq-panel__chip" …>`), mounted in the Tools dialog's EQ card (ToolsDialog.tsx:125-137). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Grep ValueChip across src (initial false negative from a bad grep -v pipeline — corrected: GraphicEQ.tsx imports from components/form); grep the tools dialogs (ToolsValueChip is a SEPARATE component). |

## 1. One-paragraph verdict
A 3-line display span that does exactly what it says — class join, children as the accessible content, no state. The interesting fact is what sits next to it: the tools dialog ships its OWN value chip (`ToolsValueChip`, ToolsDialogShell.tsx:245, `.tools-row__value` grammar) while this one serves only the EQ panel — two components + two CSS skins for the same frame readout ("audio value.svg / EQ rows"), the exact duplication shape the switch j01-3 ruling consolidated for switches. This file's header even claims "used by EQ/tool rows" — the tool rows use the other one.

## 2. What actually works (verified)
— Render path trivially correct; real consumer (GraphicEQ.tsx:258) renders live local dB state. Baseline: ValueChip.test.tsx (render + skin regex) passes (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/form/ValueChip.tsx:12-15 vs ui/src/screens/player/dialogs/ToolsDialogShell.tsx:242-254
Code:
```
 * ValueChip — the dark value readout used by EQ/tool rows (wave 26; frame:
 * audio value.svg / EQ rows): r6 recessed well, bright tabular value text.
```
What is wrong: the header claims EQ AND tool rows; the tool rows render `ToolsValueChip` (a separate span with `.tools-row__value`, styled in dialogs.css:499+ — "the dark read-out at a row's right end (frame: EQ / Compressor rows)"). Two chip implementations for one frame element; the comment overstates this one's reach.
Why it matters: same-frame duplication invites the divergence the j01-9 switch-skin bug already demonstrated in this codebase; a chip restyle lands in one of the two places.
What correct looks like: ToolsValueChip composes ValueChip (className passthrough already exists), one CSS grammar.
Confidence: HIGH — both components read; both CSS blocks located.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ValueChipProps` (interface) | 4-9 | REAL | — | |
| `ValueChip` | 17-21 | REAL | S4 (F-001, comment scope) | plain span, class join correct |

## 5. Test honesty
ValueChip.test.tsx (other agent): render assertions + a regex against form.css (ValueChip.test.tsx:29-31) — the CSS-regex style again: passes while the cascade is intact, would fail on rule deletion. Would not catch F-001 (scope/duplication).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "used by EQ/tool rows" | ValueChip.tsx:12-13 | HALF-FALSE | EQ: GraphicEQ.tsx:258 ✓; tool rows: ToolsValueChip (separate) |
| "A plain span — the surrounding row supplies the label context" | ValueChip.tsx:14-15 | REAL | no aria attributes added |

## 7. Dead / duplicated / unreachable code
F-001 (duplicate chip implementations). The component itself is not dead (EQ consumer).

## 8. Dependencies & flags
form.css (.lumen-valuechip — audited in my form.css pass, F-003 there covers the token approximation).

## 9. Open questions for the next agent
- dialogs.css owner: are `.tools-row__value` and `.lumen-valuechip` visually identical today, or already drifted (two "frame: EQ rows" citations, two wells)?
- EqualizerTool owner: the chip's values are LOCAL slider state (the EQ is staged on audio.set-eq per ToolsDialog.tsx:125-132) — the chip reads honestly but the engine never sees the dB values; covered in the EQ lane's audit?
