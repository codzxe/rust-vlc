// w26.i16 (h18 dead frame #2 / g20-P2-3, the i04 enqueue seam generalized):
// the pickable enqueue sources — the same clips + remembered openings the
// open-media dialog lists (never fabricated), one MenuEntry per source,
// each wired to `playlist.playLater` (playlist.enqueue). Lead-landed from
// the w26.i16 lane log §proposal 2.
import type { MenuEntry } from "../../components/menu";
import { useClient } from "../../client/ClientProvider";
import { DEMO_CLIPS, isDemoTransport } from "../../client/demo-transport";
import { isDesktopShell } from "../../client/app-transport";
import { useMsg } from "../../i18n/messages";

export interface EnqueueSource {
  readonly id: string;
  readonly label: string;
  readonly source: import("../../client/transport").SourceSpec;
  readonly title: string | null;
}

export function useEnqueueSources(): EnqueueSource[] {
  const m = useMsg();
  const client = useClient();
  const sources: EnqueueSource[] = [];
  const seen = new Set<string>();
  const rows = [
    ...(isDesktopShell()
      ? []
      : DEMO_CLIPS.map((clip) => ({
          id: `clip-${clip.id}`, label: m(clip.titleKey),
          source: clip.source, title: m(clip.titleKey) as string | null,
        }))),
    ...(isDemoTransport(client)
      ? client.recentOpenings.map((entry, index) => ({
          id: `recent-${index}`, label: entry.title ?? (entry.source.kind === "file" ? entry.source.path : entry.source.url),
          source: entry.source, title: entry.title,
        }))
      : []),
  ];
  for (const row of rows) {
    const location = row.source.kind === "file" ? row.source.path : row.source.url;
    if (seen.has(location)) continue;
    seen.add(location);
    sources.push(row);
  }
  return sources;
}

export function enqueueEntries(
  sources: EnqueueSource[],
  playLater: (source: EnqueueSource["source"], title: string | null | undefined) => void,
  emptyLabel: string,
  openMediaLabel: string,
  onOpenMedia: () => void,
): MenuEntry<string>[] {
  return sources.length === 0
    ? [{ id: "add-empty", labelKey: emptyLabel, disabled: true }]
    : [
        ...sources.map((row): MenuEntry<string> => ({
          id: row.id, labelKey: row.label,
          icon: row.source.kind === "file" ? "open-file" : "network-stream",
          onSelect: () => playLater(row.source, row.title),
        })),
        { id: "add-sep", kind: "separator" },
        { id: "add-open-media", labelKey: openMediaLabel, icon: "open-file", onSelect: onOpenMedia },
      ];
}
