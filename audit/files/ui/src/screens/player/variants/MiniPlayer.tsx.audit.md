# AUDIT: ui/src/screens/player/variants/MiniPlayer.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 595 / 8 (useLaneMsg, openMediaDialog, folderOf, MiniPlayer, MiniQueue, closeAdd, onRowDrop, onListKeyDown + 2 effects) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (main.tsx → App routes /player,/miniplayer → PlayerScreen.tsx:914 `<MiniPlayer …>`). SHIPPED DESKTOP BINARY: NO — tauri host crate unbuildable at HEAD (baseline #2 exit 101; W14-A01: E0425 `spawn_session_pump` missing, apps/desktop-tauri/src/app.rs:301) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | rg "MiniPlayer" (render site PlayerScreen.tsx:914); sed on PlayerScreen.tsx:900-960 (variant branch); sed MediaSurface.tsx (full variant mounts `<video>`+`<canvas>`); rg "<video|<canvas" MediaSurface.tsx:80,174; sed queue.ts:35-55 (dragToMoveIndex/formatDuration); sed laneMsg.ts:1-27; rg keys in i18n/messages.ts:2171-2173 + keys/w26.i04.ts:11-14; sed IconButton.tsx (forwardRef ok) |

## 1. One-paragraph verdict
Real, carefully-built compact player card: transport reuses the shared state
mapping (`mapTransportControls`/`seekDisableReason`), queue rows/drag/keyboard
reorder all route through the playlist context's real commands, and the "+"
popup rides the shared `enqueueSources` seam. Tests are substantive
(variants.test.tsx:164-353 cover enqueue, keyboard reorder, honest disabled
states) and passed in baseline `npm test` (1705 passed —
audit/runs/baseline/ui-vitest.log.txt). But the variant's entire reason to
exist — the 373×209 "video area" — is a permanent dark placeholder: no
`<video>`, no frame `<canvas>`, no thumbnail. The full variant mounts the real
`MediaSurface` (PlayerScreen.tsx:1062/1098), so switching to miniplayer
mid-playback DROPS the picture entirely, even inside the desktop shell where
frames exist. The in-code justification for that ("same contract as the full
shell") is stale and contradicts the code.

## 2. What actually works (verified)
— verified by rg + sed trace: MiniPlayer is rendered only from
PlayerScreen.tsx:914 under `variant === "miniplayer"`; all buttons call real
context commands (`actions.togglePlayPause`, `actions.seek`, `playlist.prev/
next/move/playItem/playLater`); prev/next honestly disabled when
`playlist.notExposed || entries.length === 0` (lines 131-141). No dead buttons.
— verified by baseline audit/runs/baseline/ui-vitest.log.txt: 1705 passed /
1 skipped incl. variants.test.tsx (miniplayer describe, queue keyboard path,
enqueue seam) — the drag/keyboard reorder logic executes under test.
— dragToMoveIndex math checked by hand (queue.ts:35-39): Alt+Down
`dragToMoveIndex(activeRow+2, activeRow, count)` → lands at activeRow+1;
Alt+Up → activeRow-1. Correct one-step moves; edge rows no-op (lines 441, 446).

## 3. Findings
### F-001 [S2] MiniPlayer "video area" is a permanent placeholder — the variant can never display any video
Where: ui/src/screens/player/variants/MiniPlayer.tsx:185-195
Code:
```
        <div
          data-region-id={thumbnailRegion.id}
          role="region"
          aria-label={m(thumbnailRegion.labelKey)}
          className="lumen-mini__video"
          data-placeholder="engine-wiring-4.H1"
        >
          {/* Honest placeholder: the thumbnail is the video-canvas seam; no
              media element until card 4.H1 (same contract as the full shell). */}
```
What is wrong: The miniplayer's media area mounts no presentation element at
all — no `<video>` (url sources), no `FrameCanvasSurface` (the w31.4 frame
channel), not even a still thumbnail. The full variant mounts both
(MediaSurface.tsx:80 `<video …>`, :174 `<canvas …>`; wired at
PlayerScreen.tsx:1062/1098). The comment's justification — "no media element
until card 4.H1 (same contract as the full shell)" — is FALSE today: card 4.H1
landed (w26.i02 url-video + w31.4 frame canvas) and the full shell's contract
changed; this file never followed.
Why it matters: A user in the desktop shell playing a 4:2:0 MP4 (baseline
mp4-smoke-open-session-420.txt: Playing) who toggles the miniplayer (keymap "B",
variants.test.tsx:440) loses the picture entirely — audio keeps playing (if a
real device existed) over a dead dark card. The product's miniplayer frame
(Miniplayer.svg) is a compact VIDEO player; this is a compact nothing player.
What correct looks like: Mount `<MediaSurface source={queueSource}
playing={…}/>` (or at minimum FrameCanvasSurfaceLive) inside
`.lumen-mini__video`, delete the stale comment, drop `data-placeholder`.
Confidence: HIGH — Verification: read (PlayerScreen.tsx:1062/1098 +
MediaSurface.tsx:80/174 traced; no MediaSurface import exists in MiniPlayer.tsx).

### F-002 [S3] Fifth private copy of the lane-key resolver that laneMsg.ts was created to eliminate
Where: ui/src/screens/player/variants/MiniPlayer.tsx:89-93
Code:
```
function useLaneMsg() {
  const m = useMsg();
  const laneCatalog: Record<string, string> = { ...laneKeys, ...i04Keys };
  return (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key];
}
```
What is wrong: `screens/player/laneMsg.ts` documents that the wave-J review
counted four divergent copies of this resolver and made "this module the
single shared shape" (`useLaneMsg(catalog)`). This file keeps its own divergent
copy (merging two catalogs by spread) instead of
`useLaneMsg({ ...laneKeys, ...i04Keys })`.
Why it matters: The exact drift the shared module exists to prevent — a fix to
the fallback chain (e.g. pseudo-locale behavior) lands in laneMsg.ts and
silently misses this surface.
What correct looks like: import { useLaneMsg } from "../laneMsg"; pass the
merged catalog; delete the local copy.
Confidence: HIGH — Verification: read (laneMsg.ts:1-27 compared verbatim).

### F-003 [S3] Stale `dragFrom` state poisons the next drop after a cancelled drag; external drops silently ignored
Where: ui/src/screens/player/variants/MiniPlayer.tsx:413-415
Code:
```
    const raw = dragFrom !== null ? dragFrom : event.dataTransfer.getData("text/plain");
    const from = raw === "" ? null : Number(raw);
    setDragFrom(null);
```
What is wrong: `dragFrom` is set in onDragStart (line 584) and cleared only in
onRowDrop. A drag that ends in cancel/escape or a drop outside any row leaves
`dragFrom` set forever; the NEXT drop event (including a genuine external
drag of text onto a row) then reorders using the stale index instead of the
payload. There is no `onDragEnd` cleanup. Also, dropping OS files onto the
queue does nothing by design (empty text payload → return) — VLC's miniplayer
accepts file drops — a functional gap left undocumented in the UI.
Why it matters: Wrong reorder from stale state is a real misbehavior path
(drag row 3, press Escape, drag row 3 again but drop on row 7 — happens to
work; drag external text, drop → row 3 moves without any drag gesture).
What correct looks like: clear `dragFrom` in an onDragEnd handler; treat a
non-null dragFrom on drop as internal-only and validate `from <
playlist.entries.length` before use (line 417's `entry` check partially saves
this, but only AFTER computing from the wrong source).
Confidence: MEDIUM — Verification: read; no test covers cancelled-drag then
drop (variants.test.tsx covers the happy drag path only).
### F-004 [S4] Dead fallback anchor {left: 0, top: 0}
Where: ui/src/screens/player/variants/MiniPlayer.tsx:524-525
Code:
```
              const rect = addTriggerRef.current?.getBoundingClientRect();
              setAddAnchor(rect ? { left: rect.left, top: rect.bottom + 4 } : { left: 0, top: 0 });
```
What is wrong: The click handler runs on a mounted, focused button —
`getBoundingClientRect()` always returns a rect in every DOM incl. jsdom;
the `(0,0)` branch is unreachable dead code that would pop the menu into the
screen corner if it ever ran.
Why it matters: Dead defensive branch obscures the real contract.
What correct looks like: `const rect = addTriggerRef.current!.getBoundingClientRect();`
or render-null when the ref is absent.
Confidence: HIGH — Verification: read (handler bound at line 519 to the same
button that owns the ref).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useLaneMsg (local) | 89-93 | REAL | S3 (F-002) | correct fallback chain (messages.ts:3011-3020 returns undefined for unknown → `??` fires); duplicated from laneMsg.ts |
| openMediaDialog | 99-101 | REAL | — | dispatches OPEN_MEDIA_EVENT; the shell's one funnel (AppShell listens) |
| folderOf | 105-109 | REAL | — | splits on / and \, needs ≥2 segments; "clip.mp4" → null (honest hide). Verified by hand |
| MiniPlayer | 111-313 | REAL-UNPROVEN | S2 (F-001) | composition verified by tests; video area is a permanent placeholder (F-001) |
| MiniQueue | 337-595 | REAL-UNPROVEN | S3 (F-003) | runs under variants.test.tsx:250-353; drag stale-state defect |
| closeAdd | 380-383 | REAL | — | restores focus; IconButton forwards refs (IconButton.tsx forwardRef) |
| onRowDrop | 408-420 | REAL | S3 (F-003) | empty-payload guard correct (j08 P3-7); stale dragFrom unhandled |
| onListKeyDown | 435-479 | REAL | — | Alt+Up/Down one-step math verified against dragToMoveIndex; clamped, no wrap; Enter/Space play real command |
| outside-dismiss effect | 367-376 | REAL | — | pointerdown listener added/cleared correctly on addOpen toggle |

## 5. Test honesty (variants.test.tsx, same repo)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders the compact card only (165-182) | YES | asserts data-variant + absence of full-bar chrome |
| overlay prev·play·next row (184-199) | YES | queries by accessible name, asserts close present |
| C2 mapping reuse: idle ⇒ disabled (201-207) | YES | asserts disabled play/seek in idle demo state |
| sr-only h1 + queue panel (221-248) | YES | asserts exactly one h1, queue toggle aria wiring |
| + enqueues through playlist.enqueue (250-281) | YES | clicks a real popup entry, asserts queue grew via context |
| keyboard listbox ops (283-353) | YES | fires real keydowns, asserts reorder through playlist.move |
| B toggles variant (440-446) | YES | PlayerScreen-level, asserts data-variant flip |
Mutation check: delete `disabled={queueEmpty}` on prev → test 165-182's
"Close miniplayer is enabled" still passes but no test asserts prev disabled
with empty queue in the MINI branch specifically — mapping test (201-207)
covers play/seek only. Partial gap, not tautology.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "miniplayer: compact always-on-top card with video area" | file header lines 6-8 (frame Miniplayer.svg) | SHALLOW | F-001: video area renders `player.canvas.placeholder` text, no element |
| "the buttons keep C2's honest engine-state mapping" | header line 19-20 | REAL | mapTransportControls(state) line 122; test 201-207 |
| "the + is wired to the product's first real enqueue caller" | header 24-30 | REAL | enqueueEntries → playlist.playLater lines 389-401; test 250-281 |

## 7. Dead / duplicated / unreachable code
- Local `useLaneMsg` duplicate of laneMsg.ts (F-002).
- Dead `(0,0)` anchor fallback (F-004).
- `thumbnailRegion` prop is naming fiction: nothing is ever mounted in the
  region; used only for `data-region-id`/aria-label (lines 186-188) — a
  "thumbnail" feature does not exist anywhere in the UI (rg "thumbnailRegion"
  → this file + variants.test.tsx mock regions only).

## 8. Dependencies & flags
Imports from ../../../components, menu, i18n, open-media, playlist, player
context/controls/layout, variants.css — all in-tree, no feature flags, no
network. `OPEN_MEDIA_EVENT` cross-screen CustomEvent contract (AppShell
listener verified reachable in PlayerScreen.tsx:153).

## 9. Open questions for the next agent
- Does any host actually pass `onClose` (PlayerScreen.tsx:922-928 supplies a
  stop+expand stand-in marked [PROPOSAL])? If the desktop shell never passes a
  real window-close, the miniplayer "close" mislabels what it does (stops
  playback) — UI/UX honesty question, not covered by any test.
- F-001 fix ordering: if MediaSurface mounts here, `playing` must come from
  engine truth — trace `usePlayerEngineView` before rewiring.
