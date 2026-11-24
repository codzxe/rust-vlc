# AUDIT: ui/src/screens/player/keymap.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 227 / 18 it() tests, 6 describe blocks |
| File verdict | REAL (18/18 pass in baseline; exact-combo semantics pinned) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 99 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full file read; rg seed-vs-source cross-check (exit 0) |

## 1. One-paragraph verdict
Honest pure-logic seam test for the keymap parser/matcher: exact id arrays on matches,
never-guess normalization (`Hyper+X` -> null), exact-combo matching that refuses extra
modifiers (Ctrl+Shift+T is not Ctrl+T) and wrong modifier (Meta+T), disjoint keymaps (no
shadowed ids), and the forward-to-command-layer seam. I cross-verified the seeds against
the real source: DEFAULT_KEYMAP's 3 rows are verbatim copies of keymaps/default.toml
lines 154-156 and DIALOG_KEYMAP rows match lines 120/222/224.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 99 `✓ src/screens/player/keymap.test.ts (18 tests) 9ms`; count 18 == 18.
- Seed-vs-source verification (rg, exit 0): keymaps/default.toml:154-156
  `fullscreen = { key = "F", ... recipe = "fullscreen-toggle", mode = "player-focused" ...}` ==
  DEFAULT_KEYMAP_TOML rows in keymap.ts:68-73; dialog rows match :120 (Ctrl+T), :222 (Ctrl+E), :224 (Ctrl+B).
- Exactness (verified read): `expect(normalizeBindingKey("Hyper+X")).toBeNull();` and
  `matchModifierBindings(DIALOG_KEYMAP, { ...ctrl("t"), shiftKey: true }) -> []`.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/keymap.ts:68-73 (the seam this test validates) + this test
What is wrong: the shipped keymap seam runs on a hand-copied 3-row + 3-row TOML excerpt
(`DEFAULT_KEYMAP_TOML`/`DIALOG_KEYMAP_TOML`, each self-labeled `[PROPOSAL] excerpt ... When
the loader lands, swap`). The test pins the excerpt's internal consistency, but NOTHING
binds the excerpt to keymaps/default.toml — if the product keymap changes (e.g. fullscreen
key), the UI seam does not follow and no test fails. (Menu accelerators DO flow from
generated HOTKEY_BINDINGS data — ui/scripts/gen-hotkeys-data.mjs reads default.toml as
source of truth — so the drift risk is specific to the player key seam.)
What correct looks like: a drift test comparing the excerpt rows against the real file
(or landing the loader the header promises).
Confidence: HIGH — Verification: read + rg cross-check.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| describe parseKeymap | 15-32 | REAL | — | duplicate-id rejection with thrown regex |
| describe normalizeBindingKey | 34-49 | REAL | — | DOM shapes; null on unknown modifier |
| describe matchBindings | 51-81 | REAL | — | exact ids; plain-key never fires on combos |
| describe shellActionFor | 83-95 | REAL | — | shell recipes + forward seam |
| describe DIALOG_KEYMAP seed | 97-116 | REAL | S3 | excerpt rows; disjointness loop |
| describe matchModifierBindings | 118-227 | REAL | — | exact-combo semantics, mode gating |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rejects duplicate binding ids | YES | parseKeymap throws |
| never fires on extra modifiers | YES | empty-array assertion |
| stays disjoint from the shell keymap | YES | loop over DIALOG_KEYMAP keys |
| parses the default seed excerpt | YES | exact keys array + toMatchObject fields |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seeded from a verbatim excerpt of ... keymaps/default.toml lines 154-156" (keymap.ts:9-10) | keymap.ts | REAL | rg verified lines 154-156 match |
| 18 tests pass | baseline ui-test.txt:99 | REAL | vitest line |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
Imports PLAYER_SHELL_KEYMAP from variants for the disjointness check — real cross-module
coverage. No DOM needed (pure functions) — fast (9ms).

## 9. Open questions
- Did the "loader" promised by keymap.ts:11 ever land? (W-next: audit
  ui/src/screens/player/variants/variantKeymap.ts which also references default.toml.)
