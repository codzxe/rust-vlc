# AUDIT: ui/src/screens/player/variants/index.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A04) |
| Date | 2026-11-24 |
| Lines / functions | 30 / 0 (barrel: 18 exports across 5 re-export statements) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by PlayerScreen.tsx:127-132 (`./variants` barrel: MiniPlayer, MusicMode, PLAYER_SHELL_KEYMAP, loadPersistedVariant, persistVariant, resolveVariantLayout) and ShortcutsScreen.tsx (VARIANT_KEYMAP); PlayerScreen mounted from App.tsx:4 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg per-symbol export-existence + full file lists -> 18/18 exist, 5 dead chains found; sed -n '5,14p;29,30p' -> quotes verified; baseline ui-test.txt:14,32 |

## 1. One-paragraph verdict
A working barrel whose value exports all resolve to real declarations, with the two UI components (MiniPlayer, MusicMode) production-mounted in PlayerScreen and covered by passing tests. The defect is surface bloat: 5 of the 18 exports (PLAYER_VARIANTS, VARIANT_KEYMAP_TOML, MINIPLAYER_LAYOUT_TOML, MUSIC_LAYOUT_TOML, mergeKeymaps) have zero consumers anywhere in the tree — they are defined, re-exported through this barrel (and for three of them again through player/index.ts:44-50), and never imported; 4 more are test-only.

## 2. What actually works (verified)
— verified by `rg "export .* <sym>" ui/src/screens/player/variants/*` -> all 18 symbols exist (variant.ts:26-64, variantLayout.ts:38-107, variantKeymap.ts:17-44, MiniPlayer.tsx:111, MusicMode.tsx:75).
— verified by baseline ui-test.txt:14 `✓ PlayerScreen.test.tsx (60 tests)` and :32 `✓ variants.test.tsx (33 tests)` — the exported components/logic run green.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/variants/index.ts:6,17,19,26-27 (and re-exports player/index.ts:44,47,50)
Code: (verbatim, sed-verified)
```
export {
  PLAYER_VARIANTS,
  VARIANT_STORAGE_KEY,
```
What is wrong: 5 exports are dead chains — `rg -l` across the whole tree finds each only in its defining file + this barrel (+ player/index.ts): PLAYER_VARIANTS (variant.ts:29), VARIANT_KEYMAP_TOML (variantKeymap.ts:17), MINIPLAYER_LAYOUT_TOML / MUSIC_LAYOUT_TOML (variantLayout.ts:38,59), mergeKeymaps (variantKeymap.ts:31, used only internally at :44). Why it matters: the barrel advertises an API nobody uses; the TOML constants are internal parse sources, not a public surface; double re-export through player/index.ts triples the dead surface. What correct looks like: export only the consumed set; keep TOML constants module-private. Confidence: HIGH — Verification: read (exhaustive rg file lists).
F-002 [S4]
Where: ui/src/screens/player/variants/index.ts:7-8,16,18
What is wrong: VARIANT_STORAGE_KEY, clearPersistedVariant, MINIPLAYER_LAYOUT, MUSIC_LAYOUT are consumed only by variants.test.tsx (rg file lists) — test-only exports through the production barrel (same pattern as menu's resetProfilesForTests). Why it matters: production API surface existing only for tests. What correct looks like: import from the defining module in tests, or a test-only seam. Confidence: HIGH — Verification: read.

## 4. Per-function table
Not applicable (barrel, 0 functions). All 18 exports individually traced: 7 production-consumed (MiniPlayer, MusicMode, PlayerVariant, loadPersistedVariant, persistVariant, resolveVariantLayout, PLAYER_SHELL_KEYMAP, VARIANT_KEYMAP, isPlayerVariant, parsePlayerVariant — via PlayerScreen/ShortcutsScreen/player index), 5 dead (F-001), 4 test-only (F-002), 2 internal-use-only re-exported (mergeKeymaps counted in F-001).

## 5. Test honesty
No tests in this barrel. variants.test.tsx (33 tests, baseline :32) exercises the exported variant logic through the barrel — the barrel's importability is therefore test-covered.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Card 4.F2 owns … variants/** (Miniplayer + Music mode; variant model, … persistence)" | index.ts:1-3 | REAL | all five claimed concerns exist as exported modules (variant.ts, variantLayout.ts, variantKeymap.ts, MiniPlayer.tsx, MusicMode.tsx) |

## 7. Dead / duplicated / unreachable code
F-001's five dead exports; player/index.ts:40-53 re-exports 13 of these symbols, 3 of them into nothing. MiniPlayer/MusicMode themselves are live (PlayerScreen.tsx:914,1002).

## 8. Dependencies & flags
No imports — pure re-export. No feature gates. variants.css is NOT exported (imported by the components internally).

## 9. Open questions for the next agent
Is PLAYER_VARIANTS intended for a future variant-switcher UI? If yes it should carry a [staged] marker like the other seams; today it is simply dead.
