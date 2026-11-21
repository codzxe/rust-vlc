# AUDIT: ui/src/screens/settings/areas/recording.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 160 / 3 (GeneratedRecordingScreen binding, RECORDING_EXPOSED gate, RecordingArea) |
| File verdict | REAL (honest staged-inert settings area; nothing behind any control in the shipped product — disclosed) |
| Reachable from shipped entry point? | YES in the web UI (route /settings/recording → SettingsScreen → areas/index.tsx:252 `recording: RecordingArea` registry entry). NO desktop binary exists (W14-A01 E0425). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read areas/staged.tsx (StagedRow/StagedAdvancedBar/useStagedMsg — disabled controls, title + sr-only reason, aria-describedby); sed schema.ts:2427-2465 (recording.directory + recording.format both featureGate "recording", featureExposed: false); sed generated/Screens.tsx:290-298 (RecordingScreen = SettingsSections over the two gated entries) + areas/index.tsx:245-258 (area registry: `recording: RecordingArea`); Read areas/recording.test.tsx (5 tests incl. the 10-staged-rows count and the gated-rows-hidden assertions); Grep `data-advanced-staged` → staged.css:43 (style hook only, no JS consumer); sed quote verification. Baseline: ui-vitest.log.txt 136 files / 1705 passed (includes recording.test.tsx). |

## 1. One-paragraph verdict
This file is the honest-empty pattern done properly: every one of the §C12 frame's 10 rows renders full-strength but disabled, each carries the staged tooltip + sr-only reason + (on the 4 ⓘ rows) an operable reveal, the two schema-backed rows are gated behind `recording` (featureExposed: false in the generated schema — verified) and the embedded generated screen is their only live render path, and the Advanced bar is a disabled disclosure that opens to nothing and says so. The 5 tests are behavioral and pin the routing, the embed contract, the schema truth, the gate behavior, and the ⓘ census. The product reality is that NO recorder, caster, or streamer exists anywhere in the engine — this page configures a feature that is absent end-to-end — but that absence is disclosed on every row, which is exactly what the wave's honest-staging directive demands. The one real defect is the header's yield-rule comment: it claims "the two schema-backed rows — recording.directory + recording.format … render[s] staged" while the gate is off, but the staged section renders directory + the schema-less `recording.without-reencoding` row, and `recording.format` renders NOWHERE while gated (the generated screen hides gated entries; the test asserts its absence).

