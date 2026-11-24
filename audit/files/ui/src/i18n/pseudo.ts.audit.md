# AUDIT: ui/src/i18n/pseudo.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 76 / 1 exported fn + 2 consts |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: consumed by messages.ts for the `?locale=pseudo` runtime (hardcode-proof + layout-proof locale) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | manual trace of the brace-depth scanner on nested/escape/unterminated inputs; rg consumer (exit 0); baseline ui-test.txt |

## 1. One-paragraph verdict
A correct deterministic pseudo-locale transform: a brace-depth scanner lifts every top-level `{…}` region (including nested plural arms and `{{…}}` escapes) into private-use sentinels so the accent map can never touch placeholders; ASCII maps to accented/tall glyphs; every string pads to ≥1.5x en-US length and bracket-wraps so an unbracketed string in a pseudo run proves a hardcode. I traced the scanner by hand on `{n, plural, one {# item}}`, `{{braced}}`, an unterminated `{abc`, and a placeholder-only string — all behave as documented (unterminated braces pass through un-masked and get accented, a sane malformed-input choice). Generated at runtime from en-US, never a second catalog — no drift surface.

## 2. What actually works (verified)
— Hand-traced transform on 4 edge inputs (documented above); sentinel regex cannot collide with real text (U+E000/E001 private use).
— Consumer: messages.ts locale switch (rg); pseudo tests in baseline (ui-test.txt pass).

## 3. Findings
None. Checked: placeholder preservation, nesting, escapes, determinism (no randomness), lengthening math (`ceil(len*1.5)`), bracket-wrap proof contract.

## 4. Per-function table
N/A (<100 LOC): single function reviewed in full.

## 5. Test honesty
pseudo tests (baseline pass) pin placeholder passthrough and the bracket proof; the hardcode-proof contract is exercised by the l10n CI gate (ui.yml "l10n catalog check" per baseline notes).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "placeholders `{…}` ... and `{{ }}` escapes pass through VERBATIM" | L12-14 | TRUE | hand-traced scanner |
| "generated at runtime from the en-US catalog (never checked in ... no drift possible)" | L16-18 | TRUE | no pseudo catalog file (rg) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None; pure function.

## 9. Open questions for the next agent
- None.
