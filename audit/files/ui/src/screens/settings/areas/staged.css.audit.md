# AUDIT: ui/src/screens/settings/areas/staged.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 83 / 7 selector rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): consumed by the schema-blocked settings areas — extensions.tsx, recording.tsx, effects.tsx, privacy-network.tsx (network-area testid at :46), library.tsx (library-area at :50), recording.tsx:60 — all mounted under /settings/*. Baseline #22 build OK, #23 vitest 1705 passed (the areas' tests assert the staged markers this sheet styles). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` staged.css 1-83 (complete); `rg -c -- "--opacity-muted:" ../themes/generated/tokens.css` (exists); `rg -ln "data-staged|data-advanced-staged|data-staged-dimmed" src/screens/settings --glob '*.tsx'` (consumer areas verified); `rg -n 'data-testid="(network|library|recording)-area"' --glob '*.tsx'` (the three testid hooks are set in PRODUCTION components, not just tests); cross-ref: my subtitles.css F-001 (the same full-strength-while-inert grammar, one finding already recorded there). |

## 1. One-paragraph verdict
A small, purposeful sheet: the honest-empty grammar for the schema-blocked areas — full-strength inert actions (the [data-staged] un-dimming), the frame's own dimmed-row presentation at the muted opacity token, and a set of meticulously derived per-section geometry corrections (the ±1.5px header-gap arithmetic is documented to the frame y-coordinates with the lane-log derivation referenced). One hygiene defect: the section-pitch rules are keyed on `data-testid` attributes — production styling riding on test-named hooks, refactor-brittle by construction. The full-strength-while-inert design stance is the same one I flagged in subtitles.css (F-001 there, cross-referenced, not double-counted).

## 2. What actually works (verified)
— Verified by `rg`: every selector's hook exists in production TSX — `data-staged`/`data-advanced-staged` (extensions/recording/effects), the three `data-testid="*-area"` roots (privacy-network:46, library:50, recording:60) — no dead rules.
— Verified by token check: `--opacity-muted`, `--color-surface-panel`, `--color-content-secondary` all exist in the generated tokens.
— Verified by read: the last-of-type margin fix (43-45) is arithmetically coherent (the :last-child zeroing cannot fire while the bar div follows the section — the +28px defect and its fix are both documented); the 12px row-margin rules land the 48px pitch per the frame arithmetic (36+12).
— Verified by baseline #23: the consumer areas' tests (effects/recording/extensions/privacy-network/library) run green, asserting the staged markers and embedded surfaces this sheet styles.

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/settings/areas/staged.css:51-58 (and 71-83's title-margin selectors)
Code:
```css
[data-testid="network-area"] [data-section="buffering"] .settings-field:not(:last-child) .settings-field__row {
  margin-bottom: 12px; /* frame: 36 + 12 = 48 */
}
```
What is wrong: production layout is keyed on `data-testid` attributes. The three area roots do set these testids in their production components (verified), but the attribute NAME says "for tests" — any refactor guided by that name (moving the id to the test wrapper, renaming to a semantic hook, deleting "unused" test scaffolding) silently breaks the buffering/proxy row pitch and the per-section title margins, with no selector-match failure in any test (the tests assert containment, not geometry).
Why it matters: styling semantics hidden behind a test-named attribute is a refactor trap; the same selectors could use the areas' existing semantic hooks (`[data-settings-area="network"]` — the attribute effects.tsx already sets at its root).
What correct looks like: re-key the selectors on `data-settings-area` (already present on every area root) and keep testids purely for tests.
Confidence: HIGH — Verification: read (both attribute systems verified present on the same roots).

## 4. Per-function (selector) table
| Selector | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[data-staged] .settings-action:disabled` | 15-18 | REAL | — | Full-strength inert actions (grammar cross-ref: subtitles.css F-001). |
| `[data-staged] .settings-advanced__bar:disabled` (+hover) | 22-29 | REAL | — | Kills affordance without dimming; hover restated at panel@95%. |
| `.settings-field[data-staged-dimmed="true"]` | 33-35 | REAL | — | The frame's own 0.3 row groups, on the muted token. |
| `.settings-sections[data-advanced-staged] > section:last-of-type` | 43-45 | REAL | — | The +28px column-close fix; arithmetic documented. |
| network-area buffering/proxy pitch rules | 51-59 | REAL | F-001 | 48px pitch via 12px margins; testid-keyed. |
| per-section title margins (library/network/recording) | 71-83 | REAL | F-001 | Frame-derived 8.5/10.5px gaps with the full derivation in comments; testid-keyed. |

## 5. Test honesty (consumer-area tests, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| effects/recording/extensions/privacy-network staged-marker suites | YES | Assert the data-staged/data-pending-command hooks this sheet keys on. |
| library/recording embedded-surface containment (e.g. recording.test.tsx:59) | YES | Pins the testid roots F-001 keys on — but asserts containment, NOT geometry, so a selector-key rename breaks styling silently (the finding's point). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the honest-empty grammar for the schema-blocked settings areas" (1-2) | this file | REAL | All consumer areas verified staged-marked; full-strength + dimmed grammar implemented as described. |
| "never a live disclosure with nothing behind it" (9-10) | this file | REAL | The staged bar stays collapsed + inert (disabled rules). |
| Frame geometry derivations (37-70) | this file | REAL (internally consistent) | Arithmetic checked (36+12=48; 22.5+8.5=31; 22.5+10.5=33); pixel verification belongs to the visual lane. |

## 7. Dead / duplicated / unreachable code
- No dead selectors. No duplication (the grammar intentionally mirrors settings.css's existing [data-staged] rule for a different element class).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- settings.css primitives + 3 tokens (all exist). No flags.

## 9. Open questions for the next agent
- The owner of settings.css should confirm the base `[data-staged]` form-control un-dimming this sheet extends (referenced at line 3) still exists — my subtitles.css audit verified the area-scoped twin, not the settings.css original.
