import type { ReactElement } from "react";
import "./form.css";

export interface ValueChipProps {
  /** The value text (e.g. "12.0 dB", "200%"). */
  value: string;
  /** Extra class for row composition (consumer CSS owns layout). */
  className?: string;
}

/**
 * ValueChip — the dark value readout used by EQ/tool rows (wave 26; frame:
 * audio value.svg / EQ rows): r6 recessed well, bright tabular value text.
 * A plain span — the surrounding row supplies the label context, so the
 * value itself stays the accessible content.
 */
export function ValueChip({ value, className = "" }: ValueChipProps): ReactElement {
  return (
    <span className={["lumen-valuechip", className].filter(Boolean).join(" ")}>{value}</span>
  );
}
