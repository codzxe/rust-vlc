# AUDIT: ui/src/screens/open-media/OpenMediaDialog.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 396 / 4 helpers + 8 describe blocks (22 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:44 `✓ src/screens/open-media/OpenMediaDialog.test.tsx (22 tests) 437ms`; code under test ships via AppShell (dialog mounted from the shell-wide `lumen:open-media` event) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n OpenMediaDialog audit/runs/baseline/ui-test.txt` -> :44 (22 tests pass); `rg -n "dispose\("` ui/src/client/demo-transport.ts -> :347; `rg -n "export function parseSourceInput|OPEN_MEDIA_EVENT|sourceIcon"` -> OpenMediaDialog.tsx:56/:71/:81; `wc -l` -> 396 test / 326 source |

## 1. One-paragraph verdict
Genuine behavioral suite for the open-media funnel. It asserts the wire, not vibes: every open/load claim checks the actual `transport.sent` array for `session.open`/`playlist.load` with exact args ( :134-137, :190-191), asserts close/routing side effects, honest-invalid blank input (no command, no close), demo-clip gating per host/transport class, and first-field focus. The one weakness is cosmetic: the modal-layer suite pins CSS by absence-regex over the raw file.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:44 (22/22 pass; lane-wide 1705 passed line 178-179). Mutation reasoning: if single-click opened with a wrong SourceSpec, :135-137 fails; if blank input fabricated a source, :209-210 fails; if demo clips leaked to non-demo hosts, :268-269 fails; if focus landed on ✕, :237-238 fails.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/open-media/OpenMediaDialog.test.tsx:374-395
Code:
```
  // jsdom never cascades the real stylesheet (vitest stubs .css imports),
  // so layering is pinned at the SOURCE level (node:fs, Button.test.tsx
  // precedent). Since w26.a08 the shared .lumen-scrim primitive carries
  // z-index 20 — above the player stage chrome and the menubar popup layer
```
What is wrong: source-level CSS pins (`css).not.toMatch(/z-index/)`) can only catch literal re-introduction in this one file; a z-index violation moved into another sheet, or introduced via the shared class, is invisible to this guard. Why it matters: the "retirement guard" claims protection it structurally cannot provide tree-wide; it is honest about its own scope (comment admits source-level pinning) but the test name sells a layer contract. What correct looks like: a repo-wide lint rule or a computed-style assertion on the scrim primitive. Confidence: MEDIUM — Verification: read.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RecordingTransport | 38-53 | REAL | — | records wire, answers completed unit |
| RecordingDemoTransport | 59-66 | REAL | — | extends real DemoTransport, keeps instanceof semantics; dispose() real (demo-transport.ts:347) |
| LocationProbe / renderDialog | 68-84 | REAL | — | real router + provider chain |
| parseSourceInput suite | 86-106 | REAL | — | scheme/trim/blank contract |
| single-click flow suite | 108-218 | REAL | — | 8 tests incl. playlist handoff :182-199 (one-funnel: no session.open) |
| decoupled trigger | 220-226 | REAL | — | event-name pin |
| first-field focus suite | 228-260 | REAL | — | initial + re-open focus |
| demo-clip gating suite | 262-301 | REAL | — | non-demo transport, demo transport, desktop (TAURI stub :287-291 — env stub, not subject mock) |
| activation-rows suite | 303-350 | REAL | — | no aria-pressed, live footer Open, honest blank |
| source-kind suite | 352-372 | REAL | — | glyph mapping + decorative aria-hidden |
| modal-layer suite | 374-395 | REAL | S4 | F-001 absence-regex guard |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| single-click open :127-146 | YES | asserts sent command args + close + route |
| playlist handoff :182-199 | YES | asserts playlist.load args AND zero session.open |
| blank invalid :201-211 | YES | asserts zero commands + no close |
| demo gating desktop :283-300 | YES | relies on TAURI internals stub — environment double, subject (gating) still real code |
| css absence :388-394 | PARTIALLY | catches literal z-index/:has() re-add in this file only |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 22 tests run | baseline ui-test.txt:44 | REAL | `✓ ... (22 tests) 437ms` |

## 7. Dead / duplicated / unreachable code
None. `readFileSync` import used by the css suite (:23/:383).

## 8. Dependencies & flags
All imports resolve (DemoTransport/ClientProvider/transport verified by rg). jsdom + Tauri-internal stub is the only environment assumption; baseline green proves the harness runs.

## 9. Open questions for the next agent
None.
