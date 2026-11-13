// w26.i08 — useDropdownScrollportFlip: tool-side clip-aware list placement
// for the shared Dropdown (h15-P1 / g17-P1-1).
//
// WHERE THIS SITS after i16's root fix landed mid-wave: the shared
// `components/menu/Dropdown.tsx` now measures the nearest CLIPPING
// CONTAINER (its `clippingBounds`, w26.i16) and flips the list up/down
// accordingly — that root fix resolves the h15 repro wherever the list
// FITS one side of the clip box (the Gradient sub-dropdown: 3 rows flip up
// inside the tools-dialog body at both contract viewports). The video mounts
// keep this hook for the ONE case the flip cannot solve: the HDR frame's
// own rest state, where the 6-row list (280px at the frame's 44@46 grammar)
// fits NEITHER side of the dialog body (1440×900: 48px below, 236px above —
// the body is content-sized and does not scroll at rest), so any in-flow
// placement paints rows outside the clip:
//
//   1. measure the REAL clip box — the intersection of every clipping
//      ancestor (overflow ≠ visible), falling back to the viewport;
//   2. if the list fits either side of that box → do NOTHING: the shared
//      primitive's own data-direction decision already lands it (no
//      double-ownership, no CSS fight);
//   3. else ESCAPE the clip: position: fixed at the viewport spot that fits
//      — below the trigger first (the frame draws the list below:
//      "HDR Options.svg" state=open), above as fallback (1024×768 rest).
//      `position: fixed` escapes every overflow clip (the dialog surface's
//      overflow + the body's scrollport) while staying a DOM child of the
//      dropdown, so the shared outside-click/Escape/typeahead behavior is
//      untouched. Coordinates are computed/dynamic → inline styles (contract
//      §6); the wrapper's data-list-escape marker is the test/CSS seam;
//   4. last resort (list taller than the viewport on both sides): cap the
//      list to the roomier side via --video-dropdown-cap so it scrolls
//      internally instead of clipping (the w26.g17 long-list grammar).
//
// While an escape is active the fixed list follows the trigger on scroll /
// resize — and w26.k11 (j11 P3-1) re-runs the FULL placement decision, not
// just the coordinate glue: if the body scrolling under an open list lets it
// fit a clip side again, the list reverts to the shared in-flow flip instead
// of staying stuck in escape mode. DOM-written, not React state: measured
// positioning is imperative by nature and the markers/coordinates are
// purely presentational.
//
// w26.k11 (j11 P3-1): the DOM half is a jsdom-testable seam — `applyPlacement`
// / `clearPlacement` below take an explicit decision + elements, so the
// escape/cap/clear writes are unit-pinned (a selector rename or clear-order
// bug now fails tests instead of passing silently).
//
// ROOT-FIX NOTE for i16/Lead (also in docs/waves/w26/logs/w26.i08.md): the
// escape belongs in the shared primitive eventually — a `Dropdown` that
// knows about scrollports should also fall back to the viewport when
// neither side of the clip box fits (portal or fixed), so every
// dialog-hosted dropdown gets it; this hook is the scoped stopgap that
// closes h15's HDR NO this wave. DELETE THIS MODULE when the primitive
// gains the escape. (Known wrinkle kept for that consolidation:
// `clipBoxAround` intersects EVERY clipping ancestor while the primitive's
// `clippingBounds` takes the nearest only — two algorithms that can
// disagree; unifying them means editing Dropdown.tsx, which is the menu
// lane's file, not this lane's.)

import { useEffect, useRef } from "react";
import type { RefObject } from "react";

/** menu.css list seam: `top: calc(100% + var(--space-grid))` (4px). */
const LIST_GAP = 4;

/** Viewport edge margin for escaped lists (the f07 menu-clamp grammar). */
const VIEWPORT_EDGE_MARGIN = 8;

/** Floor for the last-resort cap — never smaller than ~2 rows. */
const MIN_LIST_CAP = 96;

/** A measured box in viewport coordinates. */
export interface PlacementBox {
  top: number;
  bottom: number;
}

/** Trigger geometry the placement decision needs (viewport coordinates). */
export interface TriggerBox extends PlacementBox {
  left: number;
  width: number;
}

