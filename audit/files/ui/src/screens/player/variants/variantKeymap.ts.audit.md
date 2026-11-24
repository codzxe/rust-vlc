# AUDIT: ui/src/screens/player/variants/variantKeymap.ts
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 47 / 1 (mergeKeymaps) + 3 module constants |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — PLAYER_SHELL_KEYMAP is the player shell's effective keymap (via player/index.ts barrel; MiniPlayer/MusicMode toggles) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; keymap-claim spot-check (no variant rows in keymaps/default.toml) |

## 1. One-paragraph verdict
A small, honest seam: two seed TOML rows (B=miniplayer-toggle, G=music-mode-toggle) parsed with the REAL keymap parser at module load, merged over the default keymap with a loud duplicate-id guard (throws, never silently shadows). The [PROPOSAL] header is accurate — the bindings are code-seeded because the data file has none yet. Variant toggles are UI-owned state (no engine command), correctly not pretending otherwise.

## 2. What actually works (verified)
— verified by read: parseKeymap over the embedded TOML (real parser, not a regex stub — keymap.ts is another lane's file but the import is the real module); mergeKeymaps duplicate throw.
— spot-check: keymaps/default.toml carries no miniplayer/music rows (rg) — the [PROPOSAL] claim holds.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/variants/variantKeymap.ts:12-16
What is wrong: hardcoded key choices ("B"/"G") live in code until the keymap loader card lands — a documented temporary inversion of the data-driven principle. Confidence: HIGH — Verification: read (self-admitted).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| VARIANT_KEYMAP_TOML/VARIANT_KEYMAP | 20-28 | REAL | seed data, parsed by real parser |
| mergeKeymaps | 33-42 | REAL | loud duplicate guard |
| PLAYER_SHELL_KEYMAP | 45-47 | REAL | effective shell keymap |

## 5. Test honesty
Covered by variants lane tests (not in my list); UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keymaps/default.toml defines NO variant-toggle binding today (verified)" | header L4-5 | REAL | rg confirms absence |
| "the v1 command catalog has no layout-switch command (it is UI-owned state)" | header L5-6 | REAL | no such command in the 65-cmd union |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
keymap.ts parser. No env vars.

## 9. Open questions for the next agent
- None.
