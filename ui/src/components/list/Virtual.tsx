import { useEffect, useId, useRef, useState } from "react";
import type { ReactNode } from "react";
import "./list.css";
import { computeRange } from "./range";
import type { VirtualRange, VirtualScrollTarget } from "./range";

/**
 * ScrollArea — custom-styled thin scrollbars via tokens over *honest* native
 * scrolling (doc 08 §4: no fake scroll physics, no wheel hijacking). The
 * webkit pseudo-elements + `scrollbar-width: thin` in list.css realize the
 * design's hairline scrollbar; this component is a labelled scroll region.
 */
export function ScrollArea({ children, label, className = "" }: { children: ReactNode; label: string; className?: string }) {
  return (
    <div role="region" aria-label={label} tabIndex={0} className={`lumen-scrollarea ${className}`.trim()}>
      <div className="lumen-scrollarea__content">{children}</div>
    </div>
  );
}

/** Shared state for windowed rendering over a native scroll container. */
function useWindowedRange(count: number, itemHeight: number, overscan: number) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [range, setRange] = useState<VirtualRange>(() => computeRange(count, itemHeight, 0, 0, overscan));

  const measure = (el: HTMLElement, scrollTop = el.scrollTop) => {
    const next = computeRange(count, itemHeight, scrollTop, el.clientHeight, overscan);
    // w26.k03 (j03 P2-6): keep the previous state object when the window is
    // unchanged — installing a fresh object re-rendered the whole row window
    // (wrappers + consumer rows) on every scroll tick inside the overscan
    // band. Identical start/end/totalHeight ⇒ same render ⇒ keep `prev`.
    setRange((prev) =>
      prev.startIndex === next.startIndex && prev.endIndex === next.endIndex && prev.totalHeight === next.totalHeight
        ? prev
        : next,
    );
  };

  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    measure(el);
    if (typeof ResizeObserver === "undefined") return; // jsdom: tests drive via scroll events
    const ro = new ResizeObserver(() => measure(el));
    ro.observe(el);
    return () => ro.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- measure reads live DOM geometry only
  }, [count, itemHeight, overscan]);

  // Programmatic scroll-to-index (doc 08 §4: scrollTo index → right rows
  // rendered). Setting scrollTop directly keeps the jump native and exact.
  const scrollToIndex = (target: VirtualScrollTarget) => {
    const el = viewportRef.current;
    if (!el) return;
    const { index, align = "start" } = target;
    const clamped = Math.max(0, Math.min(count - 1, index));
    let scrollTop = clamped * itemHeight;
    if (align === "center") scrollTop -= Math.max(0, (el.clientHeight - itemHeight) / 2);
    else if (align === "end") scrollTop -= Math.max(0, el.clientHeight - itemHeight);
    el.scrollTop = Math.max(0, scrollTop);
    measure(el);
  };

  return { viewportRef, range, onScroll: (e: React.UIEvent<HTMLDivElement>) => measure(e.currentTarget), scrollToIndex };
}

