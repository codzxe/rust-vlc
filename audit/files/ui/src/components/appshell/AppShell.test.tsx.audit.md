# AUDIT: ui/src/components/appshell/AppShell.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 792 / 43 tests in 9 describes (+7 helpers: renderShell, seedPlaylistsStore, getShellBanner, getSidebar, installTauriInternalsMock, removeTauriInternalsMock, recordToggleEvents, installMatchMedia) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — tests the real AppShell.tsx, reachable ui/src/main.tsx -> ui/src/App.tsx:3 (`import { AppShell } from "./components/appshell/AppShell"`) -> desktop/web UI |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg test inventory; rg baseline; sed quote verification. Execution cited from committed baseline audit/runs/baseline/ui-test.txt:31 `✓ src/components/appshell/AppShell.test.tsx (43 tests) 1396ms` (vitest run: :178-179 `Test Files 136 passed (136) / Tests 1705 passed | 1 skipped (1706)`) |

## 1. One-paragraph verdict
A genuinely strong UI suite: 43 behavior tests covering the banner anatomy, OS window-control Tauri command seam, icon rail, store-backed playlists sidebar (defaults per route, miniplayer suppression honesty, search/sort/grid, persistence), create-playlist flow through the shared store, the one-sidebar event seam, in-app window actions (real `playback.stop` on the mock transport), strip-portal contract, and narrow-viewport collapse with focus rescue. Assertions are state-based and mutation-sensitive: nearly every test fails if the corresponding behavior breaks (aria-expanded flips, `hidden` attribute, localStorage values, recorded window events, recorded `plugin:window|*` invokes, `transport.sent` commands). The one defect is a mock-leak hazard: the global `__TAURI_INTERNALS__` mock is removed only on each test's success path, not in a `finally`/afterEach guard, contradicting the file's own "must never leak" comment.

## 2. What actually works (verified)
— Committed baseline execution: audit/runs/baseline/ui-test.txt:31 — `✓ src/components/appshell/AppShell.test.tsx (43 tests) 1396ms`; totals :178-179 (136 files, 1705 passed, 1 skipped). All 43 tests pass in the recorded orchestrator run.
— Structural verification (this session): 43 `it(` blocks counted; every describe maps to real component behavior (rg against ui/src/components/appshell/AppShell.tsx usage).

## 3. Findings
F-001 [S4]
Where: ui/src/components/appshell/AppShell.test.tsx:166-169 and 183,208,220 (removal call sites)
Code:
```
/** The internals mock must never leak across tests (probe is global). */
function removeTauriInternalsMock(): void {
  delete (globalThis as unknown as Record<string, unknown>)["__TAURI_INTERNALS__"];
}
```
What is wrong: `removeTauriInternalsMock()` is invoked inline at the tail of each OS-controls test (lines 183, 208, 220) and never in an `afterEach`/`try-finally`. Any assertion failure or thrown error mid-test leaves `__TAURI_INTERNALS__` installed globally for every subsequent test in the file — the exact leak the comment forbids (the contrast: the matchMedia mock DOES restore in `afterEach`, lines 731-736). Why it matters: a failing test corrupts unrelated later tests (desktop-probe visible in "browser mode"-style assertions), producing misleading cascade failures and hiding the real fault. What correct looks like: register removal in the describe's `afterEach` (or wrap in try/finally). Confidence: HIGH — Verification: read (structural; failure-path not executed).

## 4. Per-function table (helpers; the 43 tests are itemized in §5)
| Helper | Lines | Verdict | Notes |
|---|---|---|---|
| renderShell | 25-44 | REAL | seeds localStorage only when absent; honest comment |
| beforeEach/afterEach | 46-57 | REAL | clears localStorage + resets store; restores mocks |
| seedPlaylistsStore | 60-63 | REAL | persisted-shape seeding |
| getShellBanner | 71-77 | REAL | unscoped-banner filter with expect guard |
| getSidebar | 83-89 | REAL | hidden:true + aria-label match |
| installTauriInternalsMock | 143-164 | REAL | records calls; per-command returns; is_maximized behavior |
| removeTauriInternalsMock | 167-169 | REAL (call-site flaw) | F-001 |
| recordToggleEvents | 504-511 | REAL | real window listener |
| installMatchMedia | 704-729 | REAL | per-query listener map; drives `change` path; restored in afterEach |

## 5. Test honesty
| Test (representative sample of 43) | Would it fail if code were broken? | Reason |
|---|---|---|
| title bar anatomy (92) | YES | 12+ concrete DOM/aria/class assertions |
| browser mode never renders OS cluster (117) | YES | absence assertions on real jsdom posture |
| controls drive plugin:window|… (186) | YES | asserts exact command strings + payload `{label:"main"}` + NOT lumen_dispatch |
| maximized flips to Restore (211) | YES | label swap + aria-pressed |
| rail stops + active route (239) | YES | per-name link + active class |
| open-media event listener (262) | YES | dispatches real OPEN_MEDIA_EVENT, asserts dialog |
| sidebar CLOSED on /player (272) | YES | aria-expanded=false |
| /miniplayer honest no-op (318) | YES | three pinned signals incl. localStorage null after 2 clicks |
| stored OPEN preference never surfaces (345) | YES | preference read-only + survives |
| search filter + drop (406) | YES | row presence/absence + reopen |
| natural vs strict sort (431) | YES | exact ordered arrays ["Playlist 2","Playlist 10"] then swapped |
| grid view 2-up (450) | YES | role switch + 3 links |
| create dialog shared + cancel (463) | YES | dialog heading + count rollback |
| Save persists to store + localStorage (484) | YES | parses the actual persisted JSON |
| one-sidebar dispatch (513) | YES | records real PLAYER_TOGGLE_SIDEBAR_EVENT |
| stays silent when dock closed (536) | YES | empty seen arrays |
| route-change collapse session-only (555) | YES | hidden + localStorage null |
| close sends playback.stop (614) | YES | transport.sent cmd asserted |
| main landmark exactly one (630) | YES | getAllByRole length 1 |
| strip portal outside main (654) | YES | closest("main") null + host classes |
| strip empty collapse (675) | YES | childElementCount 0 |
| in-place null default (686) | YES | fallback testid presence |
| narrow auto-collapse (738) | YES | hidden + reopen live |
| mid-session change path (748) | YES | drives MediaQueryList change event |
| focus rescue (764) | YES | activeElement moved to toggle |
| demo badge (127/132) | YES (weak but honest) | text presence/absence |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DOM-shape + behavior assertions; visual fidelity is proven by the dev-shots + VLM loop" | AppShell.test.tsx:7-8 | REAL (for the DOM half) | 43 behavior tests pass in baseline ui-test.txt:31 |
| "the consumer imports the context directly, exactly like PlayerScreen does" | AppShell.test.tsx:20-22 | REAL | ui/src/screens/player/PlayerScreen.tsx imports AppShell context (rg hit on AppShell in PlayerScreen.tsx) |

## 7. Dead / duplicated / unreachable code
None. All imports exercised (OPEN_MEDIA_EVENT, PLAYER_TOGGLE_SIDEBAR_EVENT, resetPlaylistsForTests, useAppShellStrip, MockTransport, createPortal).

## 8. Dependencies & flags
Dev-only: @testing-library/react, vitest, react-router MemoryRouter, react-dom createPortal. MockTransport is the suite-wide seam (not a mock OF the thing tested — the shell's transport dependency).

## 9. Open questions for the next agent
- Does PlayerScreen's real strip consumer have equivalent coverage in its own test file (the shell only tests a synthetic consumer)?
