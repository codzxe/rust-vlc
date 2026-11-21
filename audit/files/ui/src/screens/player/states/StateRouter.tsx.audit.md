# AUDIT: ui/src/screens/player/states/StateRouter.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 2 (resolveOverlay, StateRouter) + 2 exported types |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): PlayerScreen.tsx:126 imports, mounts at :1080 (browse branch) and :1126 (normal branch) — `import { StateRouter, resolveOverlay, … } from "./states"`; states/index.ts:33 re-exports. Baseline #22 `npm run build` OK. No GUI binary exists to host it (W14-A01: tauri crate cannot compile at HEAD). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` StateRouter.tsx (complete, 76 lines); `rg -n "StateRouter\|resolveOverlay" src -l` (consumers: PlayerScreen.tsx, states/index.ts, StateRouter.test.tsx); `rg -n "data-overlay\|data-state-overlay" src/screens/player/states/StageOverlays.tsx` + `rg -n "data-state-overlay" src` (attribute-name drift, F-001); `rg -n "type PlaybackState" -A1 src/generated/commands.ts` (8-member union — switch exhaustiveness); `sed -n '1070,1140p' PlayerScreen.tsx` (both mount sites + the `lumen-player__canvas` region the overlays stack in); `Read` StateRouter.test.tsx (665 lines, full matrix/home/css-skin/bridge suites); baseline ui-vitest.log.txt (aggregate only — 136 files/1705 tests green, per-file lines truncated). |

## 1. One-paragraph verdict
A genuinely pure, exhaustive, test-pinned state→overlay mapper — the honest heart of the player states lane. `resolveOverlay` covers all 8 members of the generated `PlaybackState` union with no default arm (compiler-enforced exhaustiveness — codegen enum growth breaks the build here, exactly the discipline error-display.ts documents), and the component mounts exactly zero-or-one overlay, matching the test's "never stacked" assertion. Both real defects are documentation-level: the component docstring promises a `data-state-overlay` attribute that exists nowhere in the codebase (the overlays set `data-overlay`), and the unsupported branch needs an unchecked `as NonNullable<…>` cast because the type system cannot see the invariant `resolveOverlay` just established. The engine-truth discipline (no fabricated progress — `null` → indeterminate) is enforced upstream in context-bridge.ts and pinned by tests; this file never invents state.

