# AUDIT: ui/src/components/player/MediaSurface.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 228 / 4 functions |
| File verdict | REAL |
| Reachable from shipped entry point? | SPLIT: the `<video>` half — YES as UI code (PlayerScreen mounts it when a session is up, /player; url sources carry real bytes — test-pinned, baseline #23). The frame-canvas half — REAL code reachable ONLY inside the Lumen desktop shell (`__TAURI_INTERNALS__` probe :107), and the desktop host crate CANNOT COMPILE at HEAD (W14-A01 F-001: app.rs:301 calls nonexistent `spawn_session_pump`) — so no shippable binary mounts the canvas today. In a plain browser the canvas half renders NOTHING by design |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read of the full file + frame-channel.ts:244-375 (backend probe, shim constructor/`close` idempotency, 2.1.x/2.12.x index handling, end marker) · `sed -n '55,95p' ui/src/client/transport.ts` + `sed -n '100,160p' tauri-transport.ts` (send correlation is per-invoke — requestId is an echo, NOT a routing key; basis for F-001's impact assessment) · `rg -n "nextRequestId" context.tsx AppShell.tsx` (player context base 0 at :226; AppShell deliberately 0x9000 "no collisions with the player seam's counters" at :299) · `rg -n "describe|it\(" MediaSurface.test.tsx` (5 tests: browser/shell split, RGBA render+sizing, BGRA swizzle, malformed-record resilience) + frame-channel.test.ts (498 lines) · `rg -n "session.set-frame-channel" ui/src/generated/commands.ts` (command exists in the catalog) · baseline: ui-vitest.log.txt (1705 passed incl. these tests), ui-build.log.txt; W14-A01 app.rs.audit.md (host compile failure + the canvas/f-005 cross-refs) |

## 1. One-paragraph verdict
This is the file that retires Wave-0's "the GUI mounts no canvas" pin — with two precise caveats. The canvas EXISTS in source: `FrameCanvasSurfaceLive` mounts `<canvas data-media-surface="frames">` (:174-183), binds it to the shell's frame shipper through the one real command (`session.set-frame-channel` over the app's own ClientProvider transport), and blits LUF1 records via a 2D putImageData presenter with BGRA→RGBA draw-side swizzle. The wiring is careful and test-pinned: browser mode mounts nothing (probe-gated), the bind's refusal and transport failure both degrade to the honest dark surface with a console diagnostic, decode failures skip the record without stalling the channel, and the shim's lifecycle deregisters both registrations idempotently. But (a) the canvas half is reachable only in the desktop shell, and W14-A01 proved that host crate cannot compile at HEAD — so no shippable binary mounts it; and (b) the video half — url sources through the browser's media stack, no src otherwise, never a fabricated poster — is genuinely reachable and honest. The defects are all small: a requestId counter base that collides with the player context's range (violating the codebase's own documented convention, harmless today because sends correlate per-invoke), per-frame allocation + JS swizzle cost acknowledged in-code, and the video half's silent swallowing of autoplay-policy failures (no diagnostic, unlike the frame half).

## 2. What actually works (verified)
— The shell-mode split is real and tested: MediaSurface.test.tsx:140 "browser mode: no canvas, no bind" and :158 "shell mode: mounts the canvas, binds via the app transport, deregisters on unmount" — both green in baseline ui-vitest.log.txt (1705 passed).
— The presenter is correct: canvas bitmap resized only on frame-geometry change (:200-203), `createImageData` + `set` + `putImageData` 1:1 (:225-227), BGRA swizzle in-place on the per-record payload (no shared mutation), malformed records skipped with a diagnostic while the channel keeps flowing (:127-139) — test :246 pins "the next frame still renders".
— The video half's honesty contract holds: src only for url sources (:62), no controls/poster/autoplay attribute (:90-92), the element FOLLOWS the session state and its own events never drive it (:63-77) — pinned end-to-end by PlayerScreen.test.tsx:277-378 ("url sources carry REAL bytes, file sources mount without a src").
— The bind uses the app's real transport seam (ClientProvider `client.send`, :147-152), the command exists in the generated catalog, and refused/failed binds degrade loudly-but-safely (:153-166).
— The shim underneath (frame-channel.ts:298-375) handles 2.1.x/2.12.x index keying, stale-index skipping, the terminal end marker, and idempotent close — verified by read; frame-channel.test.ts (498 lines) is green in baseline.

## 3. Findings

### F-001 [S4]
Where: ui/src/components/player/MediaSurface.tsx:115, :146-151
Code:
```
  const nextRequestId = useRef(0);
```
```
    nextRequestId.current += 1;
    void client
      .send({
        requestId: nextRequestId.current,
        cmd: "session.set-frame-channel",
```
What is wrong: the frame-surface's request ids start at 1 — the same range the player context's counter uses (context.tsx:226 `useRef(0)`). AppShell.tsx:299 documents the codebase's own convention and deliberately bases its counter at 0x9000 with the comment "no collisions with the player seam's counters"; this file violates that convention.
Why it matters: today the impact is nil — TauriTransport.send correlates each response to its own invoke promise (verified :127-144); requestId is an echo, not a routing key. But the convention exists because request ids are engine-visible metadata ("the request_id stays outstanding; pending resolves on the matching engine event" — context.tsx:340-342); if the engine ever keys pending-request state by id, a frame-bind racing a playback command with the same id becomes a misroute.
What correct looks like: base the counter outside the shared low range (0x8000, mirroring AppShell's 0x9000) with the same one-line comment.
Confidence: HIGH (convention violation + today-nil impact) — Verification: read of all three counter sites + transport correlation; engine-side id semantics UNVERIFIED (not in my file set).

### F-002 [S4]
Where: ui/src/components/player/MediaSurface.tsx:215-226
Code:
```
  if (header.formatCode === LUF1.FORMAT_BGRA8) {
    for (let i = 0; i + 3 < pixels.length; i += 4) {
```
```
  const image = ctx.createImageData(header.width, header.height);
  image.data.set(pixels);
```
What is wrong: per-frame CPU cost with no buffer reuse: a full JS byte-swizzle loop over every BGRA frame plus a fresh `createImageData` allocation per frame (GC pressure at the pacer's cadence). For a 1080p BGRA frame that is ~8.3M array operations per frame on top of the copy.
Why it matters: the code itself records this as "the recorded 2D-path cost — the WebGL2 lane, when it lands, uploads BGRA natively" (:210-212) — i.e. known perf debt. On the one real decode lane (AVI MJPEG first-vertical) frames are small enough to tolerate it; the debt compounds with any larger frame geometry.
What correct looks like: reuse the ImageData when geometry is unchanged; swizzle into it during the copy (one pass) or land the WebGL2 presenter.
Confidence: HIGH — Verification: read; magnitude arithmetic from header geometry.

### F-003 [S4]
Where: ui/src/components/player/MediaSurface.tsx:66-71
Code:
```
    if (playing) {
      // Real browsers return a promise; test DOMs may return undefined —
      // Promise.resolve normalizes both. Autoplay policy or an unloadable
      // stream stays quiet: the engine state is the only truth, never the
      // element's own.
      void Promise.resolve(el.play()).catch(() => {});
```
What is wrong: the video element is unmuted (no `muted` prop) and `play()` rejections are swallowed with NO diagnostic. When autoplay policy blocks (e.g. a url source auto-opened before any user gesture — argv opens, session restore) or the stream is unloadable, the user sees a silent dark surface while the engine state says playing. The frame half logs console.warns for every degradation (:157-165); the video half logs nothing.
Why it matters: a silently-swallowed playback failure is undebuggable from the webview and indistinguishable from "no video support"; the in-code justification ("stays quiet") covers the state-model honesty but not the observability gap.
What correct looks like: `.catch((error) => console.warn(\`media surface: play() refused: ${String(error)}\`))` — the state model stays engine-truth either way.
Confidence: MEDIUM — Verification: read; the autoplay-blocked scenario is webview-policy-dependent (sticky user activation usually unmutes the problem after first interaction), hence the diagnostic-gap framing rather than a behavior claim.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaSurface | 54-97 | REAL | S4 | video half: src only for url; play/pause follows session; F-003 silent play() catch; aria-hidden documented (no double-announce) |
| FrameCanvasSurface | 106-109 | REAL | — | probe-gated; browser renders NOTHING (honest); memoized once |
| FrameCanvasSurfaceLive | 112-184 | REAL | S4 | bind over the app transport; refused/failed bind → warn + dark surface; F-001 counter base; cleanup = shim.close(); canvas reachable only in a shell that cannot compile at HEAD (W14-A01 F-001) |
| presentLuf1Frame | 198-228 | REAL | S4 | F-002 per-frame swizzle+allocation; resize-on-change correct; ctx-null → typed throw caught by onRecord; REPEAT records redrawn (documented) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| MediaSurface.test.tsx:140 browser mode (no canvas, no bind) | YES | asserts absence — catches a probe regression that would mount canvases in browsers |
| :158 shell mode (mount, bind via transport, deregister on unmount) | YES | asserts the set-frame-channel request shape + cleanup |
| :191 RGBA render + canvas sized to frame | YES | decodes a fixture record and inspects pixel data |
| :216 BGRA swizzle | YES | pixel-level assertion of the channel swap |
| :246 malformed record → diagnostic, next frame renders | YES | resilience contract |
| frame-channel.test.ts (498 lines, shim protocol) | YES | ordering, stale-index, end-marker, close idempotency |
All green: baseline ui-vitest.log.txt (1705 passed / 1 skipped). NOTE: shell-mode tests run against a MOCKED backend — the real `__TAURI_INTERNALS__` path is exercised by no test and by no runnable binary (host compile failure).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the GUI mounts no canvas, so it displays no video" | Wave-0 evidence (wave-15 prompt) | STALE for source / TRUE for shipped artifacts | the canvas mounts in shell mode (:174-183, test :158) — but no GUI binary exists from this tree (W14-A01 F-001 E0425), so no shipped artifact mounts it |
| "Inside the desktop shell it mounts a <canvas>, binds it … and blits every arriving LUF1 record" | file header :9-14 | REAL (code) / UNREACHABLE (binary) | wiring verified + test-pinned; host cannot compile at HEAD |
| "In a plain browser it NEVER renders" | header :15-16 | REAL | probe gate :107-108; test :140 |
| "the surface does no timing of its own … composites at the shell's PRESENT-TIME cadence" | header :18-25 | REAL-UNPROVEN | design property of the w31.1 one-frame-per-tick contract (shell side, W14-audited); cannot be executed in this sandbox |
| "url sources render REAL bytes through the browser's media stack; every other source mounts with no src" | header :4-7 | REAL | PlayerScreen.test.tsx:277-378 (src assertions), baseline green |

## 7. Dead / duplicated / unreachable code
- No dead code. `FrameCanvasSurfaceLive` is unreachable-in-practice (host compile failure) but correctly gated, not orphaned.
- The `presentLuf1Frame` throw-on-null-ctx path is exercised only via unit tests (jsdom without canvas package) — documented.

## 8. Dependencies & flags
- frame-channel.ts (shim + LUF1 decode), ClientProvider transport, generated SourceSpec/command types. No feature flags — runtime gating is the `__TAURI_INTERNALS__` presence probe (same as the transport; "can never drift" per :100-103).

## 9. Open questions for the next agent
- frame-channel.ts (448 lines) has no auditor in my assignment view — its FETCH_CHANNEL_DATA_COMMAND raw-payload dance and 2.1.x legacy path deserve a dedicated audit.
- Engine-side request_id semantics (does dispatch track pending by id?) — owner: crates/commands auditor; determines whether F-001 can ever escalate.
- If W14-A01's E0425 is fixed and the shell compiles, re-run the shell-mode path against a real webview — every current "shell mode" test mocks the backend.
