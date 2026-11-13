// w26.b11 — the candidate listbox mounting SubtitlePickRow
// (APG listbox: ROVING TABINDEX only — arrows/Home/End, Enter/Space toggles;
// multi-select in the "From Computer + To This File" mode (Folder-2 — two
// rows checked), single-select for the online source (Folder-3).
// w26.k15 (j15 P3-10): aria-activedescendant was REMOVED — the APG lets a
// listbox pick activedescendant OR roving tabindex, never both; roving
// (implemented + tested since b11) stays the one model.
//
// w26.gfix-E (g19 P3-4): candidates render in HOST order — Folder-2.svg
// pins the given order (English, Spanish, Japanese, German, Dutch, French,
// the top two checked), which the previous natural-sort re-broke (checked
// rows scattered to positions 2 & 6). Hosts that want natural order pass
// natural-ordered candidates (the v2 brief's "S01E2 before S01E10" concern
// belongs to the seam that PRODUCES the list, not the picker).
//
// w26.k15 (j15 P2-1): the reset is keyed on a CONTENT SIGNATURE
// (length + first/last names), not the `candidates` ARRAY IDENTITY —
// identity churn (a host re-rendering without memoizing, exactly the
// documented mount-recipe shape) no longer resets roving focus, and the
// focus effect never yanks focus OUT of the listbox (the Folder-3 search
// field keeps its caret while the filtered list resets under it).

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
  const listRef = useRef<HTMLDivElement | null>(null);
  const rowRefs = useRef<Array<HTMLDivElement | null>>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  // w26.k15 (j15 P2-1): mount-transition flag — the FIRST focus pass moves
  // focus into the listbox (APG: the widget takes focus when it appears);
  // every later activeIndex change refocuses only while the listbox subtree
  // already owns focus, so a reset can never steal the caret from the search
  // field (or any other host control).
  const tookInitialFocus = useRef(false);

  const ordered = candidates;
  const isSelected = (name: string): boolean => selected.includes(name);

  // Focus the active row (roving tabindex). On mount the first row takes
  // focus; afterwards ONLY while focus already sits inside the listbox — a
  // programmatic activeIndex reset (filtered/replaced candidates) must not
  // yank focus from wherever the user parked it (j15 P2-1 repro A).
  useEffect(() => {
    if (tookInitialFocus.current) {
      const root = listRef.current;
      if (root === null || !root.contains(document.activeElement)) return;
    }
    tookInitialFocus.current = true;
    rowRefs.current[activeIndex]?.focus({ preventScroll: true });
  }, [activeIndex]);

  // Reset the active row when the candidate CONTENT changes (mode/source
  // switch, search filter) so the list never focuses a removed row — keyed
  // on a stable signature, NOT array identity (j15 P2-1 repro B: a
  // non-memoized host array with the same content must not reset roving
  // focus on every render).
  const signature = `${ordered.length}|${ordered[0] ?? ""}|${ordered[ordered.length - 1] ?? ""}`;
  const prevSignature = useRef(signature);
  useEffect(() => {
    if (prevSignature.current === signature) return;
    prevSignature.current = signature;
    setActiveIndex(0);
  }, [signature]);

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

  return (
    <div
      ref={(node) => {
        listRef.current = node;
      }}
      id={rootId}
      role="listbox"
      aria-label={label ?? t("playlist.subtitles.list-label")}
      aria-multiselectable={multiple || undefined}
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
