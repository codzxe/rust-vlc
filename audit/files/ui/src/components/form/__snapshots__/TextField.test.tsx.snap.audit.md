# AUDIT: ui/src/components/form/__snapshots__/TextField.test.tsx.snap
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A04) |
| Date | 2026-11-24 |
| Lines / functions | 27 / 1 snapshot entry (forwards the ref + DOM snapshot) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test artifact for TextField, production-mounted e.g. subtitles.tsx settings area; snapshot consumed by TextField.test.tsx:51-56) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (useId/aria wiring in TextField.tsx, consumers, baseline) -> hits cited; sed -n '3,9p;13,25p' snap -> quotes verified |

## 1. One-paragraph verdict
One legitimate vitest snapshot pinning TextField's full DOM in the error state, and it pins the parts that matter: `label[for]` ↔ `input[id]` identity, `aria-invalid="true"`, and `aria-describedby` ↔ the error paragraph's `id` — the exact a11y wiring TextField.tsx:25-42 builds from `useId()`. The snapshot matches the live component (baseline run green). One hygiene note: it pins React's auto-generated `useId` strings (`_r_5_`), which are React-version-internal and will force regeneration on any React upgrade.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:148 `✓ src/components/form/TextField.test.tsx (8 tests) 131ms` — snapshot matched live output at baseline.
— verified by rg: TextField.tsx:25 `useId()`, :31 `htmlFor={inputId}`, :41-42 aria-invalid/aria-describedby — the pinned attributes are the component's real wiring, not test fixtures.

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/__snapshots__/TextField.test.tsx.snap:9,14,17,22
Code: (verbatim, sed-verified)
```
  <label
    class="lumen-field__label"
    for="_r_5_"
```
What is wrong: the snapshot hard-pins React's internal `useId` output (`_r_5_`, `_r_5_-error`) — an implementation detail of the current React/jsdom version, not a contract. Why it matters: any React upgrade or a change in the number of useId calls in the render tree shifts the ids and fails the snapshot for reasons unrelated to the actual contract (id *consistency*, not id *values*), training developers to reflexively `--update`. What correct looks like: assert id wiring programmatically (as TextField.test.tsx:24-26 already does for aria-describedby) and snapshot with ids normalized. Confidence: HIGH — Verification: read.
Balancing note: this brittleness is also detection — a broken `for`/`id`/`describedby` link fails the diff, so the pin is not worthless; it pins the right relationships via the wrong values.

## 4. What the snapshot pins (mutation reasoning)
| Component change | Snapshot fails? | Why |
|---|---|---|
| label htmlFor decoupled from input id | YES | both pinned to `_r_5_` (lines 9, 17) |
| aria-describedby not pointing at error id | YES | pinned pair `_r_5_-error` (lines 14, 22) |
| error not shown / aria-invalid missing | YES | `aria-invalid="true"` + error `<p>` pinned (lines 15, 20-25) |
| class grammar renamed | YES | `lumen-field*` pinned (lines 5, 8, 16, 21) |
| label text rendered twice (hideLabel regression) | YES | single "Snapshot" text node pinned (line 11) |
| styling/hover/focus tokens | NO | CSS, covered instead by the form.css regex tests (TextField.test.tsx:64-74) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| TextField.test.tsx:51-56 (produces this snap) | YES | full-DOM diff; error state + ref forwarding exercised in the same case (ref assert :54 is independent of the snapshot) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Error state wires `aria-invalid` + `aria-describedby`" | TextField.tsx:20 | REAL | pinned attrs (lines 14-15, 22) match component code :41-42 |

## 7. Dead / duplicated / unreachable code
None — exactly one entry, one `toMatchSnapshot` call (TextField.test.tsx:55), no orphans.

## 8. Dependencies & flags
None. Vitest snapshot v1 format; no env/config keys.

## 9. Open questions for the next agent
None. (If React is upgraded and this snap fails with only id-value diffs, that is F-001 materializing — regenerate, don't investigate.)
