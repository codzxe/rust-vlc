# AUDIT: ui/src/screens/not-found/NotFoundScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 95 / 2 (NotFoundScreen + ConeMark) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:310 `<Route path="*" element={<NotFoundScreen />} />` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg route + i18n keys (exit 0); baseline ui-test.txt (not-found suite passes) |

## 1. One-paragraph verdict
A clean 404: one heading, one body line, one real CTA (`navigate("/player")`), an inline three-segment cone SVG (aria-hidden, focusable=false, currentColor ink) whose duplication from the onboarding module is documented and justified by the one-way lane-import rule. Nothing fake, nothing dead.

## 2. What actually works (verified)
— Route + CTA navigation verified (App.tsx:310; L88).
— i18n: all strings through the seam; the lane table is a never-hit fallback since the catalog registered the keys (L28-31).
— Baseline: not-found tests pass (ui-test.txt).

## 3. Findings
None. Checked: no literals, no dead controls, aria-labelledby wiring, svg a11y attributes.

## 4. Per-function table
N/A (<100 LOC): both functions reviewed in full.

## 5. Test honesty
not-found suite (baseline pass) pins heading/body/CTA.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "geometry is frame-measured, duplicated here because the onboarding module does not export it" | L40-42 | TRUE (duplication acknowledged) | rg: onboarding does not export a ConeMark |

## 7. Dead / duplicated / unreachable code
ConeMark is a documented cross-lane duplicate (kept deliberately; noted, not counted as a finding).

## 8. Dependencies & flags
Button primitive, i18n seam. No gates.

## 9. Open questions for the next agent
- None.
