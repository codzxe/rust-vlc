# AUDIT: ui/src/components/form/__snapshots__/Checkbox.test.tsx.snap (snapshot honesty)
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / snapshots | 19 / 1 (`forwards the ref + DOM snapshot 1`) |
| File verdict | REAL — current and orphan-free, but the golden pins a framework-generated id and covers only the happy path |
| Reachable from shipped entry point? | N/A (test golden); the pinned component (form/Checkbox → settings/discovery choice rows) is reachable from shipped web-UI source. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read Checkbox.test.tsx (snapshot test :33-38) + Checkbox.tsx (useId :19-20, class map :22); read Toggle snapshot audit counterpart (anti-fake mock contrast); `rg "lumen-choice--error|--disabled" form.css` (styled at :275/:280) and `-g "*.tsx"` consumers (RadioGroup, SegmentedControl assert theirs at SegmentedControl.test.tsx:90-94; Checkbox tests do not); baseline #23 (vitest 1705 passed — golden is current) |

## 1. One-paragraph verdict
One small, current golden: the native input+label anatomy (span.lumen-choice--checkbox → input.lumen-choice__input + htmlFor label), provably matching today's output (baseline pass) with no orphaned entries. Two honesty limits. First, it pins React's auto-generated `id="_r_4_"` verbatim — the useId output is deterministic only under this file's exact test order (it is the 5th render in the file), so any inserted/reordered test produces a noise-only diff whose only honest resolution is a hand-inspected `-u` — the classic anti-pattern of snapshotting framework ids. Second, it is a single happy-path golden (valid, enabled, checked), and the component's invalid/disabled modifier classes (`lumen-choice--error`, `lumen-choice--disabled`) are asserted by no Checkbox test at all — the sibling SegmentedControl lane asserts its error modifier, so the gap is specific to this file. Both fail-closed (no false passes), hence S4.

## 2. What actually works (verified)
— verified by read Checkbox.tsx:21-35 + snapshot → the golden matches the component's real grammar exactly (`lumen-choice lumen-choice--checkbox` wrapper, `__input` native checkbox, htmlFor-bound label); `checked=""` is the controlled prop riding `{...rest}`.
— verified by baseline #23 (1705 passed) → the snapshot matches current output (a stale golden fails vitest).
— verified by read Checkbox.test.tsx:33-38 → both exports/calls paired: one snapshot export, one `toMatchSnapshot` call; no orphans; counter `1` = first write, never re-recorded.
— verified by read :7-31 → the behavioral tests around the golden carry the real coverage (label association via getByLabelText, controlled checked, aria-invalid, native disabled gate).

## 3. Findings

### F-001 [S4]
Where: ui/src/components/form/__snapshots__/Checkbox.test.tsx.snap:10-11
Code:
```
    class="lumen-choice__input"
    id="_r_4_"
```
What is wrong: the golden pins the framework-generated `useId` output (`_r_4_` = the sanitized «r4», the 5th render in this test file). It is deterministic only under the file's current test order and React's id scheme — inserting one test before :33, reordering, or a React id-format change produces a snapshot diff that is pure id noise.
Why it matters: a golden whose most probable future failure is noise trains developers to run `vitest -u` reflexively, which is exactly the habit that lets REAL drift slip through blind updates. The id proves nothing (it is not the component's contract — the `id` prop override and htmlFor binding are what matter, and htmlFor mirrors whatever id is generated).
What correct looks like: render the snapshot test with an explicit `id` prop (the component supports it, Checkbox.tsx:20 `id ?? autoId`), or mask generated ids with a snapshot serializer.
Confidence: HIGH — Verification: read of component + test; React 19.3.1 per package.json; baseline proves current determinism, not order-independence.

### F-002 [S4]
Where: ui/src/components/form/__snapshots__/Checkbox.test.tsx.snap:3-19 (whole golden — the absent variants)
Code:
```
exports[`Checkbox > forwards the ref + DOM snapshot 1`] = `
<span
  class="lumen-choice lumen-choice--checkbox"
>
```
What is wrong: the only golden is the happy path (no `invalid`, no `disabled`). The component's modifier classes `lumen-choice--error` (Checkbox.tsx:22, styled form.css:280) and `lumen-choice--disabled` (:22, styled :275) are produced but asserted by NO Checkbox test — the invalid test checks only the `aria-invalid` attribute (:23-25), the disabled test only `toBeDisabled` (:27-31). The sibling SegmentedControl lane proves the house pattern exists (SegmentedControl.test.tsx:90-94 asserts its `lumen-choice--error` class).
Why it matters: a rename or drop of either modifier class passes the entire Checkbox suite silently while the styling hooks (error tint, disabled presentation) stop applying in production.
What correct looks like: either two more golden renders (invalid/disabled) or one-line `toHaveClass("lumen-choice--error")` assertions in the existing tests.
Confidence: HIGH — Verification: read of the full test file + rg over form.css and sibling tests.

## 4. Per-snapshot table
| Snapshot | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `forwards the ref + DOM snapshot 1` | 3-19 | REAL | S4 (F-001, F-002) | anatomy + label binding pinned; generated id pinned (F-001); invalid/disabled variants unpinned anywhere (F-002) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| snapshot (:33-38) | YES for structure — class renames, dropped label/for binding, input-type change all alter the DOM; NO for state variants (F-002) |
| label association (:7-15) | YES | getByLabelText + click-to-change |
| controlled checked (:17-20), aria-invalid (:22-25), disabled (:27-31) | YES | attribute/behavioral assertions |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "native input (Space toggles, checked semantics free)" | Checkbox.tsx:13 | REAL | golden shows native `type="checkbox"` input + real `<label for>` |

## 7. Dead / duplicated / unreachable code
No orphaned snapshots. Contrast with the Toggle golden: no anti-fake mock here — this snapshot echoes static test-provided copy ("Snapshot" label), which is fine for a presentational primitive with no seam.

## 8. Dependencies & flags
Vitest snapshot v1; React 19.3.1 useId; produced by first-run write (counter 1). No CI flag bypasses snapshots.

## 9. Open questions for the next agent
- RadioGroup.tsx also applies `lumen-choice--error` (:57) — its own test lane should carry the same modifier-class assertion Checkbox lacks (F-002 pattern); whoever audits form/RadioGroup should check.
