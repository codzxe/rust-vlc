# AUDIT: ui/src/screens/player/controls/state-mapping.test.ts
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 124 / 4 describes, 24 tests (it.each parametrized) |
| File verdict | REAL (executed in the committed baseline: 24/24 green) |
| Reachable from shipped entry point? | NO (vitest suite; module is shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
The doc-08 §5.1 PlayButton state table pinned exhaustively: all 8 playback
states x {glyph, disabled, pending, restart, busy} asserted object-equal;
pendingToggle semantics in playing/paused; stop enablement per state; seek
disable REASONS (i18n keys pinned: no-media vs not-seekable — "never
guesswork"); busy-while-buffering with seek still enabled; the keymap seam's
owned-command allowlist (null for anything else). The table IS the spec — pure
function, no mocks at all.

## 2. What actually works (verified)
- Baseline (ui-test.txt:122): `✓ src/screens/player/controls/
  state-mapping.test.ts (24 tests) 6ms` — green.
- Two [PROPOSAL] rows (buffering, stopped) are pinned with the proposal noted
  in-row (:37-38, :41-42) — honest about where the doc is silent.

## 3. Findings
None. Checked: fakeness (none), coverage (every enum value exercised through
it.each), honesty (a mapping change moves exactly one failing row), mock
discipline (zero mocks).

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| PlayButton table (10 tests) | 20-69 | REAL | full 8-state table + pendingToggle |
| stop button (8 tests) | 71-84 | REAL | both it.each sets |
| seek (4 tests) | 86-111 | REAL | reasons + busy |
| controlCommandOf (2 tests) | 113-124 | REAL | allowlist seam |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 24 | YES | object-equality rows over every enum value |
## 6. Doc claims: doc 08 §5.1/§5.2/§5.3 mapped 1:1 to tests. REAL.
## 7. Dead code: none.
## 8. Dependencies: vitest + the pure module.
## 9. Open questions: none.
