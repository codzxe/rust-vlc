// w26.gfixA (issue #62 fix cycle) — the "Customize Player" surface,
// rebuilt from the 320px rail era to the 9-state frame set
// `design/figma/Player - customize*.svg`:
//
//   ┌──────────────────────────────────────────────────────────────┐
//   │ header band 94px #090909 — "Customize Player" 15px + ✕ 10px  │
//   │ instruction  (the frame's verbatim drag-and-drop helper line) │
//   │ THE BOX      31-tile catalog — 9 columns × 4 rows, dashed     │
//   │              wells 27×27 #6B6B6B dash 4-4, 2-line labels      │
//   │ footer       Cancel (left) · Save (right) — 100×32 r6, Save   │
//   │              dimmed while clean (frame 0) / active (frames    │
//   │              -1…-6)                                          │
//   └──────────────────────────────────────────────────────────────┘
//
// MODAL ANATOMY (frame: rect 436,189.5 1048×655 rx14 fill #0E0E0E @0.95
// + drop shadow + blur halo, floating slightly high of center over the
// player): hosted on the shared Dialog primitive (scrim + focus trap +
// Escape), NOT a right rail — the Lead ruling of the g-wave.
//
// THE EDIT MODEL (the frame's own instruction copy, verbatim):
//   * "Drag items around … the box … to re-order them" — in-box drags
//     reorder the catalog arrangement;
//   * dragging a tile OUT of the box onto the CONTROL BAND places it in
//     the toolbar (frames -6/-7: the Favorite ghost lands in the band);
//   * "Drag items into the box to remove them from the player" — band
//     items drag (or click — the keyboard-safe path) back into the box.
// The band renders the arrangement as dashed edit wells while the modal
// is open (frames -1..-4's live-chrome preview grammar): the fixed
// transport cluster (prev · play pill · next, frame -1 measured), the
// placed toolbar items, the long drop slot (frame 0 draws the live seek
// preview inside it), and the fixed right cluster (volume + 3 + more).
// Save persists the arrangement (doc 08 §4 UI-owned layout state; the
// localStorage seam the controls lane can consume via toolbarLayout.ts),
// Cancel reverts.
//
// Data-driven composition (doc 08 §4) and truthfulness (§5) are
// unchanged: engine-bound tiles dispatch their commands and render only
// when the engine lists them (capabilities.ts); UI-owned toolbar items
// always render; cycle tiles (Playback speed, Repeat, Zoom) dispatch the
// next option from ENGINE-CONFIRMED state only — never optimistic.

import { useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { useClient } from "../../../client/ClientProvider";
import type {
  CommandRequest,
  CommandResult,
  CommandTransport,
  Event,
} from "../../../client/transport";
import { Icon } from "../../../components/icon/Icon";
import { IconButton } from "../../../components/button/IconButton";
import { Button } from "../../../components/button";
import { Dialog } from "../../../components/surface/Surface";
import type { SurfaceCloseReason } from "../../../components/surface/Surface";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as a17Keys } from "../../../i18n/keys/w26.a17";
import { keys as gfixAKeys } from "../../../i18n/keys/w26.gfixA";
import { useCommandCapabilities, visibleTiles } from "./capabilities";
import { CUSTOMIZE_TILES, cycleArgs, tileById } from "./panels";
import {
  loadToolbarLayout,
  persistToolbarLayout,
  toolbarLayoutEqual,
} from "./toolbarLayout";
import type {
  CommandArgs,
  CustomizeTile,
  ToolbarLayout,
} from "./types";
import "./customize.css";

/** dataTransfer type carrying a catalog tile id out of the box. */
const TILE_DRAG_TYPE = "application/lumen-tile";
/** dataTransfer type carrying a placed band item id. */
const BAND_DRAG_TYPE = "application/lumen-band-item";

/** Structural equality for plain option-args payloads. */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  const ka = Object.keys(a as Record<string, unknown>);
  const kb = Object.keys(b as Record<string, unknown>);
  if (ka.length !== kb.length) return false;
  return ka.every((key) =>
    deepEqual((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]),
  );
}

