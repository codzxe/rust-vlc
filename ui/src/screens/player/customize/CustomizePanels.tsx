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
// unchanged: engine-bound tiles dispatch their commands honestly — the
// box renders the frame's FULL static catalog regardless of the listing
// (the h07-defect-4 ruling; never a dead control, never a fabricated
// one); cycle tiles (Playback speed, Repeat, Zoom) dispatch the next option
// from ENGINE-CONFIRMED state only — never optimistic. The cycle mapping
// is shared with the real bar (toolbarItems.tsx) through cycleState.ts
// (w26.k08, j08 P2-3).

import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
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
// w26.k08 (j08 P2-3): the cycle mapping shared with the real bar
// (toolbarItems.tsx) — the local deepEqual/CYCLE_PAYLOADS/applyCyclePayload
// trio moved there so both consumers read one implementation.
import { applyCycleCompletion, applyCycleEvent } from "./cycleState";
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
  const titleId = useId();
  // The header ✕ — Save's focus-recovery target (h07 defect 2): an
  // always-enabled control inside the dialog.
  const closeRef = useRef<HTMLButtonElement | null>(null);

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
    applyCycleCompletion(result, setCycleSelections);
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

  // Engine events (doc 05 §3) — the other source of confirmed truth. The
  // shuffle-set engine-bool half is this surface's own; the cycle-relevant
  // events (rate, repeat) map through the shared cycleState module
  // (w26.k08, j08 P2-3 — same mapper the real bar subscribes to).
  useEffect(() => {
    const onEvent = (event: Event): void => {
      if (event.kind !== "playlist.changed") {
        applyCycleEvent(event, setCycleSelections);
        return;
      }
      const change = event.change;
      if (change.kind === "shuffle-set") {
        const tile = CUSTOMIZE_TILES.find(
          (candidate) => candidate.command === "playlist.set-shuffle",
        );
        if (tile !== undefined) {
          setEngineBools((prev) => ({ ...prev, [tile.id]: change.enabled }));
        }
        return;
      }
      applyCycleEvent(event, setCycleSelections);
    };
    return wire.subscribe(onEvent);
  }, [wire]);

  // ---- the working arrangement (UI-owned layout state, doc 08 §4) -------
  const [layout, setLayout] = useState<ToolbarLayout>(() => loadToolbarLayout());
  const [saved, setSaved] = useState<ToolbarLayout>(() => loadToolbarLayout());
  const dirty = !toolbarLayoutEqual(layout, saved);

  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

  // The dragged tile (box tile OR band item) — drives the frame-6 ghost.
  const dragTile = draggingId === null ? null : tileById(draggingId);

  /**
   * The box's contents (w26.i05, h07 defect 4): the frame's FULL 31-tile
   * catalog in the working order — `Player - customize.svg`'s box is a
   * static catalog (byte-identical across all 9 frames: rows 1–4 = 9/9/9/4
   * at 9 columns), never a capability report. Engine truth still governs
   * dispatch and aria-pressed (honest no-change on feature-not-exposed).
   */
  const boxIds = layout.order;

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
      const index = boxIds.indexOf(id);
      const target = event.key === "ArrowLeft" ? index - 1 : index + 1;
      const targetId = boxIds[target];
      if (targetId !== undefined) {
        event.preventDefault();
        moveTile(id, targetId);
      }
    },
    [boxIds, moveTile, removeFromBand],
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

  // w26.i05 (h07 defect 2): focus recovery. A control that disables under
  // the pointer (Save) makes Chromium blur to BODY — the in-surface
  // keydown/Tab trap then never fires (its listener lives on the dialog
  // node), so Escape stopped closing and Tab escaped. The Save handler
  // moves focus synchronously BEFORE the disable commits; this keeper is
  // the belt-and-suspenders half: any focus that dies to BODY (or
  // escapes the dialog) while the modal is open is pulled back onto the
  // dialog surface, keeping Escape/Tab alive from any state.
  useEffect(() => {
    if (!open) return;
    const surface = (): HTMLElement | null =>
      document.querySelector<HTMLElement>(".lumen-customize__surface");
    const onFocusOut = (): void => {
      window.setTimeout(() => {
        const node = surface();
        if (node === null) return;
        const active = document.activeElement;
        if (active === document.body || active === document.documentElement) {
          node.focus(); // tabIndex=-1 on the Dialog primitive
        }
      }, 0);
    };
    document.addEventListener("focusout", onFocusOut);
    return () => document.removeEventListener("focusout", onFocusOut);
  }, [open]);

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
        <div className="lumen-customize">
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
              ref={closeRef}
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
            {boxIds.map((id) => {
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
                      // w26.i05 (h07 defect 2, frame -6): the drag image is
                      // the STYLED icon-only 27×27 well (the frame's ghost
                      // grammar), not the native full-button snapshot — the
                      // platform ghost dies here, the pinned preview ghost
                      // renders in the band (CustomizeBand, frame -6).
                      const well = event.currentTarget.querySelector<HTMLElement>(
                        ".lumen-customize__tile-well",
                      );
                      if (well !== null) event.dataTransfer.setDragImage?.(well, 14, 14);
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
                // (doc 08 §4 UI-owned layout state; the real bar's items
                // re-read it through the lumen:toolbar-changed seam).
                setSaved(layout);
                persistToolbarLayout(layout);
                // w26.i05 (h07 defect 2): Save disabling under the pointer
                // blurs Chromium to BODY and kills Escape/Tab (the trap
                // listens on the dialog node). Move focus to the ✕ — an
                // always-enabled control INSIDE the dialog — BEFORE the
                // disable commits, so focus never leaves the surface.
                closeRef.current?.focus();
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
          drops (place) and its wells drag back into the box (remove).
          w26.i05 (h07 defect 3): the band composes per the g07/g09 frame
          maps — the seek row promoted above the transport row while
          dirty (frames -1..-6), the transport centered between split
          slots by placed count (frames -2..-4), and the frame-6 ghost
          pinned band-left while a drag is live. */}
      <CustomizeBand
        layout={layout}
        dirty={dirty}
        dragging={draggingId !== null}
        dragTile={dragTile}
        onDropTile={addToBand}
        onRemoveFromBand={removeFromBand}
        onItemDragStart={setDraggingId}
        onItemDragEnd={() => {
          setDraggingId(null);
          setDropTargetId(null);
        }}
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
 * The transport-row composition per edit state — the g07/g09 frame maps
 * (w26.i05, h07 defect 3). Six frames, one overlay:
 *
 *   f0  clean — transport left; ONE long slot carrying the live seek
 *       preview + the band timecodes (frame 0: slot 1405×35 @170.5 with
 *       the 1238×28 track inside; timecodes 00:02:10 | 00:25:40).
 *   f1  dirty · 0 placed — the seek promoted to a full-width row ABOVE
 *       the transport row, the transport slot left EMPTY (frame -1:
 *       1879×35 @20.5,981.5 + empty slot 1431×35 @155.5).
 *   f2  dirty · 1 placed — transport CENTERED between split slots
 *       866/551 (frame -2: cluster 895.5/933.5/995.5).
 *   f3  dirty · 2 placed — a 130×26 well at the left edge narrows the
 *       left slot to 729 (frame -3).
 *   f4  dirty · ≥3 placed — the topbar well run 27/27÷23/60×27 + 130×26
 *       before the left slot 563 (frame -4).
 *   f5  dragging — frames -5/-6 merged: the ghost pinned band-left before
 *       the left slot 526, the right cluster shifted 51px left with the
 *       35×23 far-right well (frame -5), and the lifted tile's box slot
 *       vacated (frame -6).
 */
type BandVariant = "f0" | "f1" | "f2" | "f3" | "f4" | "f5";

/** Derive the frame variant from the edit state (see {@link BandVariant}). */
function bandVariant(dirty: boolean, placedCount: number, dragging: boolean): BandVariant {
  if (dragging) return "f5";
  if (!dirty) return "f0";
  if (placedCount <= 0) return "f1";
  if (placedCount === 1) return "f2";
  if (placedCount === 2) return "f3";
  return "f4";
}

/**
 * Split-slot flex ratios per frame (frame @1920 widths; the row is fluid,
 * so the ratios reproduce each frame's proportions — the same
 * frame-pinning approach as the modal's 1:1 sizes):
 *   f2 866/551 · f3 729/551 · f4 563/551 · f5 526/499 (frames -2..-6).
 */
const SLOT_FLEX: Readonly<Record<"f2" | "f3" | "f4" | "f5", [number, number]>> = {
  f2: [866, 551],
  f3: [729, 551],
  f4: [563, 551],
  f5: [526, 499],
};

/**
 * The control band's dashed edit-well preview: the placed toolbar items as
 * draggable wells around the fixed transport cluster, the frame's drop
 * slots, and the fixed volume/effect/subtitle/PiP/more wells. Fixed wells
 * are the preview chrome of controls this lane does not own (aria-hidden);
 * placed wells are real buttons (click = remove — the keyboard-safe path
 * back into the box). The seek row (frames -1..-6) rides ABOVE the real
 * strip: the frames' bottom chrome is two frosted strips (bgblur 972..1026
 * + 1026..1080 @1920) with the seek well in the upper one — the overlay
 * extends 54px above the measured strip for it.
 */
function CustomizeBand({
  layout,
  dirty,
  dragging,
  dragTile,
  onDropTile,
  onRemoveFromBand,
  onItemDragStart,
  onItemDragEnd,
}: {
  readonly layout: ToolbarLayout;
  readonly dirty: boolean;
  readonly dragging: boolean;
  readonly dragTile: CustomizeTile | null;
  readonly onDropTile: (id: string) => void;
  readonly onRemoveFromBand: (id: string) => void;
  readonly onItemDragStart: (id: string) => void;
  readonly onItemDragEnd: () => void;
}) {
  const t = useCustomizeMsg();
  const geometry = useBandGeometry();
  const [dropActive, setDropActive] = useState(false);

  const placed = layout.band
    .map((id) => tileById(id))
    .filter((tile): tile is CustomizeTile => tile !== null);

  const variant = bandVariant(dirty, layout.band.length, dragging);
  const centered = variant !== "f0" && variant !== "f1";
  const split = variant === "f2" || variant === "f3" || variant === "f4" || variant === "f5";
  const [leftFlex, rightFlex] = split ? SLOT_FLEX[variant] : [1, 1];

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
      {/* The promoted seek row (frames -1..-6): a full-width dashed well
          above the transport row — the frames' upper chrome strip. All of
          it is decorative preview chrome (aria-hidden): the real seekbar
          and timecodes belong to the control bar underneath. Values are
          frame-pinned (Current Time.svg / Duration.svg read 00:02:10 |
          00:25:40; track 1712×28 with buffered #371F06 and the 8×24
          accent playhead — `Player - customize-1.svg`). */}
      {variant !== "f0" ? (
        <div className="lumen-customize-band__seekrow" aria-hidden="true">
          <span className="lumen-customize-band__time">00:02:10</span>
          <span className="lumen-customize-band__seek lumen-customize-band__seek--row">
            <span className="lumen-customize-band__seek-buffered" />
            <span className="lumen-customize-band__seek-played" />
          </span>
          <span className="lumen-customize-band__time">00:25:40</span>
        </div>
      ) : null}

      <div className="lumen-customize-band__row">
        {/* f3's left-edge 130×26 well (frame -3: @20.5, before the 729
            slot). */}
        {variant === "f3" ? (
          <span className="lumen-customize-band__well lumen-customize-band__well--flat" aria-hidden="true" />
        ) : null}

        {/* f4/f5's topbar well run (frame -4: 27 @21.5 · 27 @59.5 wrapping
            the 2×12 divider mark · 23 @87.5 · 60 @111.5 · 130×26 @184.5 —
            the title bar's icons edited into the band). */}
        {variant === "f4" || variant === "f5" ? (
          <span className="lumen-customize-band__topbarrun" aria-hidden="true">
            <span className="lumen-customize-band__well lumen-customize-band__well--27" />
            <span className="lumen-customize-band__well lumen-customize-band__well--27">
              <span className="lumen-customize-band__well-mark" />
            </span>
            <span className="lumen-customize-band__well lumen-customize-band__well--23" />
            <span className="lumen-customize-band__well lumen-customize-band__well--60" />
            <span className="lumen-customize-band__well lumen-customize-band__well--flat" />
          </span>
        ) : null}

        {/* The frame-6 drag ghost: icon-only 27×27, pinned band-left just
            before the (shrunk) left slot, z-top — outside the modal's
            x-range, the frame's "dropped-state preview at the band's left
            edge" (h07 state 6). The OS drag image is the same styled well
            (setDragImage in the box tile's dragstart). */}
        {variant === "f5" && dragTile !== null ? (
          <span className="lumen-customize-band__well lumen-customize-band__ghost" aria-hidden="true">
            {dragTile.icon ? <Icon name={dragTile.icon} size={18} /> : null}
          </span>
        ) : null}

        {/* The left split slot (frames -2..-6) — preview chrome like the
            clusters (j08 P3-8/P3-9: the old aria-label on a generic span
            was an AT no-op; decorative slots stay out of the tree). */}
        {split ? (
          <span
            className="lumen-customize-band__slot lumen-customize-band__slot--left"
            style={{ flexGrow: leftFlex, flexBasis: 0 }}
            aria-hidden="true"
          />
        ) : null}

        {/* Fixed transport preview (frame -1, measured: prev well 29×27
            @20.5 · play well 53×33 @58.5 wrapping the 48×28 r6 accent pill
            · next well 29×27 @120.5 — pitch 38/62; frames -2..-6 center the
            cluster at 960). The real controls are the controls lane's. */}
        <span className="lumen-customize-band__cluster" data-centered={centered || undefined} aria-hidden="true">
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
         * to the box) riding after the cluster (frame -6's placement
         * grammar, h07-blessed), shrinking the trailing slot. A lifted
         * band item vacates its well (frame -6's mid-drag lift). */}
        {placed.map((tile) => (
          <button
            key={tile.id}
            type="button"
            className="lumen-customize-band__well"
            data-field-id={tile.id}
            data-dragging={dragTile?.id === tile.id || undefined}
            aria-label={t("customize.band.remove").replace("{item}", t(tile.labelKey))}
            draggable
            onDragStart={(event) => {
              event.dataTransfer.effectAllowed = "move";
              event.dataTransfer.setData(BAND_DRAG_TYPE, tile.id);
              const well = event.currentTarget;
              event.dataTransfer.setDragImage?.(well, 14, 14);
              onItemDragStart(tile.id);
            }}
            onDragEnd={onItemDragEnd}
            onClick={() => onRemoveFromBand(tile.id)}
          >
            {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
          </button>
        ))}

        {/* The trailing slot(s). f0 (frame 0: 1405×35 @170.5) carries the
         * live seek preview + the band timecodes; f1 (frame -1: 1431×35
         * @155.5) is left EMPTY — the seek moved to its own row above;
         * f2..f5 split (see SLOT_FLEX). All preview chrome (j08 P3-8:
         * aria-hidden, not aria-labelled spans). */}
        {variant === "f0" ? (
          <span className="lumen-customize-band__slot" aria-hidden="true">
            <span className="lumen-customize-band__time">00:02:10</span>
            <span className="lumen-customize-band__seek">
              <span className="lumen-customize-band__seek-buffered" />
              <span className="lumen-customize-band__seek-played" />
            </span>
            <span className="lumen-customize-band__time">00:25:40</span>
          </span>
        ) : variant === "f1" ? (
          <span className="lumen-customize-band__slot" aria-hidden="true" />
        ) : (
          <span
            className="lumen-customize-band__slot lumen-customize-band__slot--right"
            style={{ flexGrow: rightFlex, flexBasis: 0 }}
            aria-hidden="true"
          />
        )}

        {/* Fixed right cluster preview (frame -1, measured: volume well
            129×35 @1591.5 · three 29×31 wells @1736.5/1778.5/1820.5 · more
            29×31 @1870.5; frame -5 shifts the whole run 51px left). */}
        <span className="lumen-customize-band__right" data-shifted={variant === "f5" || undefined} aria-hidden="true">
          <span className="lumen-customize-band__well lumen-customize-band__well--wide">
            <Icon name="volume-medium" size={18} />
            <span className="lumen-customize-band__volume-track" />
          </span>
          <span className="lumen-customize-band__well lumen-customize-band__well--tall">
            <Icon name="equalizer" size={18} />
          </span>
          <span className="lumen-customize-band__well lumen-customize-band__well--tall">
            <Icon name="subtitles" size={18} />
          </span>
          <span className="lumen-customize-band__well lumen-customize-band__well--tall">
            <Icon name="pip" size={18} />
          </span>
          <span className="lumen-customize-band__well lumen-customize-band__well--tall">
            <Icon name="more" size={18} />
          </span>
        </span>

        {/* f5's far-right 35×23 well (frame -5: @1864.5, the window-edge
            anchor after the 51px cluster shift). */}
        {variant === "f5" ? (
          <span className="lumen-customize-band__well lumen-customize-band__well--low" aria-hidden="true" />
        ) : null}
      </div>
    </div>
  );
}
