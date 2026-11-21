# AUDIT: ui/src/components/form/SwatchGroup.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 106 / 2 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): consumed by the settings appearance area (AccentColorPicker.tsx) and the onboarding appearance step (OnboardingScreen.tsx) — both persist their picks through the settings overrides store. Baseline #22 build OK, #23 vitest 1705 passed incl. SwatchGroup.test.tsx (13 cases). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` SwatchGroup.tsx 1-106 (complete); `Read useRadiogroupKeyboard.ts 1-46` (shared roving hook: wrap-around, disabled-skipping, value-not-found → -1 lands on first enabled); `rg -n "export const ACCENTS" tokens/generated/tokens.ts` (generated set is ALPHABETICAL — the frame order here is a deliberate, different list); `rg -n "SwatchGroup" src --glob '*.tsx' --glob '!**/form/*'` (onboarding + appearance consumers); `sed -n '186,206p' OnboardingScreen.tsx` (onboarding carries its OWN identical ACCENT_ORDER copy — duplication evidence); `rg -n "describe|it(" SwatchGroup.test.tsx` (13 cases incl. frame order, roving semantics, disabled group + per-option, custom options, CSS skin checks). |

## 1. One-paragraph verdict
A correct, well-tested radiogroup primitive: proper `role="radiogroup"`/`role="radio"` semantics, roving tabindex with the selected swatch as the only tab stop, the shared four-arrow keyboard hook (wrap-around, disabled-skipping — the j01-2 dedup of a bug that existed in all three copies), no-op re-click on the checked swatch, and 13 tests including the CSS skin contract. Both consumers persist their picks (settings appearance + onboarding both write the overrides store). Two drift vectors, both S4: the default swatch ORDER list hard-codes the 9 accent ids (membership is not derived from the generated `ACCENTS`, so a tenth accent in the token pipeline would be accepted by `setAccent`/persistence but invisible to both pickers — and the identical order list is duplicated in OnboardingScreen), and a `value` outside the custom `options` leaves the group with no tab stop.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 13 SwatchGroup cases — 9 accents in frame order (orange first), checked+tabbable selected swatch, click select + re-click no-op, arrow wrap-around, disabled group and per-option disabled with arrow-key skipping, id wiring, custom options, DOM snapshot, and the form.css skin checks (track/radius, per-accent dot colors, 20×20 dot / 32×32 hit).
— Verified by read: the keyboard grammar delegates to the shared `useRadiogroupKeyboard` (j01-2 dedup); `onChange` fires only on actual changes (97: `if (!checked)`); per-option disabled composes with group disabled (95).
— Verified by read + `rg`: both consumers persist — AccentColorPicker writes the settings draft (my ThemeProvider audit chain), OnboardingScreen writes `global.accent-color`/`interface.theme` through the same SettingsProvider overrides store (OnboardingScreen.tsx:505-534, header 16-19); ThemeProvider bootstraps from that store, so onboarding accent picks survive reloads.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/form/SwatchGroup.tsx:36-48; cross-file: ui/src/tokens/generated/tokens.ts:28, ui/src/screens/onboarding/OnboardingScreen.tsx:190-201
Code:
```tsx
const DEFAULT_SWATCH_ORDER: readonly AccentId[] = [
  "orange",
  "red",
  "yellow",
  "green",
  "cyan",
  "blue",
  "violet",
  "pink",
  "gray",
];
```
What is wrong: the frame order list hard-codes accent MEMBERSHIP as well as order. The generated truth set is `ACCENTS` (tokens.ts:28, alphabetical — order genuinely cannot come from it, as OnboardingScreen's own comment at 188-189 explains), but nothing ties this list's membership to `ACCENTS`: a tenth accent added to themes/tokens.json flows into `AccentId`, `ThemeProvider.setAccent` accepts it, the overrides store persists it — and neither this picker nor onboarding's identical `ACCENT_ORDER` copy (190-201) renders it. The existing "renders the 9 design accents" test would keep passing (it asserts the nine, not completeness).
Why it matters: silent drift vector across three files (this list, onboarding's copy, the generated set) with no mechanical check; also plain duplication — the same nine-id list is maintained twice for order.
What correct looks like: keep the frame ORDER here once (export it), derive membership by assertion — e.g. a test `expect(new Set(DEFAULT_SWATCH_ORDER)).toEqual(new Set(ACCENTS))`, and have onboarding import the order instead of copying it.
Confidence: HIGH — Verification: read (all three sites compared; generated ACCENTS read).

### F-002 [S4]
Where: ui/src/components/form/SwatchGroup.tsx:93-94
Code:
```tsx
            // roving tabindex: only the selected swatch is tabbable
            tabIndex={checked ? 0 : -1}
```
What is wrong: if `value` is not among `options` (possible only with custom options — the default set covers all nine), NO swatch matches, so no swatch gets `tabIndex={0}`: the group has no tab stop and keyboard users cannot enter it (once entered by pointer, arrows work — the hook's `findIndex` returns −1 and lands on the first enabled option). Current consumers always pass a set containing the live value, so the edge is latent.
Why it matters: a11y edge for custom option sets; violates the roving-tabindex rule that the group must always have exactly one tab stop.
What correct looks like: `tabIndex={checked || noMatch ? 0 : -1}` with `noMatch = !options.some((o) => o.value === value)` (fall back to the first enabled option as the tab stop).
Confidence: HIGH — Verification: read (tabIndex logic + hook behavior traced; no consumer currently triggers it).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DEFAULT_SWATCH_ORDER`/`DEFAULT_SWATCHES` (consts) | 36-48 | REAL | F-001 | Frame order (differs from generated alphabetical ACCENTS by design); membership not tied to the generated set. |
| `SwatchGroup` (component) | 58-106 | REAL | F-002 | Radiogroup semantics, roving tabindex, shared keyboard hook, per-option/group disabled, change-only onChange. |

## 5. Test honesty (SwatchGroup.test.tsx, 13 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| frame order / radiogroup semantics / click+no-op / arrow wrap (10-44) | YES | DOM + keyboard-event assertions over the real component. |
| disabled group + per-option disabled w/ arrow skip (46-80) | YES | j01-14 regressions pinned. |
| custom options (87) | YES | Localized labels override the defaults. |
| skin checks (109-129) | YES | form.css rule assertions (dot colors, geometry). |
| NOT covered: membership-vs-ACCENTS drift (F-001), value-outside-options tab stop (F-002) | — | The gaps the findings live in. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Swatch set order per Settings - global.svg (frame: orange first …)" (31-34) | this file | REAL | The array matches the frame order; test pins orange-first. |
| "Same radiogroup semantics as RadioGroup/SegmentedControl: roving tabindex + arrow keys with wrap-around" (53-56) | this file | REAL | Shared hook + tests; F-002 documents the latent no-tab-stop edge. |

## 7. Dead / duplicated / unreachable code
- Duplication: the nine-id order list exists here AND in OnboardingScreen.tsx:190-201 (F-001).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Generated AccentId, shared useRadiogroupKeyboard, form.css skin. No flags.

## 9. Open questions for the next agent
- useRadiogroupKeyboard.ts + form.css are in the w15-g003 group — their owners should confirm the hook's −1-index behavior (first-enabled landing) is intentional and tested there.
- Whoever owns onboarding's ACCENT_ORDER: F-001's fix should collapse the two lists into one exported constant.