export interface VirtualListProps<T> {
  /** Full dataset — only the window (± overscan) is rendered. */
  data: readonly T[];
  /** Fixed row height in px (doc 08 §4: fixed itemHeight windowing). */
  itemHeight: number;
  /** Extra rows rendered above/below the viewport. @default 5 */
  overscan?: number;
  /** Row renderer; receives the item and its absolute index. */
  renderItem: (item: T, index: number) => ReactNode;
  /** Accessible name for the listbox region. */
  label: string;
  /** When this prop changes, the list scrolls so `index` is at `align`. */
  scrollToIndex?: VirtualScrollTarget | null;
  /**
   * Widget semantics (w26 Lead fix, axe nested-interactive):
   * - "listbox" (default): rows are `role="option"` — ARIA options must NOT
   *   contain interactive descendants (links, buttons). Use only for pure
   *   selection lists.
   * - "grid": rows are `role="row"` + `role="gridcell"` — the ARIA pattern
   *   for media rows that embed links/buttons (sidebar playlists, queue
   *   rows with per-row play actions, kebab affordances).
   *
   * w26.k03 (j03 P3-11): GRID-FIRST. All four production mounts use
   * `"grid"`; it is the only path that carries keyboard support surface
   * (`activeIndex`/aria-activedescendant, aria-rowindex) and honest
   * selection semantics (`isItemSelected`). The default stays `"listbox"`
   * for wave-1 API compatibility, but that path implements no option
   * focus/selection/activedescendant (options are unfocusable; the
   * container owns the tab stop) — new interactive consumers should mount
   * `semantics="grid"`. Making `grid` the default is a next-major question.
   */
  semantics?: "listbox" | "grid";
  /**
   * w26.k03 (j03 P2-3), additive: row selection truth. When provided, every
   * rendered row/option carries `aria-selected` with this predicate's answer
   * for that row (e.g. the current queue track, the open playlist). When
   * omitted the attribute is omitted entirely — an unselectable grid must
   * not announce `aria-selected="false"` rows that are visually selected
   * (the pre-fix pinned `false` lied on all three grid consumers).
   */
  isItemSelected?: (item: T, index: number) => boolean;
  /**
   * w26.k03 (j03 P2-4), additive: the active row for the
   * aria-activedescendant flavor of the APG grid pattern. The primitive
   * stamps every rendered row with a stable DOM id (`<container
   * id>-row-<index>`) and, while `activeIndex` is a valid data index,
   * points `aria-activedescendant` at that row and marks it
   * `data-active="true"` (consumer CSS paints it — see playlist.css). The
   * CONSUMER still owns the keyboard model (arrows/Home/End move the index;
   * keep the active row inside the rendered window via `scrollToIndex`) —
   * this replaces the imperative id/activedescendant DOM surgery consumers
   * had to run post-render (PlaylistScreen's useGridRoving sync pass).
   * `null` = nothing active.
   */
  activeIndex?: number | null;
  className?: string;
  style?: React.CSSProperties;
}

/**
 * VirtualList — windowed listbox (doc 08 §4 virtualization requirement;
 * §4.5: playlist → `listbox`/`option`). Native scrolling; rows are absolutely
 * positioned children of a full-height spacer.
 *
 * The rendered window is clamped to the CURRENT data length: the measured
 * range can lag one render behind a shrinking dataset (a filter applied),
 * and consumers must never see a stale `data[i]` (AppShell relies on this).
 */
export function VirtualList<T>({
  data,
  itemHeight,
  overscan = 5,
  renderItem,
  label,
  scrollToIndex = null,
  semantics = "listbox",
  isItemSelected,
  activeIndex = null,
  className = "",
  style,
}: VirtualListProps<T>) {
  const { viewportRef, range, onScroll, scrollToIndex: scroll } = useWindowedRange(data.length, itemHeight, overscan);
  // w26.k03 (j03 P2-4): stable row identity — every row wrapper carries
  // `<id>-row-<absolute index>` so aria-activedescendant (and consumer CSS)
  // can address rows without imperative DOM writes.
  const listId = useId();
  const rowId = (index: number): string => `${listId}-row-${index}`;

  useEffect(() => {
    if (scrollToIndex) scroll(scrollToIndex);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- scroll on target change only
  }, [scrollToIndex?.index, scrollToIndex?.align]);
  const startIndex = Math.min(range.startIndex, Math.max(0, data.length - 1));
  const endIndex = Math.min(range.endIndex, data.length - 1);
  const activeValid = activeIndex !== null && activeIndex >= 0 && activeIndex < data.length;
  const rows: ReactNode[] = [];
  for (let i = startIndex; i <= endIndex; i++) {
    rows.push(
      semantics === "grid" ? (
        <div
          key={i}
          id={rowId(i)}
          role="row"
          /* w26.k03 (j03 P2-3): omitted entirely while `isItemSelected` is
           * unset — a hard `false` announced visually-selected rows as
           * unselected on every grid consumer. */
          aria-selected={isItemSelected?.(data[i]!, i)}
          /* w26.k03 (j03 P2-5): APG — dynamically rendered rows of a grid
           * that sets aria-rowcount must expose their 1-based position. */
          aria-rowindex={i + 1}
          data-active={activeValid && i === activeIndex ? "true" : undefined}
          className="lumen-vrow"
          style={{ top: i * itemHeight, height: itemHeight }}
        >
          <div role="gridcell" className="lumen-vrow__cell">
            {renderItem(data[i]!, i)}
          </div>
        </div>
      ) : (
        <div
          key={i}
          id={rowId(i)}
          role="option"
          aria-selected={isItemSelected?.(data[i]!, i)}
          data-active={activeValid && i === activeIndex ? "true" : undefined}
          className="lumen-vrow"
          style={{ top: i * itemHeight, height: itemHeight }}
        >
          {renderItem(data[i]!, i)}
        </div>
      ),
    );
  }
  return (
    <div
      ref={viewportRef}
      id={listId}
      role={semantics === "grid" ? "grid" : "listbox"}
      aria-label={label}
      aria-rowcount={semantics === "grid" ? data.length : undefined}
      aria-activedescendant={activeValid ? rowId(activeIndex) : undefined}
      tabIndex={0}
      className={`lumen-vlist ${className}`.trim()}
      style={style}
      onScroll={onScroll}
    >
      <div className="lumen-vlist__spacer" style={{ height: range.totalHeight }}>
        {rows}
      </div>
    </div>
  );
}