/** The cycle tiles' command → args-payload builders (completion mapping). */
const CYCLE_PAYLOADS: ReadonlyArray<{
  readonly command: string;
  readonly payload: (result: CommandResult) => CommandArgs | null;
}> = [
  { command: "playlist.set-repeat", payload: (r) => (r.kind === "applied-repeat" ? { mode: r.mode } : null) },
  { command: "playback.set-rate", payload: (r) => (r.kind === "applied-rate" ? { rate: r.rate } : null) },
  { command: "video.set-zoom", payload: (r) => (r.kind === "applied-zoom" ? { factor: r.factor } : null) },
];

/** Apply a cycle payload to the matching cycle tile (engine truth only). */
function applyCyclePayload(
  command: string,
  payload: CommandArgs,
  setCycleSelections: React.Dispatch<React.SetStateAction<Record<string, string>>>,
): void {
  const tile = CUSTOMIZE_TILES.find(
    (candidate) => candidate.command === command && candidate.control.kind === "cycle",
  );
  if (tile === undefined || tile.control.kind !== "cycle") return;
  const option = tile.control.options.find((candidate) => deepEqual(candidate.args, payload));
  if (option !== undefined) {
    setCycleSelections((prev) => ({ ...prev, [tile.id]: option.id }));
  }
}

/**
 * Seam-first label resolver: central catalog first, then the lane's
 * pre-registration keys (w26.a17 + w26.gfixA). After the Lead registers
 * the keys into messages.ts the first branch resolves them; until then
 * the local fallback keeps the copy total.
 */
function useCustomizeMsg() {
  const m = useMsg();
  return useCallback(
    (key: CustomizeTile["labelKey"] | MessageKey): string =>
      m(key as MessageKey) ??
      a17Keys[key as keyof typeof a17Keys] ??
      gfixAKeys[key as keyof typeof gfixAKeys] ??
      key,
    [m],
  );
}

export interface CustomizePanelsProps {
  /** Transport override (tests); defaults to the client context. */
  readonly transport?: CommandTransport;
  /**
   * Close affordance (the header ✕ + Escape). When the host provides a
   * callback the dialog calls it; otherwise it emits the
   * `lumen:close-panel` seam event (detail `{ panel: "customize" }`).
   */
  readonly onClose?: () => void;
  /** Whether the modal is mounted (the per-open pattern mounts fresh). */
  readonly open?: boolean;
}

/**
 * The "Customize Player" surface: the frame's centered modal — the static
 * 31-tile catalog box, the Cancel/Save footer, and the control band's
 * dashed edit-well preview while open. Mount per-open (fresh arrangement
 * state from persistence each time).
 */
