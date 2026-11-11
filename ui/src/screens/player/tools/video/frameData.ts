// w26.c4 — frame-measured DATA for the video tools. These are not theme
// values: they are the literal contents the frames show in editable fields
// (the color-threshold hex inputs + their swatch fills), exactly like the
// analysis' color-picker values note ("#FF0000/#FFFFFF — data, not theme").
// They live in a .ts data module with frame citations, out of the .tsx
// semantic-token discipline (raw hex in components is banned; this is user
// data flowing through local state, never a styled color).

/* frame: Video - Colors.svg — row 1 "Color Extraction" + row 2 "Color
 * Threshold": hex input text "#FF0000" and the 22×22 swatch fill #FF0000. */
export const FRAME_EXTRACTION_COLOR = "#FF0000";

/* frame: Video - Colors.svg — Color Threshold row (same literal as above in
 * the frame; kept a separate constant so the two fields are independently
 * editable, matching the two separate input+swatch pairs the frame exports). */
export const FRAME_THRESHOLD_COLOR = "#FF0000";

/** `#RRGGBB` (the frames' format — 7 chars, uppercase hex). */
export function isHexColor(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}
