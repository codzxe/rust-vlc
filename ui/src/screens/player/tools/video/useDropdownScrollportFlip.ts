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
// resize (the body scrolling under an open dropdown moves the trigger; the
// list re-glues instead of detaching). DOM-written, not React state:
// measured positioning is imperative by nature and the markers/coordinates
// are purely presentational.
//
// ROOT-FIX NOTE for i16/Lead (also in docs/waves/w26/logs/w26.i08.md): the
// escape belongs in the shared primitive eventually — a `Dropdown` that
// knows about scrollports should also fall back to the viewport when
// neither side of the clip box fits (portal or fixed), so every
// dialog-hosted dropdown gets it; this hook is the scoped stopgap that
// closes h15's HDR NO this wave.

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

/** Escape coordinates the hook re-glues on scroll/resize (mode is sticky). */
interface EscapeState {
  side: "below" | "above";
  listHeight: number;
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
    const dropdown = root.querySelector(".lumen-dropdown");
    if (dropdown === null) return;

    let escape: EscapeState | null = null;
    let reposition: (() => void) | null = null;

    const clear = (): void => {
      root.removeAttribute("data-list-direction");
      root.removeAttribute("data-list-escape");
      root.style.removeProperty("--video-dropdown-cap");
      const list = dropdown.querySelector<HTMLElement>(".lumen-dropdown__list");
      if (list !== null) {
        for (const property of ["position", "top", "bottom", "left", "width"] as const) {
          list.style.removeProperty(property);
        }
      }
    };

    const apply = (): void => {
      const trigger = dropdown.querySelector<HTMLElement>(".lumen-dropdown__trigger");
      const list = dropdown.querySelector<HTMLElement>(".lumen-dropdown__list");
      if (trigger === null || list === null) {
        escape = null;
        reposition = null;
        clear();
        return;
      }
      const rect = trigger.getBoundingClientRect();
      const listHeight = list.getBoundingClientRect().height;
      if (rect.height <= 0 || listHeight <= 0) {
        // jsdom / pre-layout: nothing to decide — the shared default holds.
        escape = null;
        reposition = null;
        clear();
        return;
      }

      const placement = resolveListPlacement({
        trigger: { top: rect.top, bottom: rect.bottom, left: rect.left, width: rect.width },
        clip: clipBoxAround(trigger),
        listHeight,
        viewportHeight: window.innerHeight,
      });

      clear();
      if (placement.mode === "inflow") {
        escape = null;
        reposition = null;
        return;
      }
      if (placement.mode === "escape") {
        escape = { side: placement.side, listHeight };
        root.setAttribute("data-list-escape", placement.side);
        // Computed/dynamic coordinates — inline styles by contract §6.
        // `bottom: auto` is explicit: menu.css's data-direction="up" rule
        // would otherwise stretch a fixed box pinned by both insets.
        list.style.position = "fixed";
        list.style.top = `${placement.top}px`;
        list.style.bottom = "auto";
        list.style.left = `${placement.left}px`;
        list.style.width = `${placement.width}px`;
        reposition = (): void => {
          const current = trigger.getBoundingClientRect();
          list.style.top = `${
            placement.side === "below"
              ? current.bottom + LIST_GAP
              : current.top - LIST_GAP - escape!.listHeight
          }px`;
          list.style.left = `${current.left}px`;
          list.style.width = `${current.width}px`;
        };
        return;
      }
      // Last resort: capped internal scroll on the roomier side (the
      // video.css data-list-direction rules pin the side + the cap var).
      escape = null;
      reposition = null;
      root.setAttribute("data-list-direction", placement.side === "below" ? "down" : "up");
      root.style.setProperty("--video-dropdown-cap", `${placement.cap}px`);
    };

    // The shared Dropdown renders its list only while open — watch for it.
    const observer = new MutationObserver(apply);
    observer.observe(dropdown, { childList: true });

    // While escaped, the fixed list follows the trigger (scroll under an
    // open dropdown moves the trigger; the list re-glues, never detaches).
    const onScrollOrResize = (): void => {
      if (reposition !== null) reposition();
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
