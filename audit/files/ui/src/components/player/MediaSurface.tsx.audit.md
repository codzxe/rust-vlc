# ui/src/components/player/MediaSurface.tsx — the canvas (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `ui/src/components/player/MediaSurface.tsx` (228 LOC) — the w31.4 `ui-canvas` lane |
| Reachability | lumen-tauri binary → embedded ui/dist (frontendDist, tauri.conf.json:7) → /player route → `PlayerScreen` (mounted while `hasMedia`) → `MediaSurface` → `FrameCanvasSurface` → (only when `window.__TAURI_INTERNALS__` present, line 107) `FrameCanvasSurfaceLive` → `<canvas data-media-surface="frames">` (line 174) + `session.set-frame-channel` bind over the app transport (lines 147-152) → shell ticker ships LUF1 records → `onRecord` → `decodeLuf1Record` → `presentLuf1Frame` → `ctx.putImageData(image, 0, 0)` (line 227). **Runtime pixels UNVERIFIED in this sandbox (jsdom ships no rasterizer; no webview exists here — MediaSurface.test.tsx:5-7 states it).** |
| Verdict | **Real canvas consumer, honestly scoped: 2D putImageData presenter, shell-mode only, browser-mode renders nothing and binds nothing.** The code path is complete and committed-tested at the seam level (mocked 2D context); the actual on-screen blit has no committed runtime evidence. |

## Per-function table

| Function | Lines | What it does (sed-verified) |
|---|---|---|
| `MediaSurface` | 54-97 | renders the `<video>` half (url sources only get a `src`, line 62/85) + `<FrameCanvasSurface />` (line 94); `playing` drives `el.play()`/`el.pause()` (lines 63-77), element events never drive state. |
| `FrameCanvasSurface` | 106-109 | `useMemo(() => detectFrameChannelBackend(), [])` (line 107) — null outside the desktop shell → renders NOTHING (line 108). |
| `FrameCanvasSurfaceLive` | 112-184 | the effect: builds `FrameChannelShim` with `onRecord` → `presentLuf1Frame(canvas, decodeLuf1Record(record))` (line 129); decode failure → `console.warn` skip, never crash/stall (lines 130-138); sends the bind `{ cmd: "session.set-frame-channel", args: { channel: shim.id } }` through the app's own transport (lines 147-152); refused bind / failed transport → warn + honest dark surface (lines 153-166); unmount → `shim.close()` (line 170). Renders `<canvas className="lumen-player__media" data-media-surface="frames" aria-hidden="true">` (lines 174-182). |
| `presentLuf1Frame` | 198-228 | resizes the bitmap to the FRAME's width/height only on change (lines 200-203); `getContext("2d")` null → typed throw (205-209); BGRA8 swizzle loop in JS (lines 215-221); `createImageData` + `image.data.set(pixels)` + `ctx.putImageData(image, 0, 0)` (lines 225-227). |

## Findings

- **S2 — the entire SEE-path's last meter is mock-tested only.** The committed tests (`MediaSurface.test.tsx`) fake `__TAURI_INTERNALS__` (lines 42-62) AND fake the 2D context (`mockCanvas2d`, lines 114-132: "jsdom ships no canvas rasterizer — the presenter's putImageData calls are asserted at the seam"). No committed test or CI lane anywhere in the repo verifies a real webview rasterizes these bytes. Combined with INFRA-BLOCKED.md §B.2 ("deep verdicts ... remain manual"), the claim "a user can SEE video through lumen-tauri" is, at HEAD, a statically-wired, engine-side-proven, UI-seam-mocked claim. UNVERIFIED at runtime — and the repo's own documents agree it is unproven.
- **S3 — per-frame full-bitmap work on the UI main thread.** `image.data.set(pixels)` copies w·h·4 bytes per frame; the BGRA swizzle is a per-byte JS loop (lines 216-220). At 1080p that is ~8.3 MB copied per frame (~250 MB/s at 30 fps), plus a fresh `createImageData` allocation per frame. RGBA8 (the engine's shipped converter output, pipeline_host.rs:1506 `format: media_types::PixelFormat::Rgba8`) skips only the swizzle, not the copies. The code's own comment defers the WebGL2 lane (lines 210-213). Correctness unaffected; sustained 30 fps at 1080p in a webview is UNVERIFIED.
- **S3 — bind is mount-time, mount is state-gated.** `FrameCanvasSurface` mounts only when `PlayerScreen` renders `MediaSurface`, which requires `hasMedia` (PlayerScreen.tsx:1061-1066/1097-1102, `effectiveEngineState.state !== "idle"`). Therefore the tap is installed AFTER the session is live; frames presented before the bind mount are lost (see shell.rs audit S3). The bind itself is idempotent and stash-safe (w31.1), so this is a startup window, not a permanent gap.
- **S4 — `presentLuf1Frame` throws on a null 2D context** (line 208) — caught by the `onRecord` try/catch (lines 127-138) as a console diagnostic; canvas stays dark with no user-visible signal. Consistent with the honest-dark-surface posture.
- **S4 — the `<video>` half's `Promise.resolve(el.play()).catch(() => {})`** (line 71) swallows autoplay-policy refusals silently — by design ("the engine state is the only truth"), but a user with audio blocked by browser policy on a url source gets video-with-no-audio and no notice. Browser surface only; not the tauri shell's issue.

## Test honesty (committed UI evidence)

`MediaSurface.test.tsx` (275 LOC, 5 tests): browser mode renders NO canvas and sends nothing (lines 140-156 — `expect(document.querySelector("canvas")).toBeNull()`, `transport.sent` empty); shell mode mounts the canvas with `data-media-surface="frames"` + `aria-hidden`, sends `session.set-frame-channel` with the allocated id 41 (lines 158-189), deregisters on unmount (`window._41` deleted, `unregisterCallback` called); renders a decoded RGBA record into the (mocked) canvas with correct bitmap size + payload equality (191-214); BGRA swizzle correctness (216-244); malformed record → console diagnostic and the channel keeps flowing (246-274). **All five pin the right contract at the seam; none rasterizes.** The suite total (1705 passed) is the orchestrator-claimed + worklog-claimed number (worklog.md:2280-2281: "29 new tests; full UI suite 1705 green") — not re-run here (rg/sed-only mandate).

## Doc claims

- Header comment lines 9-16 describe exactly what ships (canvas half, shell-only, browser never renders). Accurate.
- Line 18-25 "A/V SYNC, HONESTLY: the surface does no timing of its own ... the webview composites at the shell's PRESENT-TIME cadence" — accurate, and the root of the 32.65 fps ship ceiling (see app.rs audit).
- `docs/KNOWN-LIMITS.md:50` (w31-era rows) does not yet carry a "GUI video display" row; the w31.6 commit message itself ("w31: GUI video display end to end") is the strongest doc claim — the runtime half remains unproven per INFRA-BLOCKED §B.2 (see PlayerScreen audit for the stale in-file doc contradiction).

## Dead code / deps / flags

- No dead code. Deps: React + the in-repo `frame-channel.ts` (no `@tauri-apps/api` — the allowlist posture, lines 13-16).
- `data-media-surface` + `data-media-bytes` attributes are test hooks (also used by PlayerScreen tests) — intentional.

## Open questions

1. `canvas.getContext("2d")` per frame (line 204): correct but re-queried per record; the context handle is never cached. Cosmetic cost (implementations cache), recorded for the record.
2. CSP: `putImageData` needs no special policy; the `img-src`/`media-src` directives in tauri.conf.json are for the `<video>` half only. No conflict found between the canvas path and the shipped CSP.