## 2. What actually works (verified)
— Verified by `rg -n "type PlaybackState" src/generated/commands.ts`: the union is exactly `idle|opening|buffering|playing|paused|stopped|ended|error` — every member has a case at StateRouter.tsx:30-44, no default, so a codegen-side state addition fails `tsc` (baseline #22 build green proves today's set compiles exhaustive).
— Verified by read of both PlayerScreen mount sites (sed 1070-1140): overlays render inside the labelled `lumen-player__canvas` region; the browse branch suppresses only the idle face (error/loading/buffering stay assertive above the grid); the normal branch suppresses the empty hero while media is present — the "never a fabricated state" claims in the comments match the actual gating expressions.
— Verified by StateRouter.test.tsx:56-92: the 8-row state matrix asserts both the pure function's return AND the rendered `data-overlay` attribute, plus the two unsupported-code routes (F-001's real attribute name is what the tests pin).
— Verified by baseline ui-vitest.log.txt EXIT=0 (136 files / 1705 passed aggregate; the log body lists only 5 sample files, so StateRouter.test.tsx's green is inferred from the aggregate, not a pasted per-file line — the same caveat recorded for every UI file this wave).

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/player/states/StateRouter.tsx:57-59
Code:
```tsx
/** Renders the resolved overlay over the canvas region. Exactly zero or one
 * overlay is ever mounted; `data-state-overlay` carries the kind for tests
 * and layout tooling. */
```
What is wrong: the documented hook `data-state-overlay` does not exist. `rg -n "data-state-overlay" src` matches exactly one line — this comment. The overlays actually carry the kind on `data-overlay` (StageOverlays.tsx:121 `data-overlay={kind}`, :308, :448), which is what StateRouter.test.tsx:73-78 asserts and what PlayerScreen's layout consumes.
Why it matters: the comment explicitly addresses test and layout-tooling authors; anyone following it greps for a phantom attribute and finds nothing — a naming lie in the one place people look for the contract.
What correct looks like: the comment names `data-overlay` (or the attribute is renamed to match — but 4 test assertions and the css-skin suite already pin `data-overlay`).
Confidence: HIGH — Verification: rg over the whole ui/src tree + read of StageOverlays.tsx and the test.

### F-002 [S4]
Where: ui/src/screens/player/states/StateRouter.tsx:70
Code:
```tsx
      return <UnsupportedOverlay error={view.error as NonNullable<typeof view.error>} onOpenMedia={onOpenMedia} />;
```
What is wrong: an unchecked cast to satisfy `UnsupportedOverlay`'s non-null error prop. The invariant (kind === "unsupported" implies `view.error !== null`) is established at line 38 by the same module's `resolveOverlay`, but the type of `kind` (a flat string union) does not carry it, so the compiler is silenced rather than satisfied. The identical cast pattern was flagged in ToolsDialog.tsx (F-003 there) and MusicMode.tsx — a tree-wide pattern.
Why it matters: if `resolveOverlay`'s error branch is ever edited (e.g. a new "unsupported" trigger that doesn't set `error`), this line becomes a silent runtime `null` hand-off into a surface that renders `error.code` — a crash only in the unsupported path, invisible to the type checker.
What correct looks like: `OverlayKind` as a discriminated payload union (`{ kind: "unsupported"; error: CommandError } | …`) so the switch arm recovers the non-null error without a cast.
Confidence: HIGH — Verification: read; invariant traced at :38; no other null-risk path into UnsupportedOverlay (single call site).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `resolveOverlay(view)` | 28-45 | REAL | — | Pure, exhaustive over the 8-state PlaybackState union, no default arm (compiler-enforced). Unsupported routing re-checks `view.error !== null` before `isUnsupportedError` — no null deref. Reachable: PlayerScreen.tsx:1078, :1115 gating + this file's own use. |
| `StateRouter({view,onRetry,onOpenMedia})` | 60-76 | REAL | F-002 | Zero-or-one overlay switch, exhaustive over OverlayKind; `null` for "none". `onRetry`/`onOpenMedia` are optional and forwarded verbatim (no ignored params). The unsupported arm's cast is F-002. |
| `type OverlayKind` | 17 | REAL | — | 6-member literal union; flat (see F-002's payload-union suggestion). |
| `interface StateRouterProps` | 47-55 | REAL | — | Both affordances optional + documented (ErrorOverlay degrades to an honest disabled stub without onRetry — pinned by test :202-219). |

## 5. Test honesty (StateRouter.test.tsx, green in baseline #23 aggregate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| state matrix (:56-92) | YES | Mutation check: map `opening`→`"buffering"` — `resolveOverlay(v)` returns the wrong string AND `overlays[0]` misses `data-overlay="loading"`; double-ended assertion. |
| unsupported-code routing (:83-91) | YES | Both `unsupported` and `unsupported-for-host` asserted against `"unsupported"` — delete the isUnsupportedError branch and both fail. |
| honest-progress suites (:96-134) | YES (guards the data the router forwards) | Asserts determinate/indeterminate from `openingProgress`/`bufferingProgress` — the exact fields StateRouter:66-68 forwards. |
| empty-surface affordances (:224-260) | YES | Pins the real `lumen:open-media` dispatch + onOpenMedia wiring + hero art — the props this file forwards. |
| css-skin suite (:338-419) | NO (not this file) | Pins states.css by regex — tests the stylesheet, not the router (relevant to StageOverlays/states.css owners). |
| PlaylistBrowse/context-bridge suites (:423-665) | NO (not this file) | Test StageOverlays/PlaylistBrowse/context-bridge — imported into the same file but outside this audit's subject. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Exactly zero or one overlay is ever mounted" (:57-58) | this file | REAL | Test :74 `expect(overlays.length).toBeLessThanOrEqual(1)`; switch returns a single element or null. |
| "`data-state-overlay` carries the kind for tests and layout tooling" (:58) | this file | FALSE | F-001 — attribute is `data-overlay` (StageOverlays.tsx:121); `data-state-overlay` exists nowhere. |
| "the engine view comes from context-bridge … never from optimistic UI state" (:3-4) | this file | REAL | context-bridge.ts read: state only from `session.state-changed` / snapshot seed; no timers, no synthesized values. |
| "renders over the (4.C1) canvas region" (:1-2) | this file | REAL (naming) / standing condition | Overlays do render inside the `lumen-player__canvas` region div (PlayerScreen:1008) — but that region displays no engine-decoded video in any buildable config (my PlayerScreen#part2 F-001 S0; W14-A01). The overlay layer is honest about states that a video-displaying surface never arrives to contradict. |

## 7. Dead / duplicated / unreachable code
- None dead: both exports consumed (PlayerScreen + tests). Duplication note: PlayerScreen calls `resolveOverlay` three times for gating (:1078, :1115, :1102) and StateRouter recomputes it internally — four evaluations per render of the same pure function on the same view. Harmless (pure, cheap), recorded as observation, not a finding.
- Unreachable as a shipped binary only in the tauri sense (W14); the web build ships it.

## 8. Dependencies & flags
- Imports: type-only from ./context-bridge (zero runtime coupling), isUnsupportedError from ./error-display, five overlay components from ./StageOverlays. No feature flags, no generated-code imports beyond the types. No i18n dependency (copy lives in StageOverlays).

## 9. Open questions for the next agent
- StageOverlays.tsx owner: my F-002 crash scenario (null error reaching UnsupportedOverlay) depends on UnsupportedOverlay rendering `error.code` unconditionally — confirm and, if so, consider the payload-union fix at the OverlayKind source.
- context-bridge.ts (another agent's lane): the snapshot-seed's `live`-backoff logic is the subtlest state machine in this directory — audit it against a remount-during-error scenario.
