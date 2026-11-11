// /discovery (card 7.D1, issue #42) — the device-discovery panel: the
// UI surface of crates/device-discovery's honest capability posture.
//
// W26 Figma language (w26.b12 rebuild; no direct frame — composed from the
// design language, CONTRACT §2, with playlist.svg rows / thumbnail.svg cards /
// sidebar.svg quiet meta as the nearest grammars):
//   * a compact top-anchored column (~680px) on the near-black surface —
//     working-screen density, not a centered hero;
//   * page heading + section headers = the 15px-class semibold sentence-case
//     grammar (Settings - *.svg section titles), composed through the
//     SectionHeader primitive;
//   * the two lanes ride the r14 #191919 bordered Card primitive
//     (thumbnail.svg / playlist.svg panel grammar; hairline #404040);
//   * volume rows = the ListRow primitive (playlist.svg row anatomy:
//     icon · title · meta · right-aligned trailing, r10 hover fill);
//   * the empty state = the quiet ListEmpty (no artwork — lists stay calm);
//   * NO orange at rest: nothing here is selected/active, so the ONE accent
//     stays on focus rings and the shell's active rail stop only.
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
//   * Volume rows are read-only (no browse command exists) and volume
//     labels are untrusted OS text (V7): they render as text nodes only —
//     React escapes them; they are never interpolated into markup, ids
//     or aria-labels.
//   * w26.c6 — VOLUME-ROW CONTEXT MENU (folder context menu.svg, 6 icon
//     rows: plus · folder+plus · subtitle card · waveform · refresh ·
//     info). The frame's labels are "Option" placeholders, so semantics
//     derive from the pinned icons. Honest wiring: "Add file…" fires the
//     shell-wide open-media event (the app's one real add-file seam — not
//     volume-scoped, the catalog has no browse command); every other row
//     has NO seam in this build (no folder-picker, no subtitle/audio-track
//     add, no rescan command — the table is event-driven — and no
//     volume-info command) and renders disabled — the screen's honest-
//     capability posture, never theater.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useCallback, useEffect, useState } from "react";
import type { VolumeKind } from "../../generated/commands";
import { useClient } from "../../client/ClientProvider";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b12";
import { keys as c6Keys } from "../../i18n/keys/w26.c6";
import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import { ContextMenu } from "../../components/menu";
import type { MenuEntry } from "../../components/menu";
import { ListEmpty, ListRow } from "../../components/list";
import { Card, SectionHeader } from "../../components/surface";
import { OPEN_MEDIA_EVENT } from "../open-media";
import "./discovery.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead). */
type LaneKey = keyof typeof laneKeys | keyof typeof c6Keys;
const laneCatalog: Readonly<Record<string, string>> = { ...laneKeys, ...c6Keys };

/**
 * useDiscoveryMsg — `useMsg` extended with this lane's pre-registration keys.
 * Lane keys win while they are unregistered; once the Lead merges them into
 * the catalog both paths return the same string. Every user-visible string
 * flows through here — no literals (doc 07 §6.5).
 */
function useDiscoveryMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

/** One live volume row — the exact `devices.volume-added` payload. */
interface VolumeRow {
  readonly id: string;
  readonly label: string;
  readonly volumeKind: VolumeKind;
}

/** The l10n key for a volume-kind meta line (catalog wire names → keys). */
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
 * The volume-row context menu (folder context menu.svg — 6 rows, pinned
 * icon order; row 1's plus is the frame's only all-gray icon). Seams only:
 * the open-media event is real, the rest have no seam and stay disabled.
 */
function volumeRowMenu(): MenuEntry<LaneKey>[] {
  return [
    // 1 — Add file… (frame: gray plus) → the shell's open-media event
    // (AppShell owns the single dialog listener).
    {
      id: "add-file",
      labelKey: "discovery.menu.add-file",
      icon: "add",
      onSelect: () => window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT)),
    },
    // 2 — Add folder… (frame: gray folder + orange plus). No folder-picker
    // seam exists — disabled, never a fake picker.
    { id: "add-folder", labelKey: "discovery.menu.add-folder", icon: "open-folder", iconAccent: true, disabled: true },
    // 3 — Add subtitle tracks (frame: subtitle card + orange dashes). No
    // event seam from a volume row — disabled.
    { id: "add-subtitles", labelKey: "discovery.menu.add-subtitles", icon: "subtitles", iconAccent: true, disabled: true },
    // 4 — Add audio tracks (frame: two-tone waveform) — no seam, disabled.
    { id: "add-audio", labelKey: "discovery.menu.add-audio", icon: "audio", iconAccent: true, disabled: true },
    // 5 — Refresh (frame: two-tone resync arrows). The volume table is
    // `devices.volume-*` event-driven — the catalog has no rescan command
    // (a no-op "refresh" would be theater) — disabled.
    { id: "refresh", labelKey: "discovery.menu.refresh", icon: "refresh", iconAccent: true, disabled: true },
    // 6 — Info (frame: gray circle + orange dot). No volume-info command —
    // disabled.
    { id: "info", labelKey: "discovery.menu.info", icon: "info", iconAccent: true, disabled: true },
  ];
}

