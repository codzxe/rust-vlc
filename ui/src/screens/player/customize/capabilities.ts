// Capability seam for the customize family.
//
// POLICY (rewritten w26.k08, j08 finding #4): the h07-defect-4
// static-catalog ruling ships — the customize box renders the frame's FULL
// 31-tile catalog against ANY `system.list-commands` listing (the box is a
// catalog, never a capability report); engine truth governs DISPATCH only
// (honest no-change on feature-not-exposed). The retired gating helpers
// (tileVisible/visibleTiles) are deleted together with their re-exports and
// their own test block, per the j08 suggested fix.
//
// The listing query below therefore has NO product consumer inside the
// customize surface anymore; it stays exported (frozen barrel API —
// removals are not additive) as the capability seam other designs cite
// (convertQueue's comment) and is pinned by its own hook test. Deleting it
// wholesale is a Lead call for a point where the API freeze allows
// removals; until then this header documents the honest state.

import { useEffect, useState } from "react";
import type { CommandTransport } from "../../../client/transport";

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
