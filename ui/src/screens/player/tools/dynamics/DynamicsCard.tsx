// w26.c2 — DynamicsCard: the dynamics tools' shared collapsible card, per
// the tool-card grammar (w26.a20 analysis §D1; frames "Audio - Compressor
// Settings.svg", "Audio - Spatializer Settings.svg", "Audio - Stereo
// Widener.svg" — identical shells, only the row sets differ):
//
//   COLLAPSED  360×48 rx6 #090909 card · "Expand Settings" #6B6B6B ≈14px ·
//              chevron-› 5×10 (Vector 27 family).
//   EXPANDED   360×H rx6 #1B1B1B card · 59px header "Collapse Settings" +
//              chevron-∨ INSIDE the card (one surface, not bar+card) ·
//              rows pitch 76 · 20px bottom pad.
//
// Chevron direction encodes the state (› collapsed / ∨ expanded — analysis
// §D1 pitfall 7); the expanded body stays in the DOM (`hidden`) so
// aria-controls always resolves.
//
// HONEST STAGING (the effects-area pattern, settings/areas/effects.tsx): no
// compressor/spatializer/stereo-widener engine stage or catalog command
// exists. Controls stay fully interactive with LOCAL state, visibly marked
// staged — a role="note" line + data-pending-command naming the
// forward-declared command they await — never a dead control that looks
// engine-bound.

import { memo, useCallback, useId, useState } from "react";
import type { ReactElement } from "react";
import { Icon } from "../../../../components/icon/Icon";
import { useDynamicsMsg } from "./copy";
import { ToolSliderRow } from "./ToolSliderRow";
import type {
  DynamicsCardProps,
  DynamicsRowDef,
  DynamicsRowState,
} from "./types";
import "./dynamics.css";

/**
 * Memoized row adapter (w26.k10, j10 P3-5): the card re-renders on every
 * drag-move (`setValues` per `onChangeLive`), and the map's per-row
 * `onChange` closures were fresh every render — memo(ToolSliderRow) alone
 * could never bail out. This adapter takes the STABLE module-level row def
 * + the card-level `onRowChange` sink, so only the row whose `value`
 * changed re-renders (its i18n hook + tick math stay idle for the rest).
 */
const DynamicsRow = memo(function DynamicsRow({
  row,
  value,
  onRowChange,
}: {
  row: DynamicsRowDef;
  value: number;
  onRowChange: (rowId: string, next: number) => void;
}): ReactElement {
  return (
    <ToolSliderRow
      row={row}
      value={value}
      onChange={(next) => {
        onRowChange(row.id, next);
      }}
    />
  );
});

/** Renders a tool card (see module header for the grammar). */
export function DynamicsCard({
  tool,
  nameKey,
  pendingCommand,
  rows,
  defaultOpen = false,
  id: idProp,
}: DynamicsCardProps): ReactElement {
  const m = useDynamicsMsg();
  const [open, setOpen] = useState(defaultOpen);
  const [values, setValues] = useState<DynamicsRowState>(() =>
    Object.fromEntries(rows.map((row) => [row.id, row.initial])),
  );
  const generatedId = useId();
  const id = idProp ?? `${tool}-${generatedId}`;
  const bodyId = `${id}-body`;

  const setRowValue = useCallback((rowId: string, next: number): void => {
    setValues((prev) => ({ ...prev, [rowId]: next }));
  }, []);

  return (
    <section
      className={["dynamics-tool", open ? "dynamics-tool--open" : ""]
        .filter(Boolean)
        .join(" ")}
      data-tool={tool}
      data-staged-tool={tool}
      data-pending-command={pendingCommand}
      aria-label={m(nameKey)}
    >
      <button
        type="button"
        id={id}
        className="dynamics-tool__toggle"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => {
          setOpen((prev) => !prev);
        }}
      >
        <span className="dynamics-tool__toggle-label">
          {open
            ? m("player.tools.collapse-settings")
            : m("player.tools.expand-settings")}
        </span>
        {/* w26.i07 (h14 defect 3): size 24 paints the shared glyph at its
         * native stroke — 6.5×11.5 (›) / 11.5×6.5 (∨) at sw 1.5, the frame's
         * Vector 27 / Vector 27_2 measure — where the 16px default rendered
         * it ~30% undersized at stroke ~1.0. The −7.5px margin in
         * dynamics.css calibrates the right inset (18.25px, frame). */}
        <Icon
          name={open ? "chevron-down" : "chevron-right"}
          size={24}
          className="dynamics-tool__chevron"
        />
      </button>
      <div
        id={bodyId}
        className="dynamics-tool__body"
        role="group"
        aria-labelledby={id}
        hidden={!open}
      >
        <div className="dynamics-tool__rows">
          {rows.map((row) => (
            <DynamicsRow
              key={row.id}
              row={row}
              value={values[row.id] ?? row.initial}
              onRowChange={setRowValue}
            />
          ))}
        </div>
        <p className="dynamics-tool__staged" role="note">
          {m("player.tools.dynamics.staged")}
        </p>
      </div>
    </section>
  );
}
