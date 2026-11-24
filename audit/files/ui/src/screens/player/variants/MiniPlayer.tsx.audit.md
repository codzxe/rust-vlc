# AUDIT: ui/src/screens/player/variants/MiniPlayer.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 595 / 5 (useLaneMsg, openMediaDialog, folderOf, MiniPlayer, MiniQueue) |
| File verdict | SHALLOW (controls real; video area permanently a placeholder; stale parity comment) |
| Reachable from shipped entry point? | YES — App.tsx:305 `/miniplayer` -> PlayerRoute -> PlayerScreen (PlayerScreen.tsx:914 renders `<MiniPlayer>`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | `sed` quote verifies (L188-196, L499-503); rg for `<video`/MediaSurface across ui/src; grep DEMO_CLIPS/isDemoTransport/recentOpenings in client/ |

## 1. One-paragraph verdict
Every control in the mini card is genuinely wired: play/pause/seek go through `usePlayerControls` (real command context), prev/next through the playlist context with honest disabled+reason labels, close renders only when a host callback exists, drag-reorder and the keyboard listbox issue real `playlist.move`/`playItem`. But the card's video area is a permanent placeholder — the full player gained a real `<video>` MediaSurface (PlayerScreen.tsx:1089-1096, components/player/MediaSurface.tsx:80) while the mini variant never mounts one, so entering miniplayer while a URL stream plays kills the picture (audio/engine state continue). The queue "+" popup is real but, on the actual desktop shell, offers zero pickable sources (demo clips are browser-only, recentOpenings exist only on the demo transport).

## 2. What actually works (verified)
— verified by read + rg: MediaSurface/`<video` exists in PlayerScreen + components/player/MediaSurface.tsx only (mini has none); DEMO_CLIPS gating `isDesktopShell()` in enqueueSources.ts:18-24; committed variants tests (variants.test.tsx) cover queue keyboard/reorder. UI suite pass cited from baseline (vitest 1705 passed).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/variants/MiniPlayer.tsx:185-194
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
What is wrong: the mini variant NEVER renders a media surface. The parenthetical claim "(same contract as the full shell)" is FALSE as of w26.i02 — the full shell mounts a real `<video>` (PlayerScreen.tsx:1093-1096 `{hasMedia ? <MediaSurface .../> : null}`) with URL bytes played by the browser stack and a FrameCanvasSurface for engine frames in the desktop shell; the miniplayer has neither. Why it matters: a user playing a stream who shrinks to the miniplayer loses all video with no indication beyond an sr-only note; on desktop the engine's frame canvas also does not follow into the mini card. What correct looks like: mount MediaSurface (url sources) / FrameCanvasSurface (desktop) in the mini video region, or visibly label the region as audio-only. Confidence: HIGH — Verification: read + rg (no `<video` under variants/).
F-002 [S3]
Where: ui/src/screens/player/variants/MiniPlayer.tsx:389-401 (popup built from screens/playlist/enqueueSources.ts:18-24)
Code:
```
  const addEntries = enqueueEntries(
    enqueueSources,
    (source, title) => {
      playlist.playLater(source, title);
      closeAdd();
    },
```
What is wrong: in the browser the "+" popup lists fabricated demo clips with real-looking labels ("Big Buck Bunny (1080p, 10 min)", messages.ts:2056) — mock data presented as pickable media, with no per-item "Demo" marking; the only disclosure is the shell-level Demo badge. Inverted problem on desktop: `isDesktopShell()` strips demo clips AND `isDemoTransport(client)` strips recentOpenings (enqueueSources.ts:22-27; recentOpenings exists only on DemoTransport — client/demo-transport.ts:354, none in tauri-transport.ts), so the REAL product's "+" popup contains only a disabled empty row + the open-media shortcut. Why it matters: browser users can be misled into thinking they opened real files; desktop users get a source picker that picks nothing. What correct looks like: per-item demo labels in browser; real recent-openings tracking on the desktop transport. Confidence: HIGH — Verification: read + grep.
F-003 [S3]
Where: ui/src/screens/player/variants/MiniPlayer.tsx:499-502
Code:
```
          {/* The frame's sort affordance has no playlist command behind it
              yet (feature-not-exposed) — omitted rather than faked; the
              collapse (—) and add (+) affordances are real. The + opens the
              enqueue popup (w26.i04). */}
```
What is wrong: nothing in the code — noted as the honest handling of a would-be dead button (sort omitted, not faked). Recorded so the dead-surface census stays accurate: mini queue header has NO dead buttons. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useLaneMsg | 89-93 | REAL-UNPROVEN | — | same pre-registration seam as AppShell; keys registered per catalog greps |
| openMediaDialog | 99-101 | REAL-UNPROVEN | — | dispatches the shell's single OPEN_MEDIA_EVENT (AppShell listens, AppShell.tsx:312-316) |
| folderOf | 105-109 | REAL-UNPROVEN | — | pure path logic; null on non-file (honest hide) |
| MiniPlayer | 111-313 | SHALLOW | S2 | controls real; video area placeholder (F-001) |
| MiniQueue | 337-595 | REAL-UNPROVEN | S3 | drag + keyboard reorder real (`playlist.move` via dragToMoveIndex); "+" popup thin on desktop (F-002) |

## 5. Test honesty
variants.test.tsx (same dir) asserts queue composition, keyboard ops, and `data-placeholder` attributes — tests pin the placeholder state as INTENDED, so they would not catch F-001's parity drift (they assert the mini art stays a placeholder; the full-shell change made that stale). Would fail if reorder/play wiring broke: yes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no media element until card 4.H1 (same contract as the full shell)" | MiniPlayer.tsx:192-193 | FALSE | full shell mounts `<video>` MediaSurface (PlayerScreen.tsx:1093, MediaSurface.tsx:80) |
| "never fake: the buttons keep C2's honest engine-state mapping" | header L19-20 | REAL | disabled/loading states driven by mapTransportControls(state) (L122-123, 228-229) |
| "the same pickable sources the open-media dialog offers (never fabricated)" | enqueueSources.ts:2-3 | PARTLY FALSE | sources are the DEMO_CLIPS fabrications in browser context; open-media dialog at least groups them under "Demo clips" (messages.ts:2055) |

## 7. Dead / duplicated / unreachable code
No dead code; sort affordance honestly omitted (F-003 note).

## 8. Dependencies & flags
components barrel, MenuPopup, playlist context (real commands), player controls context, enqueueSources seam, variants.css. No feature flags.

## 9. Open questions for the next agent
- Does the desktop FrameCanvasSurface (w31.4) have a route into ANY compact surface, or is miniplayer video a product-level gap? (PlayerScreen lane)
- enqueueSources.ts + open-media dialog demo-clip labeling belongs to that lane's audit (cross-ref F-002).
