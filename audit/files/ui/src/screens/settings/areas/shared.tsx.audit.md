# AUDIT: ui/src/screens/settings/areas/shared.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 5 (makeAreaMsg+inner hook, schemaSectionViews, partitionAdvanced; types LaneKeyFile, SchemaSectionView) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the dev/test shell (call chain: SettingsScreen → areas/index.tsx registry → all 13 area files → makeAreaMsg/schemaSectionViews/partitionAdvanced; 21 import sites verified). NO shipped binary exists at all — W14 pin: apps/desktop-tauri app.rs:301 E0425, tauri build UNVERIFIED (baseline INDEX #2). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `rg -n "makeAreaMsg\|schemaSectionViews\|partitionAdvanced\|LaneKeyFile" ui` (21 consumer sites, all inside areas/); `rg -n "useMsg" ui/src/i18n/messages.ts` (fallback chain); sed 42-50/80-92/97-107 (quote verification); lane-file duplicate-key scan across all 5 merge sets (none); `rg "settings.window-title.interface" messages.ts` (registered: line 2306) |

## 1. One-paragraph verdict
Genuine, consumed consolidation layer: the seam-hook factory and the two section-splitting helpers are imported by every schema-driven area and exercised by the vitest baseline (1705 passing, INDEX #23). No fakeness, no swallowed errors, no perf hazards — the module is pure functions plus one hook wrapper. All four findings are latent hygiene: an unguarded last-wins lane-file merge, a type-level hole in the resolver's `Partial` contract, a stringly-typed area id that fails silent-empty, and a needless export. The header's "nothing outside areas/ may import it" boundary claim is TRUE today (verified: the only `../shared` imports outside areas/ resolve to the distinct ui/src/screens/player/tools/shared.tsx module).

## 2. What actually works (verified)
- All 21 import sites of `./shared`/`../shared` (areas layer) enumerated by rg — every one inside ui/src/screens/settings/areas/**; consumers mounted via the settings nav registry (areas.test.tsx:87-251 renders interface/global/video/audio/playback through these helpers).
- Fallback chain verified end to end: `useMsg` (messages.ts:3011-3016) is `catalog[key] ?? messages[key]`; for an unregistered lane key both lookups are undefined at runtime, so `m(key as MessageKey) ?? catalog[key]` (shared.tsx:48) reaches the lane file; once the Lead registers the key (spot-check: "settings.window-title.interface" now at messages.ts:2306), the catalog wins — the documented catalog-first ordering is real, not aspirational.
- No duplicate keys across any merged lane-file set (checked w26.b01+b02+f01; b01+b02+b07+gfixB+i15; gfixB+i14; c1+c5) — the Object.assign last-wins hazard is latent, not active.
- schemaSectionViews' filtering (featureExposed + hiddenKeys) matches the shell's own partition (SettingsSections.tsx:188-196) — same feature gate, same advanced split; WIDGET_GROUPED_KEYS already excluded upstream by schemaSectionsOf (SettingsSections.tsx:72).
- Baseline vitest 1705/1705 pass (INDEX #23) — areas.test.tsx renders the screens using these helpers.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/shared.tsx:45
Code:
```ts
  const catalog: LaneKeyFile = Object.assign({}, ...files) as LaneKeyFile;
```
What is wrong: Merging lane key files with `Object.assign` is silent last-wins: if two files in one `makeAreaMsg(...)` call ever define the same key with different strings, the earlier file's copy vanishes with no error, no warning, no test.
Why it matters: The seam exists precisely because ~10 parallel lanes add keys independently; a cross-lane key collision is the expected failure mode of that protocol, and this merge is where it would land invisibly (wrong English string rendered, nothing fails).
What correct looks like: A dev-time assertion (or unit test) that the merged key sets are disjoint before assigning; at minimum a comment documenting the last-wins rule.
Confidence: HIGH — Verification: read; duplicate-key scan of all 5 active merge sets run today (zero collisions — latent only).

### F-002 [S4]
Where: ui/src/screens/settings/areas/shared.tsx:43-48
Code:
```ts
  ...files: Readonly<Partial<Record<K, string>>>[]
): () => (key: MessageKey | K) => string {
```
What is wrong: The parameters are `Partial`, so the type system permits a lane file to declare key `k: K` in its type while omitting it at runtime; `catalog[key]` (line 48) is an untyped `Record<string,string>` index, so the resolver then returns `undefined` typed as `string`. The `??` on line 48 is also dead per types (`string ?? string`) — it survives only because `m(key as MessageKey)` is a blind cast, the same pattern recorded as Screens.tsx F-001.
Why it matters: The factory's advertised "typo safety" (header lines 39-40) is not total: a Partial-shaped or stale lane file type-checks and yields `undefined` into JSX children at runtime.
What correct looks like: Non-`Partial` rest params (`Readonly<Record<K, string>>[]`) so K is inferred only from keys actually present, plus a `?? ""` (or explicit undefined widen) so the return type is honest.
Confidence: HIGH — Verification: read; all current lane files are complete `as const` objects (sampled w26.b02, keys/README pattern), so the hole is untriggered today.

### F-003 [S4]
Where: ui/src/screens/settings/areas/shared.tsx:73-79
Code:
```ts
export function schemaSectionViews(
  area: string,
```
What is wrong: `area` is a bare `string`; `schemaSectionsOf` (SettingsSections.tsx:65-68) returns `[]` for an unknown id, so a typo'd call site renders a completely empty settings screen with no error at type or runtime.
Why it matters: Seven call sites pass literals ("privacy", "audio", "performance", "playback", "interface", "video", "global") — one rename in schemas/settings.toml plus a missed edit here ships a blank area silently.
What correct looks like: `area: SettingAreaId` (union of SETTINGS_AREA_IDS) or a dev-time invariant when `schemaSectionsOf` finds nothing.
Confidence: HIGH — Verification: read; all 7 call-site literals enumerated via rg and matched against the registry test (areas.test.tsx:44-54 pins the 13 ids).

### F-004 [S4]
Where: ui/src/screens/settings/areas/shared.tsx:27
Code:
```ts
export type LaneKeyFile = Readonly<Record<string, string>>;
```
What is wrong: `LaneKeyFile` is exported but has zero external consumers (rg: only shared.tsx:27 and its own line-45 use) — dead surface on a module whose header promises "pure functions + hooks only" as its API.
Why it matters: Exported names are the module's contract; an unused export invites import (and coupling) the layer's own comment forbids.
What correct looks like: Drop the `export` keyword; keep the local alias.
Confidence: HIGH — Verification: rg over ui/ — no other file names `LaneKeyFile`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LaneKeyFile` (type) | 27 | REAL | S4 | dead export (F-004); erases key-type info by design |
| `makeAreaMsg` | 42-50 | REAL | S4 | pure factory; Object.assign last-wins unguarded (F-001); Partial hole (F-002); zero-arg call degenerates to `{}` (no call site does) |
| `useAreaMsg` (returned hook) | 46-49 | REAL | — | calls useMsg() each render (correct hook usage); catalog-first fallback verified live; returns closure (no memoization needed — trivial) |
| `SchemaSectionView` (interface) | 53-64 | REAL | — | pure data shape; doc comment accurate (no search filter — shell-side swap verified in SettingsSections.tsx:16-21) |
| `schemaSectionViews` | 73-92 | REAL | S4 | stringly area param (F-003); O(entries×hidden) filter — hidden ≤ handful, fine; pure, no hooks despite living in a .tsx (by design, header line 17-18) |
| `partitionAdvanced` | 97-107 | REAL | — | drops all-advanced sections from `sections` but keeps their rows in the flat `advanced` list — matches global-1.svg grammar; two allocations per call, called per render of 2 screens — negligible |

## 5. Test honesty (if tests exist in/for this file)
No shared.test.* file exists; coverage is indirect.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| areas.test.tsx:63 "area titles resolve via the catalog or the lane keys" | YES (for the seam half) | asserts `isMessageKey(key) || key in laneKeys` for all 13 titles — breaks if the merge loses a key |
| areas.test.tsx:87-251 (renders interface/global/video/audio/playback, ⓘ counts, merged-area nav) | YES | these render THROUGH schemaSectionViews/partitionAdvanced; deleting the feature-gate filter or the advanced split changes the rendered row sets the tests pin |
| Mutation check (reasoned, not run): make `partitionAdvanced` also drop all-advanced sections from `advanced` | would FAIL areas.test.tsx:115 ("Advanced bar is LAST; About rides behind the disclosure") | advanced-row counts are pinned per area |
| Mutation check: reverse catalog-first to lane-first in line 48 | NOT caught by any test | no test asserts a registered key beats its lane copy (the 26 window-title keys are registered AND still in lane files — a lane-first flip would silently change copy if strings ever diverge; today they are identical) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "nothing outside areas/ may import it" | shared.tsx:7-8 header | REAL | rg: all 21 `./shared`/`../shared` importers are under areas/; the two `../shared` hits in player/tools/ resolve to a different module (tools/shared.tsx) |
| "the ten-copy dedup" / "six near-identical loops" | shared.tsx:2-5 header | UNVERIFIED | historical pre-refactor state no longer in tree; count cannot be re-derived |
| "no search filter … the shell renders cross-area results" | shared.tsx:10-15, 57-58 | REAL | SettingsSections.tsx:16-21 documents the same removal; search.ts:44 walks schema entries only |
| "Exported here: pure functions + hooks only (no components)" | shared.tsx:17-18 | REAL | verified: no JSX, no component exports in 107 lines |

## 7. Dead / duplicated / unreachable code
- `LaneKeyFile` export (F-004). No duplicated logic remains: the six per-area loop copies are gone (consumers now call schemaSectionViews); `SettingsSections.tsx`'s own `partition` (lines 188-196) is the shell-side analogue, not a copy (different row type: SettingSectionRow vs SettingEntry).

## 8. Dependencies & flags
Imports: i18n/messages (useMsg, MessageKey), generated/schema (SettingEntry type), SettingsSections (schemaSectionsOf, SchemaSection). No feature flags, no engine/bridge imports — the module cannot touch the backend, consistent with its scope. No new deps.

## 9. Open questions for the next agent
- The catalog-vs-lane drift mutation (§5, last row) is unguarded by tests — worth one assertion in a future lane: a key present in BOTH messages.ts and a lane file must render the catalog string.
- performance.tsx (next file) consumes partitionAdvanced — confirm its Advanced bar is last, as areas.test.tsx:115 pins for global.
