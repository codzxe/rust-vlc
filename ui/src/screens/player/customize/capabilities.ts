// Card 4.F3 — capability seam for the customize panels.
//
// The customize renderer shows a field ONLY when its command is reported
// available. Source of truth: `system.list-commands` (doc 05 §3) — the
// engine's own listing of supported command ids. The scaffold's
// MockTransport answers with an EMPTY listing, which maps honestly to "no
// capabilities" → every gated field hidden (no dead controls). This is the
// customize-surface policy: hidden (contrast the settings seam's
// disabled-with-reason — same exposure state, different presentation).

import { useEffect, useState } from "react";
import type { CommandTransport } from "../../../client/transport";
import type { CustomizeField, CustomizePanelDef } from "./types";

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

/** A field renders only when its command is reported available. */
export function fieldVisible(field: CustomizeField, caps: CommandCapabilities): boolean {
  return caps.available.has(field.command);
}

/** The subset of a panel's fields that pass the capability gate. */
export function visibleFields(
  panel: CustomizePanelDef,
  caps: CommandCapabilities,
): readonly CustomizeField[] {
  return panel.fields.filter((field) => fieldVisible(field, caps));
}

/** A panel with zero visible fields is itself hidden — no empty dead panels. */
export function panelVisible(panel: CustomizePanelDef, caps: CommandCapabilities): boolean {
  return visibleFields(panel, caps).length > 0;
}
