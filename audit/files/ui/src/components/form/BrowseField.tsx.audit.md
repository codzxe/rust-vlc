# AUDIT: ui/src/components/form/BrowseField.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 68 / 1 (BrowseField, forwardRef) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — zero production consumers. `rg -n "BrowseField" ui/src` finds only: the component, its own test, the form/index.ts barrel, states.md/form.css/setting-field.css references, and TextField's comment. The settings rows it was built for (interface.skin-file, global.log-file — both `type: "path"` in generated/schema.ts:346-348) render through SettingField's OWN inline path composition (SettingField.tsx:311-325), not this component |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file + BrowseField.test.tsx (81 lines) · `rg -n "BrowseField" ui/src` (consumer closure — test-only) · `rg -n -i "skin\|log.file" src/screens/settings src/i18n/messages.ts` (the frame-named settings exist as keys) · `rg -n -B2 -A6 "skin-file" src/screens/settings/generated/schema.ts` (type "path") · `rg -n -B4 -A14 '=== "path"' src/screens/settings/SettingField.tsx` (the ACTUAL path-row renderer — TextField + its own disabled-with-reason Browse) |

## 1. One-paragraph verdict
The component itself is competently built — correct label/error/hideLabel a11y mechanics aligned to TextField's single-mechanism rule, honest native-disabled handling, and a genuinely good test file (behavioral + CSS-skin assertions) — but it is an orphan: nothing in the shipped app renders it. It was built explicitly for the settings path rows ("frame: Settings - interface.svg 'Skin file' / Settings - global.svg 'Log file'", header :22-23), yet those rows are served by SettingField's own inline `settings-field__path` composition, which re-implements the input-plus-Browse-button grammar and — unlike this component — disables its Browse button with a visible honest reason because the catalog has no file-dialog command. So the app carries two implementations of the same frame grammar: one live (SettingField's), one dead (this file's), and the dead one's `onAction?` option would render an enabled no-op button if ever adopted by a caller that forgets the callback.

## 2. What actually works (verified)
— The a11y mechanics are correct and test-pinned: label htmlFor ↔ input id (auto-id via useId, caller id wins), error → aria-invalid + aria-describedby pointing at the rendered message id (:37-43 test), hideLabel → aria-label with NO label text node (the j01-8 one-mechanism rule, test :48-53), ref forwarding (test :55-59).
— Disabled honesty: the native `disabled` on both controls makes the button inert without a JS guard (the j01-13 note), test :27-35.
— The skin tests assert the frame grammar directly against form.css (button radius/colors/min-width 100px/height 32px) — would fail if the CSS drifted.
— All green: baseline ui-vitest.log.txt (1705 passed). But every green test tests code no user can reach.

## 3. Findings

### F-001 [S3]
Where: ui/src/components/form/BrowseField.tsx:21-26
Code:
```
 * BrowseField — the settings path input + action combo (wave 26; frame:
 * Settings - interface.svg "Skin file" / Settings - global.svg "Log file"):
 * the standard TextField control with a 32px r6 panel button (min-width 100)
 * beside it. The action is caller-wired (`onAction`) so the component stays
```
What is wrong: the component has no caller. The two frame-named settings it cites are `type: "path"` entries (schema.ts SETTING_INTERFACE_SKIN_FILE :346-348; global.log-file likewise) and they render through SettingField.tsx:311-325's own composition (`settings-field__path` + TextField + its own Browse button, disabled with an honest note). BrowseField is exported by the form barrel and imported by nothing but its test.
Why it matters: dead code with a misleading provenance — a reader of this header believes the settings path rows use this component; they do not. The grammar now exists twice (live inline copy + dead shared copy), so the next path-row change lands in one and silently misses the other. Tree-shaking keeps it out of the bundle, so the cost is source-level only.
What correct looks like: either adopt it in SettingField (replace the inline composition) or delete it (and its barrel export, test, and form.css block).
Confidence: HIGH — Verification: exhaustive rg of the export's consumers + read of the actual path-row renderer.

### F-002 [S4]
Where: ui/src/components/form/BrowseField.tsx:10-11, 54
Code:
```
  /** Trailing action — opens the file picker (screen-level concern). */
  onAction?: () => void;
```
```
          <button type="button" className="lumen-browse__action" onClick={() => onAction?.()} disabled={disabled}>
```
What is wrong: `onAction` is optional, but the button always renders enabled — a caller that omits the callback gets a fully-alive-looking Browse button that does nothing. This violates the codebase's own "no dead affordance" rule, enforced elsewhere in the same feature: SettingField's Browse is disabled with a visible honest reason precisely because no file-dialog command exists (SettingField.tsx:315-317), and ThumbnailGrid degrades to inert presentation for the same reason.
Why it matters: latent only (F-001 means no caller exists today), but the component is the one place in the form library where the no-dead-affordance rule is structurally unenforced.
What correct looks like: `disabled={disabled || onAction === undefined}` or omit the button when no action is provided.
Confidence: HIGH — Verification: read; consumer absence verified (F-001).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BrowseField | 28-66 | REAL (code) / UNREACHABLE (usage) | S3/S4 | mechanics correct + test-pinned; orphaned (F-001); optional-action dead button (F-002); `type` prop set twice (rest spread re-applies) — harmless redundancy |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :11 label wiring + typing flows | YES | label association + onChange called |
| :20 action button fires onAction | YES | callback assertion |
| :27 disabled disables both, no action | YES | both controls + no-call assertion |
| :37 error aria wiring | YES | describedby id matches the message |
| :48 hideLabel single mechanism | YES | aria-label + absence of label node |
| :55 ref forwarding | YES | instance assertion |
| :61 DOM snapshot | WEAK | snapshot honesty reviewed separately (ValueChip snap audit, file 26); standard structure pin |
| :67/:78 CSS skin assertions | YES | regex-anchored grammar checks against form.css |
All green in baseline — but the entire suite exercises unreachable code (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the settings path input + action combo (… Settings - interface.svg 'Skin file' / Settings - global.svg 'Log file')" | header :22-23 | FALSE as usage / REAL as intent | those rows render via SettingField's inline composition (F-001) |
| "The action is caller-wired (`onAction`) so the component stays transport-agnostic" | :25-26 | REAL (design) / trap (F-002) | optional callback + always-rendered enabled button |
| "no dead `if (!disabled)` guard — the native disabled attribute already makes the button inert" | :52-53 | REAL | test :27-35 |
| "j01-8: one prop, one mechanism" (hideLabel) | :14-17 | REAL | test :48-53 |

## 7. Dead / duplicated / unreachable code
- The entire component is unreachable production code (F-001): barrel-exported, test-imported, screen-never-imported, tree-shaken from the build.
- Duplication: the input+Browse grammar exists twice (this file + SettingField's inline path row).

## 8. Dependencies & flags
- React (forwardRef/useId), form.css. No engine dependency, no feature flags. The file-dialog capability gap that keeps every real Browse button disabled is catalog-level (no file-dialog command — SettingField.tsx:315-317 documents it).

## 9. Open questions for the next agent
- SettingField.tsx's inline path row deserves its own audit (it is the LIVE implementation of this grammar): does its disabled-Browse note render visibly, and does recording.directory (the third path row per SettingField's comment) match?
- If the file-dialog command ever lands in the catalog, the fix touches SettingField (enable its Browse) AND resurrects the F-001 decision (adopt vs delete this component).
