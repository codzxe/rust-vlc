# AUDIT: ui/src/screens/settings/areas/subtitles.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 223 / 2 describe blocks, 11 tests, shared makeFixtureTransport |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:78 "subtitles.test.tsx (11 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg suite in baseline ui-test.txt |

## 1. One-paragraph verdict
The staged-surface honesty test for the subtitles settings area: the component under test IS deliberately inert (no engine seam exists), and the test pins exactly that honest posture — every control disabled with aria-checked/value-now state matching the frame, 14 rows staged under data-staged="subtitles-settings-seed" with pending.* setting keys, the staged reason reachable both via 7 operable ⓘ buttons (aria-expanded + real reveal on click) and sr-only twins on the other 7 rows, ZERO transport calls (asserted), and no schema bindings / no duplication of the playback screen's subtitle rows (D-rules asserted by absence). This is the correct way to test a stub surface: assert the stub honestly rather than fake interactivity.

## 2. What actually works (verified)
- Baseline ui-test.txt:78: `✓ src/screens/settings/areas/subtitles.test.tsx (11 tests) 422ms`.
## 3. Findings
Zero findings. The production surface is STAGED by design (own audit elsewhere); this test does not fabricate capability for it — the exact inverse of a vacuous green.
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| 3 sections in frame order | 43-56 | REAL | dataset.section order + headings |
| 14 frame rows | 58-63 | REAL | full label list |
| Tracks & Languages values | 65-77 | REAL | switches disabled + aria-checked true + System language + Automatic |
| Loading Subtitles | 79-90 | REAL | toggle ON + reused match-strategy copy (no re-declared key) |
| Appearance per frame | 92-138 | REAL | font value, segmented radios, colors + swatch CSS var, opacity slider aria-valuenow 50 + aria-disabled + 0/50/100 scale, outline Normal, shadow ON |
| 9 degree ticks | 140-143 | REAL | DOM count |
| 7 ⓘ glyphs | 145-151 | REAL | count pin |
| sr-only staged reason on bare rows | 153-171 | REAL | 14 staged, 7 bare each carrying .settings-vh reason |
| staged keys, no live bindings | 175-208 | REAL | data-staged count + key regex + info buttons operable (aria-expanded + reveal asserted after click) |
| never touches transport | 210-213 | REAL | transport.sent length 0 — the anti-fabrication pin |
| no schema binding / no duplication | 215-222 | REAL | data-setting count 0 + playback rows absent |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| never touches transport | YES | a hidden config.write on toggle would appear in the log |
| staged keys regex | YES | /^pending\.subtitles\./ — a real schema key wired early fails the pin |
| ⓘ reveal behavior | YES | aria-expanded flips + hinttext caption appears after click |
| frame values | YES | every value pin (ON/OFF, 50%, hex colors, System language) fails on drift |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "config-never-invents … zero transport calls, no schema bindings" | header :4-6 | REAL | asserted :210-222 |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
@testing-library/react; makeFixtureTransport shared fixture double from ./audio-fixtures (transport-port impl). No feature flags.
## 9. Open questions for the next agent
- When the subtitles settings seed lands, this suite must be rewritten (staged pins become wrong) — the honest staged marker is the contract for that migration.
