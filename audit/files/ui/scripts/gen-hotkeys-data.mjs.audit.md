# AUDIT: ui/scripts/gen-hotkeys-data.mjs
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 166 / 4 (buildHotkeyModel, emitBindingsTs, main, isDirectRun guard) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (build-time) — hotkeys:generate/hotkeys:check (package.json:26-27); output generated/bindings.ts consumed by OnboardingScreen.tsx:45, ShortcutsScreen.tsx:60, HotkeysEditor.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | node scripts/gen-hotkeys-data.mjs --check -> exit 0 "(141 bindings, 9 platform overrides)"; read hotkeysData.test.ts drift guard |

## 1. One-paragraph verdict
Small, clean generator that turns keymaps/default.toml into the hotkey editor's
data module, reusing the settings generator's TOML parser (single parser, no
duplication). Drift check executed by me: exit 0, 141 bindings / 9 platform
overrides, no drift. The generated data is genuinely consumed by three screens,
and the drift guard runs inside the vitest suite (hotkeysData.test.ts:20).

## 2. What actually works (verified)
- `node scripts/gen-hotkeys-data.mjs --check` -> exit 0, "(141 bindings, 9 platform overrides)" — committed bindings.ts matches keymaps/default.toml.
- Consumers: HOTKEY_BINDINGS imported by OnboardingScreen.tsx:45/216, ShortcutsScreen.tsx:60/239, HotkeysEditor.tsx (rg).
- Drift guard in tests: hotkeysData.test.ts:19-29 runs the --check subprocess and asserts exit 0; a second test pins shape (>100 rows, sections/modes/command ids).
- Duplicate-binding guard (line 43) throws on duplicate ids.

## 3. Findings
F-001 [S4]
Where: gen-hotkeys-data.mjs:24
Code:
```
import { parseToml } from "./gen-settings-screens.mjs";
```
What is wrong: nothing functionally; inherits the TOML-escape limitation of
the shared parser (unknown escapes pass through silently — see the settings
generator's F-001). keymaps/default.toml currently needs no escapes (drift
check passes), so this is latent-only here.
Confidence: HIGH — Verification: ran --check (self-consistent) + read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| buildHotkeyModel | 32-76 | REAL | - | dup guard; platform_overrides surfaced as data, not rows |
| emitBindingsTs | 78-125 | REAL | - | deterministic, sorted (74) |
| main | 127-158 | REAL | - | --check byte-compare, --input/--out overrides |
| direct-run guard | 160-166 | REAL | - | |

## 5. Test honesty
hotkeysData.test.ts: drift subprocess test (would fail if bindings.ts went
stale — same mutation logic as the settings generator's suite, whose
mutation cases I verified there) + shape assertions. Honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`--check` exit 1 on drift" | gen-hotkeys-data.mjs:19 | REAL | read (145-149) + subprocess test pins exit 0 |
| "mirroring settings:generate / tokens:generate" | gen-hotkeys-data.mjs:15-16 | REAL | same byte-compare pattern verified in both |
| "Hotkey bindings are DYNAMIC data ... the editor renders from the real keymap file" | gen-hotkeys-data.mjs:11-15 | REAL | 3 consumer screens (rg) |

## 7. Dead / duplicated / unreachable code
None. Reuses the settings parser rather than duplicating it (the comment
explains why player/layout.ts's parser would not do).

## 8. Dependencies & flags
Node builtins + the sibling generator's exported parseToml. Reads
keymaps/default.toml READ-ONLY; writes only generated/bindings.ts.

## 9. Open questions for the next agent
- HotkeysEditor.tsx owner: does the editor persist overrides anywhere real
  (config.set-setting / file), or is editing local-state theater? The data
  layer is real; the write path is the question.
