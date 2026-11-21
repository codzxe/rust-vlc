# AUDIT: ui/src/screens/convert/ConvertScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 406 / 6 (1 component + 3 helpers/consts + 1 hook) |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | URL-ONLY: the /convert route exists (App.tsx:309) but NOTHING in the shipped UI links to it — no rail stop (RAIL_ITEMS, AppShell.tsx:145-158), no menu item, no navigate() call anywhere (rg: only the route def, the crumb that renders when already there, App.test.tsx:80, e2e/visual/manifest.ts:68). A user reaches it only by typing the URL. W14 caveat: no shipped desktop binary anyway (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | rg 'convert\|record' src/client/demo-transport.ts (0 hits — demo listing has no convert lane); rg '"convert\|record"' generated/commands.ts union (0 — catalog has none); sed -n '2913,2924p' docs/feature-matrix.toml (ui-dialog-convert = not_started ×4, evidence empty); rg '"/convert"' src e2e (no in-app link); convertQueue.ts reducer skim (enqueue is the only job-creating path). |

## 1. One-paragraph verdict
An honestly-gated dead screen: the capability probe (`system.list-commands` → filter for convert/record) is real and correctly conservative, and it resolves to "no lane" on EVERY transport that exists today — the generated command catalog (generated/commands.ts:24 union) carries no convert.*/record.* command and the browser DemoTransport's LISTED_COMMANDS (demo-transport.ts) lists none — so the Start button is permanently `disabled` and `startJob`'s enqueue branch is unreachable, making the queue UI (lines 314-403: job rows, cancel buttons, progress bars) dead code in every production render. Add the second death: no in-app navigation path to /convert exists at all. The screen's honesty engineering (aria-describedby gate notes, data-wiring-stub markers, no simulated progress) is genuine and well-executed — but the shipped artifact is a form that can do nothing but navigate away, on a route nothing links to. The feature matrix row (ui-dialog-convert) says not_started ×4 with empty evidence, which is the truth about capability and understates only that this chrome exists.

## 2. What actually works (verified)
— The lane probe really sends `system.list-commands` through the client transport and really parses the `command-listings` envelope (lines 122-142; demo-transport.ts:398-401 answers it). The gate logic (`laneGate = lane.resolved && !laneAvailable`) correctly stays enabled-but-inert while the probe is in flight (never presumptuous). Baseline: ConvertScreen suites pass in ui-vitest.log.txt (1705 total); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/convert/ConvertScreen.tsx:299-310 (+ 153-182, 314-403)
Code:
```
          <Button
            type="submit"
            variant="primary"
            disabled={laneGate}
            aria-describedby={laneGate ? "convert-lane-note" : undefined}
```
```
    if (laneGate) return;
```
(line 157)
What is wrong: the screen's primary action is permanently disabled in every build: no convert.*/record.* command exists in the generated catalog union (generated/commands.ts:24) or the DemoTransport listing, so `laneGate` is always true once the probe settles. Consequently `startJob`'s enqueue branch (169-181) is unreachable and the entire queue UI (314-403 — rows, cancel, progress) can never render in production (enqueue is the only job-creating path; queue is always `[]` → "queue.empty" forever). On top of that, no in-app link navigates to /convert at all.
Why it matters: a shipped route whose user-visible core is dead twice over (unreachable by navigation, non-functional by capability) — ~60% of this file's render tree is exercised only by tests and the visual harness; the "job queue wired to the command-catalog capability probe" (App.tsx:56-58) is wired to a probe that can only ever answer "no".
What correct looks like: either land the engine commands and enable the lane, or remove the route from the shipped bundle until the lane exists (the matrix already says not_started).
Confidence: HIGH — catalog union read; demo listing read; rg for navigation entry points.
### F-002 [S4]
Where: ui/src/screens/convert/ConvertScreen.tsx:83-89
Code:
```
function useConvertMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}
```
What is wrong: same dead-by-contract pre-registration `??` fallback as AppShell.tsx:128-132 (messages.ts:3011-3019 guarantees a total catalog). Copy-pasted convention across lanes.
Why it matters: divergence risk between lane copy and merged catalog; 40+ copies of the pattern per AppShell's own note.
What correct looks like: retire the seam post-wave.
Confidence: HIGH.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useConvertMsg` | 83-89 | REAL | S4 (F-002) | i18n seam |
| `LaneState` / `LANE_ABSENT` | 91-97 | REAL | — | |
| `STATUS_KEY` | 99-106 | REAL | — | 6 states, all keys exist in lane catalog |
| `ConvertScreen` (component) | 108-406 | SHALLOW | S2 (F-001) | renders; core gated dead |
| — lane probe effect | 122-142 | REAL | — | cancelled-flag cleanup correct; failure → honest absent-lane |
| — `startJob` | 153-182 | SHALLOW | S2 | gate return at 157 makes 169-181 unreachable in production |
| — `cancelJob` | 184-186 | UNREACHABLE-in-practice | S2 (via F-001) | no job can ever exist to cancel |
| — render: head/form/source/browse | 188-253 | REAL | — | Browse honestly disabled stub (238-240) with gate note |
| — render: target Dropdown + note | 255-285 | REAL | — | 2 targets only (convertTargets.ts, other agent) |
| — render: footer actions | 287-312 | REAL | S2 (F-001) | Start permanently disabled |
| — render: queue section | 314-403 | UNREACHABLE-in-practice | S2 (F-001) | queue.length always 0 in production |

## 5. Test honesty
No tests in this file. The colocated suites (other agents' files) drive the queue via the reducer/events directly — the only way those branches ever run. Baseline vitest passes them; they prove the reducer logic, not that any user can reach it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Today the catalog carries NO convert.* / record.* commands" | ConvertScreen.tsx:29-31 | REAL | generated/commands.ts:24 union — no convert/record ids |
| "the queue is driven by the ENGINE-BINDING SEAM … probes the command catalog" | ConvertScreen.tsx:27-29 | REAL (the probe) / SHALLOW (the queue) | probe real (122-142); queue can never receive a job (F-001) |
| "The UI NEVER simulates progress or completion" | ConvertScreen.tsx:37 | REAL | progress only via engine events; no job → no progress shown |
| matrix row ui-dialog-convert status not_started ×4, evidence "" | docs/feature-matrix.toml:2913-2924 | REAL (capability) / understated (chrome exists) | this file exists and renders a dead gate |
| "real screens for the former placeholder routes" (incl. /convert) | App.tsx:56-58 | SHALLOW | the screen renders; its function is dead and unlinked |

## 7. Dead / duplicated / unreachable code
F-001: enqueue branch, cancelJob, and the whole queue render are production-unreachable. Route itself unlinked from navigation.

## 8. Dependencies & flags
Button, Dropdown, Card/SectionHeader, useClient, i18n seam keys (w26.b13, w26.i18), generated CommandListing type, convertTargets/convertQueue (sibling files, other agents). All imports exist; typecheck clean in baseline.

## 9. Open questions for the next agent
- convertQueue.ts / convertTargets.ts owners: does the reducer's `progress`/`complete` path have ANY production trigger, or is it purely test-driven state machinery (confirming F-001's blast radius)?
- If the Lead lands convert.* commands, the gate flips on — is there a test asserting the gate opens when the probe lists a lane command? (Colocated suites should own that mutation case.)