/** The decision `resolveListPlacement` returns for one open. */
export type ListPlacement =
  /** The list fits a side of the clip box — the shared Dropdown's own
   * data-direction placement already lands it; the hook does nothing. */
  | { mode: "inflow" }
  /** Neither clip side fits — escape the clip at this fixed position. */
  | { mode: "escape"; side: "below" | "above"; top: number; left: number; width: number }
  /** Not even the viewport fits — cap the list on the roomier side so it
   * scrolls internally instead of clipping. */
  | { mode: "capped"; side: "below" | "above"; cap: number };

/**
 * Pure placement decision (unit-tested in jsdom without layout): where does
 * a `listHeight`-tall list go around `trigger`, given the visible `clip`
 * box (intersection of clipping ancestors) and the viewport height?
 */
export function resolveListPlacement(input: {
  trigger: TriggerBox;
  clip: PlacementBox;
  listHeight: number;
  viewportHeight: number;
}): ListPlacement {
  const { trigger, clip, listHeight, viewportHeight } = input;
  const belowClip = clip.bottom - trigger.bottom - LIST_GAP;
  const aboveClip = trigger.top - clip.top - LIST_GAP;
  if (belowClip >= listHeight || aboveClip >= listHeight) return { mode: "inflow" };

  const belowViewport =
    viewportHeight - trigger.bottom - LIST_GAP - VIEWPORT_EDGE_MARGIN;
  const aboveViewport = trigger.top - LIST_GAP - VIEWPORT_EDGE_MARGIN;
  if (belowViewport >= listHeight) {
    return {
      mode: "escape",
      side: "below",
      top: trigger.bottom + LIST_GAP,
      left: trigger.left,
      width: trigger.width,
    };
  }
  if (aboveViewport >= listHeight) {
    return {
      mode: "escape",
      side: "above",
      top: trigger.top - LIST_GAP - listHeight,
      left: trigger.left,
      width: trigger.width,
    };
  }
  const side: "below" | "above" = belowViewport >= aboveViewport ? "below" : "above";
  return {
    mode: "capped",
    side,
    cap: Math.max(MIN_LIST_CAP, Math.ceil(Math.max(belowViewport, aboveViewport))),
  };
}

/** True when the computed overflow-y actually clips content. */
function clipsY(value: string): boolean {
  return value === "auto" || value === "scroll" || value === "hidden" || value === "clip";
}

/**
 * The visible clip box around `el`: the intersection of every ancestor that
 * clips (plus the viewport). This is the box an in-flow popup must land in —
 * not `window.innerHeight` (h15-P1's root defect).
 */
function clipBoxAround(el: HTMLElement): PlacementBox {
  const box: PlacementBox = { top: 0, bottom: window.innerHeight };
  if (typeof window === "undefined") return box;
  for (
    let node = el.parentElement;
    node instanceof HTMLElement;
    node = node.parentElement
  ) {
    const style = window.getComputedStyle(node);
    if (clipsY(style.overflowY) || clipsY(style.overflow) || clipsY(style.overflowX)) {
      const rect = node.getBoundingClientRect();
      box.top = Math.max(box.top, rect.top);
      box.bottom = Math.min(box.bottom, rect.bottom);
    }
    if (node === document.body) break;
  }
  return box;
}

/**
 * Removes every placement marker/style this module writes (w26.k11, j11
 * P3-1: exported so jsdom can pin the clear path) — restores the shared
 * Dropdown's own in-flow default.
 */
export function clearPlacement(root: HTMLElement, dropdown: HTMLElement): void {
  root.removeAttribute("data-list-direction");
  root.removeAttribute("data-list-escape");
  root.style.removeProperty("--video-dropdown-cap");
  const list = dropdown.querySelector<HTMLElement>(".lumen-dropdown__list");
  if (list !== null) {
    for (const property of ["position", "top", "bottom", "left", "width"] as const) {
      list.style.removeProperty(property);
    }
  }
}

