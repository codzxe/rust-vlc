# AUDIT: ui/src/screens/settings/areas/video.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 170 / 3 exported components + 3 module consts |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:322 /settings/:area -> SettingsScreen -> area registry -> VideoArea (/settings/video); CodecsFold composed by areas/index.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg settings-wiring chain (SettingsScreen/persistence/config-settings/dispatch.rs) (exit 0); baseline ui-test.txt cited |

## 1. One-paragraph verdict
Schema-driven settings area with hand-rendered sections only where the frame demands per-row ⓘ hints; the one custom action (Reset picture) is a real behavior — it resets the five picture keys in the DRAFT store via `resetField` (L102-104), and Apply commits through a persistence seam that in the desktop shell is the REAL engine bridge: SettingsScreen.tsx:451-457 builds `createConfigSettingsPersistence(client, ...)` when `isDesktopShell()`, whose save path sends `config.set-setting` (config-settings.ts:138) — a command the CLI host actually serves (apps/cli/src/dispatch.rs:148-151). The browser dev shell honestly stays localStorage (demo transport answers feature-not-exposed for config.*). This is one of the few genuinely backend-wired settings surfaces; no defects found.

## 2. What actually works (verified)
— ResetPictureRow -> useSettings().resetField -> draft store; Apply -> persistence.save -> (desktop) config.set-setting served by dispatch.rs:148-151 `Command::ConfigSetSetting { area, key, value } => ... host.setting_set(...)`.
— Hint wiring: VIDEO_HINTS (L57-66) covers exactly the 8 frame-measured ⓘ rows; sliders/reset carry none (L22-25 comment) — matches the claimed frame extraction.
— Hydration gate before render in the desktop shell (SettingsScreen.tsx:471-481) — schema defaults never flash over engine values.
— Baseline: settings suites pass (ui-test.txt).

## 3. Findings
None. Checked: the action row is not a dead control (draft reset + Apply commit both real); the codecs fold rides the shared AdvancedDisclosure (no third hand copy); section render is schema-driven (`schemaSectionViews("video")`); no literals bypassing i18n; the retired search-filter comment (L68-72) matches the removal (no filter code remains).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useAreaMsg (makeAreaMsg) | 48 | REAL | - | shared lane seam |
| VIDEO_HINTS / PICTURE_KEYS consts | 50-81 | REAL | - | keys from generated schema consts |
| ResetPictureRow | 89-112 | REAL | - | real draft reset; frame has no ⓘ here — reproduced |
| VideoArea | 119-146 | REAL | - | schema sections + hints + reset row in frame order |
| CodecsFold | 163-170 | REAL | - | shared AdvancedDisclosure; collapsed default |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| settings area tests (baseline pass) | YES | reset action, hint props, fold behavior asserted (settings suites in ui-test.txt) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A real behavior: resets the five picture keys ... never a dead control" | L86-88 | TRUE | L102-104 + verified Apply->engine bridge chain |
| "the codecs fold ... an ACTUAL in-flow disclosure" | L16-19 | TRUE | L163-170 AdvancedDisclosure |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Generated schema entries (SETTING_VIDEO_* consts), SettingsProvider, codecs area, shared area helpers. No feature gates.

## 9. Open questions for the next agent
- Engine-side: does the CLI host's `setting_set` for video.hue/brightness/... feed any live render path, or is the settings store write-and-forget? (The Tools>Video color sliders are staged awaiting a `video.set-*` command — two surfaces claiming the same knobs by different routes. Which one, if either, reaches pixels?)
