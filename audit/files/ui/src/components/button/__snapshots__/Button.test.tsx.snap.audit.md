# AUDIT: ui/src/components/button/__snapshots__/Button.test.tsx.snap (snapshot honesty)
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 10 / 1 snapshot (`Button > DOM snapshot 1`) |
| File verdict | REAL (deterministic, current, faithful; one of 20 substantive tests) |
| Reachable from shipped entry point? | N/A (test artifact); the pinned component is the app's most-used control (21 files mount `<Button`; production variants: tinted ×8, primary ×7, default ×7, ghost ×2). Baseline #23 vitest 1705 includes this snapshot passing. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Snapshot read; Button.tsx read in full (70 lines) and compared attribute-by-attribute against the snapshot; Button.test.tsx read (20 tests: 9 behavioral + 10 CSS-token-proxy + 1 snapshot); generator located (:105-110); production variant census via `rg -o 'variant="…"'` over ui/src (tinted 8 / primary 7 / default 7 / ghost 2 — the snapshot's primary/sm is ONE of the used combos); button.css aria-busy spinner rules located (:215-220); scripts/visual-run.mjs + e2e/visual existence check (for the stale-rationale finding) |

## 1. One-paragraph verdict
Honest and, unlike the NumberField snapshot, fully deterministic: no React-internal tokens, just class composition + default type + children. It faithfully pins Button.tsx's real output for the generator's exact props (variant="primary" size="sm" → `lumen-button lumen-button--primary lumen-button--sm`, default `type="button"`, no aria-busy/disabled), it would fail on any structural or class-composition drift, and it sits inside a test file whose other 19 tests prove behavior (click wiring, keyboard operability, disabled/loading inertness, ref) and token discipline (10 regex pins against button.css). Two hygiene findings: only one of five variants is DOM-pinned (production's most-used modifier variant, tinted ×8 mounts, is not the one pinned), and the test file's header still justifies the DOM-snapshot technique with "4.G1's screenshot harness does not exist yet" — false at HEAD: that harness (scripts/visual-run.mjs + e2e/visual, 216 committed baselines) landed and is audited in this wave's file 19.

## 2. What actually works (verified)
- Faithfulness: every attribute traces to Button.tsx — `lumen-button--primary` from the non-BASE_VARIANTS branch (:57), `lumen-button--sm` from `size !== "md"` (:58), empty className filtered out (:59-61), `type="button"` default (:49,54), children pass-through, NO aria-busy (loading false → undefined, :63), NOT disabled (:64). The snapshot is the component's exact serialized output for `<Button variant="primary" size="sm">Snapshot me</Button>` (Button.test.tsx:105-110).
- Determinism: no useId, no generated tokens, no environment-dependent values — the string is stable across React upgrades (contrast NumberField's `_r_8_`, file 24 F-002).
- Would it fail if the code broke? YES: dropping the default type, breaking the BASE_VARIANTS exclusion (e.g. `--secondary` modifier leaking in), renaming either class hook, or wrapping the children in an extra node all fail the compare. Mutation-checked by reasoning against each branch of the class composition (:55-62).
- The technique's limits are compensated: the CSS-proxy describe block (:115-167+) regex-pins button.css through semantic token variables (rest/hover/pressed/focus-visible/disabled/loading + compact heights 28/32/36 + primary accent walk) — the "visual" half the snapshot cannot provide.
- Baseline #23: 1705/1705 vitest pass — the snapshot matches HEAD output TODAY.

## 3. Findings
### F-001 [S4]
Where: ui/src/components/button/__snapshots__/Button.test.tsx.snap:3-10 (single snapshot)
Code:
```
exports[`Button > DOM snapshot 1`] = `
<button
  class="lumen-button lumen-button--primary lumen-button--sm"
```
What is wrong: One DOM pin for one of five variants (primary) × one of three sizes (sm). The variant production uses MOST — `tinted`, the dialog confirm action, 8 mounts (AddSubtitlesDialog, NetworkStreamScreen, OpenMediaDialog, NewPlaylistDialog, CustomizePanels, BookmarksDialog, GotoTimeDialog, ShortcutsScreen) — has no DOM snapshot, nor does the no-modifier `default`/`secondary` branch (the non-obvious BASE_VARIANTS exclusion, behaviorally tested at :44-51 but never full-DOM-pinned), nor the loading state (aria-busy + disabled combo).
Why it matters: The snapshot's regression net covers one leaf of a 15-cell variant×size matrix; a class-composition regression in any other cell is caught only if a class-query test happens to target it. Blast radius small (the behavioral tests do cover variant classes), hence S4.
What correct looks like: Either one parametrized snapshot per variant (5 strings, cheap) or an explicit assertion that the tinted/default class compositions match their expected strings.
Confidence: HIGH — Verification: read; production variant census enumerated by rg.
### F-002 [S4]
Where: ui/src/components/button/Button.test.tsx:10-11 (the snapshot generator's stated rationale)
Code:
```ts
 * 4.G1's screenshot harness does not exist yet, and vitest stubs `.css`
 * imports so jsdom never cascades the real stylesheet.
```
What is wrong: The claim is stale at HEAD: card 4.G1's screenshot harness EXISTS — `ui/scripts/visual-run.mjs` ("Cross-platform driver for the visual harness (card 4.G1)"), `ui/playwright.visual.config.ts`, `ui/e2e/visual/{visual.spec.ts,manifest.ts}` and 216 committed Linux baselines (all verified in this wave's file-19 audit, with recorded runs: MATRIX-20261113.md 216/216 exit 0). The Button suite's "visual-snapshot adaptation" rationale (DOM snapshot + CSS-proxy BECAUSE no screenshot harness exists) no longer holds; the second half of the sentence (vitest stubs .css) remains true, so the adaptation is still reasonable — but the justification text is now false.
Why it matters: Comments that justify a technique by a false premise get copied into the next primitive's test file; a future reader may also conclude the app has no screenshot regression harness at all.
What correct looks like: "…the screenshot harness (card 4.G1) covers route-level surfaces only and vitest stubs `.css` imports, so jsdom never cascades the real stylesheet" — the true, current rationale.
Confidence: HIGH — Verification: read; harness existence + recorded runs verified (file 19 of this audit series).
## 4. Per-function table (snapshot blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Button > DOM snapshot 1` | 3-10 | REAL | S4 | deterministic + faithful (§2); single-cell coverage (F-001); generator rationale stale (F-002, in the test file) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "DOM snapshot" (:105-110, this file's generator) | YES | full-DOM string compare; every branch of the class composition enumerated in §2 |
| 9 behavioral tests (:27-103: native role/type, variant+size classes, base-variant no-modifier, click, keyboard, disabled, loading-inert, ref) | YES | real interaction/attribute proofs — the correctness net the snapshot leans on |
| 10 CSS-token-proxy tests (:115-167) | YES | regex pins on button.css mapped through semantic tokens — the "computed style" half; guards token discipline the DOM snapshot cannot see |
| Mutation check (reasoned): `--update` after breaking BASE_VARIANTS exclusion | NOT caught by the snapshot itself | regeneration rewrites the golden — intrinsic to the technique; the class-query test at :44-51 is the actual guard, which is why this stays honest |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| (implicit) the committed snapshot equals the component's output | file existence + baseline #23 pass | REAL | attribute-by-attribute comparison (§2) |
| "4.G1's screenshot harness does not exist yet" | Button.test.tsx:10 | **FALSE at HEAD** | visual-run.mjs + playwright.visual.config.ts + e2e/visual + 216 baselines exist with recorded runs (F-002) |
| loading state: "0.7 opacity + CSS spinner + aria-busy … fully inert" | Button.tsx:31-35 | REAL (behaviorally; not snapshot-pinned) | button.css:215-220 `[aria-busy="true"]` spinner rules + test :85-97 (aria-busy + disabled + no handlers) |

## 7. Dead / duplicated / unreachable code
- None. One snapshot, one generator, no orphans.

## 8. Dependencies & flags
Vitest Snapshot v1 format. No React-internal tokens (deterministic). Component coupling: Button.tsx's spread-last `{...rest}` (:65) can in principle clobber `aria-busy` (not destructured) — the only residual clobber surface (type/disabled/loading/className are all destructured); no test passes such props, latent and negligible; noted for whoever audits Button.tsx.

## 9. Open questions for the next agent
- The stale "harness does not exist yet" comment (F-002) may repeat in other primitives' test headers (Surface.test.tsx, Icon.test.tsx adopted the same adaptation per the states.md record) — one rg for "does not exist yet" would census them.
- If the Button gets a new variant, the snapshot matrix gap (F-001) grows by one unpinned cell — cheap to close at that touch point.
