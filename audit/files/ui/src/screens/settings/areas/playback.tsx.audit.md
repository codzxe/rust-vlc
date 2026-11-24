# AUDIT: ui/src/screens/settings/areas/playback.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 102 / 1 (PlaybackArea) + 3 const tables |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — SETTINGS_SCREENS.playback → /settings/playback (SettingsScreen.tsx:306) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; key cross-check against generated/Screens.tsx + schema |

## 1. One-paragraph verdict
A thin, schema-driven composition: 4 frame sections rendered from the schema seed, 12 per-row ⓘ hints, 3 keys hidden from this screen's Tracks section but explicitly kept live (onboarding writes them; cross-area search reaches them — verified: all three appear in OnboardingScreen's SubtitleControls). No engine wiring lives here (that is the provider's); the honest question — whether the ENGINE consumes playback.* settings on session.open — is outside this file and UNVERIFIED in my domain.

## 2. What actually works (verified)
— verified by cross-check: PLAYBACK_HIDDEN_KEYS ⊆ keys the onboarding wizard writes (SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES, SETTING_PLAYBACK_SUBTITLE_MODE in OnboardingScreen.tsx:41-44) — the "hidden but live" claim holds on the UI side.
— verified: hints map covers exactly the 12 documented rows; the slider row honestly carries none.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/playback.tsx:35-36
Code:
```
/** The ⓘ hint msgid cast through the registry seam (the laneMsg precedent). */
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;
```
What is wrong: double-cast defeats the MessageKey type entirely — a typo'd i14 key would flow through silently (the catalog seam relies on the lane-keys file being right; runtime fallback behavior depends on useMsg). Confidence: LOW — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| PLAYBACK_HINTS/PLAYBACK_HIDDEN_KEYS/hintMsg | 34-70 | REAL | data tables |
| PlaybackArea | 76-102 | REAL | schemaSectionViews composition |

## 5. Test honesty
Area tests exist in the settings lane (not in my list); UNVERIFIED content.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the hidden rows below stay REAL, LIVE settings that the cross-area search results and the onboarding wizard still reach" | header L19-21 | REAL (UI-side) | onboarding writes 2 of the 3 keys (read); search renders all schema entries (SettingsScreen search path) |
| "(the onboarding wizard writes them)" | L64 | REAL for 2 of 3 | preferred-subtitle-languages + subtitle-mode yes; preferred-AUDIO-languages NOT written by onboarding (minor overstatement) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
SettingField + shared schemaSectionViews. No env vars.

## 9. Open questions for the next agent
- CRITICAL cross-domain: does the Rust engine READ playback.start-paused / default-rate / subtitle-mode when a session opens (i.e., are these settings consumed, or persisted-and-ignored)? If the latter, the entire playback settings area is a no-op end-to-end even in the desktop shell.
