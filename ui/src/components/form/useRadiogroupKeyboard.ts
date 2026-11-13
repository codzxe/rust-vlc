// useRadiogroupKeyboard (w26.k01 j01-2) — the ONE roving keydown handler for
// the three radiogroup-grammar primitives (RadioGroup, SegmentedControl,
// SwatchGroup). It was copy-pasted verbatim ×3 (the reviewer's drift-risk
// warning came true in j01-9's switch skins — the same class of bug); all
// four-arrow navigation with wrap-around, skipping disabled options, now
// lives here (doc 08 §4.5 focus rules).

import type { KeyboardEvent } from "react";

/** The option shape the roving grammar needs (value + optional disabled). */
export interface RadiogroupOption {
  value: string;
  disabled?: boolean;
}

/**
 * Roving keydown handler for a `role="radiogroup"` host: ArrowUp/Down/
 * Left/Right move the selection to the next enabled option with
 * wrap-around; every other key passes through untouched. Fires `onChange`
 * with the next option's value and prevents the default scroll only when an
 * arrow was actually handled.
 */
export function useRadiogroupKeyboard<T extends RadiogroupOption>(
  options: readonly T[],
  value: string,
  onChange: (value: T["value"]) => void,
): (event: KeyboardEvent<HTMLElement>) => void {
  return (event) => {
    const vertical = event.key === "ArrowDown" || event.key === "ArrowUp";
    const horizontal = event.key === "ArrowRight" || event.key === "ArrowLeft";
    if (!vertical && !horizontal) return;
    const enabled = options.filter((option) => !option.disabled);
    if (enabled.length === 0) return;
    const currentIndex = enabled.findIndex((option) => option.value === value);
    const delta = event.key === "ArrowUp" || event.key === "ArrowLeft" ? -1 : 1;
    const next = enabled[(currentIndex + delta + enabled.length) % enabled.length];
    if (next && next.value !== value) onChange(next.value);
    event.preventDefault();
  };
}
