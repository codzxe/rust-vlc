# AUDIT: ui/src/screens/player/PlayerScreen.tsx#part3
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 1036-1553 of 1553 (chunk part3; boundary: starts mid-canvas `onKeyDown` handler opened at :1025 in part2) / 9 items |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | YES as UI code — App.tsx:203 `PlayerRoute` mounts `<PlayerScreen>` on /player (+ /player/music, /miniplayer) inside GatedShell/AppShell; UI builds green (baseline #22 vite build OK) and tests pass (baseline #23, 1705 passed). BUT the only shipped GUI host cannot compile at HEAD (W14-A01 F-001: apps/desktop-tauri/src/app.rs:301 calls nonexistent `spawn_session_pump`) — no GUI binary exists from this tree |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n` verification of every quoted range · `rg -n "SEED|localStorage" ui/src/screens/playlist/playlistsStore.ts` (localStorage `lumen.playlists.v1` + fixture SEED :57-76) · Read ui/src/screens/player/states/PlaylistBrowse.tsx (239 lines — inert cards, honest) · `rg -n "duplicate region id" ui/src/screens/player/layout.ts` (:328-331 — id-unique only) · `rg -n "clamp|innerWidth" ui/src/components/menu/MenuPopup.tsx` (:102-109 viewport clamp exists) · `rg -n "togglePlayPause|dispatch|wire.send" ui/src/screens/player/context.tsx` (actions dispatch real commands via wire :300-347, :361-405) · `rg -n "describe|it\(" ui/src/screens/player/PlayerScreen.test.tsx` (dock/media-surface/now-playing/panel tests :58-441) · `rg -n "player.dock.count-suffix" ui/src/i18n/messages.ts` (:2806 "items") · baseline: ui-vitest.log.txt, ui-build.log.txt, INDEX.md #22/#23, W14-A01 app.rs.audit.md F-001 |

## 1. One-paragraph verdict
Part3 is the bottom half of the player screen: the canvas content (media surface mount, browse face, state overlays, context-menu mount), the full-width control band with its shell-strip portal, the per-open dialog trio, the docked playlists browser, the now-playing identity mark, and the layout side panels. The code is disciplined about honesty — overlays are engine-truth-only, the browse cards are inert divs rather than dead buttons, panels without content render a labelled sr-note, and the canvas carries `data-placeholder="engine-wiring-4.H1"` openly. The real wiring exists and is test-pinned: dock rows read the localStorage playlists store, the menu dispatches real transport commands through context.tsx's wire, and PlayerScreen.test.tsx exercises all of it (baseline vitest green). The defects: the first-run dock renders SEEDED fixture playlists with invented media names as if they were user data (the store's SEED is the primary defect; this chunk is the presentation), a "1 items" plural bug, no at-most-one guard on bottom layout regions (two ControlBars possible via custom layout), and two divergent overlay-suppression idioms in adjacent branches of the same JSX.

