# AUDIT: ui/src/screens/convert/convert.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 299 / 22 selector groups |
| File verdict | REAL — with a currently-unreachable production block (F-001) |
| Reachable from shipped entry point? | PARTIAL: the form/gate/queue-empty styles render on /convert (App.tsx route; baseline #22/#23 green). The job-row block (~:192-288) is UNREACHABLE in production — the screen's enqueue branch is gated on a convert command lane that does not exist in the engine catalog (verified: no convert.*/record.* in generated/commands.ts CommandRequest union); tests reach it only via mock transports listing convert.* |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -o 'convert__[a-z-]*' ui/src/screens/convert/*.tsx` (full TSX class inventory) — diffed against the sheet: 3 TSX classes have no rule (job-cancel / job-line--meta / queue-count — markers), every sheet selector is consumed · Read ConvertScreen.tsx:1-60/140-215 (lane probe + `if (laneGate) return;` enqueue gate; the engine-binding seam comment) · `rg -n "canEnqueue|enqueue" ConvertScreen.tsx convertQueue.ts` (only startJob adds jobs) · Read ConvertScreen.test.tsx:5-12/151-222 (both branches tested: no-lane disabled + mock convert.* lane renders a queued job) · token presence verified against themes/generated/tokens.css (--color-stroke-border, --color-surface-panel-raised, --color-surface-raised, all font/size/space tokens) · baseline: ui-vitest.log.txt (1705 passed), ui-build.log.txt |

## 1. One-paragraph verdict
The styling half of the /convert screen, whose TSX half is a model of capability honesty: the screen probes the command catalog for a conversion lane, finds none, and renders the queue action disabled-from-data with an aria-describedby gate — no fabricated jobs, no simulated progress. The sheet itself is clean and token-disciplined (semantic tokens throughout, raw px only as cited structural values), its selectors are fully consumed, and its `data-state="running"` accent rule and partial-progress tabular numerals implement the documented honesty rules (running state carries the one accent; a cancelled job keeps its last percent). The structural finding: roughly a third of the sheet (the job-row block) styles states that cannot occur in any shipped mount — the enqueue branch is unreachable while the catalog has no convert.* commands, so job styling is reachable only through the test mock's future-lane transport. That is deliberate forward-building (the engine-binding seam is a single documented branch), but it is dead-in-production styling today and must be flagged as such. The remaining defect is three marker classes with no rules.

## 2. What actually works (verified)
— Selector closure: every selector in the sheet maps to a ConvertScreen.tsx class (verified by full class extraction); the lane-note/gate styles the honesty machinery renders are all present (`convert__field-gate` :112, `convert__lane-note` :167, disabled-action grammar via Button).
— The honesty contract is test-pinned end to end: ConvertScreen.test.tsx:163-179 asserts the no-lane state (action disabled, `data-wiring-stub`, submit enqueues NOTHING), :185-211 asserts the bound/empty refusals on the mock lane, :213-222 asserts a queued job renders on a convert.*-listing transport — green in baseline ui-vitest.log.txt.
— The running-state accent rule (:236-238) matches the documented sparing-accent rule; `--color-surface-deep` progress track + `--color-accent` fill via both -webkit- and -moz- pseudo-elements (cross-engine `<progress>` coverage).
— Light-mode card step (:296-299) addresses the measured Δ4/255 panel-on-surface contrast problem with a documented fallback chain (`--color-surface-panel-raised, --color-surface-raised`) and names the pending token-lane fix.
— The two i18 usage-side pins (:23-25 SectionHeader 15px, :129-131 Dropdown trigger 42px) are documented drift fixes with their frame citations.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/convert/convert.css:190-288 (the job-row block)
Code:
```
/* One job row — playlist row anatomy: name + meta left, status right,
 * quiet hover fill, hairline between rows (menu-row grammar). */
.convert__job {
  display: flex;
```
```
.convert__job[data-state="running"] .convert__job-status {
  color: var(--color-accent);
}
```
What is wrong: ~100 lines of the sheet (convert__job, __job-line, __job-name, __job-target, __job-status, data-state="running", __job-side, __job-partial, __job-progress + pseudo-elements, __job-detail, __queue-list) style queue-job states that CANNOT occur in any shipped mount. The only code path that adds jobs is `startJob`, which returns early at `if (laneGate) return;` (ConvertScreen.tsx:163-168) because the engine command catalog contains no convert.*/record.* commands (verified against the generated CommandRequest union); no other enqueue path exists. Reachability today is test-only (mock transports listing convert.* commands).
Why it matters: dead-in-production styling for a feature product.toml keeps off (`transcode = false`, `recording = false` — rule F2); it will drift silently (no visual regression can catch it since nothing renders it) until the engine lane lands, at which point untested-in-practice geometry greets the first real job.
What correct looks like: acceptable as staged seam code IF marked — add a header comment on the block ("unreachable until the convert lane lands; engine-binding seam ConvertScreen.tsx:170") or gate the block behind the same seam the TSX uses. The unit-tested queue reducer (convertQueue.test.ts, 196 lines) at least pins the state machine the styles dress.
Confidence: HIGH — Verification: read of ConvertScreen.tsx gate + generated commands union + test file; production unreachability is deterministic (catalog content).

### F-002 [S4]
Where: ui/src/screens/convert/convert.css (absence); classes applied at ConvertScreen.tsx:321, :352, :371
Code:
```
                <div className="convert__job-line convert__job-line--meta">
```
```
                          className="convert__job-cancel"
```
What is wrong: three TSX classes have no rule in this sheet (or any other): `convert__queue-count` (:321 — only its `-num` child span is styled), `convert__job-line--meta` (:352), `convert__job-cancel` (:371 — a Button className relying entirely on the button primitive's own styles). None is documented as an intentional marker (unlike the options.css case, where the markers at least never pretended to be styled).
Why it matters: naked class names mislead maintainers into expecting rules (and grep-based "is this styled?" checks give false negatives); the `--meta` variant in particular suggests a layout difference that does not exist.
What correct looks like: delete the dead modifiers, or add the (possibly empty, documented) rules if they are seam hooks.
Confidence: HIGH — Verification: read + rg over all css for the three names (zero hits).

## 4. Per-selector table
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .convert .lumen-section-header__title | 23-25 | REAL | — | documented 16→15px drift pin |
| .convert | 27-35 | REAL | — | 640px column, rhythm gaps |
| convert__head / __title / __subtitle | 39-57 | REAL | — | working-screen head grammar |
| convert__card | 61-66 | REAL | — | Card primitive composition (no bg here — primitive owns it) |
| convert__field (+label/--row/--top) | 70-108 | REAL | — | label-left/control-right grammar; 34px input line-height pin |
| convert__field-gate | 112-116 | REAL | — | styles the honest Browse stub reason |
| convert__target-note | 119-124 | REAL | — | negative margin tuck documented |
| .convert .lumen-dropdown__trigger | 129-131 | REAL | — | 42px frame pin (drift fix) |
| convert__actions (+button min-width) | 135-146 | REAL | — | footer grammar; disabled primary rendered by Button |
| convert__queue-section / __queue-count-num | 150-158 | REAL | — | count-num only (parent marker, F-002) |
| convert__queue | 160-164 | REAL | — | queue card body |
| convert__lane-note | 167-173 | REAL | — | THE unavailable banner (rendered today) |
| convert__queue-empty | 175-180 | REAL | — | rendered today (empty queue) |
| convert__queue-list | 182-188 | REAL | S3 | F-001 — unreachable (no jobs possible) |
| convert__job block (+states/progress/detail) | 190-288 | REAL | S3 | F-001 — unreachable job styling; running-accent + tabular partial + cross-engine progress pseudos all correct as written |
| light-mode card step | 296-299 | REAL | — | documented Δ4 contrast fix with fallback chain |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ConvertScreen.test.tsx:116-136 (two real targets + honest notes + lane-note) | YES | asserts exact target set and the banner copy |
| :139-149 Browse stub (disabled, aria-describedby gate) | YES | asserts no dead enabled control |
| :163-179 no-lane submit enqueues NOTHING | YES | the ritual is gone — a fabricated job would fail it |
| :185-211 lane-present refusals (empty source, 9-job bound) | YES | mock-lane branch; exercises canEnqueue bound |
| :213-222 mock convert.* lane renders a QUEUED job | YES (DOM-level) | the only mount where job-row classes apply — test-only reachability (F-001) |
| convertQueue.test.ts (reducer lifecycle: enqueue/progress/cancel/partial) | YES | pure state machine, mutation-sensitive |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2); raw px is structural layout only" | file header :16 | REAL | verified per-selector (only cited structural px: 42px trigger, 34px line-height, 4px track, 100px buttons) |
| "The UI NEVER simulates progress or completion" | ConvertScreen.tsx:37 | REAL | no-lane gate blocks enqueue; tests pin it (:163-179) |
| "the queue lane is now DISABLED until that seam exists" | ConvertScreen.tsx:34-36 | REAL | laneGate early-return; the flip side is F-001 (the styled job states are unreachable) |
| "no capability shown that is not implemented" (card acceptance criterion) | ConvertScreen.tsx:27 | REAL | exactly two targets (WAV/MP4-remux), both engine-real per convertTargets.ts; Browse disabled with reason |

## 7. Dead / duplicated / unreachable code
- F-001: the job-row block — unreachable production styling (test-only via mock lane).
- F-002: three marker classes with no rules.
- No orphan selectors (everything in the sheet is applied by the TSX).

## 8. Dependencies & flags
- Surface primitives (Card, SectionHeader), Dropdown, Button class grammars; token set verified. No feature flags in the CSS (the TSX gate is catalog-probe-driven, not a build flag). stylelint compliance UNVERIFIED (lint:css not in baseline — standing gap; REQUEST-ORCH-RUN already logged).

## 9. Open questions for the next agent
- ConvertScreen.tsx / convertQueue.ts / convertTargets.ts owners: the engine-binding seam (ConvertScreen.tsx:170-183) is the single branch that activates F-001's styles — flag the CSS block when auditing the TSX.
- The `system.list-commands` probe path: verify the demo transport answers with the real namespaces so the gate settles (the test at :151-161 waits for it).
