# AUDIT: ui/src/screens/settings/areas/appearance/AccentColorPicker.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 87 / 3 consts (FRAME_SWATCH_ORDER, ACCENT_LABEL_KEYS, isAccentId) + AccentColorPicker |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): App.tsx:322 /settings/:area → SettingsScreen → SettingField consults SPECIAL_FIELD_WIDGETS (specialWidgets.ts:59, derived from schema: accentEntry.key "global.accent-color" → AccentColorPicker) → rendered on the global appearance row. Behaviorally tested (AccentColorPicker.test.tsx, 3 tests, baseline vitest row 23). Ceiling: no GUI binary at HEAD (W14). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read of SwatchGroup.tsx, settingsThemeSync.ts, specialWidgets.ts, AccentColorPicker.test.tsx, SettingsScreen.tsx:145-285; rg ACCENTS (tokens.ts:28 — ALPHABETICAL order), onboarding.accent.* msgids (messages.ts:503-506, 1988-1991), SETTING_GLOBAL_ACCENT_COLOR (schema.ts:107-116), useBlocker/usePrompt (ZERO hits — no router-level dirty guard), revert usage in SettingsScreen; sed verification of :70-72, :53-54. |

## 1. One-paragraph verdict
This picker is one of the few settings controls in the whole app with a fully real end-to-end chain: SwatchGroup click/arrow-key → `setValue("global.accent-color", …)` into the settings draft → the picker's preview effect AND the SettingsScreen-mounted `useSettingsThemeSync` both push the draft into ThemeProvider → `data-accent` on `<html>` (test asserts the DOM attribute flips to "blue"), and on fresh loads `readPersistedAccent()` bootstraps the provider from the same localStorage overrides store SettingsProvider persists to — so applied accents survive reload on the web app (the W11 "settings never persist" pin applies to the engine-side config path, not this localStorage seam). The radiogroup grammar is genuine (roving tabindex, aria-checked, wrap-around arrows via useRadiogroupKeyboard), the fallback for unknown draft values is disclosed and keyboard-safe, and the registration is derived from the schema (no hand-typed key). Two S4s: (1) the 9-accent list exists in FOUR copies with TWO orderings — generated ACCENTS (alphabetical, tokens.ts:28), this file's FRAME_SWATCH_ORDER (frame order), SwatchGroup's DEFAULT_SWATCH_ORDER (the same frame order duplicated verbatim), and the schema enumeration (whose own 9 msgids are carried in the w26.f01 lane but rendered by no runtime consumer, since the special widget supersedes the default enum renderer) — membership is aligned today (test pins it) but nothing derives the picker's list from the generated source, and a divergent member would crash `setAccent` (it throws outside the generated 9); (2) the live preview has no teardown against unguarded router exits: the dirty-gate covers only the app's own navigation calls — no useBlocker/usePrompt exists — so browser-back/URL/AppShell-link navigation with a dirty draft silently discards the draft while the previewed accent sticks on `<html>` for the session (reload reverts to the persisted value).

