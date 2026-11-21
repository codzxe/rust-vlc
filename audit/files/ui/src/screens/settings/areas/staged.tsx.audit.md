# AUDIT: ui/src/screens/settings/areas/staged.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 210 / 3 (useStagedMsg, StagedRow, StagedAdvancedBar + StagedControl type) |
| File verdict | REAL (well-built honest-dead factory; the composed screens are the problem — see F-001) |
| Reachable from shipped entry point? | NO binary at HEAD (ui-wide). SPA-internal: library.tsx:29-30, privacy-network.tsx:26-27, recording.tsx:29-30 import it; those areas are in SETTINGS_AREA_IDS (13-area registry) → SettingsScreen → /settings/:area. Executed under vitest (baseline run #23). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read complete; consumer census via `rg -n "StagedRow\|StagedAdvancedBar" areas/` — library: 7 rows + advanced bar (:59-122); privacy-network: 9 rows + bar (:51-131); recording: 9 rows + bar (:71-156); subtitles.tsx has its OWN local StagedRow (pending-state rows, different module — excluded); test census: library.test.tsx:93-99 asserts data-staged markers + dimmed count; areas tests green in baseline run #23. |

## 1. One-paragraph verdict
This file is the disciplined implementation of a bad situation: a factory that renders settings rows permanently INERT ("staged") because the schema keys behind them were never seeded, with genuinely good a11y honesty (sr-only reasons, aria-describedby on disabled controls, operable ⓘ reveals, data-staged markers, tooltips). The hostile finding is the composed reality: /settings/library and /settings/network are ENTIRE screens of dead controls (7 and 9 staged rows respectively, zero live rows), and /settings/recording ships 2 live rows against 9 staged + a staged Advanced bar — 25 permanently-dead rows plus 3 dead disclosure bars in the shipped sidebar's 13-area registry, all waiting on a Lead-owned schema seed that the comments say has not landed.

## 2. What actually works (verified)
— The honesty machinery is real and tested: library.test.tsx:93-99 asserts every `[data-staged-row]` carries `data-staged="schema-seed"` and the dimmed-row count; baseline run #23 green.
— a11y: disabled controls carry aria-describedby → a visually-hidden reason span (:165-168) or the operable SettingInfoDot reveal (:152-160); the k13 comment documents the keyboard-unreachable-title problem this solves.
— No fabricated behavior: controls are disabled, onChange is a no-op, values are static display strings.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/settings/areas/staged.tsx:1-9 (factory) with consumers library.tsx:59-122, privacy-network.tsx:51-131, recording.tsx:71-156
Code:
```
// keys exist behind them yet (schemas/settings.toml is the Lead's — see the
// lane log for the exact rows needed). Per the wave-I fix directive the
// frames' rows render in the frame's own visual grammar, STAGED INERT — the
```
What is wrong: The factory formalizes permanently-dead settings UI. Census of its consumers: /settings/library = 7 staged rows + staged Advanced bar and ZERO live controls; /settings/network = 9 staged rows + staged bar, zero live controls; /settings/recording = 2 live schema rows (directory/format via GENERATED_SCREENS.recording) against 9 staged rows + staged bar. Total: 25 inert rows + 3 dead disclosure bars shipped in the sidebar's registry.
Why it matters: Two of the thirteen settings screens are completely non-functional — every control disabled with a tooltip excuse — and a third is 82% dead. A user navigating the settings sidebar finds zombie screens. The honesty markers (data-staged, tooltips, sr notes) disclose the state to inspectors and AT users, but disclosure is not functionality; VLC's library/network/recording settings work. This is the "UI settings wired to nothing" checklist item at screen scale.
What correct looks like: The schema seed the comments reference (the row inventory in the lane log) landing in schemas/settings.toml so E1's generated screens render the rows live — the file's own exit condition (:36-37).
Confidence: HIGH — Verification: read + consumer census by rg (counts above); sidebar registry verified in areas/index.tsx (13 areas incl. library/network/recording).

### F-002 [S4]
Where: ui/src/screens/settings/areas/staged.tsx:106, 109, 122, 126
Code:
```
        disabled
        onChange={() => undefined}
```
What is wrong: Four no-op onChange handlers exist solely to satisfy the prop types of controls rendered permanently disabled. Harmless (disabled controls never fire), but they imply interactivity that does not exist to every reader of the code.
Why it matters: Code-level noise; a future un-disabling could silently ship the no-op as the real handler.
What correct looks like: Controls accepting `onChange?` optional, or an explicit `inert` prop on the primitives.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: ui/src/screens/settings/areas/staged.tsx:115-119
Code:
```
        options={[{ value: control.value, labelKey: control.value }]}
        // w26.k13 (j13 P3-12): the option's labelKey carries ALREADY-RESOLVED
        // copy — the identity resolver, made explicit instead of relying on
        // the documented @default identity fallback.
        resolveLabel={(value) => value}
```
What is wrong: Already-resolved display copy is funneled through the Dropdown's i18n `labelKey` field with an identity resolver — type-abuse of the i18n contract, acknowledged in the comment.
Why it matters: Defeats the "no user-visible literal is untyped" grammar the settings lane enforces elsewhere; a future l10n pass scanning labelKeys would treat display copy as a key.
What correct looks like: A `label` (pre-resolved) option field on the Dropdown primitive.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useStagedMsg` | 52 | REAL | — | makeAreaMsg(stagedKeys) — the shared factory (j13 dedup). |
| `StagedControl` type | 55-59 | REAL | — | 4 variants matching §C0 inventory. |
| `StagedRow` | 68-179 | REAL | F-001, F-002, F-003 | Builds disabled Switch/TextField/Dropdown/path+Browse per variant; aria-describedby wiring; operable ⓘ reveal; vh reason twin; data-staged markers. |
| `StagedAdvancedBar` | 191-210 | REAL | F-001 | Dead disclosure bar with vh reason twin; testid settings-advanced-staged. |

## 5. Test honesty (consumers' tests — no test file for staged.tsx itself)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| library.test.tsx:93-99 (staged markers + dimmed count) | YES | Asserts data-staged="schema-seed" on every row and the exact dimmed count (2). |
| privacy-network.test.tsx / recording.test.tsx (staged assertions, rg-confirmed presence) | YES | Same marker grammar per area. |
Gap: no direct unit test of StagedRow's a11y contract (aria-describedby → vh twin) — covered only indirectly via the area tests' marker checks.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no [[setting]] keys exist behind them yet (schemas/settings.toml is the Lead's)" | :4-5 | REAL | schemas/settings.toml carries no library/network section keys for these rows (generated/Screens.tsx registry has no library/network screens — verified in my file 7 audit). |
| "never a dead control that looks alive, never invented rows" | :8-9 | REAL | Controls disabled + marked; rows frame-census-backed (ⓘ circle census cited :12-13). |
| "When the schema keys land … these staged compositions yield" | :36-37 | UNVERIFIED (future) | The yield path exists (registry replaces), but no schema keys have landed as of HEAD. |

## 7. Dead / duplicated / unreachable code
None in-file. NOTE: subtitles.tsx:83 defines its OWN local `StagedRow` (different semantics: pending-state rows) — name collision across the areas tree, confusion hazard (S4-level note for subtitles' owner).

## 8. Dependencies & flags
SettingInfoDot (SettingField.tsx), Dropdown, Switch, TextField (my file 20), makeAreaMsg (shared.tsx — my file 15), staged.css. No feature flags — the staged state is unconditional.

## 9. Open questions for the next agent
- schemas/settings.toml owner: is the "lane log" row inventory (library 7 / network 9 / recording 9) recorded anywhere actionable, or only in wave logs? If the latter, the exit condition has no tracked owner.
- SettingsProvider (my file 12): do the LIVE rows on other screens actually persist (W11 prior says no)? The staged screens make that prior's blast radius smaller but sharper.
