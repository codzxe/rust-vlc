# AUDIT: ui/src/components/form/__snapshots__/TextField.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / entries | 27 / 1 snapshot export |
| File verdict | REAL (honest, current, single-owner — with a fragility defect) |
| Reachable | Test artifact: consumed by TextField.test.tsx:51-55 (`toMatchSnapshot()`); the suite ran green in baseline vitest (INDEX.md row 23: 1705 passed) — the snapshot is CURRENT at HEAD (a stale snapshot would have failed the suite). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read of the .snap + TextField.test.tsx (snapshot owner block :51-56, error-wiring test :20-27, full describe inventory); rg toMatchSnapshot census (exactly 1 call → no obsolete exports); TextField.tsx:25 useId source check; baseline row 23 citation. |

## 1. One-paragraph verdict
A single vitest snapshot of TextField's error-state DOM, honest on the dimensions that matter: it has exactly one owner test (no obsolete exports — the file's only `toMatchSnapshot` is :55), it is current (the baseline's 1705-pass proves a match at HEAD), and the test name ("forwards the ref + DOM snapshot") does what it says — the ref assertion (:54, instanceof HTMLInputElement) is real, the snapshot is the second assertion, not the whole test. As evidence, the snapshot correctly plays the supplementary role: it pins DOM structure (lumen-field label/input/error nesting, class hooks, the aria-invalid + aria-describedby wiring shape) against accidental drift, while the file's four behavior-first tests do the correctness work — notably :20-27 already pins the a11y id CHAIN assertively (aria-describedby → the error element's id) WITHOUT pinning the id value, which is the superior form. The one defect is the classic snapshot anti-pattern: the committed bytes embed React `useId()` output (`_r_5_`, `_r_5_-error`) — implementation-volatile values that have changed format across React majors before (18's `:r1:` → 19's `_r_1_`) and shift with tree position, so a React upgrade can break this snapshot with zero functional drift, inviting the reflexive `--update` that erodes the file's drift-detection value.

## 2. What actually works (verified)
- Single owner, no orphans: exactly one `toMatchSnapshot()` (TextField.test.tsx:55) for the file's single export — no obsolete-snapshot rot.
- Current at HEAD: baseline row 23 green (a mismatched snapshot fails the suite).
- Honest test name: ref forwarding asserted directly (:53-54) before the snapshot.
- The a11y chain is pinned TWICE, once well: the snapshot shows label[for=_r_5_] ↔ input[id=_r_5_] ↔ aria-describedby=_r_5_-error ↔ p[id=_r_5_-error] (structural), and :20-27 asserts the same chain dynamically (aria-describedby resolves to the live error element's id) — the snapshot adds the label-for/input-id half and the class/nesting pins.
- Snapshot content sanity: error state (aria-invalid, error paragraph "boom"), label text node present — consistent with TextField.tsx's documented contract (:20 "Error state wires aria-invalid + aria-describedby").
## 3. Findings
### F-001 [S4]
Where: ui/src/components/form/__snapshots__/TextField.test.tsx.snap:9-17 (and the test that bakes them, TextField.test.tsx:51-55)
Code:
```
  <label
    class="lumen-field__label"
    for="_r_5_"
  >
```
```
    aria-describedby="_r_5_-error"
```
What is wrong: The snapshot embeds React `useId()` output (`_r_5_`, `_r_5_-error` — TextField.tsx:25 `const autoId = useId()`). useId values are implementation details: their FORMAT has changed across React majors (18 `:r1:` → 19 `_r_1_`) and their VALUES depend on the component's position in the render tree. A React upgrade or a test-tree reshuffle breaks this snapshot with zero functional change.
Why it matters: A false-alarm snapshot failure trains the reflex "run with --update" — after one such cycle the file's real value (detecting unintended DOM drift) is weakened, because the update that fixes the id churn can silently absorb a real structural regression in the same diff. The behaviorally meaningful part of these ids (the CHAIN, not the values) is already pinned without volatility by test :20-27.
What correct looks like: Normalize the volatile ids before snapshotting (`html.replace(/_r_[\d_]+_/g, "ID")` on container.firstChild.outerHTML, or prettyDOM with a serializer override), keeping the chain shape while dropping the volatile values.
Confidence: HIGH — Verification: read of the .snap + the useId source + React's documented useId format history; the chain-pinning alternative test read.
## 4. Per-entry table
| Snapshot entry | Owner test | Current? | Verdict | Notes |
|---|---|---|---|---|
| `TextField > forwards the ref + DOM snapshot 1` | TextField.test.tsx:51-56 | YES (baseline row 23) | REAL | Error-state DOM: label/input/error nesting, class hooks, a11y attrs; F-001 volatile ids |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "forwards the ref + DOM snapshot" :51 | YES | Ref assertion is direct; the snapshot fails on ANY structural/class/a11y-attribute change — including spurious id-only changes (F-001, the flip side) |
| (context) error-wiring test :20 | YES | The dynamic chain assertion — this is the test that carries the correctness claim; the snapshot only guards drift |
| Snapshot-as-evidence caveat | — | A golden generated by the code under test proves what the code renders TODAY, not what it should render — here that limitation is neutralized by the four behavior-first siblings; the snapshot is a pin, not the proof |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| (implied by suite green) snapshot matches HEAD render | vitest state | REAL | Baseline row 23 |
| TextField contract: "Error state wires aria-invalid + aria-describedby" | TextField.tsx:20 | REAL | Both attrs present in the snapshot; dynamically pinned at :20-27 |
## 7. Dead / duplicated / unreachable code
- None: single live export, single consumer.
## 8. Dependencies & flags
Generated/maintained by vitest (Snapshot v1 header); depends on React's useId formatting (F-001). No feature flags. The .snap lives next to the test per vitest convention; committed (drift-detected in CI via ui.yml vitest step).
## 9. Open questions for the next agent
- TextField.tsx itself (not assigned to me): the hideLabel branch (aria-label instead of label element) has no snapshot — deliberate (the behavior test :43-49 covers it); if a second snapshot is ever added, the same id normalization should apply.
- If the team ever adopts vitest's `snapshotFormat` or custom serializers, F-001's fix belongs there (one place for all future snapshots).