## 2. What actually works (verified)
- End-to-end live preview: AccentColorPicker.test.tsx:36-55 clicks the "Blue" radio, asserts the draft (`latest === "blue"`) AND `document.documentElement.dataset.accent === "blue"` — ran green in baseline row 23.
- Draft binding: `setValue(entry.key, next)` (:83) into SettingsProvider's dirty/apply contract; apply → persisted via the overrides store; `readPersistedAccent()` (settingsThemeSync.ts:88-92) bootstraps from the SAME store/key on next load — round-trip closes.
- Double-push is idempotent and disclosed: the picker's own effect (:70-72) plus `useSettingsThemeSync` (SettingsScreen.tsx:150) both call setAccent; settingsThemeSync.ts:97-99 documents the idempotence contract ("equal values are React no-ops").
- SwatchGroup semantics (component read): role=radiogroup + aria-label group name, per-swatch role=radio + aria-checked + localized aria-label, roving tabindex (only the checked swatch tabbable), arrow keys with wrap-around and disabled-skip (useRadiogroupKeyboard), onClick suppressed when already checked.
- Unknown-value fallback: `isAccentId` (:53-54) guards the raw draft; fallback "orange" keeps a checked swatch so the roving tabindex always has an anchor (:61-64 comment states the reason — verified: SwatchGroup's tabIndex is keyed to `checked`).
- Registration derived, not hand-keyed: specialWidgets.ts:34-37 finds the accent entry among seed-lane key candidates (`global.accent-color` / `interface.accent` / `global.accent`) — survives seed moves; registry contract AC-asserted in areas.test.tsx (my file-8 audit).
- Labels: all nine `onboarding.accent.*` keys exist in the union + catalog (messages.ts:503-506, 1988-1991); reuse-before-new-keys (contract §4.5) honored — the onboarding appearance step uses the same vocabulary (OnboardingScreen.tsx:559 comment confirms the shared contract).
- Null-theme guard: `if (theme !== null) theme.setAccent(current)` (:71) — renders standalone without a ThemeProvider (the settingsThemeSync precedent).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/settings/areas/appearance/AccentColorPicker.tsx:28-38 (and 41-51, 53-54)
Code:
```
/** Frame swatch order (Settings - global.svg — orange first, the default). */
const FRAME_SWATCH_ORDER: readonly AccentId[] = [
  "orange",
  "red",
```
What is wrong: The 9-accent data exists in four hand/generator copies with two different orders: generated `ACCENTS` is ALPHABETICAL (tokens.ts:28: blue, cyan, gray, green, orange, pink, red, violet, yellow), FRAME_SWATCH_ORDER here and DEFAULT_SWATCH_ORDER in SwatchGroup.tsx:36-46 (identical frame order, duplicated verbatim between the two files), and the schema enumeration (schema.ts:116 — frame order, with its own nine `settings.value.global-accent-color.*` msgids that no runtime renderer displays because this special widget replaces the default enum renderer; they survive the l10n dead-key gate only because the generated schema.ts text references them). Nothing derives the picker's list from the generated source.
Why it matters: Membership is aligned today (test :32 pins the picker's nine) but unenforced across copies: a 10th accent added to tokens + schema would silently not appear in this picker; a member in FRAME_SWATCH_ORDER absent from the generated nine would reach `theme.setAccent(current)` which THROWS (settingsThemeSync.ts:115 documents the throw) — a crash seeded by data drift in a sibling file.
What correct looks like: Derive the display order from ACCENTS via one exported reorder map (`const FRAME_ORDER: AccentId[] = [...ACCENTS].sort(byFrameIndex)`), or assert alignment at load/test time (`expect(FRAME_SWATCH_ORDER.every(id => ACCENTS.includes(id)))`).
Confidence: HIGH — Verification: read of all four sites + rg of ACCENTS order.
### F-002 [S4]
Where: ui/src/screens/settings/areas/appearance/AccentColorPicker.tsx:70-72 (preview effect, no teardown); ui/src/screens/settings/SettingsScreen.tsx:253-274 (the only dirty-gate)
Code:
```
  useEffect(() => {
    if (theme !== null) theme.setAccent(current);
  }, [current, theme]);
```
What is wrong: The preview effect pushes the DRAFT accent into the global ThemeProvider with no cleanup, and the dirty-draft navigation guard covers only the app's own chrome: `requestArea` (settings sidebar) and `requestClose` (the window ✕ → /player). rg over ui/src for `useBlocker|usePrompt|unstable_useBlocker` = ZERO hits — no router-level blocker. Browser back, URL edit, or an AppShell link while the draft is dirty unmounts SettingsProvider: the draft is silently discarded (never applied), but ThemeProvider keeps the previewed accent on `<html>` for the rest of the session; the next full load re-bootstraps from the persisted store and visibly flips back.
Why it matters: A session-scoped cosmetic lie — the UI shows an accent the user previewed and abandoned, then loses it on reload. Blast radius: the accent + theme-mode pickers share this seam (ThemeModePicker has the identical no-teardown effect); the draft-silent-discard half belongs to SettingsScreen's owner.
What correct looks like: A router blocker (react-router's useBlocker) behind the same dirtyCount gate, or a revert-on-unmount in SettingsProvider's disposal, or preview teardown restoring the applied value.
Confidence: HIGH on the mechanism (all sites read; no blocker exists), MEDIUM on the exact UX impact (no browser-navigation test in the suite — jsdom tests never exercise router-level exits).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FRAME_SWATCH_ORDER | 27-38 | REAL | S4 | F-001 hand-frozen copy #2 of the accent set |
| ACCENT_LABEL_KEYS | 40-51 | REAL | — | Total Record<AccentId, MessageKey> — tsc forces a label per member if the union grows (the one copy that cannot drift silently) |
| isAccentId | 53-54 | REAL | — | Membership check vs FRAME_SWATCH_ORDER (F-001 ties its safety to that list's alignment) |
| AccentColorPicker | 56-87 | REAL | S4 | Draft bind + double-idempotent preview + null-theme guard; F-002 no-teardown preview |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders the nine frame accents in frame order, orange selected by default" (:25) | YES | Pins count, ORDER (data-accent sequence), and default selection — catches F-001 order/membership drift on THIS file's copy |
| "binds swatch clicks to the settings draft and previews live" (:36) | YES | Draft value + DOM data-accent both asserted through the real ThemeProvider; mutation: drop the effect → :54 fails; drop setValue → :53 fails |
| "registers as the special widget … wherever the schema seeds it" (:57) | WEAK | Only asserts schema constants (key string + default) — the actual specialWidgets wiring is asserted in areas.test.tsx instead (cross-file); would not catch this file's deregistration |
| (gap) | — | No test for unknown-draft fallback, no browser-navigation preview-leak case (F-002 untested) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 9-swatch group on a 320×36 rx6 track — orange … first, then red, yellow, green, teal(cyan), blue, purple(violet), magenta(pink), gray" | :12-14 | REAL | Test :32 pins the exact order; schema enum matches; SwatchGroup/form.css carry the track grammar |
| "The existing SwatchGroup component carries exactly this grammar (radiogroup, roving tabindex, arrow keys with wrap-around)" | :15-17 | REAL | SwatchGroup.tsx:69-71, 84-101 read |
| "previews the draft accent live through the ThemeContext seam (data-accent on <html>)" | :17-20 | REAL | Test :54; settingsThemeSync.ts:31-39 documents the f08 round-2 fix this picker motivated |
| "the gap note in ui/src/theme/settingsThemeSync.ts names this widget as its follow-up" | :20-21 | REAL | settingsThemeSync.ts:33 "the f03 AccentColorPicker previews it live" |
| "LABELS reuse the registered onboarding accent vocabulary — the same nine names the onboarding appearance step shows" | :23-25 | REAL | ACCENT_LABEL_KEYS ↔ messages.ts onboarding.accent.*; OnboardingScreen.tsx:559 comments the shared contract |
| "an unknown draft value … falls back to [orange] rather than rendering no selection (the swatch group's roving tabindex needs an active swatch)" | :61-64 | REAL | Code :65-66; SwatchGroup tabIndex keyed to checked (SwatchGroup.tsx:94) — no-selection would kill tab reachability |
## 7. Dead / duplicated / unreachable code
- F-001: FRAME_SWATCH_ORDER duplicates SwatchGroup's DEFAULT_SWATCH_ORDER verbatim (the picker passes explicit options, so the SwatchGroup default is dead FOR THIS CALLER but live for default-props consumers — TokenSandbox uses ACCENTS directly, not SwatchGroup; the default set's live consumers: none found beyond tests — cross-ref for the SwatchGroup owner).
- The schema enum's nine `settings.value.global-accent-color.*` msgids: runtime-dead (superseded renderer), gate-alive (schema.ts text references) — folded into F-001.
## 8. Dependencies & flags
Consumers: specialWidgets.ts:59 (schema-derived). Dependencies: SwatchGroup (form primitives), generated tokens (AccentId type + ACCENTS via settingsThemeSync), ThemeContext, useSettings, useMsg. Persistence chain: SettingsProvider overrides store ↔ localStorageSettingsPersistence ↔ readPersistedAccent bootstrap (web-real; engine-side persistence is the W11 pin). No feature flags. The i18n-no-literals rule: labels flow through useMsg — clean (no allowlist entries needed).
## 9. Open questions for the next agent
- ThemeModePicker.tsx (unassigned in my list): same no-teardown preview (F-002 cross-count) and its own hand-frozen value list — same F-001 class.
- SettingsScreen owner: the silent-discard-on-unguarded-exit half of F-002 (dirty draft lost without the confirm dialog on any non-chrome navigation) is theirs to file or refute.
- If a 10th accent lands in themes/tokens.json, F-001 becomes live immediately (picker shows 9, tokens carry 10, TokenSandbox shows 10) — promote to S3.
