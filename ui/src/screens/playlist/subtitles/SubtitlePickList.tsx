// w26.b11 — the candidate listbox mounting SubtitlePickRow
// (APG listbox: roving tabindex, aria-activedescendant, arrows/Home/End,
// Enter/Space toggles; multi-select in the "From Computer + To This File"
// mode (Folder-2 — two rows checked), single-select for the online source
// (Folder-3). Candidates arrive NATURAL-SORTED (naturalSort, the v2 flow
// brief fix: "S01E2" before "S01E10").

import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactElement } from "react";
import { naturalSort } from "../../../components/list";
import { SubtitlePickRow } from "./SubtitlePickRow";
import { useSubtitlesMsg } from "./useSubtitlesMsg";

export interface SubtitlePickListProps {
  /** Candidate subtitle file names (any order — the list natural-sorts). */
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

  const sorted = naturalSort(candidates, (name) => name);
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
    if (sorted.length === 0) return;
    setActiveIndex((index) => (index + delta + sorted.length) % sorted.length);
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
      setActiveIndex(sorted.length - 1);
      handled();
      return;
    }
    if (key === "Enter" || key === " ") {
      const name = sorted[activeIndex];
      if (name !== undefined) toggle(name);
      handled();
    }
    // Tab is NOT trapped here: the dialog's focus trap owns Tab cycling.
  };

  if (sorted.length === 0) {
    return (
      <div className="lumen-plsubs-empty" id={rootId}>
        <p className="lumen-plsubs-empty__title">{t("playlist.subtitles.empty-title")}</p>
        <p className="lumen-plsubs-empty__body">{t("playlist.subtitles.empty-body")}</p>
      </div>
    );
  }

  const activeName = sorted[activeIndex];

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
      {sorted.map((name, index) => (
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