## 2. What actually works (verified)
— Dock reads the real store: PlaylistsDock `usePlaylists()` (:1292) over playlistsStore.ts (localStorage `lumen.playlists.v1`, :44/:78-97) — verified by read; test-pinned at PlayerScreen.test.tsx:73-105 ("composes the docked sidebar as the playlists BROWSER over the real store"), green in baseline ui-vitest.log.txt.
— Dock affordances are honest: forward chevron and ··· render `disabled` with title hints (:1350-1368), never dead clicks — test :203-219 asserts back collapses the dock, forward disabled.
— Browse cards are inert, not fake buttons: PlaylistBrowse.tsx:134-141 renders `<div className="lumen-state-browse__card">` with the comment "names-only local data has no play seam" — no onClick theater.
— Canvas menu actions dispatch real commands: `onSelect: actions.togglePlayPause` etc. resolve to context.tsx:361-405 actions that `dispatch({cmd: "playback.toggle-play-pause", ...})` through `wire.send` (:303) — command-layer wired, error surfaces typed (context.tsx:300-347).
— Menu viewport clamping exists (MenuPopup.tsx:102-109 `getBoundingClientRect` + flip/shift, w26.f07) — the unclamped `style={{left, top}}` at :1152 is safe.
— The strip portal (:1198) is test-covered for the in-place case (test :105-137 asserts footer is a full-width sibling AFTER the body row).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/PlayerScreen.tsx:1449-1460 (rows render) + :967-973 (row → browse); source of the data: ui/src/screens/playlist/playlistsStore.ts:57-76
Code:
```
              {rows.map((row) => (
                <li key={row.id} className="lumen-player__dock-item">
                  <button
                    type="button"
                    className="lumen-player__dock-row"
                    title={row.label}
                    onClick={() => onOpenPlaylist(row.id)}
```
```
const SEED: readonly SavedPlaylist[] = [
  { id: "cool-playlist", title: "Cool Playlist", createdAt: 0, itemNames: [] },
  {
    id: "cool-songs",
    title: "Cool Songs",
    createdAt: 0,
    itemNames: ["The Wand (Main Theme).flac", "Voltage.mp3", "Paper Planes.mp3"],
  },
```
What is wrong: on first run the dock presents three fabricated playlists ("Cool Playlist", "Cool Songs", "Anime to Watch") as ordinary clickable rows, and opening one fills the canvas grid with invented media filenames ("The Wand_S01E01.mp4" …) — production-baked fixture data (checklist: fixtures baked into production code) with no visual marker that it is seed content. The seed lives in playlistsStore.ts (another agent's file — primary defect there); this chunk is the surface that presents it as product data.
Why it matters: a first-run user believes they have playlists and media; the grid items can never play (names-only, no sources — PlaylistBrowse renders them inert). It is at least consistently honest at the click layer (no dead buttons), but the shipped first impression is fabricated content.
What correct looks like: an explicit empty-state ("No playlists yet — create one") on first run, or seed rows visually badged as samples (and excluded from the create/browse surface).
Confidence: HIGH — Verification: read of both files + PlayerScreen.test.tsx:96-104 (the test itself pins the seed rows "Cool Songs"/"Anime to Watch" as expected UI — the fixture is asserted as product behavior).

### F-002 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:1391-1394 (same pattern: states/PlaylistBrowse.tsx:109-111)
Code:
```
        <span className="lumen-player__dock-count">
          {String(rows.length)}{" "}
          {tm("player.dock.count-suffix")}
        </span>
```
What is wrong: the count is `N` + literal suffix msgid `player.dock.count-suffix` = "items" (ui/src/i18n/messages.ts:2806). One saved playlist renders "1 items". The l10n system explicitly supports message-format v1 plural selectors (ui/scripts/l10n-check.mjs rule 5 documents `{name, plural, …}`) and the capability is unused here.
Why it matters: visible grammar defect in a shipped surface, and the pattern is duplicated in the browse header.
What correct looks like: a `player.dock.count` msgid with `{count, plural, one {# item} other {# items}}` rendered through the message formatter.
Confidence: HIGH — Verification: read + rg of the catalog value.

### F-003 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:1177-1199
Code:
```
      {regionsOf("bottom").map((region) => {
        /* w26.g03 P1-2 (Lead): the band spans the FULL window under rail +
         * sidebar (Player.svg strip 0..1920) — portal into the shell strip
```
```
        return stripHost !== null ? createPortal(node, stripHost) : node;
```
What is wrong: one footer + full `<ControlBar />` renders per bottom-anchor region. The layout parser only rejects duplicate region ids (layout.ts:328-331); a custom layoutSource with two distinct-id bottom regions stacks two complete control bands, and inside the shell BOTH portal into the single strip slot (:1198) — duplicated seekbars/volume/transport in the shipped strip.
Why it matters: layout config is a documented host seam ("config owns existence/order"); nothing guards the one-control-band invariant the design assumes.
What correct looks like: parser or render guard — e.g. LayoutParseError on a second bottom-anchor region, or render only `regionsOf("bottom")[0]`.
Confidence: HIGH — Verification: read + layout.ts:328-331 (only id-uniqueness enforced); latent for the default layout (exactly one bottom region, layout.ts:358-362).
### F-004 [S4]
Where: ui/src/screens/player/PlayerScreen.tsx:1078-1079 vs :1125
Code:
```
                      {resolveOverlay(effectiveEngineState) === "empty" ||
                      resolveOverlay(effectiveEngineState) === "none" ? null : (
```
```
                      {effectiveEngineState.state === "idle" && hasMedia ? null : (
```
What is wrong: two different idioms suppress the StateRouter in the two adjacent branches of the same canvas — the browse branch compares overlay tokens (and calls `resolveOverlay` twice per render), the normal branch compares raw state + hasMedia. They encode the same intent ("no empty hero over real content") with divergent logic that will drift on the next overlay-token addition.
Why it matters: maintenance hazard in the single most state-dense region of the screen; the double call is trivial waste but the asymmetry is the real cost.
What correct looks like: one helper (e.g. `suppressOverlay(view, hasMedia)`) used by both branches, computed once.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| canvas onKeyDown anchor tail (part of handler :1025-1046; my range :1036-1046) | 1036-1046 | REAL | — | ContextMenu/Shift+F10 open coordinates: focused-descendant → bottom-left, region → top-left+8; correct math, focus saved for restore |
| canvas region JSX (sr-note, browse/normal branches, menu mount) | 1047-1158 | REAL | S4 | engine-wiring honesty marker :1011/:1051; F-004 suppression asymmetry; MediaSurface mounted only when hasMedia (:1061, :1097) |
| bottom band render + strip portal | 1177-1199 | REAL | S4 | F-003 multi-bottom-region; portal verified for in-place case by test :105-137 |
| ToolsDialogHost + PlayerDialogs mounts | 1201-1215 | REAL | — | per-open mounts; ToolsDialogHost full-variant only (documented, miniplayer has no tools entry) |
| PlayerDialogs (component) | 1228-1250 | REAL | — | trivial conditional trio; single definition shared by both variant branches (J06-11 dedup — verified both call sites :929, :1208) |
| PlaylistsDock | 1280-1489 | REAL | S3/S4 | F-001 seed presentation; F-002 plural; search filter drop-on-close (:1306-1311); rows useMemo filter+sort correct (:1317-1325); honest disabled affordances |
| NowPlayingMark | 1493-1507 | REAL | — | engine title first, playlist title fallback, honest null → glyph only |
| PanelRegion | 1514-1553 | REAL | — | playlist→PlaylistPanel, media-info→MediaInfoProvider/Panel, else labelled sr-note placeholder (honest); music emphasis = styling hook only |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| PlayerScreen.test.tsx:59 "not a fake <video>/<canvas> element" | YES | asserts idle renders no video/canvas AND the sr-note text; a fabricated surface would fail it |
| :73 dock composition over the real store | YES | pins dock anatomy + real store rows (note: it PINS the seed rows as correct behavior — see F-001) |
| :203-275 dock affordance row | YES | back collapses (state flip asserted), forward disabled, sort aria-pressed flips order, search filters + drops, view flips list↔grid |
| :277 "url sources carry REAL bytes, file sources mount without a src" | YES | asserts `video.src` is the real URL vs absent — catches a fabricated src |
| :379-441 now-playing identity | YES | engine-reported title renders only after playing+info-ready; idle renders nothing |
| :505-540 layout config + loud throw | YES | custom source renders config regions; invalid source throws (no silent fallback) |
All green in baseline ui-vitest.log.txt (1705 passed / 1 skipped, 136 files).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the real MEDIA SURFACE is a <video> element mounted while a session is up … url sources render real bytes … file sources mount with no src" | file header :25-29 | REAL | test :277-378 (src assertions), green in baseline #23 |
| "Record … Hide UI … Quit VLC render disabled — never theater" | header :60-63 | REAL | entries `disabled: true` :343-363/:480-489 (part1 range — mount verified here at :1138-1157) |
| "HONEST PLACEHOLDERS … the DECODE/FRAMES half of the media surface is still engine-wired nowhere" | header :36-42 | REAL | `data-placeholder="engine-wiring-4.H1"` :1011; sr-note :1051; the 4.H1 handoff is the standing gap (canvas frames exist only via MediaSurface's LUF1 channel — W14 audit) |
| "Rows open the playlist's media grid in the player canvas" | :1276-1278 | REAL | onOpenPlaylist → setBrowseId → PlaylistBrowse (verified :967-973, :1053-1073) |

## 7. Dead / duplicated / unreachable code
- No dead code found in the chunk. `folderOf` duplication noted between PlaylistBrowse.tsx:56-60 and variants/MiniPlayer (acknowledged in-comment "MiniPlayer's helper, restated locally" — cross-file, belongs to states/variants owners).
- The SEED fixture data path (playlistsStore) is live production code rendering fake first-run content — see F-001.

## 8. Dependencies & flags
- React (hooks, createPortal), react-router-dom (NavigationSeam), i18n lane keys (w26.i02), client seam (useClient via context), playlistsStore (localStorage). No feature flags. `import.meta.env.DEV` not used in this chunk.

## 9. Open questions for the next agent
- playlistsStore.ts SEED (F-001 primary site) — which agent owns it? Flag there.
- Whether the shell strip slot (AppShell part2 — my next-but-one chunk) tolerates multiple portal children (F-003 blast radius).
- context.tsx `isStateProbeRefusal` whitelist (context.tsx:313-317) decides which engine errors stay invisible — W15-A10 audits that file; my chunk only consumes `lastError` via overlays.