/**
 * Applies one placement decision to the DOM (w26.k11, j11 P3-1: the hook's
 * DOM half, extracted so jsdom can pin the writes without layout):
 *   • `inflow` — nothing (the shared Dropdown's own data-direction rules own
 *     the placement; no double ownership);
 *   • `escape` — `data-list-escape` marker on the wrapper + computed inline
 *     `position: fixed` coordinates on the list (escapes every overflow clip
 *     while staying a DOM child — outside-click/Escape/typeahead intact);
 *   • `capped` — `data-list-direction` pins the roomier side and
 *     `--video-dropdown-cap` caps the list so it scrolls internally
 *     (the w26.g17 long-list grammar; video.css owns the rules).
 */
export function applyPlacement(
  root: HTMLElement,
  dropdown: HTMLElement,
  placement: ListPlacement,
): void {
  clearPlacement(root, dropdown);
  if (placement.mode === "inflow") return;
  const list = dropdown.querySelector<HTMLElement>(".lumen-dropdown__list");
  if (list === null) return;
  if (placement.mode === "escape") {
    root.setAttribute("data-list-escape", placement.side);
    // Computed/dynamic coordinates — inline styles by contract §6.
    // `bottom: auto` is explicit: menu.css's data-direction="up" rule
    // would otherwise stretch a fixed box pinned by both insets.
    list.style.position = "fixed";
    list.style.top = `${placement.top}px`;
    list.style.bottom = "auto";
    list.style.left = `${placement.left}px`;
    list.style.width = `${placement.width}px`;
    return;
  }
  root.setAttribute("data-list-direction", placement.side === "below" ? "down" : "up");
  root.style.setProperty("--video-dropdown-cap", `${placement.cap}px`);
}

/**
 * Owns list placement for the shared Dropdown mounted inside `ref`'s
 * element: defers to the primitive's flip while the list fits the clip box,
 * escapes to `position: fixed` when it does not, caps as last resort.
 * Attach the returned ref to the wrapper that hosts exactly one
 * `.lumen-dropdown`.
 */
export function useDropdownScrollportFlip<T extends HTMLElement>(): RefObject<T | null> {
  const ref = useRef<T | null>(null);

  useEffect(() => {
    const root = ref.current;
    if (root === null || typeof MutationObserver === "undefined") return;
    const dropdown = root.querySelector<HTMLElement>(".lumen-dropdown");
    if (dropdown === null) return;

    const measure = (): void => {
      const trigger = dropdown.querySelector<HTMLElement>(".lumen-dropdown__trigger");
      const list = dropdown.querySelector<HTMLElement>(".lumen-dropdown__list");
      if (trigger === null || list === null) {
        clearPlacement(root, dropdown);
        return;
      }
      const rect = trigger.getBoundingClientRect();
      const listHeight = list.getBoundingClientRect().height;
      if (rect.height <= 0 || listHeight <= 0) {
        // jsdom / pre-layout: nothing to decide — the shared default holds.
        clearPlacement(root, dropdown);
        return;
      }

      applyPlacement(
        root,
        dropdown,
        resolveListPlacement({
          trigger: { top: rect.top, bottom: rect.bottom, left: rect.left, width: rect.width },
          clip: clipBoxAround(trigger),
          listHeight,
          viewportHeight: window.innerHeight,
        }),
      );
    };

    // The shared Dropdown renders its list only while open — watch for it.
    const observer = new MutationObserver(measure);
    observer.observe(dropdown, { childList: true });

    // While escaped, the fixed list follows the trigger — and the FULL
    // decision re-runs (w26.k11, j11 P3-1): geometry that lets the list fit
    // a clip side again reverts it to the shared in-flow flip, so an escape
    // is no longer sticky for the whole open.
    const onScrollOrResize = (): void => {
      measure();
    };
    window.addEventListener("resize", onScrollOrResize, { passive: true });
    const scroller = (() => {
      for (
        let node = dropdown.parentElement;
        node instanceof HTMLElement;
        node = node.parentElement
      ) {
        const oy = window.getComputedStyle(node).overflowY;
        if (oy === "auto" || oy === "scroll") return node;
        if (node === document.body) return null;
      }
      return null;
    })();
    scroller?.addEventListener("scroll", onScrollOrResize, { passive: true });

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", onScrollOrResize);
      scroller?.removeEventListener("scroll", onScrollOrResize);
    };
    // One dropdown per wrapper for the tool's lifetime — measure per open.
  }, []);

  return ref;
}
