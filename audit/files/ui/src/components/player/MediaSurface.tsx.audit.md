# AUDIT: ui/src/components/player/MediaSurface.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 228 / 4 (MediaSurface, FrameCanvasSurface, FrameCanvasSurfaceLive, presentLuf1Frame) |
| File verdict | REAL-UNPROVEN (real code, really wired end-to-end on paper; never executed — tauri app cannot build in sandbox, deep launch verdicts owed per INFRA-BLOCKED.md) |
| Reachable from shipped entry point? | YES (code chain): PlayerScreen.tsx:97,1062,1098 `<MediaSurface ...>` -> FrameCanvasSurface -> detectFrameChannelBackend (`__TAURI_INTERNALS__` probe, frame-channel.ts:244-247) -> FrameChannelShim -> client.send `session.set-frame-channel` -> tauri-transport `lumen_dispatch` (apps/desktop-tauri/src/app.rs:293) -> shell shipper (apps/desktop-tauri/src/frame_channel.rs; apps/desktop/src/shell.rs:160). Browser lane: canvas half renders null by design. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | `rg "set-frame-channel" crates/ apps/" -> both sides present; `sed -n '50,54p' docs/KNOWN-LIMITS.md`; `rg -n "canvas" ui/src/screens/player/PlayerScreen.test.tsx` -> :59-70; `sed` quote verification |

## 1. One-paragraph verdict
This file is the center of the no-canvas pin, and the pin is CONFIRMED in
both directions. (1) The canvas is real: MediaSurface.tsx:174-183 mounts a
`<canvas data-media-surface="frames">` and :227 `ctx.putImageData(image, 0, 0)`
blits decoded LUF1 frames — so docs/KNOWN-LIMITS.md:52's claim "no `<canvas>`
exists in `ui/src`" is FALSE (stale). (2) It is desktop-only: the canvas
mounts solely behind the `__TAURI_INTERNALS__` probe (frame-channel.ts:244),
so in the browser/web build the player is an honest empty dark surface for
every non-url source, and PlayerScreen.test.tsx:68
(`expect(container.querySelector("canvas")).toBeNull()`) pins THAT truth.
The full wire exists on both sides (UI shim -> `session.set-frame-channel`
-> `lumen_dispatch` -> shell pacer/shipper), but per
apps/desktop-tauri/INFRA-BLOCKED.md the 3-OS launch deep verdicts "remain
manual" — no executed evidence exists that a user has ever seen a frame.

## 2. What actually works (verified)
— Static chain verified by rg (above). Unit tests exercise the browser
split AND the shell mode: MediaSurface.test.tsx:158 "shell mode: mounts the
canvas, binds via the app transport, deregisters on unmount", :191 "renders
decoded RGBA frames into the canvas (sized to the frame)", :216 BGRA swizzle,
:246 malformed record skips. These run in the vitest baseline (1705 passed).
NOT executed against a real webview — UNVERIFIED (desktop-tauri build fails
at gdk-sys in sandbox, baseline §13).

## 3. Findings
F-001 [S2]
Where: ui/src/components/player/MediaSurface.tsx:146-166 (cross: docs/KNOWN-LIMITS.md:52)
Code:
    void client
      .send({
        requestId: nextRequestId.current,
        cmd: "session.set-frame-channel",
        args: { channel: shim.id },
      })
      .then((response) => {
        if ("error" in response.result) {
What is wrong: nothing in the code — the finding is the EVIDENCE GAP. The
entire video-display path (the product's core screen) has zero executed
end-to-end evidence: the tauri crate is workspace-EXCLUDED, cannot build
here (gdk-sys), and INFRA-BLOCKED.md §B admits "the tauri-launch.yml deep
verdicts (window opens / file opens / playback + settings round-trip per
OS) remain manual". Every UI test runs against a faked `invoke`. What
correct looks like: one CI lane that launches the shell, opens a fixture,
and screenshots one presented frame. Confidence: HIGH — Verification: read
(UNVERIFIED execution — cargo-build-budget).
F-002 [S3]
Where: docs/KNOWN-LIMITS.md:52 vs ui/src/components/player/MediaSurface.tsx:174-183
Code:
    <canvas
      ref={canvasRef}
      className="lumen-player__media"
      data-media-surface="frames"
What is wrong: KNOWN-LIMITS.md:52 states "The desktop UI mounts an honest
empty media surface — no `<canvas>` exists in `ui/src`" and "the LUF1 wire
... is complete but unported". Both halves are stale: the canvas exists
(this file), and the wire IS ported (shell.rs:160 + frame_channel.rs:32).
The register that is supposed to be the honesty source of record is itself
wrong. Confidence: HIGH — Verification: read both files.
F-003 [S3]
Where: ui/src/components/player/MediaSurface.tsx:66-71
Code:
    if (playing) {
      // Real browsers return a promise; test DOMs may return undefined —
      // Promise.resolve normalizes both. Autoplay policy or an unloadable
      // stream stays quiet: the engine state is the only truth, never the
      // element's own.
      void Promise.resolve(el.play()).catch(() => {});
What is wrong: a url-source video that the browser refuses to autoplay (or
cannot load) is swallowed: state says Playing, screen shows a frozen first
frame or nothing, no error surface anywhere. Documented as philosophy
("the engine state is the only truth") but it is still a silent failure the
user cannot distinguish from playback. What correct looks like: surface the
play() rejection reason on the canvas region status. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaSurface | 54-97 | REAL | S3 | video half; play() swallowed (F-003) |
| FrameCanvasSurface | 106-109 | REAL | — | presence probe; browser -> null |
| FrameCanvasSurfaceLive | 112-184 | REAL-UNPROVEN | S2 | bind via client.send; error paths console.warn only |
| presentLuf1Frame | 198-228 | REAL | — | resize-on-change, BGRA swizzle in place (safe: shim hands a fresh Uint8Array per record, frame-channel.ts:420-442; decode returns subarray views, verified), createImageData+set |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :140 browser mode: no canvas, no bind | YES | asserts null canvas + zero transport calls |
| :158 shell mode: mounts canvas, binds, deregisters | YES | fake internals; asserts bind cmd + cleanup |
| :191 renders decoded RGBA frames (sized) | YES | drives present path with real records |
| :216 swizzles BGRA8 draw-side | YES | asserts pixel channel order |
| :246 malformed record: diagnostic, next frame renders | YES | feeds bad record then good one |
The `invoke`/fetch seam is faked (legit seam); no test touches a real
webview (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no `<canvas>` exists in `ui/src`" | docs/KNOWN-LIMITS.md:52 | FALSE | MediaSurface.tsx:174 mounts one; shell-mode test :158 asserts it |
| "LUF1 wire ... complete but unported" | docs/KNOWN-LIMITS.md:52 | FALSE (stale) | shell.rs:160 intercepts SessionSetFrameChannel; frame_channel.rs:32 |
| "In a plain browser it NEVER renders" | MediaSurface.tsx:15-16 | REAL | probe at frame-channel.ts:244-247; test :140 |

## 7. Dead / duplicated / unreachable code
None in-file. Browser lane: the entire canvas half is runtime-unreachable
(by design, honest). Note `nextRequestId` restarts at 0 per mount — request
ids collide across remounts with the transport's own id space if the
transport tracks them; tauri-transport treats requestId as opaque (test
:136 path) — no defect found, noted for the transport auditor.

## 8. Dependencies & flags
Imports useClient (bridge), frame-channel shim/decode (hand-rolled
`__TAURI_INTERNALS__` — NO @tauri-apps/api, see package-lock audit F-001),
generated SourceSpec. No build flags; runtime gating only.

## 9. Open questions for the next agent
- frame-channel.ts auditor: does the 2.12.x `unregisterCallback` vs 2.1.x
  window-prop cleanup actually match the bundled tauri version's internals?
  A mismatch leaks callbacks per mount.
- Does any CI lane launch the shell with a fixture (INFRA-BLOCKED §B.2
  "Windows launch smokes ride the release lane")? If yes, cite its log as
  the missing execution evidence for F-001.
