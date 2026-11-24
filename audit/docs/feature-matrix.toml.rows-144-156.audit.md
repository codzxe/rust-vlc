# CLAIM AUDIT: docs/feature-matrix.toml#rows-144-156
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 13 matrix rows (+ header rules) |
| Row verdicts: REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / truthful-absence | 6 REAL / 1 REAL-UNPROVEN / 0 / 0 / 0 / 6 FALSE / +1 row (144) verified-absence |
| Internal contradictions found | 2 (rows 148-151+156 vs code; row 153 vs code) |

## 1. Blunt verdict
Of these 13 rows, six are FALSE against the current tree. Five UNDERSTATE
(video, audio, subtitles claim "partial ... NOT registered" while all three
ARE registered in `SETTINGS_AREA_IDS`/`SETTINGS_SCREENS` and shipped via
`VideoAreaScreen`/`AudioAreaScreen`/`SubtitlesArea`; controls and recording
claim `not_started` with `evidence = ""` while `ControlsArea` and
`RecordingArea` are registered with real schema-generated rows and tests).
One OVERSTATES: `config-area-library` claims `implemented` with evidence
"playlist-services.tsx registered as the 'library' area" — but
playlist-services is RETIRED (library.tsx:21-23: "the fold mounts here ...
it renders nothing until playlist-services schema rows exist (the seam)")
and the library area renders STAGED INERT rows (library.tsx:1-14: "no
schemas/settings.toml [[setting]] keys exist behind them yet ... every row
renders ... disabled, marked data-staged='schema-seed'"). The parity ledger
is stale in both directions because the mechanical enforcement the header
promises ("status changes require new evidence (R4)") is admitted-OWED debt
("The full tools/feature-audit ... is OWED and unassigned debt" :8-9) —
matching the orchestrator baseline "parity-report: 0/252 verified".

## 2. Claim ledger (rows; statuses are windows/linux/macos/web)
| # | Row id | Statuses claimed | Evidence string (verbatim key parts) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 144 | plfeat-metadata | not_started ×4 | `evidence = ""` | No playlist tag/cover-art provider in tree: no lofty/id3/taglib crates in Cargo.lock (only `symphonia-metadata` 0.6.x as probe-side dep, :1056/:1074); media-probe exposes no tags/cover API (`rg 'cover\|tag' crates/media-probe` -> only error-sne hits). | — | truthful-absence (row honest) |
| 145 | config-area-global | implemented ×3, web not_started | "Screens.tsx (GENERATED_AREA_IDS includes global ...) + LocalePicker ... (global.language, registered in .../index.tsx)" | `Screens.tsx:318` GENERATED_AREA_IDS includes "global" ✓; `areas/language/LocalePicker.tsx` exists ✓; `areas/index.tsx` registers global via GENERATED_SCREENS + GlobalAreaScreen ✓. | UI tests not re-run here; orchestrator baseline: vitest 1705 passed (settings tests included) | REAL |
| 146 | config-area-interface | implemented ×3 | "GENERATED_AREA_IDS includes interface ... ThemeModePicker ... appearance" | GENERATED_AREA_IDS includes "interface" ✓; `areas/appearance/ThemeModePicker.tsx` (+test) ✓; `interface: InterfaceArea` in SETTINGS_SCREENS ✓. | baseline vitest | REAL |
| 147 | config-area-privacy | implemented ×3 | "... privacy; data section) + privacy-network area components" | GENERATED_AREA_IDS includes "privacy" ✓; `privacy-network.tsx` (+test) ✓; `privacy: PrivacyArea` ✓. | baseline vitest | REAL |
| 148 | config-area-video | partial ×3 | "video.tsx (+ video.test.tsx) — component exists but is NOT registered in SETTINGS_AREA_IDS/SETTINGS_SCREENS (ui/src/screens/settings/areas/index.tsx)" | FALSE TODAY: `SETTINGS_AREA_IDS` (index.tsx:189-203) includes "video"; `SETTINGS_SCREENS` maps `video: VideoAreaScreen` (:243); video.tsx 170 LOC + video.test.tsx; titles comment "registered (messages.ts, b19 merge)". | registration verified by read; vitest baseline | FALSE (stale-understated) |
| 149 | config-area-audio | partial ×3 | "audio.tsx ... NOT registered ... (live audio fields currently ship via the effects area ...)" | FALSE TODAY: "audio" in SETTINGS_AREA_IDS; `audio: AudioAreaScreen` registered (:244); audio.tsx 282 LOC + tests + fixtures. | read | FALSE (stale-understated) |
| 150 | config-area-subtitles | partial ×3 | "subtitles.tsx ... NOT registered ..." | FALSE TODAY: "subtitles" in SETTINGS_AREA_IDS; `subtitles: SubtitlesArea` (:242 import, registry entry); subtitles.tsx 478 LOC + test. | read | FALSE (stale-understated) |
| 151 | config-area-controls | not_started ×4 | `evidence = ""` | FALSE TODAY: controls.tsx 244 LOC (schema-generated Skip Amounts + Mouse rows via `<GENERATED_SCREENS.controls/>`, real hotkeys-editor actions, controls.test.tsx); "controls" in GENERATED_AREA_IDS (:318) AND SETTINGS_AREA_IDS; `controls: ControlsArea` registered. | read | FALSE (stale-understated) |
| 152 | config-area-playback | implemented ×3 | "GENERATED_AREA_IDS includes playback; tracks/playback sections" | GENERATED_AREA_IDS includes "playback" ✓; `playback: PlaybackArea` (areas/playback.tsx) ✓. | baseline vitest | REAL |
| 153 | config-area-library | implemented ×3 | "playlist-services.tsx (+ test) registered as the 'library' area in ... index.tsx" | EVIDENCE FALSE TODAY + status INFLATED: playlist-services fold "renders nothing until playlist-services schema rows exist (the seam)" (library.tsx:21-23, :46-49); LibraryArea renders staged-inert rows: "no schemas/settings.toml [[setting]] keys exist behind them yet, so every row renders full-strength, disabled, marked data-staged='schema-seed'" (library.tsx:4-7). "implemented" for an area whose watch-folders/scan/ignore rows are all inert = overstated. | read | FALSE (inflated; substance = staged shell) |
| 154 | config-area-network | implemented ×3 | "privacy-network.tsx NetworkArea (+ test) registered as the 'network' area" | `NetworkArea` exported from privacy-network.tsx (:35), imported index.tsx:22, `network: NetworkArea` registered ✓. | baseline vitest | REAL |
| 155 | config-area-performance | implemented ×3 | "GENERATED_AREA_IDS includes performance; resources section incl. hw-decode toggle per ADR-001" | GENERATED_AREA_IDS includes "performance" ✓; `performance: PerformanceArea` hand-render (areas/performance.tsx + test) ✓. | baseline vitest | REAL-UNPROVEN (hw-decode toggle binding not individually re-executed; area registration verified) |
| 156 | config-area-recording | not_started ×4 | `evidence = ""` | FALSE TODAY: recording.tsx 159 LOC + recording.test.tsx; "recording" in GENERATED_AREA_IDS; `recording: RecordingArea` registered (:246). | read | FALSE (stale-understated) |

## 3. Contradictions
| Matrix says | Code says | Which is true |
|---|---|---|
| Rows 148-150: "NOT registered in SETTINGS_AREA_IDS/SETTINGS_SCREENS" | index.tsx:189-203 lists video/audio/subtitles; :227-246 maps all three to real screens; comments cite "b19 merge" | CODE is true — matrix rows stale |
| Rows 151/156: not_started, evidence "" | controls.tsx/recording.tsx exist, registered, schema-seeded, tested | CODE is true — matrix rows stale |
| Row 153: "implemented" / "library area live under the playlist-services component" | library.tsx: playlist-services "retired", renders nothing; area rows staged inert (no schema keys) | CODE is true — "implemented" is an overstatement |
| Header (:8-9): full tools/feature-audit "OWED and unassigned debt" | baseline: xtask feature-audit FAILURE (2 evidence paths missing); parity-report 0/252 verified every platform | The header's own admission matches the baseline — the drift found here is the direct consequence |

## 4. Fabrication / inflation indicators
Row 153 is the classic inflation shape: `implemented` status anchored to an
evidence string that was true at write time and silently rotted (component
retired to a rendering-nothing seam; the area's option rows are explicitly
staged-inert). Rows 148-151/156 are the inverse (understatement) — still
ledger falsity, but conservative. Both directions exist because no
mechanical R4 re-check runs.

## 5. Matrix-row table
See §2 (the mandated per-row table: id, statuses, evidence, code found,
execution, verdict).

## 6. Findings
F-001 [S2]
Where: docs/feature-matrix.toml:2086-2193 (rows 148,149,150 at :2086/:2098/:2110; 151 at :2122; 153 at :2146; 156 at :2182)
Code: rows claiming `partial`/"NOT registered"/`not_started`/`evidence = ""`
What is wrong: 6 of 13 rows are false vs the tree (5 understated, 1
inflated). Why it matters: this file is "THE parity ledger (MASTER-PROMPT
§1)" — the parity percentage and every downstream claim inherit these
statuses. What correct looks like: re-run the owed feature-audit/parity tool
with R4 evidence resolution; flip rows 148-151/156 to match registrations;
re-verify 153 against the staged-seam reality.
Confidence: HIGH — Verification: read (index.tsx:189-246, library.tsx:1-49,
controls.tsx:1-30, all area files listed with LOC) + baseline citations.

F-002 [S3]
Where: docs/feature-matrix.toml:2032-2049 (row 144 context) —
config-feature category banner "20 rows"
Code: `# category: config-feature — 20 rows`
What is wrong: nothing wrong in the banner itself (20 config rows follow
through row 163 — out of my chunk); the row-144 boundary is the last
playlist-feature row and is clean. Recorded to note chunk boundaries
verified: rows 144-156 = 13 [[feature]] blocks at :2032-:2182 range.
Confidence: HIGH — read.
