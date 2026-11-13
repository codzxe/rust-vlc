// w26.b11 — the subtitle candidate row (`subtitle one.svg` + `subtitle
// multiple.svg`, the two row fragments the Folder states embed).
//
// Frame anatomy (subtitle one.svg "Property 1=selected" / Folder-2 rows):
//   * row band 479–1008 wide × 36px, rx6 — fill #090909 (surface) on the
//     SELECTED state only; default rows are transparent on the overlay;
//   * leading subtitle-file glyph (18×20, stroke 1.5 #6B6B6B → tertiary
//     ink) 11px off the row's left edge;
//   * filename 13–14px #969696 (→ content-secondary; 1/255-class delta
//     documented in the lane log) starting 17px past the glyph;
//   * trailing orange check (frame Vector 44: 14×11, stroke 2 #D37918) on
//     the selected row, 13px off the right edge;
//   * the MULTIPLE variant (subtitle multiple.svg "Property 1=hover") shows
//     the stacked ←/→ cycle glyph (#8F8F8F) on hover instead — the
//     "this video has several subtitle files" affordance.
//
// The row is PRESENTATIONAL: selection semantics (role, roving tabindex,
// toggling) belong to SubtitlePickList, which spreads `optionProps`.

import type { HTMLAttributes, ReactElement, Ref } from "react";
import { Icon } from "../../../components/icon";
import { SubtitleFileGlyph, CycleGlyph } from "./SubtitleGlyphs";

export interface SubtitlePickRowProps {
  /** Candidate file name, verbatim ("The_Wand_S01E01_English.srt"). */
  fileName: string;
  /** Selected rows paint the #090909 band + the orange check. */
  selected: boolean;
  /** The subtitle multiple.svg variant — hover reveals the cycle glyph. */
  multiple?: boolean;
  /** Row id (stable listbox option id — DOM/test addressing; w26.k15: no
   * longer wires aria-activedescendant — the listbox is roving-tabindex
   * only, j15 P3-10). */
  id: string;
  /** Behavior spread from SubtitlePickList (role/aria/tabIndex/handlers). */
  optionProps?: HTMLAttributes<HTMLDivElement>;
  /** React 19 ref-as-prop on the row element (roving focus target). */
  ref?: Ref<HTMLDivElement>;
}

export function SubtitlePickRow({
  fileName,
  selected,
  multiple = false,
  id,
  optionProps,
  ref,
}: SubtitlePickRowProps): ReactElement {
  return (
    <div
      id={id}
      ref={ref}
      className={[
        "lumen-plsubs-row",
        selected ? "lumen-plsubs-row--selected" : "",
        multiple ? "lumen-plsubs-row--multiple" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      {...optionProps}
    >
      <span className="lumen-plsubs-row__icon" aria-hidden="true">
        <SubtitleFileGlyph size={19} />
      </span>
      <span className="lumen-plsubs-row__name">{fileName}</span>
      {selected ? (
        <span className="lumen-plsubs-row__check" aria-hidden="true">
          {/* frame: subtitle one.svg Vector 44 — 14×11, stroke 2 #D37918 */}
          <Icon name="check" size={15} variant="strong" />
        </span>
      ) : (
        multiple && (
          <span className="lumen-plsubs-row__cycle" aria-hidden="true">
            <CycleGlyph size={18} />
          </span>
        )
      )}
    </div>
  );
}
