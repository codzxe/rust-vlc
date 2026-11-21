# AUDIT: ui/src/screens/settings/areas/library.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 125 / 1 (LibraryArea) |
| File verdict | REAL (code) — the ENTIRE routed settings area is staged-inert: zero live controls |
| Reachable from shipped entry point? | NO as product (desktop-tauri cannot compile, W14 SUMMARY l.19). Dev/web: routed at /settings/library via the area registry (areas/index.tsx:52,181,187); library.test.tsx asserts the routing; baseline rows 22-23 green. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg `library\.` in screens/settings/generated/schema.ts → 0 keys (no [[setting]] rows); rg `library` in schemas/settings.toml:45 → the area IS declared in the settings.toml area list; rg `StagedRow` across areas (library 8, subtitles 13, recording 11, privacy-network 10, staged.tsx 1); rg `SettingField` in those files (subtitles 1, others 0); rg `LibraryArea` (registry + routing verified); read library.test.tsx test list (7 cases incl. "binds zero live settings rows") |

## 1. One-paragraph verdict
This is the maximal instance of the dead-UI pattern: a fully routed, navigable settings screen ("Playlists, Folders & Files") in which every single control — 7 rows plus the Advanced bar — is permanently disabled. The cause is stated in the file's own header: "no schemas/settings.toml [[setting]] keys exist behind them yet", and it verifies — the generated schema contains zero `library.*` keys even though settings.toml:45 declares "library" as an area. The implementation is honest about it (StagedRow: disabled, full-strength (not dimmed, except the two frame-dimmed rows), data-staged="schema-seed", reason tooltips, an operable ⓘ on every row, and a test suite that proudly asserts "binds zero live settings rows"). But honesty does not make the screen functional: a user who navigates Settings → Library can change nothing, and this is not an isolated row — subtitles (13 staged rows), recording (11), and privacy-network (10) are the same pattern, so a large fraction of the settings tree is an interactive mockup.

## 2. What actually works (verified)
— verified by rg → routing real: areas/index.tsx imports + registers LibraryArea; library.test.tsx:37 asserts the /settings/library route.
— verified by rg → the staging claim is TRUE: 0 `library.*` keys in generated/schema.ts; settings.toml declares the area but seeds no rows (single match at :45 is the area list).
— verified by reading the row table (l.59-122) → 7 rows match the §C9 frame census the tests assert (7/7 ⓘ operable, test:106); the two frame-dimmed rows (file-manager, playlist-add) carry `dimmed` per §C0 (l.14-18).
— verified by reading PlaylistServicesArea composition (l.56) → the fold renders nothing today (its own file says so) and does not duplicate the area root (l.24 — one [data-settings-area="library"]).
— baseline rows 22-23: compiles, suite green.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/settings/areas/library.tsx:39-125 (whole area; header claim l.4-12)
Code:
```
        <StagedRow
          rowKey="library.use-media-library"
          label={t("settings.library.use-media-library")}
          control={{ kind: "switch", on: true }}
          seam={seam}
          reason={staged}
          info
        />
```
What is wrong: An entire routed settings screen ships with zero functional controls. All 8 interactive elements are StagedRow placeholders (disabled, tooltip'd) because no schema keys exist — the screen's purpose (configure library/opening/metadata behavior) is unachievable in any build. Worse than inert: the rows display hand-picked FAKE STATE — `on: true` for "Use media library", enum values like "Include subfolders"/"File info only" (l.88, 107) — controls showing configuration values that no configuration backs. The honesty devices (data-staged, tooltips, tests asserting the absence) are scrupulous, but the product fact stands: dead screen, navigable from the settings navigation.
Why it matters: This is the wave's central hostile question answered at screen scale, and it is SYSTEMIC: subtitles.tsx renders 13 StagedRows (+1 real field), recording.tsx 11 (0 real), privacy-network.tsx 10 (0 real). At least 4 of the settings areas are wholly or almost wholly inert — the settings application is substantially a framed mockup over a partial schema. (The areas that DO have schema rows — audio/interface/playback/performance/privacy/global/video/controls/engine/extensions — still dead-end in the persistence seam at HEAD: SettingField audit F-003, W11/W14.)
What correct looks like: seed the schema rows (the file even names the log holding the exact list: docs/waves/w26/logs/w26.i15.md) so E1's generated screen renders live rows — the module is explicitly built to yield to it — or remove the area from navigation until seeded.
Confidence: HIGH — Verification: schema grep (0 keys), settings.toml area list, sibling-area StagedRow counts, routing chain, test list.
Severity: S2 — a whole realistic user journey (open Settings → Library → try to change anything) fails; held below S0/S1 only because the UI never claims success and disables rather than fakes action.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LibraryArea` | 39-125 | REAL (code) / inert (product) | S2 | F-001; composition correct (fold + 2 sections + advanced bar); single area root; every control staged |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :37 routed at /settings/library | YES | routing asserted |
| :46 playlist-services fold mounted unchanged | YES | merge contract |
| :60 schema truth: no library rows → no GENERATED_SCREENS.library | YES | pins the ABSENCE — an honest negative test (guards against silently wiring half the rows) |
| :73 §C9 rows staged inert, "never dead-but-alive" | YES | asserts disabled + visible |
| :106 ⓘ census 7/7 operable, reveal = staged reason | YES | every ⓘ opens and shows the reason |
| :128 binds zero live settings rows | YES | the centerpiece honesty test — nothing pretends to be wired |
Mutation check (reasoned): make a row a live SettingField → tests :60/:73/:128 fail (the suite enforces the staged contract). The tests are genuine; what they verify is a dead screen, correctly.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no schemas/settings.toml [[setting]] keys exist behind them yet" | l.6-7 | REAL | schema grep 0 keys |
| "every row renders full-strength, disabled, marked data-staged … never dead controls that look alive" | l.7-9 | REAL | implementation + tests assert exactly this |
| "the two rows carry data-staged-dimmed (var(--opacity-muted))" | l.16-18 | REAL | `dimmed` props at l.74, 83 |
| "exactly ONE [data-settings-area="library"] per route" | l.23-24 | REAL | fold renders nothing; test :46 |
| settings.toml declares "library" as a settings area | schemas/settings.toml:45 | REAL | the area list includes it — an area declared but unimplemented (the schema/area drift that makes F-001 a shipped dead screen rather than an unbuilt one) |

## 7. Dead / duplicated / unreachable code
No dead code in-file (all rows render by design). The whole AREA is functionally dead (F-001). Unreachable in shipped product (W14).

## 8. Dependencies & flags
PlaylistServicesArea (renders nothing), StagedRow/StagedAdvancedBar/useStagedMsg (staged.tsx — sibling audit), staged.css, useMsg/useStagedMsg i18n. No transport, no settings provider — the area cannot touch state even by accident. No feature flags.

## 9. Open questions for the next agent
- subtitles.tsx / recording.tsx / privacy-network.tsx owners (other agents): confirm the same S2 systemic finding from your side — recording has 2 schema rows that its area does NOT render (where do they surface?).
- The w26.i15 log names the exact library row list to seed — if the orchestrator wants ONE high-leverage fix request, "seed the library/subtitles/recording/privacy-network schema rows" un-deadens four screens at once.
- StagedRow (staged.tsx) owner: does the `control={{ kind: "switch", on: true }}` fake state get aria-hidden or announced? A disabled switch reading "on" to screen readers is a small a11y lie on top of F-001.
