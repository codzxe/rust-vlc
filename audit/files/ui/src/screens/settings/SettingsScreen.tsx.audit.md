# AUDIT: ui/src/screens/settings/SettingsScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 496 / 8 (SettingsShell, SettingsScreen, groupRows, iconOf, windowTitleKey, isAreaId, focusActiveNav, onWindowKeyDown + 3 small handlers) |
| File verdict | REAL-UNPROVEN (desktop config-bridge wiring is real code, untested by me against a live engine) |
| Reachable from shipped entry point? | YES — App.tsx:40 lazy route `/settings/:area` (13 areas, App.tsx:108-109) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | rg over App.tsx routes, config-settings.ts, app-transport.ts; read of SettingsProvider/persistence; test-file scan |

## 1. One-paragraph verdict
The best-wired screen in the UI so far: in the **desktop shell** Apply is the real thing — `createConfigSettingsPersistence(client, SETTINGS_ENTRIES)` (L459-463) sends `config.get-setting`/`config.set-setting`/`config.reset-setting` over the bridge and subscribes to `config.setting-changed` events (config-settings.ts:107-140 read), with an honest hydration gate (L477-487) so schema defaults never paint over unread engine values. In the **browser dev shell** the same Apply degrades to the localStorage seam. Focus trapping, dirty-gated navigation, cross-area search, deep-link canonicalisation are all real and tested (26 it() blocks). The defect: this real bridge exists only here — the OnboardingScreen mounts `SettingsProvider` WITHOUT it (localStorage only, even in the desktop build).

## 2. What actually works (verified)
— verified by code-path read: SettingsScreen L459-463 `isDesktopShell() ? createConfigSettingsPersistence(client, SETTINGS_ENTRIES) : null` → config-settings.ts `writeKey` sends `{ cmd: "config.set-setting", args: {area, key, value} }` (L128-140); `readKey` sends `config.get-setting` (L101-110); subscribe wires `config.setting-changed`. Desktop detection: app-transport.ts:40-42 `detectTauriBackend() !== null`. NOT executed against the Tauri engine (sandbox has no GTK/desktop build — baseline records desktop-tauri build failure as environment limit) — REAL-UNPROVEN at runtime.
— verified by read: dirty-gated nav + confirm dialog (L256-277, L430-443), Tab trap with exported FOCUSABLE selector (L140-141, L208-227), Escape seam with overlay guard (L239-250), search clearing on area change (L163-165).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/settings/SettingsScreen.tsx:449-463 vs ui/src/screens/onboarding/OnboardingScreen.tsx:767
Code:
```
  // App-layer wiring (issue #3 — the 4.H1 handoff d): in the desktop
  // shell, Apply round-trips the REAL engine config over the client
  // bridge (config.set-setting / config.reset-setting; hydration via
  // config.get-setting) and `config.setting-changed` events keep the
  // store live. The browser dev shell keeps the localStorage seam —
```
What is wrong: the real config bridge is wired ONLY into this screen. The OnboardingScreen — the FIRST-RUN surface where playback/subtitle/theme preferences are first collected — mounts bare `<SettingsProvider>` (OnboardingScreen.tsx:767), which defaults to `localStorageSettingsPersistence` (SettingsProvider.tsx:50). In the desktop build, a user's onboarding answers land in webview localStorage while the engine config stays at defaults; the same keys changed later via Settings DO reach the engine — two different persistence truths for the same schema keys depending on which screen touched them. Why it matters: first-run choices are silently lost to the engine; divergence between the two stores is user-visible after the first Settings visit. What correct looks like: extract the desktop-persistence decision into the shared provider (or pass it from the app layer). Confidence: HIGH — Verification: read (both mount sites). Runtime engine effect UNVERIFIED (no desktop build in sandbox).

F-002 [S4]
Where: ui/src/screens/settings/SettingsScreen.tsx:300-303
Code:
```
  const labelOf = (entry: { description: string }): string => {
    const text = t(entry.description as MessageKey);
    return typeof text === "string" ? text : "";
```
What is wrong: `typeof text === "string"` is always true if `t` returns string — the guard suggests `t` can return undefined; if so, search silently ignores such entries (documented as intended mid-wave behavior). Harmless but the double-truth (type says string, runtime checks) signals a seam smell. Confidence: LOW — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SIDEBAR_GROUPS/AREA_ICONS constants | 75-106 | REAL | - | 13 areas, orphan-catch in last group |
| iconOf/windowTitleKey | 108-116 | REAL | - | fallbacks |
| isAreaId/groupRows | 120-132 | REAL | - | future-area safety |
| FOCUSABLE export | 140-141 | REAL | - | shared with trap test (no drift) |
| SettingsShell | 143-446 | REAL | - | search/nav/trap/applybar all real |
| focusActiveNav + mount-focus effects | 188-206 | REAL | - | ring suppression honest |
| onWindowKeyDown | 208-227 | REAL | - | Tab wrap; skips aria-hidden |
| Escape effect | 239-250 | REAL | - | overlay guard |
| requestArea/requestClose/discardAndContinue | 256-277 | REAL | - | dirty gating |
| SettingsScreen (route element) | 449-496 | REAL-UNPROVEN | S2 | the desktop config bridge (F-001 context) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| SettingsScreen.test.tsx (26 it()) | YES for UI contracts | asserts nav/search/dirty/deep-links with mock transport |
| No test exercises the desktop config persistence against a real engine | — | config-settings has its own test file (W16-A08); the desktop path is UNVERIFIED end-to-end (no Tauri build in sandbox) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "in the desktop shell, Apply round-trips the REAL engine config over the client bridge" | comment L450-453 | REAL-UNPROVEN | code path verified (config-settings.ts:107-140); engine side + Rust handler not executed by me |
| "The browser dev shell keeps the localStorage seam — the demo transport honestly answers feature-not-exposed for config.*" | comment L453-456 | REAL | demo-transport config.* handling not re-verified here (A07's file); MockTransport error shape read (transport.ts:77-85) |

## 7. Dead / duplicated / unreachable code
None found.

## 8. Dependencies & flags
react-router (lazy route), client bridge modules, generated schema. No feature gates beyond runtime `isDesktopShell()` detection. No env vars.

## 9. Open questions for the next agent
- Does the Rust side implement config.set-setting for every SETTINGS_ENTRIES key the UI can write (schema drift engine-side)? Check crates config area vs generated/schema.ts.
- W16-A08 audits config-settings.test.ts — cross-check whether the apply diff logic (only changed keys written) is covered.
