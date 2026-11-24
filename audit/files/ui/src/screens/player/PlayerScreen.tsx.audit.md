# AUDIT: ui/src/screens/player/PlayerScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 1553 / 12 components+fns, 4 exported consts |
| File verdict | REAL-UNPROVEN (structure/routes/honest-disabled real; several controls dead in the only real host) |
| Reachable from shipped entry point? | YES: main.tsx:56-66 App -> App.tsx:303-305 `<Route path="/player" element={<PlayerRoute variant="full" />}>` -> App.tsx:203 `<PlayerScreen variant=...>` |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | sed/rg quote verification (exit 0); baseline ui-test.txt cited for vitest; no builds run (load discipline) |

## 1. One-paragraph verdict
The screen itself is real, routed, and unusually honest: disabled rows are rendered `disabled` (Record, Hide UI, Quit, dock forward, dock ···) instead of dead-click theater, and the engine-state overlays derive from transport events, not fabrication. But the master question gets a split answer: the transport controls (play/pause/stop/seek via `./context` actions -> `playback.*`) ARE served by the shipped CLI/desktop host (apps/cli/src/dispatch.rs:88-120), while the volume/mute, track-selection, and playlist (shuffle/repeat/prev/next) controls dispatch commands that EVERY shipped host refuses (`audio.*`, `tracks.set-*`, `playlist.*` -> dispatch.rs:155 `other => Err(unserved(other))`, :193-194 "audio device control joins with the real backends (ADR-005)"). In the real desktop shell these controls are dead — the best of them surface a typed error chip, the worst (playlist submenu) fail totally silently. In the browser dev shell everything "works" against the fake DemoTransport, which is exactly the mock-data-as-real trap.

## 2. What actually works (verified)
— Route reachability verified by read: main.tsx -> App.tsx:303 -> PlayerRoute -> PlayerScreen.
— Baseline vitest (audit/runs/baseline/ui-test.txt, "✓ src/screens/player/PlayerScreen.test.tsx" included in 1705 passed): the `url sources carry REAL bytes, file sources mount without a src` test (PlayerScreen.test.tsx:277) pins the media-surface seam incl. the `.at(-1)` compensation.
— Honest disabled rows verified: L346-352 `id: "record" ... disabled: true`, L483-489 `id: "quit" ... disabled: true`.
— `playback.*` commands served by the real host verified by read of apps/cli/src/dispatch.rs:88-120 (Play/Pause/Toggle/Stop/Seek arms).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/PlayerScreen.tsx:1192-1196
Code:
```
            {/* Card 4.C2 — the real control bar: transport (doc 08 §5.1
                mapping), seekbar (§5.2 truthful pending), volume/mute, track
                menus — all wired to the command-layer client seam. */}
            <ControlBar />
```
What is wrong: "all wired to the command-layer client seam" overstates: `volume/mute` and `track menus` are wired to a seam whose far end refuses them. `actions.setVolume` dispatches `{ args: { volume: value }, cmd: "audio.set-volume" }` (context.tsx:389-391); the CLI host answers every `audio.*` id with the typed refusal (dispatch.rs:193-194 `} else if id.starts_with("audio.") { "audio device control joins with the real backends (ADR-005)"`); desktop adopts the same host; web host and control-server refuse too (W07-A02 log: "UI VolumeSlider -> audio.set-volume is REFUSED by every real host"; only ui/src/client/demo-transport.ts:482 implements it). Why it matters: the volume slider — a hostile-focus control — cannot change volume in any shipped build; the user gets an error chip at best. What correct looks like: a host that serves the command, or a disabled-with-reason slider. Confidence: HIGH — Verification: read (cross-file rg + Wave 7 audits).

F-002 [S2]
Where: ui/src/screens/player/PlayerScreen.tsx:373-387 (also 254-268 track rows, 331/340 prev/next)
Code:
```
          id: "playback-shuffle",
          labelKey: m("menu.playback.shuffle"),
          kind: "checkbox",
          checked: playlist.shuffle === true,
          onSelect: () => playlist.setShuffle(!(playlist.shuffle === true)),
```
What is wrong: in the real desktop shell these clicks dispatch `playlist.set-shuffle` / `playlist.set-repeat` / `tracks.set-*-track`, all refused (`playlist.*` -> dispatch.rs:191-192 "the playlist model is a Phase-4 card"; no `Command::TracksSet*` arm exists in route()). PlaylistProvider swallows the refusal silently — PlaylistProvider.tsx:203 checks ONLY `error.code === "feature-not-exposed"`, so the real host's `unsupported-for-host` leaves `notExposed` false and renders no error: the shuffle checkbox click does nothing, with zero feedback. Track submenu clicks at least surface the typed error via context.tsx:315-317. Why it matters: silent dead clicks in the shipped composition — mock transport demos them "working". What correct looks like: treat `unsupported-for-host` as not-exposed or surface the error. Confidence: HIGH — Verification: read (dispatch.rs + PlaylistProvider.tsx:197-215).

