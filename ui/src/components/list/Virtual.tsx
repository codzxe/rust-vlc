import { useEffect, useRef, useState } from "react";
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

  const measure = (el: HTMLElement, scrollTop = el.scrollTop) =>
    setRange(computeRange(count, itemHeight, scrollTop, el.clientHeight, overscan));

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
   */
  semantics?: "listbox" | "grid";
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
export function VirtualList<T>({ data, itemHeight, overscan = 5, renderItem, label, scrollToIndex = null, semantics = "listbox", className = "", style }: VirtualListProps<T>) {
  const { viewportRef, range, onScroll, scrollToIndex: scroll } = useWindowedRange(data.length, itemHeight, overscan);

  useEffect(() => {
    if (scrollToIndex) scroll(scrollToIndex);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- scroll on target change only
  }, [scrollToIndex?.index, scrollToIndex?.align]);
  const startIndex = Math.min(range.startIndex, Math.max(0, data.length - 1));
  const endIndex = Math.min(range.endIndex, data.length - 1);
  const rows: ReactNode[] = [];
  for (let i = startIndex; i <= endIndex; i++) {
    rows.push(
      semantics === "grid" ? (
        <div key={i} role="row" aria-selected={false} className="lumen-vrow" style={{ top: i * itemHeight, height: itemHeight }}>
          <div role="gridcell" className="lumen-vrow__cell">
            {renderItem(data[i]!, i)}
          </div>
        </div>
      ) : (
        <div key={i} role="option" aria-selected={false} className="lumen-vrow" style={{ top: i * itemHeight, height: itemHeight }}>
          {renderItem(data[i]!, i)}
        </div>
      ),
    );
  }
  return (
    <div
      ref={viewportRef}
      role={semantics === "grid" ? "grid" : "listbox"}
      aria-label={label}
      aria-rowcount={semantics === "grid" ? data.length : undefined}
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
 * Keyboard-Shortcuts screen (doc 08 §1.6). Sticky header stays outside the
 * windowed body so it never unmounts while scrolling.
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
      <div key={i} role="row" className="lumen-vtable__row" style={{ top: i * itemHeight, height: itemHeight, gridTemplateColumns: gridTemplate }}>
        {columns.map((column) => (
          <div role="cell" key={column.id} className="lumen-vtable__cell">
            {column.renderCell(data[i]!, i)}
          </div>
        ))}
      </div>,
    );
  }
  return (
    <div role="table" aria-label={label} aria-rowcount={data.length} className={`lumen-vtable ${className}`.trim()} style={style}>
      <div role="row" className="lumen-vtable__head" style={{ gridTemplateColumns: gridTemplate }}>
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