## 2. What actually works (verified)
— verified by recording.test.tsx:39-135 (routing asserts data-settings-area + aria-current; the embed is structural: wrapper testid contains the generated screen's root; schema truth pins keys + gate; gated rows hidden [data-setting=null] with 10 [data-staged-row] present and no empty-area note; labels/values verbatim incl. disabled Browse/switch; every staged row's title contains "Staged"; 4/10 ⓘ rows operable with reveal text containing "recording ships") + baseline ui-vitest (136 files, 1705 passed).
— verified by reading the gate: schema.ts:2435-2436 and :2454-2455 both carry `featureGate: "recording", featureExposed: false`, so RECORDING_EXPOSED (recording.tsx:38-40) is false today and the staged section renders; the module-level computation is correct because the schema is generated static data that only changes via regeneration.
— verified by reading staged.tsx:99-178: every control variant renders `disabled` with a no-op onChange, the reason is triple-carried (title, sr-only span for non-ⓘ rows, aria-describedby), and StagedAdvancedBar (:191-209) is a disabled button + sr-only reason.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/settings/areas/recording.tsx:13-16 (header yield rule) vs :71-85 (what actually renders staged)
Code:
```ts
// Yield rule: the Recording section (the two schema-backed rows —
// recording.directory + recording.format) renders staged ONLY while the
// gate is off; the moment exposure flips, GENERATED_SCREENS.recording
```
(and the staged section's actual second row)
```ts
          <StagedRow
            rowKey="recording.without-reencoding"
            label={t("settings.recording.without-reencoding")}
            control={{ kind: "switch", on: true }}
```
What is wrong: The comment claims the staged Recording section is "the two schema-backed rows — recording.directory + recording.format". Both halves are wrong: (1) the staged section renders `recording.directory` (schema-backed) + `recording.without-reencoding` (NO [[setting]] key — a frame-only row); (2) `recording.format` (the second schema-backed row) renders NOWHERE while the gate is off — the embedded generated screen hides gated entries (recording.test.tsx:76 asserts `[data-setting="recording.format"]` is null). So the gate-flip yield is not like-for-like: exposure flips → the staged section (directory + without-reencoding) is removed and the generated rows (directory + format) appear — the user LOSES "Record without re-encoding when possible" and GAINS "Format". The header describes a swap it does not perform.
Why it matters: The yield rule is this file's core contract (the "no double render" sibling-contract clause); the comment misstates which rows exist on each side of the flip. The next developer trusting it will believe `without-reencoding` survives the flip (it vanishes) and that `format` is currently staged (it is absent). The tests pin the real behavior, so the comment drifts against the file's own test file.
What correct looks like: "the staged Recording section (frame-verbatim directory + without-reencoding) yields entirely on exposure flip; the generated screen then renders directory + format live — `recording.format` has no staged representation and `without-reencoding` has no schema key."
Confidence: HIGH — Verification: read both render branches + schema.ts:2447-2462 + test assertions :76, :86-89.
### F-002 [S4]
Where: ui/src/screens/settings/areas/recording.tsx:29-31
Code:
```ts
import { StagedAdvancedBar } from "./staged";
import { StagedRow } from "./staged";
import { useStagedMsg } from "./staged";
```
What is wrong: Three separate import statements from the same module ("./staged") where one combined statement is idiomatic; a linter with no-duplicate-imports would flag it.
Why it matters: Hygiene noise; signals copy-paste assembly across the staged-area siblings (library.tsx / privacy-network.tsx share the pattern).
What correct looks like: `import { StagedAdvancedBar, StagedRow, useStagedMsg } from "./staged";`
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `GeneratedRecordingScreen` binding | 35 | REAL | — | Static binding of the generated screen; renders gated-hidden today. |
| `RECORDING_EXPOSED` | 38-40 | REAL | — | Module-level boolean over static generated schema — correct; flips only on schema regeneration. |
| `RecordingArea` | 47-159 | REAL | S3 | Composition only: embedded generated screen + conditional staged Recording section (F-001 comment drift) + always-staged Casting (2 rows) / Streaming (6 rows) + staged Advanced bar. All copy through i18n seams; wrapper carries data-testid + data-advanced-staged (style hook, staged.css:43); exactly one [data-settings-area="recording"] per route (the embedded screen's root — the g14-P3-4 fix, test-pinned). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :39 routed at /settings/recording, aria-current | YES | Routed render + registry assertions. |
| :48 embeds GENERATED_SCREENS.recording structurally | YES | Selector asserts the nested generated root inside the wrapper. |
| :62 schema truth (keys + gate) | YES | Pins the generated schema against drift (settings:check also guards). |
| :72 gated rows hidden + inert, 10 staged rows, no empty note | YES | Negative + count assertions — would fail if the gate broke or rows leaked live. |
| :83 Casting/Streaming rows staged inert, verbatim values, every staged title contains "Staged", zero [data-setting] | YES | The honesty contract pinned. |
| :114 ⓘ census 4/10, operable reveal toggle | YES | Click-driven expand/collapse with text assertions. |
| Gap: no test pins the gate-FLIPPED yield (no fixture with featureExposed: true) — F-001's misdescribed swap is unpinned. | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the two schema-backed rows — recording.directory + recording.format) renders staged ONLY while the gate is off" | recording.tsx:13-16 | FALSE (F-001) | Staged section renders directory + without-reencoding; format renders nowhere while gated (test :76). |
| "the seeded rows are feature-gated (`recording` = false, product.toml Phase 7, rule F2)" | recording.tsx:7-8 | REAL | schema.ts:2434-2435, :2453-2454 (featureGate "recording", featureExposed false). |
| "Casting/Streaming rows have no [[setting]] keys at all" | recording.tsx:8-9 | REAL | Grep recording.* in schema.ts → only directory + format exist; Casting/Streaming rows carry seam="schema-seed". |
| "exactly ONE [data-settings-area="recording"] renders per route" | recording.tsx:21-23 | REAL | Wrapper carries no data-settings-area (read); test :41 + :55-59. |
| "the frame's 10 rows visible, never dead controls that look alive" | recording.tsx:9-11 | REAL | Test :80 (10 staged rows) + :88-89 (disabled controls) + :108-110 (staged titles). |

## 7. Dead / duplicated / unreachable code
No dead code. Duplicated: the triple-import pattern (F-002) and the area-wrapper shape shared with library.tsx/privacy-network.tsx (acceptable family pattern). Product-dead-by-design: every control (the engine has no recorder/caster/streamer — no recording.* catalog commands exist at all, unlike CropTool's named-but-unserved command; here even the command layer is absent), disclosed on every row.

## 8. Dependencies & flags
staged.tsx (StagedRow/StagedAdvancedBar/useStagedMsg → i18n keys w26.i15), generated Screens + schema (SETTINGS_ENTRIES, GENERATED_SCREENS.recording), staged.css. Feature gate `recording` (product.toml Phase 7) consumed via the generated schema's featureExposed, never read dynamically at runtime. No transport, no engine imports.

## 9. Open questions for the next agent
- When the `recording` gate flips, does any area test cover the flipped state (staged section gone, generated rows live)? None exists today (see test gap) — the sibling-contract owner should add a schema-stub fixture.
- The schema seeds `recording.format` (mp4-remux/wav) with no frame row and the frame carries `without-reencoding` with no schema key — the Lead's seed and the §C12 frame disagree on the Recording section's second row; whoever reconciles should also fix the header (F-001).
