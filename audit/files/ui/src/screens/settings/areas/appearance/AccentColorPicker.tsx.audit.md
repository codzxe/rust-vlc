# AUDIT: ui/src/screens/settings/areas/appearance/AccentColorPicker.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 87 / 1 (AccentColorPicker) + 3 const tables |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — /settings/global Appearance section (schema ui=true row → SettingField special-cases this picker) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; cross-check ACCENT ids against tokens + onboarding vocabulary |

## 1. One-paragraph verdict
Correct, small, complete: 9 frame-ordered swatches wired to the settings draft (dirty/apply contract), live preview through ThemeContext.setAccent (guarded for provider-less hosts), unknown draft falls back to orange (keeps roving tabindex anchored), labels reuse the onboarding accent vocabulary (no duplicate keys). Persistence rides the settings provider (localStorage in browser, config.set-setting in desktop — the key `global.accent-color` exists in the schema).

## 2. What actually works (verified)
— verified by read: isAccentId membership check, `theme !== null` guard, effect deps correct; `setValue(entry.key, next)` is the provider's draft seam.
— cross-checked: FRAME_SWATCH_ORDER matches onboarding's ACCENT_ORDER (same 9 ids, same order — OnboardingScreen.tsx L196-206).

## 3. Findings
Zero findings. Checked: draft write path (provider), preview path (ThemeProvider), fallback path (orange), a11y (SwatchGroup radiogroup/roving — audited in its own lane), i18n reuse (onboarding.* keys exist in messages.ts per OnboardingScreen's use), key existence (`global.accent-color` in schema — settingsThemeSync.ts:44).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| FRAME_SWATCH_ORDER/ACCENT_LABEL_KEYS/isAccentId | 30-55 | REAL | data + guard |
| AccentColorPicker | 57-87 | REAL | draft + preview wiring |

## 5. Test honesty
Picker tests exist (persistence.ts:66-70 mentions "three picker tests"); UNVERIFIED content.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "previews the draft accent live through the ThemeContext seam" | header L10-11 | REAL | effect L73-75 |
| "LABELS reuse the registered onboarding accent vocabulary" | header L16-17 | REAL | ACCENT_LABEL_KEYS → onboarding.accent.* (same keys as OnboardingScreen) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
SwatchGroup, SettingsProvider, ThemeContext, generated tokens. No env vars.

## 9. Open questions for the next agent
- None material (engine ignores accent-color by design — it is presentation-only).
