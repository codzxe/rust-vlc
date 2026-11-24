# AUDIT: ui/src/screens/player/variants/variant.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 70 / 6 exported fns/consts |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: PlayerScreen.tsx:589-600 loadPersistedVariant/persistVariant drive the variant state; App.tsx PlayerRoute passes the controlled variant |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg miniplayer/music in keymaps/default.toml (no match — comment claim verified); baseline ui-test.txt |

## 1. One-paragraph verdict
A tight UI-owned state model: three variants, a type guard that rejects unknown values ("never guessed"), and localStorage persistence with honest degradation on unavailable storage (try/catch on all three ops). The header's [PROPOSAL] claim that keymaps/default.toml defines no variant-toggle binding is still TRUE today (rg: zero miniplayer/music rows in default.toml) — the B/G bindings live only in the variantKeymap.ts seed, the same hand-seed pattern flagged in the keymap.ts audit, but here the comment openly says so and the keys were chosen to avoid collisions with every player-focused claim.

## 2. What actually works (verified)
— Unknown persisted values parse to null (L37-39) — a tampered `lumen.player.variant=cinema` falls back to "full", never crashes.
— Comment-vs-repo check: default.toml has no variant rows (rg, exit 0 with no matches).
— Baseline: variants tests pass (ui-test.txt).

## 3. Findings
None. Checked: total parse, storage-failure degradation on load/persist/clear, storage-key naming consistency.

## 4. Per-function table
N/A (<100 LOC): all 6 exports reviewed (PLAYER_VARIANTS, isPlayerVariant, parsePlayerVariant, VARIANT_STORAGE_KEY, loadPersistedVariant, persistVariant, clearPersistedVariant).

## 5. Test honesty
variants.test.tsx (baseline pass) covers parse/reject/persist round-trips.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keymaps/default.toml currently defines NO variant-toggle binding (verified ...)" | L18-20 | TRUE (still) | rg no matches |
| "the command catalog has no layout-switch command" | L20-21 | TRUE | generated/commands.ts: no layout cmd |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None; pure module.

## 9. Open questions for the next agent
- When the keymap loader lands, remember variantKeymap.ts's B/G rows must move to default.toml — three hand-seeded TOML excerpts (keymap.ts x2, variantKeymap.ts) currently carry the shell's real dispatch data.
