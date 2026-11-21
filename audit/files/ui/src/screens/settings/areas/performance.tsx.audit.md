# AUDIT: ui/src/screens/settings/areas/performance.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 101 / 4 (PerformanceArea, hintMsg, PERFORMANCE_HINTS const, K13Key type) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the dev/test shell (areas/index.tsx:262 `performance: PerformanceArea` registry → SettingsScreen route /settings/performance; performance.test.tsx renders it through SettingsScreen). NO shipped binary exists — W14 pin (tauri app.rs:301 E0425; baseline INDEX #2 UNVERIFIED-BUILD). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -o 'Ellipse 9[_0-9]*' "design/figma/Settings - performance.svg"` (10 circles, r="7.5" stroke="#404040" stroke-width="1.125", cy set 132/178/224/271/462/668/714/760/807/855 — comment's census EXACT); schema flag tabulation for all 15 performance entries (sed+rg loop); title-key registration check (5/5 in messages.ts); `rg -n "PerformanceArea" areas/index.tsx` (:262); settingsMsg.ts read (k13 merged, line 60); sed 46-98 quote verification |

## 1. One-paragraph verdict
Genuine, schema-driven hand render with the most honest design-comment trail audited this wave: every frame claim in the header (10 ⓘ circles, their Ellipse ids, cy coordinates, r7.5/#404040/sw1.125 geometry, the 15-row census, the advanced split 3 resources + 5 Developer) was verified against the actual SVG and schema — all exact. The one substantive defect is a resolver split: section titles go through bare `useMsg` (registered-catalog-only) while hint copy rides the `settingsMsg` lane seam, so a title key that is not yet registered — the routine pre-registration state this repo's own lane protocol produces — renders an empty `<h2>` with no test pinning header text (same defect class as generated/Screens.tsx F-001, rated S3 there). Two hygiene findings: unnecessary double-casts that defeat an already-correct union type, and a duplicated row-render block.

## 2. What actually works (verified)
- ⓘ census claim vs the real design file: `rg -o 'Ellipse 9[_0-9]*'` over `design/figma/Settings - performance.svg` → exactly 10 elements (Ellipse 9, 9_2..9_10), each `r="7.5" stroke="#404040" stroke-width="1.125"`, cy values {271,224,178,132} (Decoding), 462 (Process priority), {668,714,760,807,855} (Developer) — the comment's numbers are verbatim-true.
- Schema split verified by tabulating all 15 SETTING_PERFORMANCE_* entries: resources 3×advanced:true, decoding 4×advanced:false, smooth-playback 3×advanced:false, developer 5×advanced:true, all featureExposed:true — the rendered layout (Decoding + Smooth Playback normal; 3+5 flat behind the bar) is exactly what partitionAdvanced produces.
- All 5 section-title msgids registered in messages.ts (settings.area.performance + 4 section keys, 1 catalog entry each) — titles resolve today.
- Hint chain traced end to end: PERFORMANCE_HINTS → SettingField `hint` prop (SettingField.tsx:171-195) → `useSettingsMsg()` → settingsLaneCatalog (settingsMsg.ts:52-62, k13Keys merged line 60) — hint copy resolves even pre-registration; k13 keys are now ALSO registered in messages.ts (spot check: "settings.hint.performance.hw-decode" count 1), so both paths agree.
- Registry wiring: areas/index.tsx:262; advanced redirect `advanced: "performance"` (:279).
- Baseline vitest 1705/1705 pass (INDEX #23) — performance.test.tsx included.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/areas/performance.tsx:78,86
Code:
```tsx
  const m = useMsg();
```
```tsx
          <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
```
What is wrong: Section titles resolve through bare `useMsg` (registered catalog only — messages.ts:3011-3016 returns `catalog[key] ?? messages[key]`, undefined for an unregistered key, which React renders as nothing), while every other msgid on this screen (row labels, hints, badges via SettingField/useSettingsMsg) rides the lane-first `settingsMsg` seam. The repo's own protocol (CONTRACT §4.5, k13 lane header lines 1-4: "the Lead registers them … post-wave") guarantees msgids routinely exist unregistered; this file's hints were exactly such keys until registration. A future/re titled section key renders an EMPTY section header on this screen only.
Why it matters: The screen mixes two resolvers with different totality contracts; the area layer's own `makeAreaMsg` (shared.tsx:42-50) exists precisely to close this hole and is not used here. No test pins any performance section-header text (performance.test.tsx asserts rows, values, ⓘ census — never the `<h2>`s), so the blank-header mutation fails zero tests. Same class as generated/Screens.tsx F-001 (S3, W15-A08 file 7).
What correct looks like: Resolve titles through `useSettingsMsg()` (or the shared makeAreaMsg seam) like the rest of the screen's strings, and add one header-text pin to performance.test.tsx.
Confidence: HIGH — Verification: read; resolver chain traced (messages.ts:3011-3016, settingsMsg.ts:68-72); test file read end-to-end (no heading assertion).

### F-002 [S4]
Where: ui/src/screens/settings/areas/performance.tsx:50,55,86
Code:
```ts
const hintMsg = (key: K13Key): MessageKey => key as string as MessageKey;
```
What is wrong: Two casts that cancel the type system's own guarantees. (1) `K13Key = keyof typeof k13Keys` is ALREADY a member of `SettingsMsgKey` (settingsMsg.ts:39-49 includes `keyof typeof k13Keys`), so `hintMsg` needs no cast at all if PERFORMANCE_HINTS were typed `Record<string, SettingsMsgKey>` instead of `Record<string, MessageKey>` — the double-cast `as string as MessageKey` exists only to satisfy the map's over-narrow declared type and throws away the union check. (2) Line 86 `section.title as MessageKey` re-casts a value already typed `MessageKey` (SchemaSection.title, SettingsSections.tsx:50).
Why it matters: Casts that erase real type information hide exactly the drift F-001 describes — if the k13 file is renamed, the compile error surfaces at the wrong place (the map literal) rather than the prop contract.
What correct looks like: `const PERFORMANCE_HINTS: Readonly<Record<string, SettingsMsgKey>> = { "performance.hw-decode": "settings.hint.performance.hw-decode", ... }` with no helper; delete the line-86 cast.
Confidence: HIGH — Verification: read; SettingsMsgKey union checked (settingsMsg.ts:39-49).

### F-003 [S4]
Where: ui/src/screens/settings/areas/performance.tsx:87-89,94-96
Code:
```tsx
          {entries.map((entry: SettingEntry) => (
            <SettingField key={entry.key} entry={entry} hint={PERFORMANCE_HINTS[entry.key]} />
          ))}
```
What is wrong: The section block and the advanced block are the same three-line row grammar duplicated verbatim (only the surrounding wrapper differs); the `entry: SettingEntry` annotations are redundant (entries already SettingEntry[]).
Why it matters: Copy-paste of the row lambda is how the two sites drift (e.g. a future per-row prop added to one and forgotten in the other — precisely the ⓘ-census class of bug this file exists to fix).
What correct looks like: One local `const row = (entry: SettingEntry) => <SettingField key={entry.key} entry={entry} hint={PERFORMANCE_HINTS[entry.key]} />;` used in both maps.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `K13Key` (type) | 46 | REAL | — | keyof typeof k13Keys; correct and already in SettingsMsgKey union (F-002) |
| `hintMsg` | 48-50 | REAL | S4 | pure cast helper; double-cast unnecessary (F-002) |
| `PERFORMANCE_HINTS` | 55-68 | REAL | — | 10 entries matching the SVG census exactly (verified); keys are raw strings — a typo silently drops an ⓘ, but the census test pins the count at 10, so guarded |
| `PerformanceArea` | 77-101 | REAL | S3 | renders schema-driven sections + AdvancedDisclosure; title resolver hole (F-001); duplicated row blocks (F-003); `advanced.length > 0` guard correctly hides the bar if all advanced rows get gated; useState(false) = collapsed bar — documented divergence from the frame's expanded state (header lines 23-27, justified by the app-wide global.svg grammar) |

## 5. Test honesty (if tests exist in/for this file)
performance.test.tsx (155 lines) — substantive.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders every non-advanced schema row bound to its key" (:42-59) | YES | iterates the schema itself; missing row or wrong data-setting fails; also asserts advanced rows ABSENT before expansion |
| "frame row values render through the framework" (:61-88) | YES | pins 7 concrete control values/aria-checked states (e.g. hw-decode trigger "Automatic", threads spinbutton "0") — a broken enum/value binding fails |
| "Advanced Options bar carries the flat advanced rows … schema order" (:90-107) | YES | toEqual against the schema-derived key list — order or membership drift fails |
| "wires exactly the 10 frame circles" (:111-122) | YES | length pins 5 collapsed / 10 expanded — any hint loss or gain fails |
| "reproduces the honest absences" (:124-139) | YES | 5 keys asserted ⓘ-free — an over-eager hint map fails |
| "ⓘ are operable … reveal carries the hint copy" (:141-154) | YES | clicks the dot, asserts aria-expanded and the caption text contains "graphics hardware" |
| Mutation check (reasoned): unregister one performance section-title msgid | NOT caught | no test asserts any `<h2>` text on this screen — the F-001 gap |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the frame carries 10 r7.5 #404040 ⓘ circles (Ellipse 9..9_10 … cy 132/178/224/271/462/668/714/760/807/855)" | performance.tsx:4-7 | REAL | rg over design/figma/Settings - performance.svg: exactly 10 Ellipse 9-family ids, all r="7.5" stroke="#404040" stroke-width="1.125", cy set identical |
| "Decoding (y83) — 4 live rows, every one carries the frame's ⓘ" | performance.tsx:20 | REAL | schema: 4 decoding entries, advanced:false, featureExposed:true; all 4 in PERFORMANCE_HINTS |
| "Smooth Playback … Drop frames / real-time priority carry NO circle … Process priority does" | performance.tsx:21-22 | REAL | PERFORMANCE_HINTS omits the two toggles, includes process-priority; test :124-139 pins the absences |
| "the 3 resources rows + the 5 Developer rows, all 5 Developer rows ⓘ-carrying" | performance.tsx:26-27 | REAL | schema tabulation: resources 3×advanced, developer 5×advanced; hints cover all 5 developer keys |
| "Rescan plugins … not rendered (the honest-empty rule; flagged to the Lead)" | performance.tsx:28-30 | REAL | no such row in the render; no [[setting]] key exists (not in schema.ts) |
| "resolved through the settingsMsg seam" (hint copy) | performance.tsx:12-13 | REAL | SettingField → useSettingsMsg → settingsLaneCatalog (k13 merged, settingsMsg.ts:60) |

## 7. Dead / duplicated / unreachable code
- Duplicated row-render blocks (F-003). No dead exports; no unreachable branches. `m` used exactly once (line 86).

## 8. Dependencies & flags
Imports: react (useState), i18n/messages (useMsg, MessageKey), i18n/keys/w26.k13, generated/schema (type only), SettingsSections (AdvancedDisclosure), SettingField, ./shared (partitionAdvanced, schemaSectionViews — two import statements from the same module, lines 43-44; trivially mergeable). No engine/bridge imports: the screen itself wires nothing to the backend — all persistence goes through SettingField → useSettings → config.set-setting (the W11 pin: engine-side store is a dead in-memory BTreeMap, settings never persist in the real host). No feature flags beyond the schema's featureExposed, all true here.

## 9. Open questions for the next agent
- F-001's resolver hole is shared with other hand areas (interface? playback? video?) — a cross-area census of which areas resolve section titles via bare useMsg vs the seam would size the real blast radius.
- The `advanced: "performance"` redirect (index.tsx:279) has a standing "[Lead: confirm]" note — unresolved product decision, not a code defect.
