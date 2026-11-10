// /discovery (card 7.D1, issue #42) — the device-discovery panel: the
// UI surface of crates/device-discovery's honest capability posture.
//
// Composition mirrors the sibling screens (network-stream's centered
// APP-tier column on the calm backdrop):
//   * a main card with the screen heading + the local-volumes lane;
//   * a quiet glass panel for the network-services lane.
//
// Honesty contract (the whole screen is engine-truth, never theater):
//   * Local volumes come ONLY from the catalog's live
//     `devices.volume-added` / `devices.volume-removed` events (doc 05
//     §6.2, the 7.C1 additive rows) — there is no devices.list command
//     in the catalog, so no fabricated initial snapshot is possible.
//     Until a host wires the engine's VolumeMonitor into those events,
//     the honest empty state is the truth this screen shows.
//   * The network lane states the recorded verdict: the engine's
//     SSDP/UPnP searcher exists (crates/device-discovery, src/ssdp.rs)
//     but no command exposes it to the UI yet — an honest status line,
//     never a dead search button that pretends.
//   * Volume labels are untrusted OS text (V7): they render as text
//     nodes only — React escapes them; they are never interpolated into
//     markup, ids or aria-labels.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useEffect, useState } from "react";
import type { VolumeKind } from "../../generated/commands";
import { useClient } from "../../client/ClientProvider";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import "./discovery.css";

/** One live volume row — the exact `devices.volume-added` payload. */
interface VolumeRow {
  readonly id: string;
  readonly label: string;
  readonly volumeKind: VolumeKind;
}

/** The l10n key for a volume-kind chip (catalog wire names → keys). */
function kindKey(kind: VolumeKind): MessageKey {
  switch (kind) {
    case "fixed":
      return "discovery.kind.fixed";
    case "removable":
      return "discovery.kind.removable";
    case "optical":
      return "discovery.kind.optical";
    case "network":
      return "discovery.kind.network";
  }
}

/**
 * Fold one devices event into the live volume table (pure — pinned by
 * tests directly). A re-added id replaces its row (label/kind may have
 * changed); a removed id drops its row. Insertion order is the Map's.
 */
export function applyVolumeEvent(
  table: ReadonlyMap<string, VolumeRow>,
  event:
    | { kind: "devices.volume-added"; id: string; label: string; volumeKind: VolumeKind }
    | { kind: "devices.volume-removed"; id: string },
): Map<string, VolumeRow> {
  const next = new Map(table);
  if (event.kind === "devices.volume-added") {
    next.set(event.id, { id: event.id, label: event.label, volumeKind: event.volumeKind });
  } else {
    next.delete(event.id);
  }
  return next;
}

export function DiscoveryScreen() {
  const m = useMsg();
  const client = useClient();
  const [volumes, setVolumes] = useState<ReadonlyMap<string, VolumeRow>>(() => new Map());

  // APP-tier screen: calm backdrop while mounted, cleaned up on unmount
  // (same veil contract as the sibling screens).
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  // The live volume lane: subscribe to the catalog's devices events and
  // fold them into the table. Nothing else seeds the list — the empty
  // state before the first event is the honest truth.
  useEffect(
    () =>
      client.subscribe((event) => {
        if (event.kind !== "devices.volume-added" && event.kind !== "devices.volume-removed") {
          return;
        }
        setVolumes((table) => applyVolumeEvent(table, event));
      }),
    [client],
  );

  const rows = [...volumes.values()];

  return (
    <div className="discovery">
      <div className="discovery__group">
        <section
          className="discovery__card solstice-rise"
          aria-labelledby="discovery-title"
          data-testid="discovery-card"
        >
          <h1 id="discovery-title" className="discovery__title">
            {m("discovery.title")}
          </h1>
          <p className="discovery__subtitle">{m("discovery.subtitle")}</p>

          <section className="discovery__volumes" aria-labelledby="discovery-volumes-title">
            <div className="discovery__volumes-head">
              <h2 id="discovery-volumes-title" className="discovery__volumes-title">
                {m("discovery.volumes.title")}
              </h2>
              <span className="discovery__count solstice-chip">
                <span className="discovery__count-num">{String(rows.length)}</span>
                {m("discovery.volumes.count-suffix")}
              </span>
            </div>

            {/* Live region: mount/unmount announcements for volume
                changes (the list itself is the changing content; the
                label names the region for AT without re-announcing
                rows on every render). */}
            <ul
              className="discovery__volume-list"
              aria-label={m("discovery.volumes.live")}
              aria-live="polite"
            >
              {rows.map((row) => (
                <li key={row.id} className="discovery__volume" data-testid="discovery-volume">
                  <span className="discovery__volume-label">{row.label}</span>
                  <span className="discovery__volume-kind">{m(kindKey(row.volumeKind))}</span>
                  <span className="discovery__volume-id">{row.id}</span>
                </li>
              ))}
            </ul>
            {rows.length === 0 ? (
              <p className="discovery__volumes-empty" data-testid="discovery-volumes-empty">
                {m("discovery.volumes.empty")}
              </p>
            ) : null}
          </section>
        </section>

        <section
          className="discovery__network"
          aria-labelledby="discovery-network-title"
          data-testid="discovery-network"
        >
          <h2 id="discovery-network-title" className="discovery__network-title">
            {m("discovery.network.title")}
          </h2>
          <p className="discovery__network-status">{m("discovery.network.status")}</p>
        </section>
      </div>
    </div>
  );
}
