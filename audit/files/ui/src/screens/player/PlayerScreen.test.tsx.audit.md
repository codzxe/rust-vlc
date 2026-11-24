# AUDIT: ui/src/screens/player/PlayerScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 1263 / 14 describes, 60 `it` blocks, 5 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — via the UI test lane: `ui/package.json:14` `"test": "vitest run"`; executed in the committed baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Read full file; rg counts (60 `it`, 14 `describe`); rg baseline |

## 1. One-paragraph verdict
A genuine behavior test suite for the player screen: 60 tests over 14
describes, executed green in the orchestrator baseline
(audit/runs/baseline/ui-test.txt:14 `✓ src/screens/player/PlayerScreen.test.tsx (60 tests) 3216ms`).
The suite is honest where the product is incomplete — it asserts the
placeholder attributes (`data-placeholder="engine-wiring-4.H1"`), disabled
rows with reasons, "no media open" submenu rows, and empty-list faces rather
than papering over them. Transport is mocked at the seam (MockTransport /
DemoTransport / a local emitting harness), which is the correct boundary for
UI tests; commands issued are verified against `transport.sent`. Defects are
limited: one verbatim ~30-line helper duplicated inside the file, global
prototype patching without restore, and timing-choreography waits.

## 2. What actually works (verified)
— rg `\bit\(` → 60; `describe\(` → 14; matches baseline count exactly.
— Behavior assertions are mutation-sensitive. Examples (verified by read):
toggle seam (L181-192 asserts `data-sidebar` flips open→closed→open and the
dock unmounts), keymap fullscreen (L665-671 asserts
`root.requestFullscreen` called once on "f"), commit-storm probe (L637-661
counts React Profiler commits, expects ZERO over a 40-event pointer storm),
command issuance (L995-997 asserts `transport.sent` contains
`playlist.set-shuffle` with `args {enabled: true}`).
— Honest negatives: L963-977 disabled rows, L1000-1013 "no media open"
submenus, L1244-1251 disabled toggle with reason, L1253-1261 empty playlist
face.
— jsdom gaps handled explicitly (L303-309: `jsdom implements neither play nor
pause — assert the surface's calls through mocks`).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/PlayerScreen.test.tsx:382-412 and 542-572
Code: `function emittingTransport(): CommandTransport & { emit: (event: Event) => void } {` (identical body twice, ~30 lines each)
What is wrong: the helper (idle mock + emit fan-out) is copy-pasted verbatim
into two describes instead of hoisted to module scope.
Why it matters: divergence risk — a fix to one copy silently misses the
other. What correct looks like: one module-scope helper.
Confidence: HIGH — Verification: read + sed -n '382,388p' / '542,548p'.
F-002 [S4]
Where: :41-51 (beforeEach)
Code: `Object.defineProperty(HTMLElement.prototype, "requestFullscreen", { value: vi.fn().mockResolvedValue(undefined), configurable: true, });`
What is wrong: patches `HTMLElement.prototype` globally and never restores it
in afterEach (only `fullscreenElement` is reset). Vitest file isolation
contains the leak, but any future non-isolated setup inherits a fake
requestFullscreen. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: :277-339, :341-376, :1178-1242 — `{ timeout: 20000 }` tests driven by
DemoTransport's ~320ms real-timer choreography with nested
`waitFor(..., { timeout: 5000 })`.
What is wrong: correctness of three tests depends on wall-clock timing of the
demo transport; generous bounds prevent flake but make the tests slow (file
total 3216 ms) and timing-sensitive on 2-core CI. Confidence: MEDIUM.
F-004 [S4]
Where: file-wide style — heavy `data-*` attribute assertions
(`data-sidebar`, `data-chrome`, `data-media-bytes`, `data-region-id`) couple
the suite to internal DOM markers rather than user-visible behavior. Only a
style note: the suite also does plenty of role/label assertions, so this is
hygiene, not fakeness. Confidence: HIGH.

