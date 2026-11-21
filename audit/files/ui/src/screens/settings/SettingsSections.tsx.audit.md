# AUDIT: ui/src/screens/settings/SettingsSections.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 244 / 7 (SettingsSections, AdvancedDisclosure, AdvancedRows, partition, schemaSectionsOf, withRows, isSettingEntry) + 3 interfaces |
| File verdict | REAL for the generated-screen path (SettingsSections/AdvancedDisclosure/partition/schemaSectionsOf); STUB-dead for the hand-row half (withRows/SettingRowNode — zero consumers in the entire repo, tests included) |
| Reachable from shipped entry point? | YES as web-UI source: `App → /settings/:area → generated/Screens.tsx` renders `<SettingsSections>` for the generated areas (10 call sites at Screens.tsx:127-307); `AdvancedDisclosure` additionally imported by areas/index.tsx:9. Real shell: no GUI binary (W14). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `grep -rn "withRows\|SettingRowNode\|data-setting-row" src/` (withRows: ZERO hits outside this file; data-setting-row produced by areas/video.tsx:137, audio.tsx:268, privacy.tsx:128 — hand-composed, not via the API); `sed -n '70,110p' areas/shared.tsx` (the parallel split implementation); `grep -c "featureExposed: true/false" generated/schema.ts` (117/4); `grep -n "aria-controls\|advanced" SettingsScreen.test.tsx` (disclosure contract tested :146,:371-385); baseline #22/#23/#24 |

## 1. One-paragraph verdict
The live half of this file is good work: the Advanced Options disclosure is a single exported implementation with a real dedup history (three copies collapsed in w26.k12, the codecs copy having shipped the "expanding reveals nothing" defect), an aria-controls/aria-expanded contract asserted by tests, a guarded scrollIntoView, and a frame-accurate "bar only where advanced content exists" rule. The dead half is worse than absent: `withRows`, the `SettingRowNode` union arm, and the hand-row render branch (lines 76-93, 225-227) have ZERO consumers anywhere — the actual hand rows (video's "Reset picture", audio's Output device, privacy's "Clear history") are hand-composed in areas/*.tsx with plain `data-setting-row` divs, and `areas/shared.tsx` carries its OWN duplicate of the featureExposed+advanced split. The exported API describes a composition pattern the codebase abandoned.

## 2. What actually works (verified)
— verified by `grep -rn "SettingsSections" generated/Screens.tsx` → 10 mounts, all pure schema sections; baseline #24 (`npm run settings:check` PASS) → Screens.tsx is drift-checked against schemas/settings.toml.
— verified by SettingsScreen.test.tsx:146/:371-385 (baseline 1705-pass) → advanced segregation + `aria-controls="settings-advanced-rows"` + revealed-region identity asserted — the disclosure contract is test-pinned.
— verified by `grep -rn AdvancedDisclosure areas/index.tsx` → :9 imports THIS export — the k12 one-implementation claim holds (no hand copies remain: grep for `settings-advanced__bar` producers finds only this file).
— verified by schema counts → 117/4 featureExposed, 32 advanced: `partition`'s filters operate on real schema data (the 4 hidden entries are schema.ts's audit — my asset-group file).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/settings/SettingsSections.tsx:23-30, 76-93, 221-228
Code:
```
export const withRows = (
  sections: readonly GeneratedSection[],
  sectionId: string,
  rows: readonly { after?: string; row: SettingRowNode }[],
): readonly GeneratedSection[] => {
```
What is wrong: `withRows` and the `SettingRowNode` half of the union have zero callers in the entire repository — not from the areas (video/audio/privacy hand-compose their rows with literal `data-setting-row` divs at video.tsx:137, audio.tsx:268, privacy.tsx:128), not from generated/Screens.tsx (pure schema sections), not from any test file. The render branch at :225-227 (`<div key={row.rowId} data-setting-row={row.rowId}>{row.node}</div>`) is unreachable code. Worse, the dead API misdescribes reality: the header comment (:57-63) says "the hand compositions … interleave the frame's non-schema rows into these arrays" — they do not; they bypass this file's row pipeline entirely via areas/shared.tsx.
Why it matters: a future contributor extending a settings area will find this exported splice API, use it, and get rows silently DROPPED — because `partition` (:191) filters `isSettingEntry(row) && row.featureExposed`, hand rows fail the first clause and never reach `normal` or `advanced`. The dead API is not just dead; if resurrected as documented, it eats rows.
What correct looks like: delete withRows/SettingRowNode/`GeneratedSection`'s union (make it SchemaSection's shape) and the :225-227 branch, OR make `partition` pass hand rows into `normal` and add the test that pins it.
Confidence: HIGH — Verification: three greps (withRows, SettingRowNode, data-setting-row) over all of src/ including tests; shared.tsx + three area files read.

