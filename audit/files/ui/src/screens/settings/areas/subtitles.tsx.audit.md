# AUDIT: ui/src/screens/settings/areas/subtitles.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 478 / 8 (StagedRow, StagedToggleRow, StagedDropdownRow, StagedTextRow, StagedSegmentedRow, StagedColorRow, StagedOpacityRow, SubtitlesArea) |
| File verdict | SHALLOW (an honest, fully-disclosed, 100%-inert settings screen: every control disabled, zero transport calls, no persistence — a staged mockup awaiting a schema seed that does not exist) |
| Reachable from shipped entry point? | NO at HEAD — mounted at /settings/subtitles via areas/index.tsx:257 registry + App.tsx:242 title map; only shipped host `lumen-tauri` cannot compile (W14-A01 E0425). Rendered and tested in vitest (baseline #23). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -n '^key = "subtitles\.' schemas/settings.toml` → 0 matches (header claim TRUE; playback.* subtitle keys live at :617,:628,:723 as claimed); `rg -n "SubtitlesArea" ui/src` → areas/index.tsx:21,257 (route mount) + test; read subtitles.test.tsx (asserts all-14-rows disabled + zero transport calls); `sed -n` verification of quotes; baseline #23/#22 for execution. |

## 1. One-paragraph verdict
This is the cleanest specimen yet of the repo's "staged hand row" pattern: the entire Settings → Subtitles screen — 3 sections, 14 rows of switches, dropdowns, a segmented control, text fields, color wells, and a slider — is rendered completely inert, because schemas/settings.toml contains zero `subtitles.*` keys (verified: 0 matches). Every claim the module header makes about its own hollowness is true: controls are disabled with frame-verbatim display values, `data-staged="subtitles-settings-seed"` markers are present, notes disclose the awaited seam, the tests assert both the disabledness and zero transport calls. As engineering honesty it is exemplary; as a product it means the user-facing subtitle configuration surface does not exist — a user who opens Settings → Subtitles cannot change anything, and the screen discloses this only via disabled states and a ⓘ toggle. The W11 "settings never persist" pin here degenerates further: this screen never even attempts persistence.

## 2. What actually works (verified)
— verified by `rg -c '^key = "subtitles\.' schemas/settings.toml` → 0: the header's central claim (no subtitle-screen schema rows) is TRUE; `playback.preferred-subtitle-languages`/`playback.subtitle-mode` exist only on the playback screen (settings.toml:617,628) exactly as the header states.
— verified by reading areas/index.tsx:21,197,222,257 + App.tsx:242: the area is genuinely routed at /settings/subtitles with a window title.
— verified by subtitles.test.tsx (baseline #23, 1705 passed set): 14 rows render with frame values; all switches/dropdowns/radios/text assert toBeDisabled; a dedicated describe asserts "zero transport calls, no schema bindings".

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/settings/areas/subtitles.tsx:22-45 (header) and 353-478 (SubtitlesArea)
Code:
```
//   • NO `subtitles`-screen [[setting]] rows exist in schemas/settings.toml
//     (the b02 seed grew controls/recording/extensions only), so the
//     SettingsProvider carries ZERO subtitle keys and every row here is a
//     STAGED hand row: control rendered DISABLED with the frame-verbatim
```
and (lines 164-171):
```
      <Switch
        checked={checked}
        onChange={() => {
          /* staged — disabled means never fired */
        }}
        disabled
        aria-label={label}
```
What is wrong: The whole screen is non-functional by construction. 14 rows, every control `disabled`, every onChange an empty comment, zero SettingsProvider bindings, zero transport calls. There is no subtitle configuration in the product: not the toggle, not the language, not font/size/color/opacity/outline/shadow. The engine side has no persisted-subtitle-defaults seam either (the header correctly notes the catalog's subtitle commands are session-only).
Why it matters: A settings screen that cannot set anything is a mockup shipped as a route. The disclosure is honest, but the capability gap is total and permanent until a schema seed lands — and nothing in the tree schedules that seed (handoff is a w26 log note). This is the W11 settings-never-persist disease at its terminal stage.
What correct looks like: schemas/settings.toml gains `subtitles.*` rows (the E1 generated screen then retires this file, per the header's own migration note) — or the route is hidden until the seed exists.
Confidence: HIGH — Verification: read file + schema + tests; route wiring verified.

### F-002 [S4]
Where: ui/src/screens/settings/areas/subtitles.tsx:73-77 (+ subtitles.css:84)
Code:
```
/** One staged row shell — label (+ optional ⓘ) left, control right in the
 *  320px column, the frame's row grammar (settings.css owns the classes).
 * w26.gfix-B (g13-P2-3): rows render FULL-STRENGTH while inert (the
 *  frame's live-looking controls; the honesty lives in the disabled state +
 *  the ⓘ note), and carry data-kind so the row pitch follows the frame's
```
What is wrong: A deliberate design choice renders inert controls visually indistinguishable from live ones ("no dimming" — subtitles.css:84). The only signals that a row does nothing are the native disabled interaction and a ⓘ tooltip two interactions away.
Why it matters: "Never a dead control that looks alive" is the file's own stated rule (line 23); full-strength presentation pushes against it — the visual grammar says live, the state says inert.
What correct looks like: a persistent (non-hover) staged affordance — e.g. the data-staged marker surfaced as a subtle badge — so the honesty does not depend on the user attempting the control.
Confidence: HIGH — Verification: read both files; quote sed-verified.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module header | 1-60 | REAL (accurate self-description) | — | every claim verified against schemas/settings.toml |
| SubtitlesLabelKey / useSubtitlesMsg | 62-70 | REAL | — | makeAreaMsg lane seam; duplicate-key sync with messages.ts by design |
| StagedRow | 83-143 | REAL | — | data-staged/data-setting-key="pending.*" markers; operable ⓘ (aria-expanded + hint); sr-only note on ⓘ-less rows |
| StagedToggleRow | 147-174 | REAL (inert) | S2 | F-001; disabled switch, empty onChange |
| StagedDropdownRow | 178-208 | REAL (inert) | S2 | F-001; single frame-verbatim option, disabled |
| StagedTextRow | 212-232 | REAL (inert) | S2 | F-001; disabled TextField |
| StagedSegmentedRow | 235-265 | REAL (inert) | S2 | F-001; disabled radios |
| StagedColorRow | 270-295 | REAL (inert) | S2 | F-001; hex + swatch from lane data; swatch via custom property |
| OPACITY_TICKS + StagedOpacityRow | 297-345 | REAL (inert) | S2 | F-001; disabled Slider at 50 + tick overlay; labels aria-hidden (redundant with slider's aria-label — correct) |
| SubtitlesArea | 353-478 | REAL (renders the mock) | S2 | F-001; 3 sections/14 rows frame order |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders sections/14 rows in frame order | YES | exact dataset/label assertions |
| frame toggles/dropdowns with frame values | YES | asserts aria-checked + disabled + verbatim values |
| honest staged surface (config-never-invents) | YES | asserts zero transport calls over a fixture transport — enabling any control with a real call breaks it |
| baseline #23 | — | executed green |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "NO subtitles-screen [[setting]] rows exist in schemas/settings.toml" | subtitles.tsx:24-27 | REAL | rg → 0 `subtitles.*` keys |
| "the schema's two subtitle-ish keys … are placed on the PLAYBACK screen" | subtitles.tsx:31-33 | REAL | settings.toml:617,628 under playback.* |
| "no enumeration command (inventing font/encoding lists would violate MP §5)" | subtitles.tsx:39-40 | REAL-UNPROVEN | command catalog not re-audited here (W2 lane); no UI-side invention found |
| frame has 3 sections / 14 rows | subtitles.tsx:2-3 | REAL | test asserts exactly 14 labels |

## 7. Dead / duplicated / unreachable code
No dead exports (SubtitlesArea consumed by areas/index.tsx:257; useSubtitlesMsg consumed by tests + area). The seven staged-row components exist only to render inert controls — 250 lines of scaffolding whose entire purpose retires when the schema seed lands (disclosed). Unreachable from shipped binaries (host unbuildable, W14-A01).

## 8. Dependencies & flags
Consumes shared form/menu/slider components, SettingField's SettingInfoDot, makeAreaMsg, lane keys w26.f04, subtitles.css. No flags. All imports typecheck (baseline #22).

## 9. Open questions for the next agent
- Whoever audits schemas/settings.toml / the E1 generated screens: is a `subtitles.*` seed scheduled anywhere (xtask? docs?), or is this screen's retirement path pure aspiration?
- The generated settings screens (gen-settings-screens.mjs, settings:check) — do they also validate that hand-written staged areas don't drift from the frame (no, per test scope) — the 14-row pin lives only in this test file.