## 4. Per-function table (every describe + test)
| Describe (lines) | Tests | Verdict | Severity | Notes |
|---|---|---|---|---|
| ARIA landmarks + honest placeholders (58-171) | 6: canvas-region, dock=Playlists browser, control band span, control bar slot, no dup title bar, open panels | REAL | — | asserts no fake video/canvas at idle (L67-68) |
| sidebar toggle seam (173-201) | 2: toggle event open/closed, openPanels forces visible | REAL | — | event-driven, mount asserted |
| dock affordance row (203-274) | 4: chevrons+disabled forward, sort aria-pressed, search live filter (3→2→0 rows), list/grid flip | REAL | — | real store rows counted |
| real media surface (276-377) | 2: file source mounts srcless + url source real src; surface follows engine states, play/pause only after engine | REAL | S4 timing | honest `data-media-bytes: none` |
| now-playing identity (379-440) | 2: engine-reported title centered, idle renders nothing | REAL | — | engine-truth-only via emit |
| first-run empty state (442-502) | 4: open-media CTA fires shell event, hides once media, has-media gating, chrome visible+armed | REAL | — | click path asserted (L462-463) |
| layout-config consumption (504-606) | 3: custom TOML layout regions, invalid layout throws, has-video via engine tracks both directions | REAL | — | negative-throw test at L529-533 |
| chrome lock + auto-hide (608-662) | 3: panel locks chrome, fullscreen idle-hide+pointermove reveal, pointer storm 0 commits | REAL | — | Profiler commit-count probe is strong |
| keymap fullscreen + ESC (664-727) | 5: F requests, Escape exits, Escape forwarded to seam, unbound/modifier unclaimed, I reveals chrome | REAL | — | exact-modifier matching |
| Go-to-Time dialog (729-773) | 4: Ctrl+T opens prefilled, idle ignores, Ctrl+Shift+T not combo, Escape closes | REAL | — | |
| Ctrl+E tools dialog (775-804) | 2: opens on Audio tab, Escape closes | REAL | — | |
| Customize interface trigger (806-900) | 6: Ctrl+A seam event, Ctrl+Shift+A no, Ctrl+A in input stays select-all, seam mounts modal, ✕ closes, end-to-end open | REAL | — | dead-seam regression cover |
| canvas context menu (902-1133) | 10: 15 labels pinned order, accelerators from keymap data, honest disabled rows, Playback submenu issues real cmd, V/A/S submenus honest "no media open", Playlist submenu toggles dock, Customize row fires seam, Open Media shell event + route, Escape/outside dismiss, ContextMenu+Shift+F10 keyboard reach + focus restore | REAL | — | strongest describe |
| browse face + now-playing card (1135-1262) | 6: dock row→grid→exit, band persists under browse, session-over-browse floating card lifecycle, no-session toggle disabled, empty playlist face | REAL | S4 timing | card collapse/reopen through real toggle |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| toggle seam (L174-192) | YES | data-sidebar + dock mount/unmount both asserted |
| commit-storm probe (L637-661) | YES | commit count must stay at baseline over 40 events |
| Playback submenu (L979-998) | YES | asserts the exact command on transport.sent |
| media surface (L277-339) | YES for mount/src/attrs | playback of real bytes UNVERIFIABLE in jsdom (inherent) |
| all 60 | YES (mutation reasoning) | every test asserts observable DOM/events; no tautologies found |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DOM-shape + behavior assertions per the card ACs" | file header L1-5 | REAL | 60 tests, all behavioral/structural asserts |
| "visual fidelity: dev-shots + VLM loop" | header L5 | NOT-IN-THIS-FILE | covered by ui/e2e/visual baselines, not asserted here |

## 7. Dead / duplicated / unreachable code
Duplicated: emittingTransport (F-001). No dead tests; no `.skip`/`.only`/
`.todo` (rg: only comment-text matches at L107/L133).

## 8. Dependencies & flags
Imports only first-party seams (ClientProvider, MockTransport, DemoTransport,
openPathRequest, fixtures, PlayerScreen) + RTL/vitest — no network, no test
env keys.

## 9. Open questions for the next agent
The `data-placeholder="engine-wiring-4.H1"` marker (L63) is the UI's own
admission that engine wiring for canvas video is unbuilt — whoever audits
the engine-wiring card should link this evidence.
