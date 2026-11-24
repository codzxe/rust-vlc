# AUDIT: ui/src/screens/settings/areas/subtitles.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 (revised after display-pipeline artifact correction) |
| Lines / functions | 479 / 9 (useSubtitlesMsg, StagedRow, StagedToggleRow, StagedDropdownRow, StagedTextRow, StagedSegmentedRow, StagedColorRow, StagedOpacityRow, SubtitlesArea) |
| File verdict | STUB (compiles; all 14 rows staged-dead by design, zero transport) |
| Reachable from shipped entry point? | YES — App.tsx:322 `/settings/:area` -> SettingsScreen -> areas/index.tsx:21 `import { SubtitlesArea } from "./subtitles"` (registry index.tsx:257) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 byte-level check of L106 (b'  const [hintOpen, setHintOpen] = useState(false);' — correct code; the earlier "syntax error" was a tool-display artifact eating the `[h` sequence); grep import chain; messages.ts staged-key greps; subtitles.test.tsx read |

## 1. One-paragraph verdict
CORRECTION RECORD: my first pass reported a syntax error at L106 — that was FALSE, an artifact of the output pipeline swallowing the two-character sequence `[h` (verified: python3 byte equality `lines[105] == b'  const [hintOpen, setHintOpen] = useState(false);'` -> True; the printed repr showed the mangled form). The file parses fine and the Wave-0 "UI all pass" baseline is NOT contradicted. The real verdict: the entire Subtitles settings area is 14 rows of STAGED dead controls — every switch/dropdown/slider/text field rendered `disabled` with frame-verbatim canned values, empty onChange handlers, zero transport calls, awaiting a `subtitles.*` schema seed that does not exist. The staging is honestly disclosed (operable ⓘ notes with reveal captions, `data-staged`/`data-pending` markers, "Staged — controls activate when the subtitles settings seed lands"). Master-question answer for this screen: nothing here is wired to a backend.

## 2. What actually works (verified)
— verified by python3 byte check of L106 (correct destructuring); i18n keys registered (messages.ts:1174, 2655); the ⓘ hint toggle (SettingInfoDot with aria-expanded/controls) is real interactive behavior; subtitles.test.tsx:210-212 asserts `transport.sent` length 0 — the deadness is pinned by test. UI suite pass per Wave-0 baseline.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/subtitles.tsx:164-171 (representative; all 14 rows L369-474)
Code:
```
      <Switch
        checked={checked}
        onChange={() => {
          /* staged — disabled means never fired */
        }}
        disabled
        aria-label={label}
      />
```
What is wrong: the entire area is inert UI — 14 disabled controls with hard-coded frame values ("#FFFFFF" swatch, slider pinned at 50, "System language"), no persistence, no engine commands; callbacks are empty. Honest (labels + data-staged markers + tested zero-transport), but a full settings screen ships as decoration. Why it matters: master-question census — /settings/subtitles is reachable from the settings nav and wires to NOTHING. What correct looks like: schema-seeded rows like the E1-generated areas. Confidence: HIGH — Verification: read + test file.
F-002 [S4 — measurement note for the audit infrastructure]
Where: (meta — tool output pipeline)
Code:
```
b'  const [hintOpen, setHintOpen] = useState(false);'   # real bytes, L106
```
What is wrong: the Bash/Read tool display layer eats the literal two-char sequence `[h` (terminal cursor-home escape), which manufactured an apparent `const intOpen, setHintOpen]` syntax error in my first pass. Same artifact hit SettingsScreen.tsx:464 (`[hydrated`). All future agents must byte-verify any "corrupted" line before reporting S0. Confidence: HIGH — Verification: ran (python3 subprocess echo test: `[h` eaten, `[w`/`[a`/`[foo` survive).

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useSubtitlesMsg | 66 | REAL-UNPROVEN | — | makeAreaMsg factory; keys registered |
| StagedRow | 82-143 | REAL-UNPROVEN | — | row shell + operable ⓘ hint toggle (real interaction) |
| StagedToggleRow | 147-174 | STUB | S3 | disabled switch, empty onChange |
| StagedDropdownRow | 178-208 | STUB | S3 | single canned option, disabled |
| StagedTextRow | 212-232 | STUB | S3 | disabled TextField, canned value |
| StagedSegmentedRow | 235-265 | STUB | S3 | disabled SegmentedControl |
| StagedColorRow | 270-295 | STUB | S3 | static hex + swatch, no picker |
| StagedOpacityRow | 303-345 | STUB | S3 | disabled Slider pinned at 50 |
| SubtitlesArea | 353-478 | STUB | S3 | composes the 14 staged rows; honest staged notes |

## 5. Test honesty
subtitles.test.tsx:210-212 "never touches the transport (no engine seam exists for these rows)" — pins the dead behavior; honest about it. Would fail if someone wired a row without removing the staged pattern: yes (sent length would grow). Structure tests pin sections/rows.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "controls activate when the subtitles settings seed lands" | messages.ts:2655 | TRUE (disclosure) | no `subtitles.*` rows in the schema per module header; generated-schema proof belongs to settings lane |
| "Zero transport calls (asserted in subtitles.test.tsx)" | module header L26 | REAL | subtitles.test.tsx:212 |
| "UI: npm ci/typecheck/test/build all pass" | Wave-0 baseline | NOT CONTRADICTED (my earlier challenge withdrawn — display artifact) | byte check L106 correct |

## 7. Dead / duplicated / unreachable code
All 14 rows staged-dead by design (F-002 of the first pass is withdrawn; see correction record).

## 8. Dependencies & flags
form primitives (Switch/SegmentedControl/TextField), menu Dropdown, base Slider, SettingInfoDot, makeAreaMsg, subtitles.css. No feature flags.

## 9. Open questions for the next agent
- When does the `subtitles.*` schema seed land (schemas/settings.toml owner lane)?
- The `[h`-eating display artifact (F-002) affects every agent reading files with identifiers like `[hint`, `[hydrated`, `[has` — byte-verify before crying corruption.
