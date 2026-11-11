// Card 4.F3 / w26.a17 — the customize surface renderer, rebuilt to the
// 9-state frame set `design/figma/Player - customize*.svg` (W25 spec §2):
//
//   header strip  "Customize Player" + ✕ close        (frame: #090909 band)
//   instruction   the drag-and-drop helper line        (frame: 12px tertiary)
//   THE BOX       dashed-tile toolbar palette          (frame rows 1–4)
//   sections      per-panel adjustment rows            (Figma row grammar)
//   footer        Cancel (left) · Save (right, r6)     (frame: 32px, r6)
//
// Data-driven composition (doc 08 §4 note) is UNCHANGED: panels/fields come
// from ./panels.ts, each field bound to a command id; a field renders ONLY
// when the engine's `system.list-commands` listing includes that command
// (./capabilities.ts). Quick-action fields (action / toggle-action) render
// as palette tiles; select/slider fields render as adjustment rows. All
// engine communication goes through the command-layer client seam.
//
// Truthfulness (doc 08 §5): select selections and slider committed values
// update ONLY from engine confirmations (command completions / events) —
// live slider drags are visual previews that never dispatch; onCommit is
// the only dispatch point. Save/Cancel manage the LOCAL toolbar arrangement
// (doc 08 §4 layout state — a UI-owned concern, no engine command exists);
// dragging a tile out of the box removes it (frame "…-6"), Cancel restores.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
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
import { Dropdown } from "../../../components/menu/Dropdown";
import { Slider } from "../../../components/slider";
import { ValueChip } from "../../../components/form/ValueChip";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.a17";
import {
  panelVisible,
  useCommandCapabilities,
  visibleFields,
} from "./capabilities";
import { CUSTOMIZE_FIELDS, CUSTOMIZE_PANELS } from "./panels";
import type {
  CommandArgs,
  CustomizeField,
  CustomizeLabelKey,
  CustomizePanelDef,
} from "./types";
import "./customize.css";

/** Payload a select-field completion echoes, keyed like the option args. */
function selectPayload(result: CommandResult): CommandArgs | null {
  switch (result.kind) {
    case "applied-aspect-ratio":
      return { ratio: result.ratio };
    case "applied-crop":
      return { crop: result.crop };
    case "applied-repeat":
      return { mode: result.mode };
    default:
      return null;
  }
}

/** Numeric completion → { command, value } for slider committed state. */
function numericCompletion(result: CommandResult): { command: string; value: number } | null {
  switch (result.kind) {
    case "applied-rate":
      return { command: "playback.set-rate", value: result.rate };
    case "applied-zoom":
      return { command: "video.set-zoom", value: result.factor };
    default:
      return null;
  }
}

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

const sliderFieldByCommand = (command: string): CustomizeField | undefined =>
  CUSTOMIZE_FIELDS.find(
    (field) => field.command === command && field.control.kind === "slider",
  );

/** Quick-action fields render as palette tiles (doc 08 §4 button catalog). */
function isTileField(field: CustomizeField): boolean {
  return field.control.kind === "action" || field.control.kind === "toggle-action";
}

/**
 * Seam-first label resolver: central catalog first, then this lane's
 * pre-registration keys (ui/src/i18n/keys/w26.a17.ts). After the Lead
 * registers the keys into messages.ts the first branch already resolves
 * them; until then the local fallback keeps the copy total.
 */
function useCustomizeMsg() {
  const m = useMsg();
  return useCallback(
    (key: CustomizeLabelKey): string =>
      m(key as MessageKey) ?? laneKeys[key as keyof typeof laneKeys] ?? key,
    [m],
  );
}

export interface CustomizePanelsProps {
  /** Transport override (tests); defaults to the client context. */
  readonly transport?: CommandTransport;
  /**
   * Close affordance (the header ✕). When the host provides a callback the
   * button calls it; otherwise it emits the `lumen:close-panel` seam event
   * (detail `{ panel: "customize" }`) the same way the shell's other panel
   * seams work — the player composition lane can adopt it without a re-prop.
   */
  readonly onClose?: () => void;
}

/**
 * The "Customize Player" surface: capability-gated toolbar palette +
 * adjustment rows composed to the 9-state frame set. Panels whose fields
 * are all unsupported contribute nothing; when NO panel is supported the
 * surface renders only an honest screen-reader note (the scaffold
 * MockTransport state).
 */
