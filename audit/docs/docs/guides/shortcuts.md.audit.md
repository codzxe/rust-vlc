# CLAIM AUDIT: docs/guides/shortcuts.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A04) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Unusually honest guide: it refuses to duplicate the keymap ("source of truth
this page deliberately does not duplicate") and every mechanism it names is
real — 150 bindings exactly, generators wired, sections present, typed refusal
error real. Only the "verified against pinned macOS run" items stay [VERIFY]
in the keymap itself.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "`keymaps/default.toml` (150 bindings, task 0.B3)" | :6 | REAL | `rg -c '^[a-z0-9-]+ = \{ key = ' keymaps/default.toml` -> 150 |
| 2 | "hotkey editor render from data generated ... (`npm run hotkeys:generate`)" | :10-11 | REAL | ui/package.json:26-27 hotkeys:generate + hotkeys:check (`node scripts/gen-hotkeys-data.mjs`) |
| 3 | Player core table: Space/S/./Shift+S/Ctrl+T/Ctrl+L ... | :17-35 | REAL | keymap:83 `toggle-play-pause = { key = "Space"`, :84 `stop = { key = "S"`, :94 `frame-next = { key = "."`, :161 `snapshot = { key = "Shift+S"`, :120 `jump-to-time = { key = "Ctrl+T"`, :207 `panel-toggle = { key = "Ctrl+L"` |
| 4 | "marked [VERIFY] against a pinned macOS run" overrides in `[platform_overrides.macos]` | :37-39 | REAL | keymap:267 `[platform_overrides.macos]` + header "[VERIFY] against a pinned macOS run" |
| 5 | "feature-gated in v1 ... typed `feature-not-exposed` error, never a dead key" | :41-46 | REAL | crates/commands/src/error.rs:122 `ErrorCode::FeatureNotExposed => "feature-not-exposed"` (byte-checked) |
| 6 | "remaining sections (`[playlist]`, `[window]`, `[advanced]`, ~150 bindings total)" | :48-49 | REAL | keymap:201/:215/:234 sections exist; 150 counted exactly |
| 7 | CLI verb list "from `lumen --help`" | :54-58 | REAL | apps/cli/src/parse.rs:208 `"engine-info" =>`, help.rs:83 `"decode-bench <path>"`, :88 `"advance [ticks]"`; baseline rc-smoke engine-info PASS |
| 8 | "provenance ... `docs/inventory/ui-inventory.md` §5–§8" | :62-65 | REAL | ui-inventory.md:200/346/402/426 — §5 hotkeys, §6 crosswalk, §7 conflicts, §8 VERIFY ledger |
| 9 | "`hotkeys:check` generator gate fails on drift" | :66-67 | REAL-UNPROVEN | package.json:27 gate exists (--check mode); drift-failure behavior not executed (cargo/npm budget) |
| 10 | Media keys "bound as VLC's alternates" | :46-47 | REAL | keymap:61 `media-key-play-pause = { key = "MediaPlayPause"` |

## 3. Contradictions
None found.

## 4. Fabrication / inflation indicators
None — the doc's central design is *not* duplicating data it can't verify;
counts reconcile exactly (150). ShortcutsScreen.tsx + test exist at the named
route. Findings: none.