export interface VirtualColumn<T> {
  id: string;
  header: ReactNode;
  width?: number;
  renderCell: (item: T, index: number) => ReactNode;
}

export interface VirtualTableProps<T> {
  data: readonly T[];
  /** Fixed row height in px. */
  itemHeight: number;
  overscan?: number;
  columns: readonly VirtualColumn<T>[];
  /** Accessible name for the table region. */
  label: string;
  /** When this prop changes, the table body scrolls so `index` is at `align`. */
  scrollToIndex?: VirtualScrollTarget | null;
  className?: string;
  style?: React.CSSProperties;
}

/**
 * VirtualTable — same windowing math as VirtualList, exposed with table
 * semantics (`role=table`/`row`/`columnheader`/`cell`) for grids like the
 * Keyboard-Shortcuts screen (doc 08 §1.6). The header row stays outside the
 * windowed body so it never unmounts while scrolling.
 *
 * w26.k03 (j03 P2-5): rows are APG-numbered — the always-rendered header is
 * row 1, windowed body rows carry `aria-rowindex={i + 2}`, and
 * `aria-rowcount` counts the header too (`data.length + 1`). Dynamically
 * rendered rows without a rowindex are invisible to SR position while
 * `aria-rowcount` is set; values must be unique and increasing, hence the
 * header-aware offset (the review's shorthand `i + 1` would collide with the
 * header's position).
 */
export function VirtualTable<T>({ data, itemHeight, overscan = 5, columns, label, scrollToIndex = null, className = "", style }: VirtualTableProps<T>) {
  const { viewportRef, range, onScroll, scrollToIndex: scroll } = useWindowedRange(data.length, itemHeight, overscan);

  useEffect(() => {
    if (scrollToIndex) scroll(scrollToIndex);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- scroll on target change only
  }, [scrollToIndex?.index, scrollToIndex?.align]);
  // Same shrink-lag clamp as VirtualList — never render past the data.
  const startIndex = Math.min(range.startIndex, Math.max(0, data.length - 1));
  const endIndex = Math.min(range.endIndex, data.length - 1);
  const gridTemplate = columns.map((c) => (c.width ? `${c.width}px` : "1fr")).join(" ");
  const rows: ReactNode[] = [];
  for (let i = startIndex; i <= endIndex; i++) {
    rows.push(
      <div
        key={i}
        role="row"
        aria-rowindex={i + 2} /* +2: the header is row 1 (APG uniqueness) */
        className="lumen-vtable__row"
        style={{ top: i * itemHeight, height: itemHeight, gridTemplateColumns: gridTemplate }}
      >
        {columns.map((column) => (
          <div role="cell" key={column.id} className="lumen-vtable__cell">
            {column.renderCell(data[i]!, i)}
          </div>
        ))}
      </div>,
    );
  }
  return (
    <div role="table" aria-label={label} aria-rowcount={data.length + 1} className={`lumen-vtable ${className}`.trim()} style={style}>
      <div role="row" aria-rowindex={1} className="lumen-vtable__head" style={{ gridTemplateColumns: gridTemplate }}>
        {columns.map((column) => (
          <div role="columnheader" key={column.id} className="lumen-vtable__cell lumen-vtable__cell--head">
            {column.header}
          </div>
        ))}
      </div>
      <div ref={viewportRef} className="lumen-vtable__body" onScroll={onScroll} tabIndex={0}>
        <div className="lumen-vlist__spacer" style={{ height: range.totalHeight }}>
          {rows}
        </div>
      </div>
    </div>
  );
}