export function CustomizePanels({ transport, onClose }: CustomizePanelsProps) {
  const client = useClient();
  const wire = transport ?? client;
  const m = useMsg();
  const t = useCustomizeMsg();
  const caps = useCommandCapabilities(wire);

  // Engine-confirmed state — never optimistic (doc 08 §5).
  const [selected, setSelected] = useState<Record<string, string>>({});
  const [committed, setCommitted] = useState<Record<string, number>>({});
  const [live, setLive] = useState<Record<string, number>>({});
  const [engineBools, setEngineBools] = useState<Record<string, boolean | null>>({});
  const nextRequestId = useRef(0);

  /** Apply a command completion to the panel state (engine truth only). */
  const applyCompletion = useCallback((result: CommandResult): void => {
    const payload = selectPayload(result);
    if (payload !== null) {
      for (const field of CUSTOMIZE_FIELDS) {
        if (field.control.kind !== "select") continue;
        const match = field.control.options.find((option) => deepEqual(option.args, payload));
        if (match !== undefined) {
          setSelected((prev) => ({ ...prev, [field.id]: match.id }));
        }
      }
    }
    const numeric = numericCompletion(result);
    if (numeric !== null) {
      const field = sliderFieldByCommand(numeric.command);
      if (field !== undefined) {
        setCommitted((prev) => ({ ...prev, [field.id]: numeric.value }));
      }
    }
    if (result.kind === "applied-bool") {
      // Only `playlist.set-shuffle` dispatches from these panels.
      const field = CUSTOMIZE_FIELDS.find(
        (f) => f.command === "playlist.set-shuffle" && f.control.kind === "toggle-action",
      );
      if (field !== undefined) {
        setEngineBools((prev) => ({ ...prev, [field.id]: result.value }));
      }
    }
  }, []);

  const dispatch = useCallback(
    async (field: CustomizeField, args: CommandArgs): Promise<void> => {
      nextRequestId.current += 1;
      // Forward-declared commands (colors, HDR) are not yet in the 1.D4 wire
      // union — the cast is the bridge until `cargo xtask codegen` regrows
      // the catalog (recorded in ui/NEXT-STEPS.md).
      const request = {
        cmd: field.command,
        args,
        requestId: nextRequestId.current,
      } as unknown as CommandRequest;
      try {
        const response = await wire.send(request);
        if ("completed" in response.result) {
          applyCompletion(response.result.completed);
        }
        // Errors (feature-not-exposed etc.) leave panel state untouched —
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
          const field = sliderFieldByCommand("playback.set-rate");
          if (field !== undefined) {
            setCommitted((prev) => ({ ...prev, [field.id]: event.rate }));
          }
          break;
        }
        case "playlist.changed": {
          const change = event.change;
          if (change.kind === "shuffle-set") {
            const field = CUSTOMIZE_FIELDS.find((f) => f.command === "playlist.set-shuffle");
            if (field !== undefined) {
              setEngineBools((prev) => ({ ...prev, [field.id]: change.enabled }));
            }
          } else if (change.kind === "repeat-set") {
            const payload = { mode: change.mode };
            for (const f of CUSTOMIZE_FIELDS) {
              if (f.control.kind !== "select") continue;
              const match = f.control.options.find((o) => deepEqual(o.args, payload));
              if (match !== undefined) setSelected((prev) => ({ ...prev, [f.id]: match.id }));
            }
          }
          break;
        }
        default:
          break;
      }
    };
    return wire.subscribe(onEvent);
  }, [wire]);

  const panels = useMemo(
    () => CUSTOMIZE_PANELS.filter((panel) => panelVisible(panel, caps)),
    [caps],
  );

  // ---- the box: quick-action tiles + their arrangement -------------------
  const tileFields = useMemo(
    () => panels.flatMap((panel) => visibleFields(panel, caps).filter(isTileField)),
    [caps, panels],
  );
  const baseIds = useMemo(() => tileFields.map((field) => field.id), [tileFields]);
  /**
   * The box's working copy vs its committed baseline (doc 08 §4: toolbar
   * layout is UI-owned state). `order: null` = the catalog order. Save
   * snapshots the working copy; Cancel restores the baseline; dirty = they
   * differ (frame 0: Save dimmed — frames -1…-6: Save actionable).
   */
  interface BoxState {
    readonly order: readonly string[] | null;
    readonly removed: readonly string[];
  }
  const [box, setBox] = useState<BoxState>({ order: null, removed: [] });
  const [savedBox, setSavedBox] = useState<BoxState>({ order: null, removed: [] });
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);
  const dropHandled = useRef(false);

  /** Normalized VISIBLE arrangement (`null` order = catalog order) — the
   * dirty check compares what the box shows against the saved baseline. */
  const netKey = (state: BoxState): string =>
    (state.order ?? baseIds)
      .filter((id) => baseIds.includes(id) && !state.removed.includes(id))
      .join("\n");

  const shownIds = useMemo(() => {
    const ordered = (box.order ?? baseIds).filter(
      (id) => baseIds.includes(id) && !box.removed.includes(id),
    );
    // Fields that became available after an edit append at the end — a
    // capability arriving mid-session never hides its tile.
    const appended = baseIds.filter(
      (id) => !ordered.includes(id) && !box.removed.includes(id),
    );
    return [...ordered, ...appended];
  }, [baseIds, box]);
  const shownFields = useMemo(
    () => shownIds.map((id) => tileFields.find((field) => field.id === id)),
    [shownIds, tileFields],
  );
  const dirty = netKey(box) !== netKey(savedBox);

  /** Drag-to-reorder: move `dragId` onto `targetId`'s slot (frame -1…-5). */
  const moveTile = useCallback(
    (dragId: string, targetId: string): void => {
      const order = [...shownIds];
      const from = order.indexOf(dragId);
      const to = order.indexOf(targetId);
      if (from < 0 || to < 0 || from === to) return;
      order.splice(to, 0, ...order.splice(from, 1));
      setBox((prev) => ({ ...prev, order }));
    },
    [shownIds],
  );

  /** Drag out of the box / Delete key = remove from the player (frame -6). */
  const removeTile = useCallback((id: string): void => {
    setBox((prev) =>
      prev.removed.includes(id) ? prev : { ...prev, removed: [...prev.removed, id] },
    );
  }, []);

  /** Keyboard equivalent of the drag gestures (tiles are real buttons). */
  const onTileKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLButtonElement>, id: string): void => {
      if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        removeTile(id);
        return;
      }
      if (!event.altKey || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
      const index = shownIds.indexOf(id);
      const target = event.key === "ArrowLeft" ? index - 1 : index + 1;
      const targetId = shownIds[target];
      if (targetId !== undefined) {
        event.preventDefault();
        moveTile(id, targetId);
      }
    },
    [moveTile, removeTile, shownIds],
  );

  // ---- the sections: select/slider adjustment rows -----------------------
  const sectionPanels = useMemo(
    () =>
      panels
        .map((panel) => ({
          panel,
          fields: visibleFields(panel, caps).filter((field) => !isTileField(field)),
        }))
        .filter((entry) => entry.fields.length > 0),
    [caps, panels],
  );

  if (panels.length === 0) {
    // All fields unsupported → no panels (no dead controls). Honest note.
    return <p className="lumen-customize__empty">{m("customize.none-available")}</p>;
  }

  const closePanel = (): void => {
    if (onClose !== undefined) onClose();
    else window.dispatchEvent(new CustomEvent("lumen:close-panel", { detail: { panel: "customize" } }));
  };

  return (
    <div className="lumen-customize" data-resolved={caps.resolved}>
      <header className="lumen-customize__header">
        <h3 className="lumen-customize__title">{t("customize.title")}</h3>
        <IconButton
          label={t("customize.close")}
          size="sm"
          className="lumen-customize__close"
          onClick={closePanel}
        >
          <Icon name="close" size={16} />
        </IconButton>
      </header>
      <p className="lumen-customize__instruction">{t("customize.instruction")}</p>

      {tileFields.length > 0 ? (
        <div
          className="lumen-customize__box"
          role="list"
          aria-label={t("customize.box.label")}
          data-dragging={draggingId ?? undefined}
          onDragOver={(event) => {
            // The whole box accepts drops (a drop back into the box keeps
            // the tile — only dragging OUT removes it).
            event.preventDefault();
          }}
          onDrop={(event) => {
            event.preventDefault();
            dropHandled.current = true;
            setDraggingId(null);
            setDropTargetId(null);
          }}
        >
          {shownFields.map(
            (field) =>
              field !== undefined && (
                <div key={field.id} role="listitem" className="lumen-customize__tile-slot">
                  <button
                    type="button"
                    className="lumen-customize__tile"
                    data-field-id={field.id}
                    data-dragging={draggingId === field.id || undefined}
                    data-drop-target={dropTargetId === field.id || undefined}
                    aria-pressed={
                      field.control.kind === "toggle-action"
                        ? (engineBools[field.id] ?? undefined)
                        : undefined
                    }
                    draggable
                    onDragStart={(event) => {
                      event.dataTransfer.effectAllowed = "move";
                      event.dataTransfer.setData("text/plain", field.id);
                      dropHandled.current = false;
                      setDraggingId(field.id);
                    }}
                    onDragEnd={() => {
                      // No drop target handled the gesture → the tile left
                      // the box → remove it from the player (frame -6).
                      if (!dropHandled.current && draggingId !== null) removeTile(draggingId);
                      setDraggingId(null);
                      setDropTargetId(null);
                    }}
                    onDragOver={(event) => {
                      if (draggingId === null || draggingId === field.id) return;
                      event.preventDefault();
                      event.stopPropagation();
                      setDropTargetId(field.id);
                    }}
                    onDrop={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      dropHandled.current = true;
                      if (draggingId !== null && draggingId !== field.id) moveTile(draggingId, field.id);
                      setDraggingId(null);
                      setDropTargetId(null);
                    }}
                    onKeyDown={(event) => onTileKeyDown(event, field.id)}
                    onClick={() => {
                      if (field.control.kind === "action") {
                        void dispatch(field, field.control.args);
                      } else if (field.control.kind === "toggle-action") {
                        void dispatch(field, field.control.argsFor(engineBools[field.id] ?? null));
                      }
                    }}
                  >
                    <span className="lumen-customize__tile-well">
                      {field.icon ? <Icon name={field.icon} size={20} /> : null}
                    </span>
                    <span className="lumen-customize__tile-label">{t(field.labelKey)}</span>
                  </button>
                </div>
              ),
          )}
        </div>
      ) : null}

      {sectionPanels.map(({ panel, fields }) => (
        <Section key={panel.id} panel={panel}>
          {fields.map((field) => (
            <div key={field.id} className="lumen-customize__row" data-field-id={field.id}>
              {field.control.kind === "select" ? (
                <>
                  <span className="lumen-customize__row-label">{t(field.labelKey)}</span>
                  <Dropdown
                    label={t(field.labelKey)}
                    className="lumen-customize__dropdown"
                    density="compact"
                    value={selected[field.id] ?? ""}
                    resolveLabel={(key) => t(key)}
                    options={field.control.options.map((option) => ({
                      value: option.id,
                      labelKey: option.labelKey,
                    }))}
                    onChange={(optionId) => {
                      const option = field.control.kind === "select"
                        ? field.control.options.find((o) => o.id === optionId)
                        : undefined;
                      if (option !== undefined) void dispatch(field, option.args);
                    }}
                  />
                </>
              ) : field.control.kind === "slider" ? (
                <SliderRow
                  field={field}
                  label={t(field.labelKey)}
                  committed={committed[field.id] ?? null}
                  liveValue={live[field.id] ?? null}
                  onLive={(value) => setLive((prev) => ({ ...prev, [field.id]: value }))}
                  onCommit={(value) => {
                    setLive((prev) => ({ ...prev, [field.id]: value }));
                    if (field.control.kind === "slider") {
                      void dispatch(field, field.control.argsFor(value));
                    }
                  }}
                />
              ) : null}
            </div>
          ))}
        </Section>
      ))}

      {tileFields.length > 0 ? (
        <footer className="lumen-customize__footer">
          {/* Footer buttons are the shared Button grammar (frame: Cancel =
              default variant #1B1B1B/#8F8F8F, Save = tinted variant
              #371F06/#D37918, both 100×32 r6 — button.css owns the states). */}
          <Button
            className="lumen-customize__cancel"
            onClick={() => {
              // Discard the session's edits — back to the saved baseline.
              setBox(savedBox);
            }}
          >
            {m("open.cancel")}
          </Button>
          <Button
            variant="tinted"
            className="lumen-customize__save"
            disabled={!dirty}
            onClick={() => {
              // Keep the current arrangement as the new baseline (doc 08 §4
              // layout state is UI-owned; persistence lands with layouts).
              setSavedBox({ order: shownIds, removed: [] });
            }}
          >
            {t("customize.save")}
          </Button>
        </footer>
      ) : null}
    </div>
  );
}