F-003 [S3]
Where: ui/src/screens/player/PlayerScreen.tsx:710-721
Code:
```
  const demoSource = isDemoTransport(client)
    ? (client.recentOpenings.at(-1)?.source ?? null)
    : null;
  const surfaceSource: SourceSpec | null = queueSource ?? demoSource;
```
What is wrong: this seam is coupled to a KNOWN-BROKEN sibling contract: demo-transport's `recentOpenings` getter documents "most recent first" but ships oldest-first (`rememberOpening` prepends at demo-transport.ts:980-986; getter reverses at :353-356). The screen compensates with `.at(-1)` and a test pins it, but any fix to the transport silently flips the seam — the inline comment admits the trap. The underlying contract bug is demo-transport's (other agent). Why it matters: negative-index coupling to a lying getter is drift waiting to happen. Confidence: HIGH — Verification: read both files.

F-004 [S3]
Where: ui/src/screens/player/PlayerScreen.tsx:690-695
Code:
```
            // The seam carries no chapter truth — conservative false; hosts
            // that need it pass the full PlayerMediaContext.
            hasChapters: false,
```
What is wrong: chapters are hard-coded false in the default composition; `media.get-chapters` exists in the catalog but nothing calls it, so chapter-gated layout regions and the chapter UX never activate for real users. Documented as conservative, so honest, but it is a permanent stub. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:1-75
What is wrong: 75 lines of design-log header comments (wave/card citations) before code; the file documents its own honesty more than any code needs. Not a behavior defect. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fromEditableField | 210-214 | REAL | - | guards Ctrl+A vs select-all; correct |
| acceleratorOf | 220-228 | REAL | - | display-only reformat of keymap DATA |
| canvasTrackSubmenu | 234-271 | REAL | S2 | dispatches refused tracks.set-* in real host (F-002) |
| canvasMenuEntries | 280-491 | REAL | S2 | shuffle/repeat/prev/next dead in real host; Record/Hide-UI/Quit honestly disabled |
| NavigationSeam / RoutedNavigation | 499-510 | REAL | - | router-presence fallback; jsdom-safe |
| PlayerScreen | 555-568 | REAL | - | providers mounted once |
| PlayerScreenInner | 570-1218 | REAL | S2/S3 | keymap listener real; volume/playlist wiring dead (F-001/2); demoSource `.at(-1)` (F-003) |
| PlayerDialogs | 1228-1250 | REAL | - | per-open mounts, no state leaks |
| PlaylistsDock | 1280-1489 | REAL | S3 | localStorage store + SEED fixture rows (see §7); forward/··· honestly disabled |
| NowPlayingMark | 1493-1507 | REAL | - | engine title first, playlist title fallback — no fabrication |
| PanelRegion | 1514-1553 | REAL | - | honest placeholder text for unknown regions |
| consts PLAYER_TOGGLE_SIDEBAR_EVENT / PLAYER_OPEN_CUSTOMIZE_EVENT / canvasCatalog / CUSTOMIZE_DIALOG_KEYMAP | 149/158/176-206 | REAL | - | CustomEvent seams verified consumed (customize listener L623-627) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| PlayerScreen.test.tsx:277 media surface (file no-src / url real bytes) | YES | asserts `data-media-bytes` + real src attr against DemoTransport with real timers; would fail if `.at(-1)` compensation regressed (a demo-transport fix flipping order breaks it, as designed) |
| dock list/grid sort/search tests (test file ~L240-268) | YES | click-driven role/aria assertions |
| baseline: vitest 1705 passed / 1 skipped (ui-test.txt) | executed by orchestrator | cited, not re-run (load discipline) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "volume/mute, track menus — all wired to the command-layer client seam" | L1192-1194 | MISLEADING | wired to the seam; every real host refuses the commands (F-001) |
| "Real data only; renders only while an overlay-free session state is up" (NowPlayingMark) | L1104-1112 | TRUE | read of L1493-1507 |
| "rows read the REAL saved-playlists store" | L17-19 | TRUE-with-caveat | playlistsStore.ts localStorage; but first-run SEED = fabricated frame rows (§7) |

## 7. Dead / duplicated / unreachable code
- Honestly-disabled rows (not dead clicks): Record L346-352, Hide UI L357-363, Quit L483-489, dock forward L1350-1358, dock ··· L1360-1368.
- Dead-in-real-host controls: volume/mute (ControlBar), shuffle/repeat radios, prev/next (disabled via empty queue, so honest), track-selection rows — F-001/F-002.
- Mock data as real: PlaylistsDock first-run rows come from playlistsStore SEED fixtures ("Cool Playlist", "Cool Songs", "Anime to Watch") — fabricated playlists presented to the user exactly like user-created ones (no badge); opening them browses `itemNames` ("The Wand" episodes) that have no sources and can never play. Documented in playlistsStore.ts comments, invisible to the user.

## 8. Dependencies & flags
No feature gates; plain React/router/DOM. Imports client (transport seam), keymap DATA, i18n. No env vars read.

## 9. Open questions for the next agent
- Who owns the PlaylistProvider silent-swallow of `unsupported-for-host` (F-002) — UI lane or command lane?
- Does any shipped host plan to serve `tracks.set-*`? ControlBar track menus share the deadness.
