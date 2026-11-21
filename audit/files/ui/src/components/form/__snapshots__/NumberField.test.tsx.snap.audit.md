# AUDIT: ui/src/components/form/__snapshots__/NumberField.test.tsx.snap (snapshot honesty)
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 71 / 1 snapshot (`NumberField > DOM snapshot 1`) |
| File verdict | REAL (current, faithful, supplements — not substitutes — behavioral tests) |
| Reachable from shipped entry point? | N/A (test artifact); the component it pins is production-reachable (SettingField.tsx:294-309 renders NumberField for every int settings row). Baseline #23 vitest 1705 includes this snapshot passing. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Snapshot read in full; NumberField.tsx read in full and compared attribute-by-attribute against the snapshot (every attribute accounted for — present, omitted, or value-matched); NumberField.test.tsx test inventory read (9 tests); the generating test located (:86-89, props label="Snapshot" value={3} min={0}); Values.svg probed for stepper glyphs (no stroke-width paths found — glyph frame-match UNVERIFIED); SettingField.tsx:300 checked (production uses labelHidden) |

## 1. One-paragraph verdict
Honest snapshot: it is CURRENT (matches NumberField.tsx's real output attribute-for-attribute for the props the generating test passes), it pins meaningful structure (label association via for/id, both class hooks, native min/step/type/inputmode/value, the two pointer-stepper buttons with aria-hidden + tabindex -1, and their exact svg paths), and it is the NINTH test in a file that otherwise proves behavior (clamping, bounds-disable, native-attr parity, numeric-string handling, error a11y, disabled group) — the snapshot supplements rather than substitutes, which is the only honest use of the technique. Two hygiene findings: only the happy-path visible-label variant is pinned (production mounts NumberField with `labelHidden` — the DOM users actually get is unpinned), and the embedded React-19 `useId` token `_r_8_` makes the file churn on React major upgrades, where a blind `--update` would launder real structural changes alongside the id noise.

## 2. What actually works (verified)
- Faithfulness: every attribute in the snapshot traces to NumberField.tsx — `class="lumen-field__control lumen-number__control"` (:79), `type="number"` `inputMode="numeric"` (:80-81), `min="0"` `step="1"` (test prop / default :36), `value="3"` (:85), no `max` (test omits it — and test :42 "bounds are omitted from the DOM when not provided" pins that omission deliberately), no aria-* (labelHidden false and error undefined → React omits undefined attrs :82-84), label `for="_r_8_"` ↔ input `id="_r_8_"` (:38,71), steppers `aria-hidden="true"` `tabindex="-1"` (:98-99,111), svg plus/minus paths verbatim (:104,:117), `stroke-width="1.5"` from `strokeWidth={1.5}`. Both steppers enabled in the snapshot is CONSISTENT with the test props (value 3 > min 0, no max → atMin/atMax false, :65-66).
- The generating test is real and minimal: `render(<NumberField label="Snapshot" value={3} min={0} onChange={() => {}} />)` (NumberField.test.tsx:86-89) — the snapshot is not a hand-edited fantasy; it is the component's serialized output for exactly those props.
- Would it fail if the code broke? YES for structural drift: removing the label association, renaming either class hook, dropping the steppers, changing inputmode/type, or altering the svg paths all fail the string compare. Verified by reasoning through each mutation against the pinned string.
- Baseline #23: 1705/1705 vitest pass — this snapshot matches HEAD's component output TODAY.

## 3. Findings
### F-001 [S4]
Where: ui/src/components/form/__snapshots__/NumberField.test.tsx.snap:3-71 (single snapshot) vs the production call site SettingField.tsx:297-303
Code:
```jsx
      <NumberField
        className="settings-field__bare"
        label={rowLabel}
        labelHidden
```
What is wrong: The one pinned DOM state is the visible-label, no-class, no-error happy path — but every production mount of NumberField renders the `labelHidden` variant with an extra className (SettingField.tsx:297-303): no visible `<label>` node, `aria-label` on the input instead, extra class tokens. None of that DOM is snapshot-pinned anywhere.
Why it matters: The snapshot's regression net covers a variant that never ships, while the variant that does ship (accessible-name-via-aria-label, the w26.f02 row grammar) is guarded only by attribute-level queries. A structural regression in the hidden-label branch (e.g., the label node leaking back in, breaking the one-text-node row contract) would pass the suite.
What correct looks like: A second `toMatchSnapshot()` case with `labelHidden` + a className — or an inline structural assertion for that branch in NumberField.test.tsx.
Confidence: HIGH — Verification: read; production call sites enumerated (SettingField is the only in-app consumer family; the harness/e2e mounts reuse the same branch).
### F-002 [S4]
Where: ui/src/components/form/__snapshots__/NumberField.test.tsx.snap:9,18
Code:
```
    for="_r_8_"
```
What is wrong: The snapshot embeds React's internal `useId` token (`_r_8_`). React changed the id format between majors (18: `:r8:`-family → 19: `_r_8_`-family) and may change it again — every React major bump churns this file for reasons unrelated to the component, and the standard remediation (`vitest -u`) regenerates ALL snapshots in the run, laundering any real structural change that happened to coincide.
Why it matters: Churny snapshots train developers to reach for `--update` without reading diffs; an id-only change is also indistinguishable from a broken label/input association at a glance (both touch the same two lines).
What correct looks like: Pass an explicit `id="snap-number"` in the snapshot test (the component accepts `id` — :36,38), making the pinned DOM fully deterministic and React-internal-free.
Confidence: HIGH — Verification: read; component's id-override path traced (:38).

## 4. Per-function table (snapshot blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `NumberField > DOM snapshot 1` | 3-71 | REAL | S4 | current + faithful (§2); happy-path-only (F-001); useId-churn (F-002) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "DOM snapshot" (NumberField.test.tsx:86-89, this file's generator) | YES (structural) | full-DOM string compare; every mutation in §2 enumerated |
| The other 8 tests in NumberField.test.tsx (labels, clamp, bounds-disable, native-attr parity j01-1, bounds-omission, numeric-string j01-11, error a11y, disabled group) | YES (behavioral) | these are the correctness proofs the snapshot leans on — the snapshot cannot detect wrong-but-stable output (e.g., if the stepper glyphs were wrong from day one, the snapshot blesses them; frame cross-check UNVERIFIED — Values.svg contains no stroke-width glyph paths I could match, see Commands run) |
| Mutation check (reasoned): regenerate with `--update` after breaking the label association | NOT caught by the snapshot itself | `--update` rewrites the golden — this is the technique's intrinsic hole, mitigated here only by review discipline on snapshot diffs |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| (implicit) the committed snapshot equals the component's output | the file's existence + baseline #23 pass | REAL | attribute-by-attribute comparison (§2) |
| "Stepper step (doc 08 §1.5 Values.svg steppers)" — the glyphs mirror the frame | NumberField.tsx:15,26-27 | UNVERIFIED | Values.svg carries no small stroke paths matching `M5 1v8M1 5h8`; the frame's stepper marks may be fills or live in a different frame — no match found either way, so the snapshot pins glyphs whose frame-fidelity is unproven (harmless: decorative, aria-hidden) |

## 7. Dead / duplicated / unreachable code
- None. One snapshot, one generator, no orphans (rg: no other test in the file calls toMatchSnapshot).

## 8. Dependencies & flags
Vitest Snapshot v1 format (header line 1). Generated under React 19.3.1 (the `_r_8_` id format). No other coupling.

## 9. Open questions for the next agent
- NumberField.tsx itself carries the same `{...rest}`-after-aria-props spread-last hole recorded for TextField.tsx F-001 (line 91) — the snapshot test passes no rest props, so it cannot catch it; whoever audits NumberField.tsx should rate it.
- The Button.test.tsx.snap snapshot (next file, same technique) deserves the same two checks: variant coverage vs production mounts, and embedded unstable tokens.
