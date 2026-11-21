# AUDIT: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 267 / 8 items |
| File verdict | REAL — with S2 write-only persistence (F-001) |
| Reachable from shipped entry point? | YES as UI code — /settings/controls → Keyboard Shortcuts fold → editor dialog (controls.tsx mounts it on demand; controls.test.tsx:93 "the editor DIALOG mounts on demand"). Builds + tests green (baseline #22/#23). Desktop host binary cannot compile at HEAD (W14-A01 F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -n "localStorageHotkeyOverrides\|HotkeyOverridesStore\|lumen.hotkeys.overrides" ui/src` (consumers: ONLY HotkeysEditor.tsx, controls.tsx:176 `save({})`, and tests — NO runtime key handler reads it) · Read overrides.ts (37 lines — header admits "a MOCK localStorage round-trip today, the real write path … is the app layer's handoff") · `rg -n "HOTKEY_BINDINGS\|override" ui/src/screens/player/keymap.ts` (the player's matcher parses the SEED TOML; no override read) · `rg -n "acceleratorOf" ui/src/screens/player/PlayerScreen.tsx` (menu accelerators read the generated DEFAULTS) · `rg -n "staged" ui/src/screens/settings/areas/controls.tsx` (:102-123 — the Remotes section IS staged in the same area file; the hotkeys editor is not) · `rg -n '"settings.hotkeys.edit-hint":' messages.ts` (:1780 "Type a key combination, e.g. Ctrl+K") · `rg -n "describe\|it\(" HotkeysEditor.test.tsx controls.test.tsx` (7 + 3 relevant tests) · baseline: ui-vitest.log.txt (1705 passed), ui-build.log.txt |

## 1. One-paragraph verdict
The editor itself is well built: 141 rows from the REAL generated keymap data, memoized so a keystroke re-renders one row, conflict detection that marks both rows of a duplicated combo, per-binding reset, a dirty-gated apply bar, a beforeunload guard, and disabled-from-data bindings rendered inert — all test-pinned and green in baseline. The fatal gap is where Apply's output goes: the overrides store (`lumen.hotkeys.overrides.v1`, localStorage) is consumed by NOTHING except this editor and the controls area's reset-all. The player's actual key matchers (keymap.ts parseKeymap over the seed TOML, DIALOG_KEYMAP, PLAYER_SHELL_KEYMAP) and every menu accelerator display read the generated DEFAULTS; no code path applies an override to runtime behavior. overrides.ts admits it in a comment the user never sees ("a MOCK localStorage round-trip today"), and the editor UI carries no staged/gated marker — unlike the Remotes rows in the very same settings area, which follow the project's own honesty convention for unwired surfaces. A user who rebinds Play/Pause to Ctrl+P, clicks Apply, and reloads (the override round-trips, so it LOOKS durable) has changed nothing about how the app responds to keys. Secondarily, the combo field is unvalidated free text.

## 2. What actually works (verified)
— Draft/apply/revert lifecycle is locally correct: `isEdited` (draft vs overrides) drives the apply bar; the row `changed` (draft vs keymap default) drives the reset button; Apply collapses equal-to-default and empty entries out of the store (:248-252); Revert restores draft ← overrides (:260). Tests: HotkeysEditor.test.tsx:48-93 cover edit/conflict-both-rows/reset/apply-persist/revert — green in baseline ui-vitest.log.txt.
— The w26.k13 re-render hotspot fix holds as claimed: `onComboChange`/`onReset` are `useCallback([])`-stable (setDraft identity), row props are primitives + stable callbacks, `HotkeysRow` is `memo` — only the edited row's `combo` prop changes.
— Conflict detection memoized on [bindings, draft] (recomputed per keystroke, not per render); numeric-aware ordering memoized on [bindings, m] — the resolver identity changes only on locale switch.
— `orderBindingsNumericAware` (:56-77) is exported and unit-tested (hotkeysData.test.ts); run detection only reorders same-prefix numbered runs, preserving curated order elsewhere.
— beforeunload guard registers only while dirty (:198-205) and cleans up.
— Disabled-from-data bindings (feature-gated source rows) render inert inputs (:114, :122) — test :94.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx:254 (`store.save(next)`); store consumed by no runtime key handler; the editor renders no gate
Code:
```
              setOverrides(next);
              store.save(next);
              setDraft({ ...next });
```
What is wrong: Apply persists the overrides to `localStorageHotkeyOverrides` (`lumen.hotkeys.overrides.v1`), and repo-wide the store's only consumers are this editor, the controls area's reset-all (`localStorageHotkeyOverrides.save({})`, controls.tsx:176), and tests. The runtime key handlers — keymap.ts `parseKeymap` (parses the seed TOML), PlayerScreen's DIALOG_KEYMAP/CUSTOMIZE_DIALOG_KEYMAP, variants' PLAYER_SHELL_KEYMAP, `matchBindings`/`matchModifierBindings` — and the menu accelerator renderer (`acceleratorOf` over HOTKEY_BINDINGS defaults, PlayerScreen.tsx:220-228) never read the overrides. overrides.ts:2-4 states the truth in a comment: "a MOCK localStorage round-trip today, the real write path (keymap config over the client bridge) is the app layer's handoff."
Why it matters: the dialog is titled "Edit Keyboard Shortcuts", every affordance is enabled, changes survive reload (the store round-trips — making it look durable and real), and yet no rebinding affects a single keystroke or even a displayed menu accelerator. The project's own honesty convention exists in the SAME area file — controls.tsx:102-123 renders the Remotes rows as staged inert rows with a reason — and the editor, whose runtime effect is equally absent, gets no gate, no staged note, no disabled state. A realistic user flow (rebind, apply, try the key) fails silently.
What correct looks like: either wire the seam (keymap.ts loads overrides over the defaults at match time; acceleratorOf reads the effective combo) or render the editor's action bar with the same staged/gated grammar the convert screen and Remotes rows use ("shortcut changes are not yet applied by the player"), with Apply disabled.
Confidence: HIGH — Verification: read + repo-wide rg of every consumer of the store and the storage key; the player matcher's import graph traced (keymap.ts imports only parseTomlSubset; DIALOG_KEYMAP seeds from keymap data).

### F-002 [S4]
Where: ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx:118-127
Code:
```
        <input
          type="text"
          className="hotkeys-row__input"
          value={combo}
          disabled={binding.disabled}
          aria-label={label}
          placeholder={placeholder}
```
What is wrong: the combo field is free text with zero validation: the hint says "Type a key combination, e.g. Ctrl+K" (:1780) but any string ("hello", "Ctrl+", "ctrl+k") is accepted as a draft, conflict-checked as-is, and persistable. There is no key-capture interaction, no modifier+key grammar check, no normalization (case, "Ctrl+Shift" vs "Shift+Ctrl" ordering) — so even in a future where overrides ARE consumed, "ctrl+k" and "Ctrl+K" would be distinct combos and "hello" would be a dead binding.
Why it matters: an editor whose data model is unvalidated strings accumulates garbage that conflict detection then reasons about; the defect is masked today only because the output goes nowhere (F-001).
What correct looks like: combo-shape validation (modifier set + terminal key), canonical spelling (capitalize modifiers, fixed order), and ideally a press-to-capture field instead of typing.
Confidence: HIGH — Verification: read; no validation code exists between the input and the store.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| comboOf | 37-38 | REAL | — | draft ▸ override… (actually draft ▸ default; overrides enter via draft seeding :163) |
| labelPrefix / trailingNumber | 41-47 | REAL | — | regex strip/extract; consistent pair |
| orderBindingsNumericAware | 56-77 | REAL | — | run-sort verified; only numbered same-prefix runs reordered; exported + unit-tested |
| HotkeysRow (memo) | 84-152 | REAL | — | stable-prop memoization verified; disabled bindings inert; conflict `role="alert"` |
| HotkeysEditor | 154-266 | REAL | S2/S4 | F-001 write-only store; F-002 unvalidated input; lifecycle locally correct |
| — isEdited / dirtyIds | 165-172 | REAL | — | draft-vs-overrides dirty semantics (apply bar) |
| — conflicted / ordered memos | 173-183 | REAL | — | dependency arrays correct for the documented recompute policy |
| — onComboChange / onReset | 187-196 | REAL | — | stable callbacks; reset deletes the draft key (falls back to default) |
| — beforeunload effect | 198-205 | REAL | — | registers only while dirty; cleanup correct |
| — apply/revert handlers | 240-264 | REAL | S2 | F-001 — the store.save destination |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| HotkeysEditor.test.tsx:30-46 (rows from real keymap, command column) | YES | renders generated data; asserts per-row content |
| :48-56 edit writes draft | YES | input event → value + changed marker |
| :57-72 conflict both rows + clear on fix | YES | the O(n) detector is exercised end-to-end |
| :73-81 reset restores default | YES | draft key deleted → default renders |
| :82-93 Apply persists via store; Revert discards | YES | asserts the STORE contents (a memory double) — pins the local contract, cannot see F-001 (nothing consumes the store) |
| :94-101 disabled bindings inert; real data unique ids | YES | data-shape assertions |
| controls.test.tsx:119-132 Reset clears overrides + defaults re-read | YES | cross-component reset flow |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Editing writes DRAFT state; Apply commits the overrides to the persistence seam" | file header :18-19 | REAL (locally) / SHALLOW (end-to-end) | the seam commit happens (:254) but the seam is a write-only mock (F-001) |
| "hotkey entries are dynamic data, not schema-static [[setting]] rows" | :14-16 | REAL | HOTKEY_BINDINGS generated from keymaps/default.toml (141 bindings, baseline hotkeys:check PASS) |
| "both seed bindings + editor live" | docs/feature-matrix.toml:3365 note | REAL (editor renders/edits) | the runtime EFFECT of edits is absent — F-001 |
| "conflict detection marks every row of a duplicated (mode, combo) group" | :19-20 | REAL | test :57-72 |

## 7. Dead / duplicated / unreachable code
- The persisted overrides data is the dead end (F-001): live code, write-only store, zero readers.
- No dead functions; orderBindingsNumericAware exported for tests (justified).

## 8. Dependencies & flags
- Generated bindings module (keymaps/default.toml via gen-hotkeys-data.mjs — baseline hotkeys:check PASS), conflicts.ts (not in my list), overrides.ts (mock store), Button, hotkeys.css. No feature flags. `binding.disabled` (feature-gated source rows, e.g. record) handled honestly.

## 9. Open questions for the next agent
- keymap.ts owner: if the override-consumption seam ever lands, it belongs in your file (parseKeymap callers) — until then F-001 stands.
- conflicts.ts (not yet audited by anyone in my view): verify detectConflicts' mode-awareness matches the editor's usage (the header promises "(mode, combo) groups").
- The /keyboard-shortcuts reference dialog (w26.i17) presumably also reads defaults — same F-001 exposure for its display.
