# AUDIT: ui/src/screens/player/keymap.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 261 / 7 exported fns + 2 seeds |
| File verdict | REAL (with an unguarded hand-copied data seam) |
| Reachable from shipped entry point? | YES: PlayerScreen.tsx:114-120 imports DIALOG_KEYMAP/matchBindings/shellActionFor -> /player keydown listener; ShortcutsScreen consumes DEFAULT_KEYMAP |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg seed rows in keymaps/default.toml (exit 0); baseline ui-test.txt (keymap.test.ts passes) |

## 1. One-paragraph verdict
Clean data-driven keymap matcher: a small TOML-subset parser, a normalizer that refuses unknown modifiers ("never guessed"), plain-key and exact-modifier-set matchers, and a table-driven shell-action map. The logic is real and tested. The structural weakness is admitted in the header: "The keymap LOADING pipeline has not landed" — the shell dispatches from HAND-COPIED excerpts (`DEFAULT_KEYMAP_TOML`, `DIALOG_KEYMAP_TOML`) of keymaps/default.toml, while the hotkeys editor and the Shortcuts screen render the GENERATED, drift-guarded mirror (bindings.ts + `npm run hotkeys:check`). Today the excerpts match default.toml exactly (verified F/Esc/I @154-156, Ctrl+T @120, Ctrl+B @224, Ctrl+E @222), but nothing enforces that: edit default.toml and the shipped shell silently keeps the old combos while every settings screen advertises the new ones.

## 2. What actually works (verified)
— No drift today: rg in keymaps/default.toml confirms `fullscreen = { key = "F", ... }` (L154), `exit-fullscreen` Esc (L155), `show-controls-fullscreen` I (L156), `jump-to-time` Ctrl+T (L120), `bookmarks-dialog` Ctrl+B (L224), `effects-and-filters` Ctrl+E (L222) — byte-equal to the seeds at keymap.ts:68-90 (modulo dropped annotation fields).
— Matching logic: plain-key bindings never fire on combos (L157, L163); modifier combos require the EXACT modifier set (sameModifiers, order-insensitive, L180-192).
— Baseline: keymap.test.ts passes (ui-test.txt).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/keymap.ts:62-73 and 78-94
Code:
```
export const DEFAULT_KEYMAP_TOML = `# [PROPOSAL] excerpt of keymaps/default.toml (card 4.C1 shell seam).
[player-fullscreen]
fullscreen = { key = "F", recipe = "fullscreen-toggle", mode = "player-focused" }
```
What is wrong: the shell's live dispatch data is a hand-copied excerpt with NO drift guard, while the rest of the keymap UI (editor, Shortcuts screen) renders the generated bindings.ts which DOES have one (`hotkeys:check`, bindings.ts:1-3). The two sources can silently diverge: change `fullscreen` from "F" to "F11" in default.toml, regenerate bindings (check passes), and the Shortcuts screen will display F11 while the shell still toggles on F. Why it matters: displayed and actual hotkeys disagreeing is exactly the kind of lie this audit exists to catch; the repo already built the guard pattern and did not apply it here. What correct looks like: generate these seeds from default.toml too (same script), or run parseKey over the full file once the loader lands. Confidence: HIGH — Verification: read + rg cross-check.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parseKeymap | 31-60 | REAL | - | duplicate-id throw; skips non-binding rows |
| DEFAULT_KEYMAP_TOML / DIALOG_KEYMAP_TOML seeds | 62-94 | REAL-but-fragile | S3 | F-001: hand-copied, unguarded |
| normalizeBindingKey | 107-136 | REAL | - | unknown modifier -> null (never guessed); esc->escape mapping |
| matchBindings | 152-167 | REAL | - | plain keys only; combo-reject |
| pressedModifiers / sameModifiers | 170-192 | REAL | - | exact set equality |
| matchModifierBindings | 201-216 | REAL | - | exact modifier-set matching |
| shellActionFor + tables | 224-261 | REAL | - | table-driven; forward fallback |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| keymap.test.ts (baseline pass) | YES | parser/matcher/shellActionFor units asserted incl. duplicate-id throw |
| No test pins the seeds against default.toml | NO TEST EXISTS | drift would pass the whole suite — F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seeded from a verbatim excerpt of the three shell-relevant bindings from keymaps/default.toml lines 154-156" | L9-10 | TRUE today | rg L154-156 match |
| "copied VERBATIM (key/ui_action/mode/when) from keymaps/default.toml" | L80-81 | TRUE today | rg L120/222/224 match |
| "When the loader lands, swap DEFAULT_KEYMAP_TOML for the loaded source" | L11 | PAPER-ONLY (promise) | no loader exists (rg: no keymap loader in ui/src) |

## 7. Dead / duplicated / unreachable code
None dead. The duplication is data-level: same rows exist in three places (default.toml, keymap.ts seeds, generated bindings.ts) with a guard on only one of the three.

## 8. Dependencies & flags
No gates. Depends on parseTomlSubset from ./layout.

## 9. Open questions for the next agent
- Does ui/NEXT-STEPS.md still list the keymap loader as pending? If the [PROPOSAL] has aged multiple waves, the drift guard is the cheaper fix.
