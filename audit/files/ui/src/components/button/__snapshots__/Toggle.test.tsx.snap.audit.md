# AUDIT: ui/src/components/button/__snapshots__/Toggle.test.tsx.snap (snapshot honesty)
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / snapshots | 54 / 2 (`toggle-off 1`, `toggle-on 1`) |
| File verdict | REAL — current (baseline-passing), orphan-free, and generated under an anti-fake mock by design |
| Reachable from shipped entry point? | N/A (test golden); the component it pins — button/Toggle → SwitchCore — is reachable from every settings toggle surface (settings areas mount Toggle/Switch). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read Toggle.test.tsx (mock at :15-18, snapshot test at :87-92, CSS string-pins :95-184); read Toggle.tsx + switchCore skin map (:15-20); cross-check snapshot DOM vs component grammar; baseline #23 (vitest 1705 passed — snapshots match current output or the suite would fail) |

## 1. One-paragraph verdict
An honest snapshot pair. The German-looking "AN"/"AUS" labels are not a localization leak — they are the test file's deliberately unique mock values (`vi.mock` of useMsg serving "AN"/"AUS", "values that exist NOWHERE else, so a bypassed seam cannot fake a pass", Toggle.test.tsx:12-18): a hardcoded "ON"/"OFF" literal in Toggle would fail both the j01-4 behavioral test and this snapshot. Both snapshots are exercised (two `toMatchSnapshot` calls), carry the first-write `1` counter (no churn), and are provably current (baseline vitest 1705-pass would fail on drift). The one honesty limit: the two snapshots differ ONLY in `aria-checked` — every state-dependent visual (fill shown when checked, off segment hidden) lives in CSS that a jsdom snapshot cannot see, so the pair's state coverage is exactly one attribute.

## 2. What actually works (verified)
— verified by read Toggle.test.tsx:15-18 + :87-92 → the snapshotted DOM is produced under the mocked catalog; the mock values are unique to this test, making the golden an active participant in the seam-honesty proof, not a blind echo.
— verified by baseline #23 (1705 passed, 0 failed) → both snapshots match the component's current output (a stale golden fails vitest, not silently passes).
— verified by read Toggle.tsx:15-20 (toggleSkin: root/onSegment/knob/offSegment) → every class in the snapshot (`lumen-toggle`, `__fill`, `__knob`, `__off`) is the component's real grammar; role="switch"/type="button"/aria-hidden structure matches SwitchCore's documented contract.
— verified by read :69-76 → the labels are pinned as decorative (aria-hidden) in a behavioral test too, independent of the golden.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/button/__snapshots__/Toggle.test.tsx.snap:3-53
Code:
```
exports[`Toggle (doc 08 §4.5: role=switch) > DOM snapshot (on and off) > toggle-off 1`] = `
<button
  aria-checked="false"
```
What is wrong: nothing dishonest — a recorded limit: the on/off golden pair is byte-identical except `aria-checked="false"` vs `"true"` (both render the "AN" fill span AND the "AUS" off span; the shown/hidden toggling is pure CSS `[aria-checked="true"]` display rules). A mutation that inverted which label displays in which state passes both snapshots.
Why it matters: a future reader could cite this file as proof of state-dependent presentation; the actual coverage for that lives in the CSS string-pins (Toggle.test.tsx:117-139 assert the `display: none`/`display: flex` rules in button.css) — the two together are sound, either alone is not.
What correct looks like: one comment line in the test above `toMatchSnapshot` noting the golden pins structure only, presentation lives in the CSS pins below.
Confidence: HIGH — Verification: read of both files; diff of the two snapshot bodies.

## 4. Per-snapshot table
| Snapshot | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `toggle-off 1` | 3-27 | REAL | S4 (F-001) | full anatomy pin: role=switch, type=button, 3 aria-hidden spans, class grammar, mocked labels |
| `toggle-on 1` | 29-53 | REAL | S4 (F-001) | identical except aria-checked="true" — the only state delta the DOM carries |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| DOM snapshot (on and off) | YES | structural mutations (class rename, dropped span, role change, literal labels instead of the seam) all change the serialized DOM; a `checked`-prop → aria-checked break flips the only differing attribute |
| companion j01-4 seam test (:78-85) | YES | queries the mock-only strings; a bypassed seam renders the lane fallback and fails |
| companion CSS string-pins (:95-184) | YES | covers what the snapshot cannot see (state display rules, tokens, shared skin) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| (states.md, cited in test :71) "checked state is conveyed by aria-checked alone; micro labels decorative" | snapshot + Toggle.test.tsx:69-76 | REAL | both snapshots show aria-hidden labels + aria-checked attr; behavioral test pins it |

## 7. Dead / duplicated / unreachable code
No orphaned snapshots (2 exports, 2 calls). No duplication.

## 8. Dependencies & flags
Vitest snapshot v1 format; produced by the standard first-run write (counter `1` = never re-recorded → no snapshot churn history). No CI flag bypasses snapshots (baseline ran plain `vitest run`).

## 9. Open questions for the next agent
- switchCore.tsx itself (the shared core both Toggle and form/Switch render) is not in my assignment — if unowned, its aria/class assembly deserves the same honesty pass; the golden here pins only the Toggle grammar.
