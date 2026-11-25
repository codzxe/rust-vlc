// w31.4 (lane `ui-canvas`) — the media surface: the canvas's real
// presentation elements. Two halves:
//
// * the <video> half (w26.i02, h03 #2 — moved here verbatim from
//   PlayerScreen.tsx so the surface lives in one owned file): url
//   sources render REAL bytes through the browser's media stack; every
//   other source mounts with no src — an honest empty dark surface.
//
// * the frame-canvas half (w31.4): the webview consumer of w31.1's
//   frame channel. Inside the desktop shell it mounts a <canvas>,
//   binds it to the shell's shipper via the ONE command
//   (`session.set-frame-channel` — the channel id rides the standard
//   command envelope through the app's own transport), and blits every
//   arriving LUF1 record with a 2D putImageData presenter. In a plain
//   browser it NEVER renders (the same `__TAURI_INTERNALS__` presence
//   probe the transport uses) — the honest empty surface stands.
//
// A/V SYNC, HONESTLY: the surface does no timing of its own. The shell
// ships exactly one frame per pacer tick (w31.1's one-frame-per-tick
// contract — the ticker drains the mailbox's newest record), so the
// webview composites at the shell's PRESENT-TIME cadence and audio
// stays in step with whatever the engine scheduled. A frame that
// arrives is drawn immediately; a paused engine ships nothing and the
// canvas keeps the last presented frame (a paused video shows its
// frame — that is the honest behavior).

import { useCallback, useEffect, useMemo, useRef } from "react";
import { useClient } from "../../client/ClientProvider";
import {
  FrameChannelShim,
  LUF1,
  decodeLuf1Record,
  detectFrameChannelBackend,
  type DecodedLuf1Frame,
  type FrameChannelBackend,
} from "../../client/frame-channel";
import { keys as laneKeys } from "../../i18n/keys/fx6_a05";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import type { SourceSpec } from "../../generated/commands";

// ---------------------------------------------------------------------------
// FX6-A05 (wave FX6, FIX-PLAN §3 F1.2) — the frame-counter readout's lane
// i18n seam (the w26.a10 / w28.window pre-registration pattern): the
// readout's keys live in ../../i18n/keys/fx6_a05.ts until the Lead
// registers them into messages.ts; the resolver answers catalog-first so
// the copy never drifts between the two states.
// ---------------------------------------------------------------------------

/** Pre-registration fallbacks (this lane's own keys only). */
const FRAME_STAT_FALLBACKS: Readonly<Record<string, string>> = { ...laneKeys };

/** Resolves a readout label: central catalog → lane pre-registration → key. */
function useFrameMsg(): (key: keyof typeof laneKeys) => string {
  const m = useMsg();
  return useCallback(
    (key) => m(key as MessageKey) ?? FRAME_STAT_FALLBACKS[key] ?? key,
    [m],
  );
}

/**
 * The webview's OWN frame-lane accounting (FX6-A05 — honest client-side
 * observables only): presented/skipped are counted HERE, at the draw call
 * and the refusal path; lastFrameId is the engine-assigned id carried in
 * the LUF1 record header — engine truth riding the existing transport.
 *
 * THE HONEST GAP (F1.2's "frame counter that matches the decode count"):
 * the engine's produced/shipped/dropped-at-mailbox counters do NOT ride
 * the transport to the webview in this wave (the FX6-A03 seam is pending;
 * the Event union carries no frame-stat kind, the ticker ships records
 * only). This surface therefore NEVER fabricates them — it shows the
 * counts this webview can actually see, labeled as its own.
 */
interface FrameLaneCounters {
  /** Records decoded AND drawn (putImageData returned) by this surface. */
  presented: number;
  /** Records refused client-side (decode/present failure — drift defense). */
  skipped: number;
  /** The last drawn record's engine-assigned id (null before the first). */
  lastFrameId: number | null;
}

/**
 * Repaint the readout's values — imperative, the same lane as the canvas
 * draw itself (a per-frame React re-render would be pure cost; the
 * counters ARE the frame lane's presentation state, and the JSX initial
 * values re-read the live counters on every render, so a re-render can
 * never desync the readout from the painter).
 */
function paintFrameStats(element: HTMLElement | null, counters: FrameLaneCounters): void {
  if (element === null) {
    return;
  }
  const setValue = (name: string, value: string): void => {
    const target = element.querySelector(`[data-frame-stat-value="${name}"]`);
    if (target !== null) {
      target.textContent = value;
    }
  };
  setValue("presented", String(counters.presented));
  setValue("last-id", counters.lastFrameId === null ? "" : String(counters.lastFrameId));
  setValue("skipped", String(counters.skipped));
}

