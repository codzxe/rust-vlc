// Card 4.F3 — the customize panels renderer.
//
// Data-driven composition (doc 08 §4 note): panels/fields come from
// ./panels.ts, each field bound to a command id; a field renders ONLY when
// the engine's `system.list-commands` listing includes that command
// (./capabilities.ts). All engine communication goes through the
// command-layer client seam. Truthfulness (doc 08 §5): select/radio
// selections and slider committed values update ONLY from engine
// confirmations (command completions / events) — live slider drags are
// visual previews that never dispatch; onCommit is the only dispatch point.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useClient } from "../../../client/ClientProvider";
import type {
  CommandRequest,
  CommandResult,
  CommandTransport,
  Event,
} from "../../../client/transport";
import { Button } from "../../../components/button";
import { RadioGroup } from "../../../components/form";
import { Slider } from "../../../components/slider";
import { useMsg } from "../../../i18n/messages";
import {
  panelVisible,
  useCommandCapabilities,
  visibleFields,
} from "./capabilities";
import { CUSTOMIZE_FIELDS, CUSTOMIZE_PANELS } from "./panels";
import type { CommandArgs, CustomizeField } from "./types";
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

export interface CustomizePanelsProps {
  /** Transport override (tests); defaults to the client context. */
  readonly transport?: CommandTransport;
}

/**
 * Renders the four design panels (Crop/Aspect, Video Colors, Essential, HDR)
 * with per-field capability gating. Panels whose fields are all unsupported
 * are hidden entirely; when NO panel is supported the surface renders only
 * an honest screen-reader note (the scaffold MockTransport state).
 */
export function CustomizePanels({ transport }: CustomizePanelsProps) {
  const client = useClient();
  const wire = transport ?? client;
  const m = useMsg();
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

  if (panels.length === 0) {
    // All fields unsupported → no panels (no dead controls). Honest note.
    return <p className="lumen-customize__empty">{m("customize.none-available")}</p>;
  }

  return (
    <div className="lumen-customize" data-resolved={caps.resolved}>
      {panels.map((panel) => (
        <section key={panel.id} className="lumen-customize__panel" data-panel-id={panel.id}>
          <h3 className="lumen-customize__panel-title">{m(panel.labelKey)}</h3>
          {visibleFields(panel, caps).map((field) => (
            <div key={field.id} className="lumen-customize__field" data-field-id={field.id}>
              {renderField(field, {
                m,
                selected: selected[field.id] ?? "",
                committed: committed[field.id] ?? null,
                live: live[field.id] ?? null,
                engineBool: engineBools[field.id] ?? null,
                onSelect: (args) => void dispatch(field, args),
                onLive: (value) => setLive((prev) => ({ ...prev, [field.id]: value })),
                onCommit: (value) => {
                  setLive((prev) => ({ ...prev, [field.id]: value }));
                  void dispatch(field, field.control.kind === "slider" ? field.control.argsFor(value) : {});
                },
              })}
            </div>
          ))}
        </section>
      ))}
    </div>
  );
}

interface FieldRenderProps {
  readonly m: ReturnType<typeof useMsg>;
  readonly selected: string;
  readonly committed: number | null;
  readonly live: number | null;
  readonly engineBool: boolean | null;
  readonly onSelect: (args: CommandArgs) => void;
  readonly onLive: (value: number) => void;
  readonly onCommit: (value: number) => void;
}

function renderField(field: CustomizeField, props: FieldRenderProps) {
  const { control } = field;
  switch (control.kind) {
    case "select":
      return (
        <RadioGroup
          label={props.m(field.labelKey)}
          options={control.options.map((option) => ({
            value: option.id,
            label: props.m(option.labelKey),
          }))}
          value={props.selected}
          onChange={(optionId) => {
            const option = control.options.find((o) => o.id === optionId);
            if (option !== undefined) props.onSelect(option.args);
          }}
        />
      );
    case "slider": {
      const value = props.committed ?? control.initial;
      const shown = props.live ?? value;
      return (
        <div className="lumen-customize__slider">
          <span className="lumen-customize__label">{props.m(field.labelKey)}</span>
          <output className="lumen-customize__value">{control.format(shown)}</output>
          <Slider
            aria-label={props.m(field.labelKey)}
            value={value}
            min={control.min}
            max={control.max}
            step={control.step}
            formatValue={control.format}
            onChangeLive={props.onLive}
            onCommit={props.onCommit}
          />
        </div>
      );
    }
    case "action":
      return (
        <Button size="sm" className="lumen-customize__action" onClick={() => props.onSelect(control.args)}>
          {props.m(field.labelKey)}
        </Button>
      );
    case "toggle-action":
      return (
        <Button
          size="sm"
          className="lumen-customize__action"
          aria-pressed={props.engineBool ?? undefined}
          onClick={() => props.onSelect(control.argsFor(props.engineBool))}
        >
          {props.m(field.labelKey)}
        </Button>
      );
  }
}