export function CustomizePanels({ transport, onClose, open = true }: CustomizePanelsProps) {
  const client = useClient();
  const wire = transport ?? client;
  const m = useMsg();
  const t = useCustomizeMsg();
  const caps = useCommandCapabilities(wire);
  const titleId = useId();

  // ---- engine-confirmed tile state (doc 08 §5 — never optimistic) --------
  const [engineBools, setEngineBools] = useState<Record<string, boolean | null>>({});
  const [cycleSelections, setCycleSelections] = useState<Record<string, string>>({});
  const nextRequestId = useRef(0);

  /** Apply a command completion to the tile state (engine truth only). */
  const applyCompletion = useCallback((result: CommandResult): void => {
    if (result.kind === "applied-bool") {
      // Only `playlist.set-shuffle` dispatches a bool from this surface.
      const tile = CUSTOMIZE_TILES.find(
        (candidate) =>
          candidate.command === "playlist.set-shuffle" &&
          candidate.control.kind === "toggle-action",
      );
      if (tile !== undefined) {
        setEngineBools((prev) => ({ ...prev, [tile.id]: result.value }));
      }
      return;
    }
    for (const entry of CYCLE_PAYLOADS) {
      const payload = entry.payload(result);
      if (payload !== null) {
        applyCyclePayload(entry.command, payload, setCycleSelections);
        return;
      }
    }
  }, []);

  const dispatch = useCallback(
    async (tile: CustomizeTile, args: CommandArgs): Promise<void> => {
      if (tile.command === null) return;
      nextRequestId.current += 1;
      // Forward-declared commands (PiP, HDR) are not yet in the 1.D4 wire
      // union — the cast is the bridge until `cargo xtask codegen` regrows
      // the catalog (recorded in ui/NEXT-STEPS.md).
      const request = {
        cmd: tile.command,
        args,
        requestId: nextRequestId.current,
      } as unknown as CommandRequest;
      try {
        const response = await wire.send(request);
        if ("completed" in response.result) {
          applyCompletion(response.result.completed);
        }
        // Errors (feature-not-exposed etc.) leave tile state untouched —
        // honest no-change, no fabricated success.
      } catch {
        // Transport failure — same honest no-change policy.
      }
    },
    [applyCompletion, wire],
  );

  // Engine events (doc 05 §3) — the other source of confirmed truth.
  useEffect(() => {
    const onEvent = (event: Event): void => {
      switch (event.kind) {
        case "playback.rate-changed": {
          applyCyclePayload("playback.set-rate", { rate: event.rate }, setCycleSelections);
          break;
        }
        case "playlist.changed": {
          const change = event.change;
          if (change.kind === "shuffle-set") {
            const tile = CUSTOMIZE_TILES.find(
              (candidate) => candidate.command === "playlist.set-shuffle",
            );
            if (tile !== undefined) {
              setEngineBools((prev) => ({ ...prev, [tile.id]: change.enabled }));
            }
          } else if (change.kind === "repeat-set") {
            applyCyclePayload("playlist.set-repeat", { mode: change.mode }, setCycleSelections);
          }
          break;
        }
        default:
          break;
      }
    };
    return wire.subscribe(onEvent);
  }, [wire]);

  // ---- the working arrangement (UI-owned layout state, doc 08 §4) -------
  const [layout, setLayout] = useState<ToolbarLayout>(() => loadToolbarLayout());
  const [saved, setSaved] = useState<ToolbarLayout>(() => loadToolbarLayout());
  const dirty = !toolbarLayoutEqual(layout, saved);

  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

  /** Capability-visible tiles in the working order (the box's contents). */
  const visibleIds = useMemo(() => {
    const visible = new Set(visibleTiles(CUSTOMIZE_TILES, caps).map((tile) => tile.id));
    return layout.order.filter((id) => visible.has(id));
  }, [caps, layout.order]);

  /** Reorder the catalog arrangement ("drag items around the box"). */
  const moveTile = useCallback(
    (dragId: string, targetId: string): void => {
      setLayout((prev) => {
        const order = [...prev.order];
        const from = order.indexOf(dragId);
        const to = order.indexOf(targetId);
        if (from < 0 || to < 0 || from === to) return prev;
        order.splice(to, 0, ...order.splice(from, 1));
        return { ...prev, order };
      });
    },
    [],
  );

  /** Place a catalog tile into the control band (frames -6/-7). */
  const addToBand = useCallback((id: string): void => {
    setLayout((prev) =>
      prev.band.includes(id) ? prev : { ...prev, band: [...prev.band, id] },
    );
  }, []);

  /** Remove a placed item from the band ("drag items into the box"). */
  const removeFromBand = useCallback((id: string): void => {
    setLayout((prev) =>
      prev.band.includes(id) ? { ...prev, band: prev.band.filter((band) => band !== id) } : prev,
    );
  }, []);

  /** Keyboard equivalents of the drag gestures (tiles are real buttons). */
  const onTileKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLButtonElement>, id: string): void => {
      if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        removeFromBand(id);
        return;
      }
      if (!event.altKey || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
      const index = visibleIds.indexOf(id);
      const target = event.key === "ArrowLeft" ? index - 1 : index + 1;
      const targetId = visibleIds[target];
      if (targetId !== undefined) {
        event.preventDefault();
        moveTile(id, targetId);
      }
    },
    [moveTile, removeFromBand, visibleIds],
  );

  const closePanel = useCallback((): void => {
    if (onClose !== undefined) onClose();
    else
      window.dispatchEvent(
        new CustomEvent("lumen:close-panel", { detail: { panel: "customize" } }),
      );
  }, [onClose]);

  const onDialogClose = useCallback(
    (reason: SurfaceCloseReason): void => {
      if (reason === "escape") closePanel();
      // Scrim clicks never close (the frames' modal floats inert); the ✕
      // and Escape are the close affordances.
    },
    [closePanel],
  );

  if (!open) return null;

  return (
    <>
      <Dialog
        open
        onClose={onDialogClose}
        labelledBy={titleId}
        tone="deep"
        className="lumen-customize__surface"
      >
        <div className="lumen-customize" data-resolved={caps.resolved}>
          {/* header band — frame: 94px #090909, title 15px semibold
              #8F8F8F at (28,23), ✕ 10px 1.5px #8F8F8F at (20,20), the
              instruction line inside the band at (23,~75). */}
          <header className="lumen-customize__header">
            <h3 id={titleId} className="lumen-customize__title">
              {t("customize.title")}
            </h3>
            <p className="lumen-customize__instruction">{t("customize.instruction")}</p>
            <IconButton
              label={t("customize.close")}
              size="sm"
              className="lumen-customize__close"
              onClick={closePanel}
            >
              <Icon name="close" size={10} />
            </IconButton>
          </header>

          {/* THE BOX — the frame's static 31-tile catalog: 9 columns,
              dashed 27×27 wells, 2-line labels. The whole box accepts
              drops (band items dropped here leave the toolbar; tile drops
              reorder). */}
          <div
            className="lumen-customize__box"
            role="list"
            aria-label={t("customize.box.label")}
            data-dragging={draggingId ?? undefined}
            onDragOver={(event) => {
              event.preventDefault();
            }}
            onDrop={(event) => {
              event.preventDefault();
              const bandId = event.dataTransfer.getData(BAND_DRAG_TYPE);
              if (bandId !== "") {
                removeFromBand(bandId);
              }
              setDraggingId(null);
              setDropTargetId(null);
            }}
          >
            {visibleIds.map((id) => {
              const tile = tileById(id);
              if (tile === null) return null;
              return (
                <div key={id} role="listitem" className="lumen-customize__tile-slot">
                  <button
                    type="button"
                    className="lumen-customize__tile"
                    data-field-id={id}
                    data-in-toolbar={layout.band.includes(id) || undefined}
                    data-dragging={draggingId === id || undefined}
                    data-drop-target={dropTargetId === id || undefined}
                    aria-pressed={
                      tile.control.kind === "toggle-action"
                        ? (engineBools[tile.id] ?? undefined)
                        : undefined
                    }
                    title={
                      tile.control.kind === "ui" ? t("customize.band.drop-hint") : undefined
                    }
                    draggable
                    onDragStart={(event) => {
                      event.dataTransfer.effectAllowed = "move";
                      event.dataTransfer.setData(TILE_DRAG_TYPE, id);
                      setDraggingId(id);
                    }}
                    onDragEnd={() => {
                      setDraggingId(null);
                      setDropTargetId(null);
                    }}
                    onDragOver={(event) => {
                      if (draggingId === null || draggingId === id) return;
                      event.preventDefault();
                      event.stopPropagation();
                      setDropTargetId(id);
                    }}
                    onDrop={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      const bandId = event.dataTransfer.getData(BAND_DRAG_TYPE);
                      if (bandId !== "") {
                        removeFromBand(bandId);
                      } else if (draggingId !== null && draggingId !== id) {
                        moveTile(draggingId, id);
                      }
                      setDraggingId(null);
                      setDropTargetId(null);
                    }}
                    onKeyDown={(event) => onTileKeyDown(event, id)}
                    onClick={() => {
                      if (tile.control.kind === "action") {
                        void dispatch(tile, tile.control.args);
                      } else if (tile.control.kind === "toggle-action") {
                        void dispatch(tile, tile.control.argsFor(engineBools[tile.id] ?? null));
                      } else if (tile.control.kind === "cycle") {
                        void dispatch(
                          tile,
                          cycleArgs(tile.control, cycleSelections[tile.id] ?? null),
                        );
                      }
                      // kind "ui": the palette item is inert by design.
                    }}
                  >
                    <span className="lumen-customize__tile-well">
                      {tile.icon ? <Icon name={tile.icon} size={20} /> : null}
                    </span>
                    <span className="lumen-customize__tile-label">{t(tile.labelKey)}</span>
                  </button>
                </div>
              );
            })}
          </div>

          {/* footer — frame: Cancel 100×32 r6 #1B1B1B/#8F8F8F left, Save
              100×32 r6 #371F06/#D37918 right, Save dimmed while clean. */}
          <footer className="lumen-customize__footer">
            <Button
              className="lumen-customize__cancel"
              onClick={() => {
                // Discard the session's edits — back to the saved baseline.
                setLayout(saved);
              }}
            >
              {m("open.cancel")}
            </Button>
            <Button
              variant="tinted"
              className="lumen-customize__save"
              disabled={!dirty}
              onClick={() => {
                // Keep the arrangement as the new baseline + persist it
                // (doc 08 §4 UI-owned layout state).
                setSaved(layout);
                persistToolbarLayout(layout);
              }}
            >
              {t("customize.save")}
            </Button>
          </footer>
        </div>
      </Dialog>

      {/* The control band's edit preview (frames -1..-4/-6/-7): dashed
          wells over the live strip while the modal is open. Placed
          toolbar items render as draggable wells; the band accepts tile
          drops (place) and its wells drag back into the box (remove). */}
      <CustomizeBand
        layout={layout}
        dragging={draggingId !== null}
        onDropTile={addToBand}
        onRemoveFromBand={removeFromBand}
      />
    </>
  );
}

