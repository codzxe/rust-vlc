# AUDIT: ui/src/screens/playlist/options/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 135 / 6 (HARNESS_ENTRIES, HARNESS_META, harnessTransport, OptionsMenusHarness, OptionsViewsHarness, OptionsPopupCloseups) |
| File verdict | REAL (dev-only fidelity harness; render smoke executed in baseline #23's 136-file vitest run and in w26.e09's scoped run "17 files / 174 tests PASS"; export+screenshot path executed per w26.e09 method record) |
| Reachable from shipped entry point? | NO — deliberately (header :1 "never imported by app routes"), verified: `rg -ln "options/harness" ui/src --glob '!**/harness*'` → ZERO importers outside harness.test.tsx. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -ln "options/harness" ui/src` (no app-route importers); `ls ui/src/screens/playlist/options/` (harness.css + harness.test.tsx exist); read of harness.test.tsx (85 lines) — env-gated export, smoke assertions; `rg -n "resolveMeta" ui/src` → wired ONLY by optionsTestSupport.tsx + harnesses, never by a production screen; `rg -n "SORT_KEYS" sorting.ts` → 16 keys (16-row claim true); `sed -n '1,60p' SortingOptionsMenu.tsx` → trigger+menu, click-gated, meta-seam honesty block; `sed -n '1,40p' docs/waves/w26/findings/w26.e09.md` → loop record (screenshots made; VLM compare 429-failed, verdicts DOM-measurement-backed [VERIFY]); baseline #23 log read (sample-truncated: 5 ✓ lines + summary). |

## 1. One-paragraph verdict
An honest, well-disclosed dev-only vision harness: it composes the four Figma reference states using a stub transport and a fixture meta seam, both loudly labeled, with eslint-disable comments that give reasons, and it is verifiably absent from every app route. The record trail (w26.e09) shows its HTML export and screenshot path actually ran. Three defects, all S4: the closeups comment claims the menus are "rendered permanently open" when the components are click-gated and the test must fire clicks to open them; the transport stub launders a fake state-snapshot past the type system with `as never`; and the export test asserts nothing whenever the env var is unset, so CI's green "1705 passed" includes one vacuous test. The deeper caveat is inherited, not this file's fault: the meta seam it wires (watched/partial-progress bars) has no production wiring, so one of its four reference states is unproducible in the shipped app.

## 2. What actually works (verified)
— verified by `rg -ln "options/harness" ui/src` → only harness.test.tsx imports it: the "never imported by app routes" claim (:1) is TRUE.
— verified by reading harness.test.tsx:53-82: with W26B10_HARNESS_OUT set it inlines 8 real css files (tokens, lumen, button/icon/list/menu, options, harness), renders each state, clicks the menu triggers, and writes sorting-open/view-open/views.html; the loop record in docs/waves/w26/findings/w26.e09.md:26-28 confirms "vitest exports → standalone HTML → screenshots" were produced.
— verified by `rg` on sorting.ts: SORT_KEYS contains exactly 16 keys (unwatched-first … disc-number-desc) — the "sorting (16 rows)" claim (:111) is TRUE; ViewOptionsMenu's 2 rows (Grid/List) confirmed by w26.e09 row table.
— verified by reading PlaylistOptionsProvider.tsx:7 ("HONESTY: `resolveMeta` defaults to 'no metadata reported'") and SortingOptionsMenu.tsx:6-10 (needs-meta hint in unwired state): the fixture seam this harness exploits is the same seam production leaves unwired — the harness is not hiding anything the components don't already disclose.

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/playlist/options/harness.tsx:111-113
Code:
```
/** Menu-open closeups: sorting (16 rows) and view (2 rows) popups rendered
 * permanently open for pixel comparison (the triggers are click-gated, so
 * the harness drives them via the public popup API in the test). */
```
What is wrong: Self-contradiction. The menus are NOT "rendered permanently open" — SortingOptionsMenu/ViewOptionsMenu wrap PlaylistOptionsMenu (trigger + click-gated popup), and harness.test.tsx:66 must `fireEvent.click(screen.getByRole("button", { name: "Sorting options" }))` to open them. "The public popup API" is a trigger click, not an API.
Why it matters: The harness exists to make pixel claims precise; its own spec comment misstates the mechanism a maintainer would need to extend it (add a state → wonder why your "permanently open" menu renders closed).
What correct looks like: "popups opened by clicking the triggers in the test (they are click-gated)".
Confidence: HIGH — Verification: SortingOptionsMenu.tsx:17-39 read (triggerLabel/triggerIcon props); harness.test.tsx:38-39,66 read.

### F-002 [S4]
Where: ui/src/screens/playlist/options/harness.tsx:45-49
Code:
```
  send: async () => ({
    gen: 1,
    requestId: 1,
    result: { completed: { kind: "state-snapshot", snapshot: { playlist: {} } as never } },
  }),
```
What is wrong: `as never` launders a hand-rolled fake snapshot past the wire type. If the ClientProvider's expected snapshot shape grows fields the stub omits, the cast keeps compiling silently and the harness drifts from the real protocol shape it is supposed to mirror.
Why it matters: A fidelity harness whose transport fixture is exempt from type-checking can silently validate against a protocol that no longer exists.
What correct looks like: type the stub as the real snapshot type with a fixture object (or `satisfies`), so field drift fails to compile.
Confidence: HIGH — Verification: read; cast semantics traced.
(Note: the stub itself is disclosed at :42 "Transport stub — the harness never talks to an engine" — the disclosure is honest, the typing is the defect.)

### F-003 [S4]
Where: ui/src/screens/playlist/options/harness.tsx:6-9 (+ harness.test.tsx:53-55)
Code:
```
// Rendered by harness.test.tsx into static HTML (jsdom → document), then
// screenshotted headlessly — see the lane log for the loop record.
//
//   W26B10_HARNESS_OUT=/tmp/w26b10-harness npm run test -- harness.test
```
What is wrong: The export test's body is `if (out === undefined || out === "") return;` (harness.test.tsx:55) — in every normal run (CI, baseline #23) it performs ZERO assertions yet reports as a passed test; the always-on coverage for this harness is one smoke test asserting `innerHTML.length > 0` (harness.test.tsx:46,50).
Why it matters: The green "1705 passed | 1 skipped" line includes a vacuous pass; the harness's real guarantees (menus open, 16 rows, css inlined) are only checked in manual env-gated runs, so a regression that breaks menu opening ships green.
What correct looks like: `it.skipIf(!out)` (visible skip, counted in the skip line) or assert the invariable parts (menus render their 16/2 rows) unconditionally.
Confidence: HIGH — Verification: both files read; baseline #23 log read (summary only — the file-level lines are truncated to a 5-file sample, so per-file execution is inferred from vitest's include-all semantics plus w26.e09's scoped "17 files / 174 tests PASS").

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HARNESS_ENTRIES | 26-31 | REAL | — | 4 fixture rows matching media.svg frames; includes durationSeconds null (E10) and 42s ("00:00:42" chip confirmed in w26.e09) — deliberate edge coverage |
| HARNESS_META | 35-40 | REAL | — | fixture meta seam: full bar, 0.45 partial, 2 unwatched; production wires NO resolveMeta (rg: only testSupport + harnesses) — partial-progress bar unproducible in shipped app (disclosed in provider) |
| harnessTransport() | 43-50 | REAL | S4 | F-002 `as never` type-laundering; subscribe returns noop-unsubscribe; disclosed stub |
| OptionsMenusHarness | 53-73 | REAL | — | renders toolbar inside anchor cell; i18n eslint-disable with justification (figma frame names) |
| OptionsViewsHarness | 76-109 | REAL | — | ThumbnailGrid + MediaList + toolbar; three cells |
| OptionsPopupCloseups | 114-135 | REAL | S4 | F-001 "permanently open" comment false; renders the two menu components directly |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| harness.test.tsx "renders all frame states without crashing" | only for crash-level breakage | asserts innerHTML.length > 0 — a menu rendering zero rows passes; name is honest ("without crashing") but that is all it proves |
| harness.test.tsx "exports standalone HTML snapshots when W26B10_HARNESS_OUT is set" | NO in normal runs | early-returns before any assertion when env unset (F-003); when set, it asserts only that 3 files exist — not their content |
| w26.e09 loop (screenshots + compare) | n/a (manual record) | screenshots produced; VLM comparison failed HTTP 429 ×8 — every visual verdict is DOM-measurement-backed and marked [VERIFY] (w26.e09:31-34) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "never imported by app routes" | harness.tsx:1 | REAL | rg: zero importers outside tests |
| "Rendered by harness.test.tsx into static HTML … then screenshotted headlessly — see the lane log" | :6-7 | REAL | harness.test.tsx:53-82 + w26.e09 method §3 |
| "sorting (16 rows) and view (2 rows)" | :111 | REAL | SORT_KEYS=16 in sorting.ts:20-36; w26.e09 table |
| "popups rendered permanently open" | :112 | FALSE | click-gated PlaylistOptionsMenu; test clicks triggers (F-001) |
| "the harness never talks to an engine" | :42 | REAL | stub send returns canned snapshot; subscribe noop |
| w26.e09 "media.png … MATCH [VERIFY]" | w26.e09:54 | REAL-UNPROVEN for the partial-bar state | that state is produced only via the harness meta seam; no production wiring of resolveMeta exists (rg) |

## 7. Dead / duplicated / unreachable code
None dead. By design unreachable from app routes (verified). HARNESS_META's 0.45 partial-progress state is producible only here and in tests — see per-function note.

## 8. Dependencies & flags
Imports only sibling playlist modules + client types; ./harness.css exists (ls). Runtime env flag W26B10_HARNESS_OUT is the sole trigger for the export path. No engine/network dependency (stubbed by design).

## 9. Open questions for the next agent
- Will any production surface ever wire resolveMeta (engine playlist metadata)? Until then, watched-progress bars remain a harness/test-only visual (PlaylistOptionsProvider's local Mark-as-Watched covers full bars only).
- The 1 skipped test in baseline #23 — identify it (not this file's export test; early-return counts as passed).
