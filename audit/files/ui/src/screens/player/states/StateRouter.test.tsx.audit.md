# AUDIT: ui/src/screens/player/states/StateRouter.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A01) |
| Date | 2026-06-14 |
| Lines / functions | 665 / 5 helpers + 10 describe blocks (47 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — vitest suite runs in CI (baseline ui-test.txt:34 `✓ src/screens/player/states/StateRouter.test.tsx (47 tests) 297ms`); the code under test (StateRouter.tsx:34-52 resolveOverlay, StageOverlays.tsx) is shipped via PlayerScreen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `wc -l` (665 L; StateRouter.tsx 76 L; StageOverlays.tsx 506 L); `rg -n "class MockTransport" ui/src/client/transport.ts` -> :49; `sed -n` quote checks (all verified verbatim); baseline ui-test.txt cited |

## 1. One-paragraph verdict
This is a genuine state-matrix test suite, not decoration: 47 tests execute against the real `resolveOverlay`/`StateRouter`/`StageOverlays` components and a real `context-bridge` reducer, with a scripted transport that emits real event kinds. It pins exactly the honesty properties the design docs claim (no fabricated progress %, no invented format info, stage-tagged errors, honest-unknown fallbacks) — the kind of assertions that FAIL if the component fabricates. All 47 pass in the orchestrator baseline. One hygiene defect: the `states.css` skin tests are raw-regex golden pins tightly coupled to formatting.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/ui-test.txt:34` -> `✓ src/screens/player/states/StateRouter.test.tsx (47 tests) 297ms` (whole lane 136 files / 1705 passed, line 178-179). Mutation reasoning: if `resolveOverlay` (StateRouter.tsx:34-52) returned `"none"` unconditionally, the 8-case matrix loop (test :68-81) fails immediately; if the Loading overlay fabricated an aria-valuenow without engine progress, :106-112 fails; if the bridge seeded state from a failed snapshot, :572-581 fails.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/states/StateRouter.test.tsx:338-419
Code:
```
describe("states.css — the W26 Figma skin (figma-fidelity-spec §1–§2)", () => {
  it("overlays sit on a dark scrim; failure cards are r12 panels with hairline + shadow", () => {
    expect(statesCss).toMatch(
      /\.lumen-state-overlay\s*\{[^}]*color-mix\(in srgb, var\(--color-surface-deep\) 72%, transparent\)/,
```
What is wrong: six tests assert CSS via hand-written regexes over `readFileSync(states.css)` — pins are order- and formatting-sensitive (`[^}]*--color-accent\) 32%` breaks if the property is reformatted, reordered, or moved to a different rule). Why it matters: brittle goldens produce false-red churn under innocuous refactors and silently stop matching intent when a value moves to another selector; this is pinning syntax, not the computed style. What correct looks like: assert computed styles of a rendered element (jsdom + getComputedStyle) or a normalized serialized rule set. Confidence: HIGH — Verification: read (tests pass today per baseline; the brittleness is structural, not current breakage).

## 4. Per-function table (helpers + suites; file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| view()/err()/IDLE_VIEW | 20-39 | REAL | — | honest fixtures, full-shape overrides |
| renderRouter | 41-52 | REAL | — | mounts under real ClientProvider chain like PlayerScreen |
| state × overlay matrix | 56-92 | REAL | — | 8 states + 2 unsupported-code variants; would fail on mapping regressions |
| honest-progress suite | 96-134 | REAL | — | determinate/indeterminate + aria-busy pins |
| unsupported whitelist suite | 138-172 | REAL | — | omits absent keys, rejects unknown (path leak probe :165-171) |
| error surface suite | 176-220 | REAL | — | retry wired-vs-stub pinned (data-wiring-stub :218) |
| empty/first-run suite | 224-260 | REAL | — | real window-event dispatch probe :236-240 |
| home surface suite | 262-334 | REAL | — | 3 sections, kind split, honest empty, no fake progress bars |
| states.css regex pins | 338-419 | REAL | S4 | F-001 |
| browse-grid key suite | 423-450 | REAL | — | console.error spy = honest collision probe |
| ScriptedTransport + bridge suites | 454-664 | REAL | — | remount seeding, stale-snapshot race (:629-645), transient-evidence clearing (:647-663) — race coverage is genuinely strong |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| matrix loop :68-81 | YES | resolveOverlay return value directly asserted |
| honest-progress :97-127 | YES | data-progress + aria-valuenow attributes asserted |
| bridge remount race :629-645 | YES | gated promise ordering; late snapshot must not regress live state |
| css regex pins :338-419 | YES (for text drift) | regex over the actual file content; would NOT catch semantic drift that keeps text |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "47 tests" run | baseline ui-test.txt:34 | REAL | `✓ ... (47 tests) 297ms` |

## 7. Dead / duplicated / unreachable code
None. `import.meta` dirname cast (:17) is an environment workaround, exercised green in baseline.

## 8. Dependencies & flags
Imports resolve: ClientProvider, MockTransport (ui/src/client/transport.ts:49), context-bridge, StateRouter, StageOverlays, PlaylistBrowse — all exist (rg verified).

## 9. Open questions for the next agent
None.
