# AUDIT: ui/src/components/button/switchCore.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 83 / 1 component (+2 interfaces) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — the single core behind both exported switches: form/Switch.tsx:43 (settings rows, staged rows across areas) and button/Toggle.tsx:37 (control bar parts). W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep SwitchCore consumers (the two skins, both verified to pass SwitchSkin objects); button.css shared-selector-list claim cross-checked in form.css audit (Switch.test.tsx:98 enforces the no-second-skin rule). |

## 1. One-paragraph verdict
The deduplicated switch core the j01-3 Lead ruling describes: one implementation, two class grammars via a skin object, correct switch semantics (native button, role=switch, aria-checked, Space/Enter free), decorative-but-localized micro labels through the useSwitchLabels i18n seam, and a disabled guard that is defensive but harmless. One API hazard: `{...rest}` spreads LAST, so a consumer can override `role`/`aria-checked`/`type` and silently break the switch semantics the core exists to guarantee.

## 2. What actually works (verified)
— Both skins mount the core with proper SwitchSkin objects (Toggle.tsx toggleSkin, Switch.tsx switchSkin — grep). Baseline: Switch.test.tsx (incl. the "no second skin in form.css" rule) and Toggle suites pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/button/switchCore.tsx:57-69
Code:
```
      <button
        ref={ref}
        type={type}
        role="switch"
        aria-checked={checked}
        className={[skin.root, className].filter(Boolean).join(" ")}
        disabled={disabled}
        onClick={(event) => {
```
```
        {...rest}
```
(line 69)
What is wrong: `{...rest}` is spread after every semantic prop, so any consumer passing `role`, `aria-checked`, or `type` through the ButtonHTMLAttributes inheritance silently overrides the core's guarantees (a `role="button"` switch, a stale aria-checked).
Why it matters: the core's whole reason to exist is "interaction fixes land once" — the spread order lets per-consumer props undo the shared semantics with no error.
What correct looks like: spread `{...rest}` FIRST, then the semantic props (letting only intentional internals win), or Omit "role"/"aria-checked" from the props type.
Confidence: HIGH — JSX prop ordering semantics certain.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SwitchCoreProps` (interface) | 11-17 | REAL | — | Omit children/onChange ✓ |
| `SwitchSkin` (interface) | 27-36 | REAL | — | 4-part anatomy |
| `SwitchCore` | 50-81 | REAL | S4 (F-001) | forwardRef + displayName; disabled guard defensive; labels via i18n seam |
| `displayName` assignment | 83 | REAL | — | |

## 5. Test honesty
Switch.test.tsx / Toggle tests (other agents): the j01-3 no-second-skin assertion (Switch.test.tsx:98) is a real mutation guard for the CSS side; interaction tests would fail if onChange polarity broke. F-001's override hazard is untested (no consumer currently passes role).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ONE core implementation now backs them, so interaction/i18n/skin fixes land once instead of twice" | switchCore.tsx:6-10 | REAL | both skins verified as thin wrappers |
| "a switch carries checked state, not pressed" | switchCore.tsx:41-42 | REAL | role=switch + aria-checked (not aria-pressed) |
| "The micro labels are decorative (aria-hidden …) but user-visible, so they resolve through useSwitchLabels()" | switchCore.tsx:45-47 | REAL | lines 71-77 |

## 7. Dead / duplicated / unreachable code
None (the disabled-guard at 65 is unreachable in real browsers — defensive only, noted not counted).

## 8. Dependencies & flags
useSwitchLabels (asset-group w15-g003), button.css. No external deps.

## 9. Open questions for the next agent
- useSwitchLabels owner (my asset group — audited there): does the ON/OFF label resolution cover the pseudo-locale (l10n pseudo-probe) path?
- Does any consumer actually pass extra button attributes through rest today (if none, F-001 is latent only)?