/**
 * The canvas's real media surface (w26.i02, h03 #2) — the UI half of
 * the 4.H1 render seam. A <video> element mounted while a session is
 * up: url sources render real bytes through the browser's media stack;
 * every other source (no addressable bytes yet) mounts with no src:
 * an honest empty dark surface over the near-black canvas, never a
 * fabricated poster or synthetic frames. The element FOLLOWS the
 * session state (playing → play(), anything else → pause()); its own
 * events never drive state — the engine truth (bridge view / controls
 * context) is the only playback authority.
 *
 * w31.4: inside the desktop shell the FRAME CANVAS joins the video
 * element (same absolute-fill geometry; the canvas paints after it in
 * DOM order, so the engine's presented frames are what the user sees).
 */
export function MediaSurface({
  source,
  playing,
}: {
  source: SourceSpec | null;
  playing: boolean;
}) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const src = source !== null && source.kind === "url" ? source.url : null;
  useEffect(() => {
    const el = videoRef.current;
    if (el === null) return;
    if (playing) {
      // Real browsers return a promise; test DOMs may return undefined —
      // Promise.resolve normalizes both. Autoplay policy or an unloadable
      // stream stays quiet: the engine state is the only truth, never the
      // element's own.
      void Promise.resolve(el.play()).catch(() => {});
    } else if (src !== null) {
      // Only a surface with real bytes needs holding; a src-less element
      // has nothing to pause (and no browser media stack to invoke).
      el.pause();
    }
  }, [playing, src]);
  return (
    <>
      <video
        ref={videoRef}
        className="lumen-player__media"
        data-media-surface="video"
        data-media-bytes={src === null ? "none" : "url"}
        src={src ?? undefined}
        /* Presentation only: the transport lives in ControlBar, the label on
         * the canvas region — a second accessible name here would
         * double-announce (doc 08 §4.5). */
        aria-hidden="true"
        /* No controls, no poster, no autoplay attribute — the effect follows
         * the session state; nothing pretends before the engine says so. */
        playsInline
      />
      <FrameCanvasSurface />
    </>
  );
}

/**
 * The frame-canvas consumer (w31.4). Probes the desktop shell the same
 * way the app transport does (`__TAURI_INTERNALS__` presence — the
 * consumer and the transport decision can never drift); outside the
 * shell this renders NOTHING and touches no transport (the browser's
 * honest empty surface stays exactly as it was).
 */
export function FrameCanvasSurface() {
  const backend = useMemo(() => detectFrameChannelBackend(), []);
  return backend === null ? null : <FrameCanvasSurfaceLive backend={backend} />;
}

