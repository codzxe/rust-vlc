# AUDIT: ui/src/components/slider/Slider.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 353 / 17 tests across 2 describes + 6 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped component (ui/src/components/slider/Slider.tsx — the app's slider/seekbar/volume/EQ family); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "Slider.test" audit/runs/baseline/ui-test.txt` → `49: ✓ src/components/slider/Slider.test.tsx (17 tests) 180ms`; full file read |

## 1. One-paragraph verdict
A rigorous interaction-contract test: ARIA slider semantics, scrubs with exact live-vs-commit CALL ORDER (["live:25","live:50","live:75","commit:75"]), click-commits-once, step-lattice quantization, clamping outside the track, the full keyboard set (arrows/Shift×10/Page/Home/End/pass-through), disabled inertness with truthful aria-valuenow, ref forwarding + CSS-var progress, vertical bottom→min mapping, plus a semantic-token CSS guard and — the standout — pixel-math geometry tests that PARSE slider.css, extract the calc constants, and COMPUTE fill/thumb flush identity at five probe points (a reverted formula fails the arithmetic, not a string compare). The regex helpers carry built-in grammar guards (`expect(m).not.toBeNull()` + exact-grammar regexes) so wrong-rule matches fail loudly rather than pass silently.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L49 `✓ ... Slider.test.tsx (17 tests) 180ms` (inside 1705 passed).
- Geometry math verified by reading: L309-319 — `for p in [0,25,50,75,100]`, `thumbRight === fillEnd` at every p with the extracted constants (inset 2, extentLess 4, width 8); L339-345 vertical mirror with 1e-9 tolerance and stated justification.
- Interaction order pins: L64-81 exact sequence array; L92-95 single commit on click; L133-135 keyboard live-then-commit order.
- Not re-run here (no npm); baseline citation is the execution evidence.

## 3. Findings
None. Zero-findings checklist: tautology scan — all expects bind to attributes, call sequences, computed geometry, or parsed CSS content from the real stylesheet (readFileSync of the real slider.css at L10). Mock-of-subject scan — no vi.mock; vi.spyOn only pins getBoundingClientRect (L13-25), a jsdom-limitation workaround documented in-test ("jsdom rects are all-zero", L12). Skip scan — no it.skip; baseline L49 shows 17 passed. Name-vs-body scan — names describe exact asserted behavior (checked each). Approximation honesty — StatefulSlider (L33-46) simulates a real caller feeding commits back to value, the honest integration shape. Mutation reasoning: breaking live/commit ordering fails L81; breaking quantization fails L104; breaking the flush formula fails L310-319 arithmetic; breaking the calc grammar fails the travel() grammar expect L277.

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| interaction contract | 48-237 | REAL | — | ARIA, scrub order, click, quantize, clamp, keyboard set, disabled, ref+CSS vars, vertical, token guard |
| L8 flush geometry (pixel math) | 290-352 | REAL | — | parses CSS, computes flush identity at p probes; string-level belt-and-braces stale-grammar pin L348-351 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 17 | YES | exact call-order arrays, computed geometry equalities, grammar-guarded CSS parsing; verified green baseline L49 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "These tests parse the CSS source ... and COMPUTE the geometry at real track sizes, asserting the flush identity at every probed p" | block comment L240-248 | REAL | L309-319 / L339-345 arithmetic verified by reading |
| "jsdom rects are all-zero; pin a deterministic geometry" | comment L12 | REAL-UNPROVEN | standard jsdom fact; consistent with pinned-rect tests passing |

## 7. Dead / duplicated / unreachable code
None. cssBody/decl/px/travel/insetBottom each used; pinRect/pointer/progressVar used.

## 8. Dependencies & flags
@testing-library/react, vitest, node:fs (CSS read), react createRef/useState; real Slider + real slider.css.

## 9. Open questions for the next agent
- None.