/** The live control band's measured geometry (null → CSS fallback). */
interface BandGeometry {
  readonly top: number;
  readonly left: number;
  readonly width: number;
  readonly height: number;
}

/** Measure the real control band (shell strip, else the in-place footer). */
function useBandGeometry(): BandGeometry | null {
  const [geometry, setGeometry] = useState<BandGeometry | null>(null);

  useLayoutEffect(() => {
    const measure = (): void => {
      const band =
        document.querySelector(".app-shell__strip") ??
        document.querySelector(".lumen-player__controls");
      if (band === null) {
        setGeometry(null);
        return;
      }
      const rect = band.getBoundingClientRect();
      // jsdom reports 0×0 rects — keep the CSS fallback there.
      if (rect.width === 0 || rect.height === 0) {
        setGeometry(null);
        return;
      }
      setGeometry({ top: rect.top, left: rect.left, width: rect.width, height: rect.height });
    };
    measure();
    const band =
      document.querySelector(".app-shell__strip") ??
      document.querySelector(".lumen-player__controls");
    // ResizeObserver is guarded (jsdom carries none — the CSS fallback
    // covers zero-rect hosts there).
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    if (observer !== null && band !== null) observer.observe(band);
    window.addEventListener("resize", measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);

  return geometry;
}

/**
 * The control band's dashed edit-well preview (frames -1..-4): the placed
 * toolbar items as draggable wells beside the fixed transport cluster,
 * the frame's long empty drop slot between them and the right cluster,
 * and the fixed volume/effect/subtitle/PiP/more wells. Fixed wells are
 * the preview chrome of controls this lane does not own (aria-hidden);
 * placed wells are real buttons (click = remove — the keyboard-safe path
 * back into the box).
 */
function CustomizeBand({
  layout,
  dragging,
  onDropTile,
  onRemoveFromBand,
}: {
  readonly layout: ToolbarLayout;
  readonly dragging: boolean;
  readonly onDropTile: (id: string) => void;
  readonly onRemoveFromBand: (id: string) => void;
}) {
  const t = useCustomizeMsg();
  const geometry = useBandGeometry();
  const [dropActive, setDropActive] = useState(false);

  const placed = layout.band
    .map((id) => tileById(id))
    .filter((tile): tile is CustomizeTile => tile !== null);

  return (
    <div
      className="lumen-customize-band"
      data-dragging={dragging || undefined}
      data-drop-active={dropActive || undefined}
      role="group"
      aria-label={t("customize.band.label")}
      style={
        geometry === null
          ? undefined
          : {
              top: `${geometry.top}px`,
              left: `${geometry.left}px`,
              width: `${geometry.width}px`,
              height: `${geometry.height}px`,
            }
      }
      onDragOver={(event) => {
        if (!dragging) return;
        event.preventDefault();
        setDropActive(true);
      }}
      onDragLeave={() => {
        setDropActive(false);
      }}
      onDrop={(event) => {
        event.preventDefault();
        setDropActive(false);
        const id = event.dataTransfer.getData(TILE_DRAG_TYPE);
        if (id !== "") onDropTile(id);
      }}
    >
      <div className="lumen-customize-band__row">
        {/* Fixed transport preview (frame -1, measured: prev well 29×27
            @20.5 · play well 53×33 @58.5 wrapping the 48×28 r6 accent pill
            · next well 29×27 @120.5 — pitch 38/62) — the real controls are
            the controls lane's. */}
        <span className="lumen-customize-band__cluster" aria-hidden="true">
          <span className="lumen-customize-band__well">
            <Icon name="previous" size={18} />
          </span>
          <span className="lumen-customize-band__well lumen-customize-band__well--play">
            <Icon name="play" size={18} />
          </span>
          <span className="lumen-customize-band__well">
            <Icon name="next" size={18} />
          </span>
        </span>

        {/* Placed toolbar items — real buttons (click = remove, drag = back
         * to the box) riding where frame -6 shows the dropped tile: after
         * the left cluster, shrinking the long slot. */}
        {placed.map((tile) => (
          <button
            key={tile.id}
            type="button"
            className="lumen-customize-band__well"
            data-field-id={tile.id}
            aria-label={t("customize.band.remove").replace("{item}", t(tile.labelKey))}
            draggable
            onDragStart={(event) => {
              event.dataTransfer.effectAllowed = "move";
              event.dataTransfer.setData(BAND_DRAG_TYPE, tile.id);
            }}
            onClick={() => onRemoveFromBand(tile.id)}
          >
            {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
          </button>
        ))}

        {/* The frame's long empty drop slot (frame 0: 1405×35 dashed @170.5)
         * — THE drop target (drop a catalog tile on the band to place it in
         * the toolbar). Frame 0 draws the live seekbar preview INSIDE it
         * (track 1238×28 r6 #090909, buffered #371F06, played #D37918);
         * frames -1..-4 show it empty — the seek preview rides the clean
         * state, drops tint over it. */}
        <span className="lumen-customize-band__slot" aria-label={t("customize.band.drop-hint")}>
          <span className="lumen-customize-band__seek" aria-hidden="true">
            <span className="lumen-customize-band__seek-buffered" />
            <span className="lumen-customize-band__seek-played" />
          </span>
        </span>

        {/* Fixed right cluster preview (frame -1, measured: volume well
            129×35 @1591.5 · three 29×31 wells @1736.5/1778.5/1820.5 · more
            29×31 @1870.5). */}
        <span className="lumen-customize-band__well lumen-customize-band__well--wide" aria-hidden="true">
          <Icon name="volume-medium" size={18} />
          <span className="lumen-customize-band__volume-track" />
        </span>
        <span className="lumen-customize-band__well lumen-customize-band__well--tall" aria-hidden="true">
          <Icon name="equalizer" size={18} />
        </span>
        <span className="lumen-customize-band__well lumen-customize-band__well--tall" aria-hidden="true">
          <Icon name="subtitles" size={18} />
        </span>
        <span className="lumen-customize-band__well lumen-customize-band__well--tall" aria-hidden="true">
          <Icon name="pip" size={18} />
        </span>
        <span className="lumen-customize-band__well lumen-customize-band__well--tall" aria-hidden="true">
          <Icon name="more" size={18} />
        </span>
      </div>
    </div>
  );
}
