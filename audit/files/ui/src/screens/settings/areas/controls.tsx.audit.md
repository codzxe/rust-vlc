# AUDIT: ui/src/screens/settings/areas/controls.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 244 / 4 (1 area component + 2 section helpers + 1 row helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — /settings/controls route (SettingsScreen area registry); the generated skip/mouse rows render via GENERATED_SCREENS.controls (generated/Screens.tsx:181-187, 9 schema keys verified). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | rg 'control-server' product.toml (line 167: false, "Phase 5+ [PROPOSAL]" — Remotes staging claim verified); rg localStorageHotkeyOverrides consumers (overrides.ts, HotkeysEditor.tsx, controls.tsx, tests — NO player-side consumer); read keymap.ts:1-80 (player key handling = hard-coded 3-binding seed, never user overrides); Read PlayerScreen.tsx:185-228 (HOTKEY_BINDINGS used for one dialog combo + accelerator STRINGS only); grep generated/Screens.tsx (controls screen exists). |

## 1. One-paragraph verdict
The Controls area is competently composed (hand-authored action rows + the schema-generated Skip Amounts/Mouse screen + honestly-staged inert Remotes rows whose staging claim checks out against product.toml:167) — but its flagship feature is a dead-end store: "Edit Keyboard Shortcuts" opens a full editor dialog whose Apply writes `localStorageHotkeyOverrides`, and "Reset all shortcuts" clears it, while the player's actual key dispatch (keymap.ts DEFAULT_KEYMAP — a hard-coded 3-binding TOML seed — plus dialog seed maps) never reads that store. Every hotkey customization a user makes has zero effect on any key in every build. The editor itself and the overrides store belong to other files; this file owns the two action rows that send users into the futile flow.

## 2. What actually works (verified)
— Remotes staging verified honest: product.toml:167 `control-server = false # Phase 5+ [PROPOSAL]`; rows render disabled with tooltip + sr-only reason + aria-describedby (lines 111-153) — the pattern does what it claims. Reset flow correct: `save({})` + epoch bump remounts the editor (HotkeysEditor `key={hotkeysEpoch}`), re-reading the cleared store. Baseline: controls.test.tsx passes (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/settings/areas/controls.tsx:175-178 (+ 193-206)
Code:
```
  const resetAllShortcuts = (): void => {
    localStorageHotkeyOverrides.save({});
    setHotkeysEpoch((epoch) => epoch + 1);
  };
```
```
        <ActionRow
          action="edit-shortcuts"
```
What is wrong: the two Keyboard Shortcuts actions operate on a store nothing consumes. rg for `localStorageHotkeyOverrides` across src: only the store itself, the editor, this file, and tests. The player's key handling (keymap.ts:68-76 `DEFAULT_KEYMAP_TOML` — a verbatim hard-coded excerpt of three bindings; PlayerScreen.tsx:191-206 the dialog combos) consults default data only; `HOTKEY_BINDINGS` (generated keymap data) is used for one combo and for accelerator DISPLAY strings (PlayerScreen.tsx:220-228), never merged with user overrides. A user who rebinds a key in the editor and presses Apply changes nothing about how any key behaves.
Why it matters: a complete, polished editing surface (141 bindings, conflict detection, per-binding reset, dirty-gated apply bar) writing to a void — the W11 settings-never-persist class at its most user-visible: the dialog LOOKS authoritative and is inert.
What correct looks like: keymap.ts merges overrides (`DEFAULT_KEYMAP` overlaid with the store on load), or the editor carries the same staged marker the Remotes rows do until the loader lands.
Confidence: HIGH — consumer search exhaustive; key dispatch path traced.
### F-002 [S4]
Where: ui/src/screens/settings/areas/controls.tsx:64-89
Code:
```
function ActionRow({
  action,
  label,
  buttonLabel,
  ...buttonRest
```
What is wrong: ActionRow accepts and spreads arbitrary ButtonHTMLAttributes but has no `className` passthrough, and `data-controls-action` duplicates the semantic already carried by `action` (the prop exists "for tests/inspection" per its doc — a test hook in production DOM, same pattern as appshell.css F-002).
Why it matters: minor API/dead-marker debt.
What correct looks like: drop the data attribute and let tests key off the button label/id.
Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `GeneratedControlsScreen` (const) | 49 | REAL | — | GENERATED_SCREENS.controls verified (Screens.tsx:181) |
| `useAreaMsg` (const, makeAreaMsg) | 57 | REAL | — | 5-source pre-registration seam (the AppShell F-002 pattern, shared factory) |
| `ActionRow` | 64-89 | REAL | S4 (F-002) | spreads buttonRest; aria-haspopup/expanded wired correctly by the edit row |
| `RemotesSection` | 100-160 | REAL | — | staging verified against product.toml; sr-only reason twin + aria-describedby correct |
| `ControlsArea` | 166-244 | REAL | S2 (F-001) | dialog lifecycle correct (labelledBy, closeOnOutsideClick, epoch remount) |

## 5. Test honesty
controls.test.tsx belongs to another agent. Mutation: break resetAllShortcuts (no save) → the epoch-remount test would still pass if it only asserts dialog state — the store-write assertion is the load-bearing one; not verified here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the two Keyboard Shortcuts actions are real behaviors (open the hotkeys editor / clear the overrides store), never config options" | controls.tsx:17-20 | REAL (literal) / SHALLOW (effective) | they really do both things; the store has no consumer (F-001) |
| "no controls.web-remote / controls.port [[setting]] keys exist and the control server is Phase 5+ (product.toml [features] control-server = false)" | controls.tsx:93-96 | REAL | product.toml:167 verified; schema rg: 9 controls.* keys, none web-remote/port |
| "exactly ONE [data-settings-area="controls"] renders per route" | controls.tsx:14-16 | REAL-UNPROVEN | wrapper carries no marker (read); generated root's marker not inspected (other file) |

## 7. Dead / duplicated / unreachable code
F-001 (the overrides flow is a dead end — the editor itself is another file's audit). No dead selectors/classes in this file's own markup (controls.css audited separately).

## 8. Dependencies & flags
Switch/TextField/Icon/Dialog, 5 i18n key modules, GENERATED_SCREENS, makeAreaMsg/shared, HotkeysEditor + overrides store, controls.css. All exist.

## 9. Open questions for the next agent
- HotkeysEditor.tsx owner: does the editor surface ANY hint that overrides are inert? (Header only says "Apply commits the overrides to the persistence seam" — persistence ≠ effect.)
- keymap.ts owner: the "keymap LOADING pipeline has not landed" note (keymap.ts:5-8) — is there a matrix row claiming editable/customizable shortcuts as implemented? If yes, F-001 upgrades to S0 at the doc layer.
