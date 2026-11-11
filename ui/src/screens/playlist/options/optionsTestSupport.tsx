// w26.b10 — shared test scaffolding: a minimal ClientProvider transport
// (the PlaylistPanel.test.tsx FakeTransport pattern, trimmed to what the
// options layer reads: state snapshot + playlist command spy).

import type { ReactElement } from "react";
import { render } from "@testing-library/react";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport, Event } from "../../../client/transport";
import { PlaylistProvider } from "../PlaylistProvider";
import type { PlaylistEntry } from "../PlaylistProvider";
import { PlaylistOptionsProvider } from "./PlaylistOptionsProvider";
import type { PlaylistEntryMeta, PlaylistSortKey, ResolveEntryMeta } from "./sorting";

export class FakeOptionsTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Test seam: push an engine event (e.g. current-changed). */
  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return {
      gen: 1,
      requestId: request.requestId,
      result: { completed: { kind: "unit" } },
    };
  }
}

export const OPTION_ENTRIES: readonly PlaylistEntry[] = [
  { id: 1, title: "The Wand_S01E10.mp4", source: { kind: "file", path: "/tv/w10.mp4" }, durationSeconds: 130 },
  { id: 2, title: "The Wand_S01E02.mp4", source: { kind: "file", path: "/tv/w2.mp4" }, durationSeconds: null },
  { id: 3, title: "The Wand_S01E01.mp4", source: { kind: "file", path: "/tv/w1.mp4" }, durationSeconds: 2579 },
];

/** media.svg-shaped metadata: id 3 watched (full bar), id 2 half-watched. */
export const OPTION_META: Record<number, PlaylistEntryMeta> = {
  1: {},
  2: { watched: true, watchedProgress: 0.5 },
  3: { watched: true },
};

export interface MountOptions {
  entries?: readonly PlaylistEntry[];
  resolveMeta?: ResolveEntryMeta;
  initialSortKey?: PlaylistSortKey;
  initialViewMode?: "grid" | "list";
}

/** Mounts a node inside Client+Playlist+PlaylistOptions providers and
 * returns the transport spy alongside the RTL view. */
export function mountOptions(node: ReactElement, options: MountOptions = {}) {
  const transport = new FakeOptionsTransport();
  const view = render(
    <ClientProvider transport={transport}>
      <PlaylistProvider initialEntries={options.entries ?? OPTION_ENTRIES}>
        <PlaylistOptionsProvider
          initialSortKey={options.initialSortKey}
          initialViewMode={options.initialViewMode}
          resolveMeta={options.resolveMeta}
        >
          {node}
        </PlaylistOptionsProvider>
      </PlaylistProvider>
    </ClientProvider>,
  );
  return { transport, view };
}

/** The same provider stack as a single component (renderHook wrapper). */
export function OptionsProviders({
  children,
  resolveMeta,
  initialSortKey,
  initialViewMode,
}: {
  children: React.ReactNode;
  resolveMeta?: ResolveEntryMeta;
  initialSortKey?: PlaylistSortKey;
  initialViewMode?: "grid" | "list";
}): ReactElement {
  const transport = new FakeOptionsTransport();
  return (
    <ClientProvider transport={transport}>
      <PlaylistProvider initialEntries={OPTION_ENTRIES}>
        <PlaylistOptionsProvider
          initialSortKey={initialSortKey}
          initialViewMode={initialViewMode}
          resolveMeta={resolveMeta}
        >
          {children}
        </PlaylistOptionsProvider>
      </PlaylistProvider>
    </ClientProvider>
  );
}
