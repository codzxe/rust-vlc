// w26.gfixA (issue #62) — toolbar-arrangement persistence (doc 08 §4:
// "toolbar layout is UI-OWNED state"). The customize surface's Save
// commits the working arrangement here, mirroring the settings/variant
// persistence seam (variants/variant.ts — the same localStorage pattern,
// never throwing on unavailable hosts).
//
// w26.i05 (h07 defect 1, state 7): the seam now has a CONSUMER half —
// `useToolbarLayout()` + the `lumen:toolbar-changed` event keep every
// mounted reader (the real ControlBar's toolbar items, toolbarItems.tsx)
// live across Save, the same decoupled-event pattern as
// `lumen:open-customize` (contract §4.5). The layout stays a pure UI
// concern: no engine command exists for it (verified against the 1.D4
// catalog).

import { useEffect, useState } from "react";
import { CUSTOMIZE_TILE_IDS, DEFAULT_TOOLBAR_LAYOUT, tileById } from "./panels";
import type { ToolbarLayout } from "./types";

/** localStorage key (mirrors the settings persistence seam's naming). */
export const TOOLBAR_STORAGE_KEY = "lumen.player.toolbar.v1";

/**
 * Broadcast fired by {@link persistToolbarLayout} (same tab) so live
 * consumers re-read storage — the frozen cross-component contract string
 * pattern (`lumen:open-customize`); never rename.
 */
export const TOOLBAR_CHANGED_EVENT = "lumen:toolbar-changed";

/** Structural equality for persisted-shape comparison (Save/Cancel). */
export function toolbarLayoutEqual(a: ToolbarLayout, b: ToolbarLayout): boolean {
  return (
    a.order.length === b.order.length &&
    a.band.length === b.band.length &&
    a.order.every((id, index) => id === b.order[index]) &&
    a.band.every((id, index) => id === b.band[index])
  );
}

/** Normalize unknown persisted JSON into a layout (or null = unusable). */
export function parseToolbarLayout(raw: unknown): ToolbarLayout | null {
  if (typeof raw !== "object" || raw === null) return null;
  const order = (raw as { order?: unknown }).order;
  const band = (raw as { band?: unknown }).band;
  if (!Array.isArray(order) || !Array.isArray(band)) return null;
  const known = new Set<string>(CUSTOMIZE_TILE_IDS);
  const seenOrder = new Set<string>();
  const orderIds = order.filter((id): id is string => {
    if (typeof id !== "string" || !known.has(id) || seenOrder.has(id)) return false;
    seenOrder.add(id);
    return true;
  });
  const bandIds = band.filter((id): id is string => typeof id === "string" && known.has(id));
  if (orderIds.length === 0 && bandIds.length === 0) return null;
  // Missing catalog ids (a tile added post-persist) append in frame order —
  // the arrangement stays a deduped permutation of the catalog.
  const appended = CUSTOMIZE_TILE_IDS.filter((id) => !seenOrder.has(id));
  return {
    order: [...orderIds, ...appended],
    band: [...new Set<string>(bandIds)],
  };
}

/** Load the persisted arrangement; the frame default when absent/unusable. */
export function loadToolbarLayout(): ToolbarLayout {
  try {
    const raw = globalThis.localStorage?.getItem(TOOLBAR_STORAGE_KEY);
    if (raw === null) return DEFAULT_TOOLBAR_LAYOUT;
    return (
      parseToolbarLayout(JSON.parse(raw) ?? null) ?? DEFAULT_TOOLBAR_LAYOUT
    );
  } catch {
    // Storage unavailable or corrupted JSON — session-only default, honest.
    return DEFAULT_TOOLBAR_LAYOUT;
  }
}

/** Persist an arrangement; failures degrade to session-only (never throw).
 * Broadcasts {@link TOOLBAR_CHANGED_EVENT} so live consumers re-read it. */
export function persistToolbarLayout(layout: ToolbarLayout): void {
  try {
    globalThis.localStorage?.setItem(TOOLBAR_STORAGE_KEY, JSON.stringify(layout));
  } catch {
    // Unavailable host — same honest degradation as loadToolbarLayout.
  }
  try {
    globalThis.window?.dispatchEvent(new CustomEvent(TOOLBAR_CHANGED_EVENT));
  } catch {
    // Non-window host (worker) — no broadcast surface.
  }
}

/** Clear persisted arrangement (tests / reset-to-default paths). */
export function clearPersistedToolbarLayout(): void {
  try {
    globalThis.localStorage?.removeItem(TOOLBAR_STORAGE_KEY);
  } catch {
    // Unavailable host — nothing persisted, nothing to clear.
  }
}

/** Resolve a band entry to a renderable tile (unknown ids are dropped). */
export function bandTiles(layout: ToolbarLayout): readonly (
  | NonNullable<ReturnType<typeof tileById>>
)[] {
  return layout.band
    .map((id) => tileById(id))
    .filter((tile): tile is NonNullable<typeof tile> => tile !== null);
}

/**
 * Live reader for the persisted arrangement (w26.i05, h07 defect 1): starts
 * from storage and re-reads on every {@link TOOLBAR_CHANGED_EVENT} (Save in
 * this or another mounted customize surface) and `storage` (cross-tab).
 * The real ControlBar's toolbar items consume this through
 * toolbarItems.tsx.
 */
export function useToolbarLayout(): ToolbarLayout {
  const [layout, setLayout] = useState<ToolbarLayout>(loadToolbarLayout);
  useEffect(() => {
    const reread = (): void => setLayout(loadToolbarLayout());
    window.addEventListener(TOOLBAR_CHANGED_EVENT, reread);
    window.addEventListener("storage", reread);
    return () => {
      window.removeEventListener(TOOLBAR_CHANGED_EVENT, reread);
      window.removeEventListener("storage", reread);
    };
  }, []);
  return layout;
}