/**
 * The registry glyph for a volume kind (pinned Icon.svg vocabulary only —
 * never invented geometry): fixed/removable = the folder pair, optical =
 * the disc accelerator, network = the stream glyph.
 */
function kindIcon(kind: VolumeKind): IconName {
  switch (kind) {
    case "fixed":
      return "folder";
    case "removable":
      return "open-folder";
    case "optical":
      return "open-disc";
    case "network":
      return "network-stream";
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
  const t = useDiscoveryMsg();
  const client = useClient();
  const [volumes, setVolumes] = useState<ReadonlyMap<string, VolumeRow>>(() => new Map());

  // The live volume lane: subscribe to the catalog's devices events and
  // fold them into the table. Nothing else seeds the list — the empty
  // state before the first event is the honest truth. (The Solstice
  // data-backdrop veil is retired with the v2 aesthetic, W25 §0: this
  // screen no longer touches documentElement while mounting.)
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
    <div className="discovery" data-screen="discovery">
      {/* Page head — the compact working-screen scale (Settings top-bar
       * grammar: 15px-class semibold heading + one quiet subtitle line). */}
      <header className="discovery__head">
        <h1 id="discovery-title" className="discovery__title">
          {t("discovery.title")}
        </h1>
        <p className="discovery__subtitle">{t("discovery.subtitle")}</p>
      </header>

      {/* Local volumes lane: section header at the column edge (Settings
       * grammar) + the r14 panel holding the rows / the quiet empty state. */}
      <section className="discovery__section" aria-labelledby="discovery-volumes-title">
        <SectionHeader
          as="h2"
          id="discovery-volumes-title"
          trailing={
            <span className="discovery__count">
              <span className="discovery__count-num">{String(rows.length)}</span>{" "}
              {t("discovery.volumes.count-suffix")}
            </span>
          }
        >
          {t("discovery.volumes.title")}
        </SectionHeader>

        <Card className="discovery__panel" data-testid="discovery-card">
          {rows.length === 0 ? (
            <ListEmpty
              data-testid="discovery-volumes-empty"
              title={t("discovery.volumes.empty-title")}
              body={t("discovery.volumes.empty-body")}
            />
          ) : (
            /* Live region: mount/unmount announcements for volume changes
             * (the list itself is the changing content; the label names the
             * region for AT without re-announcing rows on every render). */
            <ul
              className="discovery__volume-list"
              aria-label={t("discovery.volumes.live")}
              aria-live="polite"
            >
              {rows.map((row) => (
                <li key={row.id} data-testid="discovery-volume">
                  <ContextMenu
                    entries={volumeRowMenu()}
                    resolveLabel={t}
                    ariaLabel={t("discovery.menu.title")}
                  >
                    <ListRow
                      icon={<Icon name={kindIcon(row.volumeKind)} size={18} />}
                      title={row.label}
                      meta={t(kindKey(row.volumeKind))}
                      trailing={<span className="discovery__volume-id">{row.id}</span>}
                    />
                  </ContextMenu>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </section>

      {/* Network services lane: the same header + panel grammar, carrying
       * the recorded SSDP/UPnP verdict — a status line, never a fake
       * search control. */}
      <section className="discovery__section" aria-labelledby="discovery-network-title">
        <SectionHeader as="h2" id="discovery-network-title">
          {t("discovery.network.title")}
        </SectionHeader>
        <Card className="discovery__panel discovery__panel--prose" data-testid="discovery-network">
          <p className="discovery__network-status">{t("discovery.network.status")}</p>
        </Card>
      </section>
    </div>
  );
}
