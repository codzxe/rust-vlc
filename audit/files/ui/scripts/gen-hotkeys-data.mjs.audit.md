# AUDIT: ui/scripts/gen-hotkeys-data.mjs
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 166 / 3 exported functions + main + run-guard |
| File verdict | REAL |
| Reachable from shipped entry point? | Tooling (`npm run hotkeys:generate` / `hotkeys:check`), EXECUTED in baseline: ui-hotkeys_check.log.txt "OK (141 bindings, 9 platform overrides) EXIT=0" (INDEX.md row 24) and AGAIN inside the vitest pass (hotkeysData.test.ts runs the --check via execFileSync). Its OUTPUT (generated/bindings.ts) is production-consumed by PlayerScreen, ControlBar, ShortcutsScreen, OnboardingScreen, HotkeysEditor. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | sed/od verification of keymaps/default.toml sections; rg consumer census for HOTKEY_BINDINGS / HOTKEY_PLATFORM_OVERRIDES; keymap.ts read (matcher seam); baseline log citation; bindings.ts row count (141). |

## 1. One-paragraph verdict
The hotkeys data generator: parses keymaps/default.toml (via the shared TOML-subset parser), emits the committed bindings.ts (141 bindings + 9 macOS overrides — counts verified against the committed output and the baseline check log), and guards drift twice (the npm gate AND an in-vitest execFileSync re-run — belt and suspenders, both green at HEAD). The model builder is strict (duplicate binding ids throw; non-binding rows skipped by shape, not name), the CLI has test seams (--input/--out), and the isDirectRun guard makes the module importable for unit tests. Two S4s: the sort feeding the byte-compare guard uses locale-dependent `localeCompare` (a determinism smell in a drift gate — stable for today's all-lowercase-ASCII ids, but nothing enforces that), and the generated HOTKEY_PLATFORM_OVERRIDES array has zero production consumers — the macOS Cmd-key overrides never reach any display or the keystroke matcher, so a macOS build would show and match Linux keys. Also noted as a cross-ref: the runtime MATCHER (player/keymap.ts) ignores this generator's output entirely — it runs on a frozen verbatim 3-binding excerpt of default.toml ([PROPOSAL] disclosed in that file), so display data and match behavior can drift independently.

## 2. What actually works (verified)
- Executed evidence: baseline ui-hotkeys_check.log.txt → "gen-hotkeys-data --check: OK (141 bindings, 9 platform overrides) EXIT=0" (row 24); committed bindings.ts carries exactly 141 `id:` rows (rg count) — committed == regenerated at HEAD.
- Double drift enforcement: hotkeysData.test.ts:22-37 runs `node scripts/gen-hotkeys-data.mjs --check` from inside vitest (execFileSync, status propagated) — the gate holds even if only `npm test` runs.
- Model strictness: duplicate id → throw (:43); shape-based row detection (`typeof row["key"] !== "string"` skip — the `schema = "keymap.v1"` row excluded by shape, not a name special-case); platform_overrides excluded from editor rows and surfaced separately (:38, :60-73).
- Field mapping: key/label-msgid/command/recipe/ui_action/mode/when/disabled — matches the HotkeyBinding interface emitted (:85-106) and the keymaps file's inline-table shape (spot-read macos rows: Cmd+F etc.).
- Determinism (in practice): sort by section then id (:74); all current sections/ids are lowercase ASCII (media/navigation/audio/video/subtitles/playlist/window/advanced) — order stable across locales today (python check: codepoint sort == default sort for the section set).
- Test seams: --input/--out (:130-133) let tests point at fixture TOMLs; isDirectRun (:160) keeps main() out of import paths.
- labelMsgid fallback `hotkeys.${id}` (:50) — the l10n:check gate (row 24, PASS) validates catalog coverage of emitted msgids.

## 3. Findings
### F-001 [S4]
Where: ui/scripts/gen-hotkeys-data.mjs:74
Code:
```
  rows.sort((a, b) => (a.section === b.section ? a.id.localeCompare(b.id) : a.section.localeCompare(b.section)));
```
What is wrong: `localeCompare` without an explicit locale is environment-collation-dependent. The emitted bytes feed a byte-compare drift guard (--check, plus the in-vitest re-run) — if any two ids' relative order is collation-sensitive (punctuation vs letters, case mixtures), regeneration on a machine with a different ICU/locale could emit a different ORDER than the committed file and false-positive the gate (or worse, two machines commit different orders).
Why it matters: The whole design of this script is "deterministic emitter + byte-compare guard" (mirroring tokens:generate); a locale-dependent sort is the one nondeterminism in the pipeline. Today's ids (lowercase ASCII + hyphens + dots) sort identically under every locale I can construct for the section set (verified for sections), but nothing constrains future keymap ids to that shape.
What correct looks like: Plain codepoint comparison (`a.id < b.id ? -1 : a.id > b.id ? 1 : 0`) or `localeCompare(..., "en")` with numeric collation off — deterministic everywhere.
Confidence: HIGH on the mechanism (read), LOW on current impact (data is stable ASCII).
### F-002 [S4]
Where: ui/scripts/gen-hotkeys-data.mjs:62-73, 117-122 (consumption census)
Code:
```
  // platform_overrides.<platform> rows patch base bindings (key overrides) —
  // surfaced as data, not editor rows.
```
What is wrong: The emitted `HOTKEY_PLATFORM_OVERRIDES` (9 macOS Cmd-key overrides, e.g. Cmd+F for fullscreen, Cmd+Shift+Left for medium seek-back) has ZERO production consumers: rg over ui/src finds only hotkeysData.test.ts:45 (an existence assertion). No display surface (HotkeysEditor, ShortcutsScreen, ControlBar accelerators, onboarding tour) applies them, and the keystroke matcher (player/keymap.ts) does not read them either — so on a macOS host the UI would display and match the Linux base keys while the data that says otherwise sits unused in the bundle.
Why it matters: "Surfaced as data" is only half a seam — the data is exported but no consumer applies it. The macOS keymap itself is carefully curated ([VERIFY]-annotated against VLC 3.0.23, keymaps/default.toml:267+); none of that work reaches a user.
What correct looks like: Either a consumer (the editor/shortcuts display picks the override by detected platform; the matcher consults them) or drop the emission until one exists.
Confidence: HIGH — Verification: rg census (test-only consumer); matcher read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| buildHotkeyModel | 32-76 | REAL | S4 | Duplicate-id throw; shape-based filtering; F-001 sort; platform split |
| emitBindingsTs | 78-125 | REAL | S4 | Deterministic emission (header/interface/rows/overrides); F-002 output unconsumed |
| main | 127-158 | REAL | — | --check byte-compare (exit 1 on drift, missing file = drifted :139-144); --input/--out seams; idempotent write |
| isDirectRun guard | 160-166 | REAL | — | Import-safe; error → exit 1 |
| (module comment) | :18 | — | — | Typo "idemponent" (idempotent) — table note only |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| hotkeysData.test.ts "passes hotkeys:check against keymaps/default.toml" (:26) | YES | Re-runs the real --check; any parse/emission/drift regression fails it |
| "carries real keymap shape" (:33) | YES | >100 bindings, all 8 sections, command/mode presence, macos overrides — shape pins, not tautologies |
| gen script's own unit coverage | cross-ref | buildHotkeyModel/emitBindingsTs are exported for tests; the --input/--out seams exist for fixture-based tests (none in ui/scripts — the in-vitest gate is the coverage) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keymaps/default.toml … READ-ONLY source of truth" | :4 | REAL | Script only reads it; output committed + drift-gated |
| "zero deps: reuses the TOML-subset parser exported by gen-settings-screens.mjs" | :5-6 | REAL | :24 import; node:* only otherwise |
| "committed generated module + byte-compare drift guard, mirroring settings:generate / tokens:generate" | :14-16 | REAL | --check byte equality; baseline row 24 PASS; double-enforced in vitest |
| "platform tables are additive, not editor rows" / "surfaced as data" | :38, :60-61 | REAL (but see F-002) | Data emitted; consumer absent |
| `--check` "exit 1 on drift" | :19 | REAL | :145-149; verified by the gate's own test plumbing |

## 7. Dead / duplicated / unreachable code
- F-002: HOTKEY_PLATFORM_OVERRIDES emission — test-only consumer.
- No duplication (the parser is shared, correctly).

## 8. Dependencies & flags
gen-settings-screens.mjs (parseToml — shared, not duplicated), keymaps/default.toml (source), generated/bindings.ts (output; consumed by 5 display surfaces). npm scripts: hotkeys:generate / hotkeys:check. No feature flags. The DISPLAY-vs-MATCHER split: player/keymap.ts:7-11 discloses that the keystroke matcher runs on a frozen verbatim excerpt of 3 bindings ([PROPOSAL] loader handoff) — keymap.ts is UNASSIGNED in w15; the display data (this generator) and match behavior can drift independently until that loader lands (cross-ref, not counted here).

## 9. Open questions for the next agent
- keymap.ts (unassigned): the frozen DEFAULT_KEYMAP_TOML excerpt + DIALOG_KEYMAP seed vs the real keymap file — if a future wave owns it, the display/matcher drift risk is theirs to file.
- HotkeysEditor.tsx (A04) should confirm whether the editor displays platform overrides (it does not consume HOTKEY_PLATFORM_OVERRIDES per my census — F-002 cross-count on their side if they treat it).
- If keymaps/default.toml ever gains non-ASCII ids or case mixtures, F-001 becomes live — the fix is one line.
