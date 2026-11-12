// w26.b11 — the candidate listbox mounting SubtitlePickRow
// (APG listbox: roving tabindex, aria-activedescendant, arrows/Home/End,
// Enter/Space toggles; multi-select in the "From Computer + To This File"
// mode (Folder-2 — two rows checked), single-select for the online source
// (Folder-3).
//
// w26.gfix-E (g19 P3-4): candidates render in HOST order — Folder-2.svg
// pins the given order (English, Spanish, Japanese, German, Dutch, French,
// the top two checked), which the previous natural-sort re-broke (checked
// rows scattered to positions 2 & 6). Hosts that want natural order pass
// natural-ordered candidates (the v2 brief's "S01E2 before S01E10" concern
// belongs to the seam that PRODUCES the list, not the picker).

import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";
import { SubtitlePickRow } from "./SubtitlePickRow";
import { useSubtitlesMsg } from "./useSubtitlesMsg";

export interface SubtitlePickListProps {
  /** Candidate subtitle file names, rendered in the GIVEN order (the
   * frame's host order — Folder-2.svg; sort at the producing seam). */
  candidates: readonly string[];
  /** Selected names (controlled). */
  selected: readonly string[];
  /** Selection change (multi appends/removes; single replaces). */
  onSelectedChange: (selected: readonly string[]) => void;
  /** Multi-select (Folder-2) vs single-select (Folder-3). */
  multiple?: boolean;
  /** Accessible name override for the listbox. */
  label?: string;
  /** Extra classes for the list root. */
  className?: string;
  id?: string;
}

export function SubtitlePickList({
  candidates,
  selected,
  onSelectedChange,
  multiple = false,
  label,
  className = "",
  id,
}: SubtitlePickListProps): ReactElement {
  const t = useSubtitlesMsg();
  const autoId = useId();
  const rootId = id ?? autoId;
  const rowRefs = useRef<Array<HTMLDivElement | null>>([]);
  const [activeIndex, setActiveIndex] = useState(0);

  const ordered = candidates;
  const isSelected = (name: string): boolean => selected.includes(name);

  // Keep the active row focusable (roving tabindex) and in view.
  useEffect(() => {
    rowRefs.current[activeIndex]?.focus({ preventScroll: true });
  }, [activeIndex]);

  // Reset the active row when the candidate set changes identity (mode/source
  // switch) so the list never focuses a removed row.
  useEffect(() => {
    setActiveIndex(0);
  }, [candidates]);

  const toggle = (name: string): void => {
    if (!multiple) {
      onSelectedChange([name]);
      return;
    }
    onSelectedChange(
      isSelected(name) ? selected.filter((entry) => entry !== name) : [...selected, name],
    );
  };

  const move = (delta: number): void => {
    if (ordered.length === 0) return;
    setActiveIndex((index) => (index + delta + ordered.length) % ordered.length);
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const { key } = event;
    const handled = (): void => {
      event.preventDefault();
      event.stopPropagation();
    };
    if (key === "ArrowDown") {
      move(1);
      handled();
      return;
    }
    if (key === "ArrowUp") {
      move(-1);
      handled();
      return;
    }
    if (key === "Home") {
      setActiveIndex(0);
      handled();
      return;
    }
    if (key === "End") {
      setActiveIndex(ordered.length - 1);
      handled();
      return;
    }
    if (key === "Enter" || key === " ") {
      const name = ordered[activeIndex];
      if (name !== undefined) toggle(name);
      handled();
    }
    // Tab is NOT trapped here: the dialog's focus trap owns Tab cycling.
  };

  if (ordered.length === 0) {
    return (
      <div className="lumen-plsubs-empty" id={rootId}>
        <p className="lumen-plsubs-empty__title">{t("playlist.subtitles.empty-title")}</p>
        <p className="lumen-plsubs-empty__body">{t("playlist.subtitles.empty-body")}</p>
      </div>
    );
  }

  const activeName = ordered[activeIndex];

  return (
    <div
      id={rootId}
      role="listbox"
      aria-label={label ?? t("playlist.subtitles.list-label")}
      aria-multiselectable={multiple || undefined}
      aria-activedescendant={activeName === undefined ? undefined : `${rootId}-opt-${activeIndex}`}
      className={["lumen-plsubs-list", className].filter(Boolean).join(" ")}
      onKeyDown={onKeyDown}
    >
      {ordered.map((name, index) => (
        <SubtitlePickRow
          key={name}
          id={`${rootId}-opt-${index}`}
          ref={(element) => {
            rowRefs.current[index] = element;
          }}
          fileName={name}
          selected={isSelected(name)}
          multiple={multiple}
          optionProps={{
            role: "option",
            tabIndex: index === activeIndex ? 0 : -1,
            "aria-selected": isSelected(name),
            onClick: () => {
              setActiveIndex(index);
              toggle(name);
            },
            onFocus: () => setActiveIndex(index),
          }}
        />
      ))}
    </div>
  );
}