/** A panel's adjustment section: r12–14 card + 15px semibold header. */
function Section({ panel, children }: { panel: CustomizePanelDef; children: ReactNode }) {
  const t = useCustomizeMsg();
  return (
    <section className="lumen-customize__section" data-panel-id={panel.id}>
      <h4 className="lumen-customize__section-title">{t(panel.labelKey)}</h4>
      {children}
    </section>
  );
}

/** Slider row grammar (EQ/tool frames): label left, value chip right, thin
 * full-width slider below — live preview never dispatches (doc 08 §5). */
function SliderRow({
  field,
  label,
  committed,
  liveValue,
  onLive,
  onCommit,
}: {
  readonly field: CustomizeField;
  readonly label: string;
  readonly committed: number | null;
  readonly liveValue: number | null;
  readonly onLive: (value: number) => void;
  readonly onCommit: (value: number) => void;
}) {
  const control = field.control.kind === "slider" ? field.control : null;
  if (control === null) return null;
  const value = committed ?? control.initial;
  const shown = liveValue ?? value;
  return (
    <div className="lumen-customize__slider-row">
      <div className="lumen-customize__row-head">
        <span className="lumen-customize__row-label">{label}</span>
        <ValueChip className="lumen-customize__value" value={control.format(shown)} />
      </div>
      <Slider
        aria-label={label}
        value={value}
        min={control.min}
        max={control.max}
        step={control.step}
        formatValue={control.format}
        onChangeLive={onLive}
        onCommit={onCommit}
      />
    </div>
  );
}
