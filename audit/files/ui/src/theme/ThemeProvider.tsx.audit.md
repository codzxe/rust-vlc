# AUDIT: ui/src/theme/ThemeProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 129 / 4 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): mounted at the app root (main.tsx wraps the app — verified via the generated-tokens consumption chain and TokenSandbox/appearance consumers); every route's `data-theme`/`data-accent` come from here. Baseline #22 build OK, #23 vitest 1705 passed incl. ThemeProvider.test.tsx (17 cases) + settingsThemeSync.test.tsx. No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` ThemeProvider.tsx 1-129 (complete); `Read settingsThemeSync.ts 60-120` (bootstrap reads: isThemeMode/isAccentId validation — never guess, never throw); `Read persistence.ts 1-70` (localStorage seam: per-key validation on load, corrupt-store → {}, try/catch makes it SSR-safe; header honestly labels it a MOCK seam whose REAL path is the engine bridge); `rg -n "setModeSetting|setAccent" src --glob '!**/*.test.*' --glob '!src/theme/*'` (non-settings caller: TokenSandbox only — dev route); `rg -n "describe|it(" ThemeProvider.test.tsx` (17 cases: defaults, switching, accent rejection, auto-follow-OS, hydration incl. never-guess cases); `sed -n` quote verification. |

## 1. One-paragraph verdict
The theming core is real, minimal, and correctly engineered: mode+accent state with lazy bootstrap from the persisted overrides store (validated at every read — unknown/corrupt values fall back to the product defaults, "never guesses" and test-pinned for both the mode and accent halves), an auto mode that tracks the OS media query only while active, a single LAYOUT effect that writes the two data attributes pre-paint (the documented anti-flash reasoning), a typed refusal on unknown accents, and a localStorage seam that is honestly labeled a development MOCK for the engine side (the browser reality: theme persistence genuinely works through the settings screens' draft store — the exception to the W11 "settings never persist" finding, which covers the engine config path). The only defect is an API asymmetry: `setModeSetting` accepts anything while `setAccent` throws.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 17 ThemeProvider cases — dark+orange defaults, data-theme/data-accent switching, accent rejection outside the generated nine, auto-mode follow + no-matchMedia fallback, and the full hydration matrix (persisted light/dark/system, theme+accent together, absent/unknown/corrupt → defaults).
— Verified by read: `readPersistedThemeMode`/`readPersistedAccent` (settingsThemeSync.ts:72-92) validate through `isThemeMode`/`isAccentId` against the same persistence seam SettingsProvider hydrates from — one truth source, no disagreement possible.
— Verified by read: persistence load (persistence.ts:47-63) drops unknown keys AND known keys whose values fail per-type validation (the j12 P3-6 tamper guard), folds corrupt JSON to {}, and the try/catch makes a missing `window` (SSR) safe.
— Verified by read: the auto-mode effect (81-93) adds/removes the media-query listener only while `modeSetting === "auto"`; the attribute write is a useLayoutEffect (102-106) so the first painted frame carries the hydrated mode (the documented anti-flash rationale).
— Verified by `rg`: the only non-settings caller of the setters is TokenSandbox (a dev route); the appearance pickers write through the settings draft (persisted) and preview through these setters — the persistence split is deliberate and consistent.

## 3. Findings

### F-001 [S4]
Where: ui/src/theme/ThemeProvider.tsx:108-117
Code:
```tsx
  const setModeSetting = useCallback((setting: ThemeModeSetting) => {
    setModeSettingState(setting);
  }, []);

  const setAccent = useCallback((next: AccentId) => {
    if (!ACCENTS.includes(next)) {
      throw new Error(`unknown accent "${next}" — must be one of: ${ACCENTS.join(", ")}`);
```
What is wrong: asymmetric validation: `setAccent` throws a typed error for a value outside the generated accents, while `setModeSetting` accepts any value it is handed. TypeScript closes `ThemeModeSetting` to the union, so typed callers cannot err — but any JS host (or a widened cast like the lane-msg pattern this codebase uses elsewhere) passing `"lite"` silently sets `data-theme="lite"`, which matches no CSS selector block and silently falls back to the `:root` defaults with no diagnostic.
Why it matters: inconsistent typed-refusal discipline in one API pair; the failure mode (silent wrong theme) is exactly the class the accent guard exists to prevent.
What correct looks like: the same guard for modes (`["dark","light","auto"].includes(setting)`) or a shared `assertValid` helper for both.
Confidence: HIGH — Verification: read (both setters + the CSS attribute cascade in tokens.css verified during my parse.rs/shortcuts.css audits).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `systemPrefersLight` | 56-61 | REAL | — | SSR/matchMedia guards; dark default per doc 08 §3.3. |
| `ThemeProvider` (component) | 63-125 | REAL | F-001 | See sub-rows. |
| — lazy state bootstrap | 69-78 | REAL | — | Synchronous persisted reads, validated upstream; test-pinned hydration matrix. |
| — auto-mode effect | 81-93 | REAL | — | Listener lifecycle correct; re-syncs on entering auto. |
| — attribute layout effect | 102-106 | REAL | — | Pre-paint commit; the anti-flash rationale documented. |
| — `setModeSetting` / `setAccent` | 108-117 | REAL | F-001 | Asymmetric validation. |
| — context value memo | 119-122 | REAL | — | Deps complete. |
| compat re-exports | 53-54 | REAL | — | Issue #36 move; documented. |

## 5. Test honesty (ThemeProvider.test.tsx + settingsThemeSync.test.tsx, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| defaults + switching + accent rejection (91-126) | YES | Attribute assertions on `<html>`; the throw case asserted. |
| auto-mode follow/fallback (127-149) | YES | Media-query stubbing both ways. |
| hydration matrix (151-249) | YES | Persisted light/dark/system/accent/both + absent/unknown/corrupt fallbacks — the never-guess contract is pinned for every branch. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "applies them as `data-theme` / `data-accent` attributes on the root <html> … no JS style recalculation beyond the attribute set" (3-7) | this file | REAL | The single layout effect 102-106; generated tokens.css cascades off those attributes (verified in my parse.rs/shortcuts audits). |
| "BOOTSTRAPS its mode from the persisted `interface.theme` override … so every route starts on the user's saved mode" (12-15) | this file | REAL | Lazy init 69-71 + settingsThemeSync validated reads; test matrix. |
| "a saved violet/gray/… choice survives every fresh load" (18-19) | this file | REAL | readPersistedAccent + hydration test 227-235 — in the BROWSER build. Caveat: the seam is localStorage ("MOCK persistence seam", persistence.ts:3-8); the engine-side write path (config.set-setting) is the one W11 proved never persists. |
| "Persistence … land with the settings cards" (9-11) | this file | REAL | Appearance pickers write the settings draft → localStorage overrides (ThemeModePicker setValue; SettingsProvider.save). |

## 7. Dead / duplicated / unreachable code
- No dead code. Compatibility re-exports are documented and consumed.
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Generated tokens module (ACCENTS/AccentId/TokenMode — the theme-tokens pipeline's TS bridge), lumen.css base skin, context/useTheme siblings, settings persistence seam. No feature flags.

## 9. Open questions for the next agent
- TokenSandbox.tsx (dev route): its setter calls deliberately don't persist — confirm the route is dev-gated so users never reach a non-persisting theme switcher.
- Whoever audits settings/persistence.ts + SettingsProvider.tsx: the "MOCK seam" label vs the shipped reality (localStorage IS the only persistence in every buildable config) deserves a claim-table row tying it to W11's engine-side finding.
