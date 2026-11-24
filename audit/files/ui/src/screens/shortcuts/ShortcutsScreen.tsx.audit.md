# AUDIT: ui/src/screens/shortcuts/ShortcutsScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 511 / 12 (1 component + 11 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:62-63 lazy import -> App.tsx:306 `<Route path="/keyboard-shortcuts" ...>` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg section/disabled/platform-override counts (exit 0); baseline ui-test.txt cited (14 tests pass) |

## 1. One-paragraph verdict
A read-only shortcuts reference dialog built from REAL generated keymap data (HOTKEY_BINDINGS, the generated mirror of keymaps/default.toml with a drift guard `npm run hotkeys:check`), with a working search + scope filter and a numeric-aware sort for numbered series. The two frame-dead affordances are honestly inert: Save renders `disabled` ("read-only by design", L504-506) and the Global Hotkey column's pencil is `aria-hidden` decorative because the keymaps model carries no global bindings — neither is theater. Real defect: macOS platform overrides exist as generated data (`HOTKEY_PLATFORM_OVERRIDES`, bindings.ts:174-184) but NOTHING consumes them, so a macOS user sees Windows/Linux combos; the screen's `macosSectionGroup` helper is dead code because no binding section ends in `-macos`.

## 2. What actually works (verified)
— Data is real and generated: bindings.ts:1-3 `// GENERATED FILE — do not edit ... Source of truth: keymaps/default.toml ... Regenerate: npm run hotkeys:generate  Drift guard: npm run hotkeys:check`; all 141 bindings carry section ids that all map into the four groups (rg section counts: advanced 22, audio 7, media 18, navigation 35, playlist 8, subtitles 12, video 27, window 12 — every section present in SECTION_TO_GROUP, L108-117).
— Baseline vitest: "✓ src/screens/shortcuts/ShortcutsScreen.test.tsx (14 tests)" (audit/runs/baseline/ui-test.txt:52).
— Edit pencil routes to a real route: L472-478 `to="/settings/hotkeys"`; App.tsx:322 serves `/settings/:area`.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/shortcuts/ShortcutsScreen.tsx:119-131 + generated/bindings.ts:174-184
Code:
```
/** Platform-variant rows (section "<base>-macos") map by their base action. */
function macosSectionGroup(section: string): ShortcutGroupId | undefined {
  const base = section.replace(/-macos$/u, "");
```
What is wrong: dead code + a data seam nobody consumes. No binding in the generated data has a section ending `-macos` (rg over bindings.ts: sections are exactly advanced/audio/media/navigation/playlist/subtitles/video/window), so `macosSectionGroup` never fires. Meanwhile the real macOS overrides live in a separate export `HOTKEY_PLATFORM_OVERRIDES` (bindings.ts:174-184, e.g. `{ platform: "macos", id: "fullscreen-macos", key: "Cmd+F" }`) which has ZERO production consumers (rg: only bindings.ts itself, non-test). A macOS user of this screen sees the non-macOS combos; the overrides data ships unused. Why it matters: reference screen shows wrong combos on macOS; dead helper misleads maintainers into thinking the case is handled. What correct looks like: resolve rows through HOTKEY_PLATFORM_OVERRIDES on darwin, delete or exercise macosSectionGroup. Confidence: HIGH — Verification: read + rg counts.

F-002 [S4]
Where: ui/src/screens/shortcuts/ShortcutsScreen.tsx:248-249
Code:
```
    if (binding.disabled) continue; // feature-gated rows are not reference rows
```
What is wrong: currently a no-op guard — zero bindings carry `disabled: true` (rg -c returns no match; all 141 rows are `disabled: false`). Defensive only; keep but note. Confidence: HIGH — Verification: rg count.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useShortcutsMsg | 78-84 | REAL | - | lane-key fallback resolver |
| ShortcutRow/Group types + SECTION_TO_GROUP | 90-117 | REAL | - | all 8 real sections mapped; no silent drops |
| macosSectionGroup | 119-126 | DEAD | S3 | F-001 |
| groupOfHotkey | 128-131 | REAL | - | -macos half dead (F-001) |
| shellRow + SHELL_BINDING_LABELS | 133-145 | REAL | - | 5 shell seed rows, labels reuse catalog keys |
| KEYCAP_LABELS / keycapText / keyChips / keycapCombo | 156-235 | REAL | - | display-only map, verbatim fallback (never invented) |
| buildShortcutGroups | 238-263 | REAL | - | pure, test-pinnable; disabled-guard no-op today (F-002) |
| labelPrefix / trailingNumber / sortRowsNumericAware | 265-301 | REAL | - | numeric series ordering; stable within runs |
| rowMatches / filterGroups | 303-340 | REAL | - | scope filter matches label, raw combo, friendly caps |
| ShortcutsScreen | 349-511 | REAL | - | msgRef memo pin (L362-366) documented; focus on search field; exit -> /player |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ShortcutsScreen.test.tsx (14, baseline pass) | YES for grouping/search/sort | pure helpers (buildShortcutGroups/filterGroups/sortRowsNumericAware) are exported and directly tested; DOM tests assert sections/rows |
| macOS combo correctness | NO TEST EXISTS | no test consumes HOTKEY_PLATFORM_OVERRIDES — F-001 invisible to the suite |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "built from REAL keymap data" | L4-9 | TRUE | HOTKEY_BINDINGS generated from default.toml w/ drift guard (bindings.ts:1-3) |
| "Save ... disabled — this reference screen is read-only by design" | L32-34 | TRUE | L504-506 `disabled` on the Save button |
| "the keymaps model has no global-hotkey bindings ... Global Hotkey keycaps render empty" | L36-39 | TRUE | L486-490 empty keycap + aria-hidden decorative pencil |

## 7. Dead / duplicated / unreachable code
- macosSectionGroup / the `-macos` branch of groupOfHotkey (F-001).
- disabled-row guard (F-002, currently no-op).
- No duplication.

## 8. Dependencies & flags
No feature gates. Depends on generated bindings (hotkeys area), player keymap seed, variants keymap, Dialog/Dropdown primitives.

## 9. Open questions for the next agent
- Does the hotkeys EDITOR (/settings/hotkeys) consume HOTKEY_PLATFORM_OVERRIDES? If not, macOS is wrong there too — worth a lane-wide finding.