/** The live consumer — mounted only with a probed backend in hand. */
function FrameCanvasSurfaceLive({ backend }: { backend: FrameChannelBackend }) {
  const client = useClient();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const nextRequestId = useRef(0);
  // FX6-A05: the counter readout's element + this webview's own lane
  // accounting (see FrameLaneCounters — client-side observables only).
  const statsRef = useRef<HTMLDivElement | null>(null);
  const lane = useRef<FrameLaneCounters>({ presented: 0, skipped: 0, lastFrameId: null });
  const frameMsg = useFrameMsg();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null) {
      return; // unmounting — the cleanup below is the whole story
    }
    // Allocate the channel + register the callback (the shim owns id
    // allocation, window._<id>, ordering, and the raw-payload fetch
    // dance; a decode failure inside onRecord is a console diagnostic,
    // never a crash and never a stall).
    const shim = new FrameChannelShim(backend, {
      onRecord: (record) => {
        try {
          // Decode first, then present: a refusal anywhere in the pair is
          // ONE skipped record (the typed skip below), and presented counts
          // only frames putImageData actually blitted.
          const frame = decodeLuf1Record(record);
          presentLuf1Frame(canvas, frame);
          lane.current.presented += 1;
          lane.current.lastFrameId = frame.header.frameId;
        } catch (error) {
          lane.current.skipped += 1;
          // Typed skip + diagnostic: unknown magic/version/format codes,
          // truncation, geometry lies — the channel keeps flowing (the
          // shell's encoder guarantees the record form, so this branch
          // is drift defense, exercised by the unit tests).
          console.warn(
            `lumen frame channel: skipping a record: ${error instanceof Error ? error.message : String(error)}`,
          );
        }
        paintFrameStats(statsRef.current, lane.current);
      },
    });
    // Bind the shell's shipper to THIS channel: the id rides the ONE
    // command envelope through the app's own transport (the same
    // ClientProvider seam every other dispatch uses). The shell's
    // bind is idempotent and stash-safe before the first open (w31.1),
    // so mount-time is the right time.
    nextRequestId.current += 1;
    void client
      .send({
        requestId: nextRequestId.current,
        cmd: "session.set-frame-channel",
        args: { channel: shim.id },
      })
      .then((response) => {
        if ("error" in response.result) {
          // A refused bind (an older shell build) — no frames will
          // arrive; say so and keep the honest dark surface.
          console.warn(
            `lumen frame channel: the bind was refused (${response.result.error.code}); no frames will arrive`,
          );
        }
      })
      .catch(() => {
        // The bind never reached the shell (transport down) — same
        // honest outcome: no frames, nothing fabricated.
        console.warn("lumen frame channel: the set-frame-channel bind failed; no frames will arrive");
      });
    // Lifecycle: unmount deregisters the callback (window._<id> is
    // deleted, the internals' registry entry released) — no leak, and
    // the shell's next bind (a re-mount) re-establishes a fresh channel.
    return () => shim.close();
  }, [backend, client]);

  return (
    <>
      <canvas
        ref={canvasRef}
        className="lumen-player__media"
        data-media-surface="frames"
        /* Presentation only — the video surface's own a11y note applies
         * verbatim (the label lives on the canvas region; a second
         * accessible name would double-announce). */
        aria-hidden="true"
      />
      {/* FX6-A05 (F1.2): the frame-counter readout — the honest surfacing
          of what the webview's frame lane can see. presented/skipped are
          THIS webview's counts at the draw call; the last frame id is the
          engine-assigned id from the record header. It mounts only with
          the live consumer (a browser has no frame lane, so it never
          renders there), starts zeroed (0 drawn frames is the honest
          pre-bind state; the last-id cell stays EMPTY until a frame
          arrives — the engine assigns id 0 to the FIRST record, so a
          "0" there would claim a frame was drawn), and repaints
          imperatively per frame (paintFrameStats — the canvas lane's own
          cadence, never React state churn). */}
      <div className="lumen-player__frame-stats" data-frame-stats="webview" ref={statsRef}>
        <span className="lumen-player__frame-stat">
          <span className="lumen-player__frame-stat-label">
            {frameMsg("player.frames.presented")}
          </span>
          <span data-frame-stat-value="presented">{lane.current.presented}</span>
        </span>
        <span className="lumen-player__frame-stat">
          <span className="lumen-player__frame-stat-label">
            {frameMsg("player.frames.last-id")}
          </span>
          <span data-frame-stat-value="last-id">
            {lane.current.lastFrameId === null ? "" : lane.current.lastFrameId}
          </span>
        </span>
        <span className="lumen-player__frame-stat">
          <span className="lumen-player__frame-stat-label">
            {frameMsg("player.frames.skipped")}
          </span>
          <span data-frame-stat-value="skipped">{lane.current.skipped}</span>
        </span>
      </div>
    </>
  );
}

/**
 * The 2D presenter: blit one decoded frame onto the canvas.
 *
 * SIZING: the canvas's bitmap matches the FRAME's width/height (resized
 * only on change — putImageData is 1:1, no scaling); the CSS class
 * (`.lumen-player__media`, the video surface's own) scales the bitmap
 * into the surface with object-fit: contain semantics, so the picture
 * keeps its aspect inside the canvas region.
 *
 * REPEAT records are drawn like any other (the record carries the full
 * payload — re-presenting the identical frame IS honoring the flag).
 */
function presentLuf1Frame(canvas: HTMLCanvasElement, frame: DecodedLuf1Frame): void {
  const { header, payload } = frame;
  if (canvas.width !== header.width || canvas.height !== header.height) {
    canvas.width = header.width;
    canvas.height = header.height;
  }
  const ctx = canvas.getContext("2d");
  if (ctx === null) {
    // A webview that refuses 2D contexts (or a test DOM without the
    // canvas package) — an honest typed refusal, never a crash.
    throw new Error("the 2D canvas context is unavailable (no presenter backend)");
  }
  // BGRA8 frames swizzle draw-side (the recorded 2D-path cost — the
  // WebGL2 lane, when it lands, uploads BGRA natively where the
  // extension exists); RGBA8 crosses untouched. The swizzle is local
  // to the delivered record — mutation never escapes this call.
  const pixels = payload;
  if (header.formatCode === LUF1.FORMAT_BGRA8) {
    for (let i = 0; i + 3 < pixels.length; i += 4) {
      const b = pixels[i]!;
      pixels[i] = pixels[i + 2]!;
      pixels[i + 2] = b;
    }
  }
  // createImageData + set (not `new ImageData(view, w, h)`): one copy
  // into the ImageData either way, and createImageData works on every
  // 2D context (the ImageData constructor is missing in some DOMs).
  const image = ctx.createImageData(header.width, header.height);
  image.data.set(pixels);
  ctx.putImageData(image, 0, 0);
}
