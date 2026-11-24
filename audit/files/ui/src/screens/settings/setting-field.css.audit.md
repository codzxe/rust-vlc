# AUDIT: ui/src/screens/settings/setting-field.css
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 449 / ~45 rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — skin for SettingField rows, rendered by SettingsScreen search results + every settings area screen (areas/*.tsx) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg structure scan (45 rule selectors), `rg -c 'var(--'` → 53, hex scan (comments only), corruption sweep (`!important`/`idden]`/`[h` → none), sed spot-reads |

## 1. One-paragraph verdict
Clean, disciplined settings-row control skin: all colors via semantic tokens (53 var() uses; raw hex only inside frame-provenance comments), structural px documented per rule, a deliberate one-class-deeper specificity strategy instead of `!important` (zero uses), and NO corruption from the tree-wide `[h`-deletion event. Nothing here pretends to be engine-wired — it is presentation for controls whose wiring lives in SettingField.tsx/SettingsProvider.

## 2. What actually works (verified)
— verified by `rg`: 0 `!important`, 0 corrupted selectors, 0 raw hex outside comments; every override pair (trigger/hover/focus-visible, option/selected, slider track/fill/thumb, values rows/radio/input-wrap) has matching states.

## 3. Findings
F-001 [S4]
Where: file-wide (rule depth strategy, header L5-8)
What is wrong: overrides win "on specificity alone, independent of bundle import order" — coupling this sheet's correctness to the primitive sheets' selector shapes; a primitive refactor (e.g. menu.css flattening `.lumen-dropdown--compact .lumen-dropdown__trigger` to one class) silently breaks these skins with no test to catch it (CSS untested by jsdom suite). Confidence: MEDIUM — Verification: read.

## 4. Per-function table
CSS rule groups: dropdown trigger/arrow/value/list/option (L29-110), field input hover/active/focus (L112-145), number steppers (L146-170), info hint + hinttext (L171-210), slider track/fill/thumb/ticks/scale (L211-287), values rows/radio/with-input (L288-377+). All REAL presentation rules; none dead.

## 5. Test honesty
No tests target this file (CSS; jsdom suite cannot assert geometry). UNVERIFIED visually.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Colors map to semantic tokens" | header L21-23 | REAL | rg: all color declarations use var(--…); hex in comments only |
| "Selectors are deliberately one class deeper ... so the settings skin wins on specificity" | header L5-8 | REAL | read: every rule is `.settings-field__control <primitive chain>` |

## 7. Dead / duplicated / unreachable code
None found.

## 8. Dependencies & flags
Token layer + primitive sheets (menu.css, form.css). No env vars.

## 9. Open questions for the next agent
- None material.