### F-002 [S4]
Where: ui/src/screens/settings/SettingsSections.tsx:188-196 vs areas/shared.tsx:74-106
Code:
```
  const visible = rows.filter((row) => isSettingEntry(row) && row.featureExposed);
  return {
    normal: visible.filter((row) => !isSettingEntry(row) || !row.advanced),
    advanced: visible.filter(isSettingEntry).filter((entry) => entry.advanced),
```
What is wrong: the featureExposed+advanced split exists twice — `partition` here (for generated screens) and `schemaSectionViews`/`partitionAdvanced` in areas/shared.tsx (for composed areas) — acknowledged in search.ts:13 but unfixed. The two agree today; the next policy change (e.g. "advanced entries also feature-gated", or a new flag) must be made twice.
Why it matters: generated and hand-composed areas could silently diverge on which rows render — a settings screen grammar split across two implementations of one rule.
What correct looks like: shared.tsx re-exports/uses this file's partition (it already imports schemaSectionsOf from here).
Confidence: HIGH — Verification: read of both.

### F-003 [S4]
Where: ui/src/screens/settings/SettingsSections.tsx:156 and 166-170
Code:
```
        aria-controls={ADVANCED_ROWS_ID}
```
What is wrong: when the disclosure is collapsed, the rows region (`id={ADVANCED_ROWS_ID}`) is not rendered (:166 `{open ? … : null}`), so `aria-controls` points at a nonexistent node for the collapsed state — the only state in which an AT user is deciding whether to expand. Browsers/AT resolve the target only when it exists; the reference is dangling exactly when it would be informative.
Why it matters: minor a11y contract weakness on every settings area with advanced rows (32 schema entries); tests assert the expanded-state wiring only.
What correct looks like: render the rows container always (hidden via `hidden` attribute when collapsed), keeping one live node for aria-controls — also improves expand performance perception (no remount).
Confidence: HIGH — Verification: read + test assertions at SettingsScreen.test.tsx:371-385 (expanded state only).

### F-004 [S4]
Where: ui/src/screens/settings/SettingsSections.tsx:85-86
Code:
```
      const at = after === undefined ? -1 : out.findIndex((e) => "key" in e && e.key === after);
      out.splice(at < 0 ? out.length : at + 1, 0, row);
```
What is wrong: an unknown `after` key (typo in a schema key) silently appends the row at the section END instead of failing — a placement bug that would ship invisibly. Moot while the API is dead (F-001) but a trap the moment it's resurrected.
What correct looks like: dev-mode console.warn or throw on unresolved `after`.
Confidence: HIGH — Verification: read; splice semantics traced (findIndex -1 → out.length).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SettingRowNode` / `GeneratedSection` / `SchemaSection` (interfaces) | 23-52 | HALF-DEAD | S3 | SchemaSection live (shared.tsx); SettingRowNode dead (F-001); the type-split commentary (:38-47) is accurate history |
| `isSettingEntry` | 54 | REAL | — | `"key" in row` guard |
| `schemaSectionsOf` | 65-74 | REAL | — | live (shared.tsx:23); WIDGET_GROUPED_KEYS filter; unknown id → [] |
| `withRows` | 77-93 | STUB (dead) | S3/S4 | zero callers; silent-append trap F-004 |
| `AdvancedDisclosure` | 126-173 | REAL | S4 | one implementation (k12 verified); aria-controls dangling when collapsed F-003; scrollIntoView guarded for jsdom |
| `AdvancedRows` | 176-184 | REAL | — | flat SettingField map |
| `partition` | 188-196 | REAL (generated path) | S3 | drops hand rows by construction (F-001's resurrect-trap); duplicates shared.tsx (F-002) |
| `SettingsSections` | 198-244 | REAL | — | 10 generated mounts; header-drop for all-advanced sections documented frame-faithful; disclosure only when advanced entries exist (:237) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| SettingsScreen.test.tsx advanced-segregation + aria contract (:146, :371-385) | YES | asserts rows land behind the disclosure and the id wiring — pins partition + disclosure |
| baseline #24 `npm run settings:check` | YES (drift) | generated Screens.tsx/schema.ts diff vs schemas/settings.toml |
| (absence) no test touches withRows/SettingRowNode | — | the dead half is also untested — the strongest evidence it was never used |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the hand compositions (areas/video · areas/audio · areas/privacy) interleave the frame's non-schema rows into these arrays" | SettingsSections.tsx:57-59 | FALSE | they hand-compose with literal divs; withRows unused (F-001) |
| "w26.k12: exported — ONE implementation [AdvancedDisclosure]" | SettingsSections.tsx:116-122 | REAL | areas/index.tsx:9 imports it; grep finds no other producer of `settings-advanced__bar` |
| "Schema entries flagged `advanced` are segregated behind that disclosure" | SettingsSections.tsx:100-101 | REAL | partition + test :146; 32 advanced entries in schema |

## 7. Dead / duplicated / unreachable code
Dead: `withRows`, `SettingRowNode`, hand-row render branch (:221-228's else arm). Duplicated: partition vs areas/shared.tsx schemaSectionViews/partitionAdvanced (F-002).

## 8. Dependencies & flags
generated/schema.ts (SETTINGS_AREAS/SettingEntry — my asset-group file 27), SettingWidgets (WIDGET_GROUPED_KEYS), SettingField, settingsMsg. No flags.

## 9. Open questions for the next agent
- The 4 `featureExposed: false` schema entries (hidden from ALL settings UI by partition/shared.tsx) — which are they, and does any doc claim them as available settings? Schema.ts audit (my file 27) will list them; docs wave should cross-check the settings matrix rows.
- If F-001's API is deleted, `GeneratedSection` collapses into `SchemaSection` — one more round of k12-style dedup for the areas wave.
