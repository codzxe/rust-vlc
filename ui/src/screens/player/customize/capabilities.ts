// Capability seam for the customize catalog.
//
// The customize box renders an ENGINE-BOUND tile only when its command is
// reported available. Source of truth: `system.list-commands` (doc 05 §3)
// — the engine's own listing of supported command ids. The scaffold's
// MockTransport answers with an EMPTY listing, which maps honestly to "no
// capabilities" → every engine-bound tile hidden (no dead controls).
// UI-owned toolbar items (command: null — Spacer, Go Home, the Name
// pills, …) carry no engine surface and always render (w26.g07-P1-2's
// catalog split: the frame's box is a 31-tile catalog, not a capability
// report).

import { useEffect, useState } from "react";
import type { CommandTransport } from "../../../client/transport";
import type { CustomizeTile } from "./types";

export interface CommandCapabilities {
  /** Command ids the engine listed as supported. */
  readonly available: ReadonlySet<string>;
  /** True once the listing query settled (success OR typed failure). */
  readonly resolved: boolean;
}

const NO_CAPABILITIES: CommandCapabilities = { available: new Set<string>(), resolved: false };

/**
 * Query `system.list-commands` once per transport and expose the available
 * command-id set. A failed/unexposed listing resolves to an EMPTY set —
 * absence of proof is never proof of support.
 */
export function useCommandCapabilities(wire: CommandTransport): CommandCapabilities {
  const [caps, setCaps] = useState<CommandCapabilities>(NO_CAPABILITIES);

  useEffect(() => {
    let cancelled = false;
    wire
      .send({ cmd: "system.list-commands", requestId: 1 })
      .then((response) => {
        if (cancelled) return;
        const result = response.result;
        const ids =
          "completed" in result && result.completed.kind === "command-listings"
            ? result.completed.commands.map((listing) => listing.id)
            : [];
        setCaps({ available: new Set<string>(ids), resolved: true });
      })
      .catch(() => {
        // Transport failure: no capability claims (honest empty).
        if (!cancelled) setCaps({ available: new Set<string>(), resolved: true });
      });
    return () => {
      cancelled = true;
    };
  }, [wire]);

  return caps;
}

/**
 * A tile renders when it is UI-owned (no engine command) OR its command is
 * reported available — the customize-surface policy: hidden, never a dead
 * control (contrast the settings seam's disabled-with-reason).
 */
export function tileVisible(tile: CustomizeTile, caps: CommandCapabilities): boolean {
  return tile.command === null || caps.available.has(tile.command);
}

/** The subset of the catalog that passes the capability gate. */
export function visibleTiles(
  tiles: readonly CustomizeTile[],
  caps: CommandCapabilities,
): readonly CustomizeTile[] {
  return tiles.filter((tile) => tileVisible(tile, caps));
}