// w26.k14 (j14 P2-2) — the shared enqueue seam's own tests. The module is
// the ONE source of truth both add-to-queue consumers build on (the queue
// panel's "+" and the MiniPlayer's ADD, k08's migration of the i04 inline
// copy) and had zero coverage: the source kinds (demo clips / remembered
// openings), the desktop-shell clip hiding, the location dedup, the
// empty-state menu branch and the per-source playLater dispatch were all
// unguarded. The cross-consumer guard at the bottom keeps the duplication
// dead (j14 #1 verified: MiniPlayer builds through this module — a
// re-inlined copy fails the guard).
//
// The panel-level "+" affordance (open/dispatch/close/focus) is covered in
// PlaylistPanel.test.tsx; the MiniPlayer-level path in variants.test.tsx.

import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { DemoTransport } from "../../client/demo-transport";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
import type { MenuEntry } from "../../components/menu";
import { enqueueEntries, useEnqueueSources, type EnqueueSource } from "./enqueueSources";

/** Minimal non-demo transport (the browser dev shell without recents). */
class StubTransport implements CommandTransport {
  subscribe(): () => void {
    return () => undefined;
  }
  async send(request: CommandRequest): Promise<CommandResponse> {
    return { gen: 1, requestId: request.requestId, result: { completed: { kind: "unit" } } };
  }
}

const clientWrapper =
  (transport: CommandTransport) =>
  ({ children }: { children: ReactNode }) => <ClientProvider transport={transport}>{children}</ClientProvider>;

// jsdom is a browser context: no __TAURI_INTERNALS__, so isDesktopShell()
// reads false and the demo clips are the sources. The desktop-shell tests
// stub the internals probe (the real desktop app runs the Tauri transport
// — never a demo transport, never clips).
const withDesktopShell = <T,>(run: () => T): T => {
  const host = globalThis as { __TAURI_INTERNALS__?: unknown };
  host.__TAURI_INTERNALS__ = { invoke: () => Promise.resolve({}), transformCallback: () => 0 };
  try {
    return run();
  } finally {
    delete host.__TAURI_INTERNALS__;
  }
};

describe("useEnqueueSources — the pickable sources (never fabricated)", () => {
  it("browser shell: exactly the four demo clips, labels via the registered catalog", () => {
    const { result } = renderHook(() => useEnqueueSources(), {
      wrapper: clientWrapper(new StubTransport()),
    });
    expect(result.current.map((s) => s.id)).toEqual(["clip-bunny", "clip-ambient", "clip-live", "clip-radio"]);
    const bunny = result.current[0]!;
    expect(bunny.label).toBe("Big Buck Bunny (1080p, 10 min)");
    expect(bunny.title).toBe("Big Buck Bunny (1080p, 10 min)");
    expect(bunny.source).toEqual({ kind: "file", path: "/media/demo/big-buck-bunny.mkv" });
    expect(result.current[2]!.source).toEqual({ kind: "url", url: "https://demo.lumen.dev/live/edge-hsl.m3u8" });
  });

  it("demo transport: remembered openings surface after the clips, most recent first", async () => {
    const transport = new DemoTransport();
    try {
      await transport.send({
        requestId: 1,
        cmd: "session.open",
        args: { source: { kind: "url", url: "https://example.com/live.m3u8" } },
      });
      await transport.send({
        requestId: 2,
        cmd: "session.open",
        args: { source: { kind: "file", path: "/tmp/movie.mkv" } },
      });
      const { result } = renderHook(() => useEnqueueSources(), { wrapper: clientWrapper(transport) });
      expect(result.current.map((s) => s.id)).toEqual([
        "clip-bunny",
        "clip-ambient",
        "clip-live",
        "clip-radio",
        "recent-0",
        "recent-1",
      ]);
      // The seam mirrors the transport's recentOpenings order verbatim
      // (demo-transport hands back [first-opened, …, latest] — its getter
      // comment claims "most recent first"; that mismatch is the demo
      // lane's, noted in the lane log). Labels: the file's basename; the
      // url's honest hostname fallback when it carries no title.
      expect(result.current[4]).toMatchObject({
        id: "recent-0",
        label: "example.com",
        source: { kind: "url", url: "https://example.com/live.m3u8" },
      });
      expect(result.current[5]).toMatchObject({
        id: "recent-1",
        label: "movie.mkv",
        title: "movie.mkv",
        source: { kind: "file", path: "/tmp/movie.mkv" },
      });
    } finally {
      transport.dispose(); // the demo clock + opening choreography timers
    }
  });

  it("dedups by location: a remembered opening matching a clip never doubles it", async () => {
    const transport = new DemoTransport();
    try {
      await transport.send({
        requestId: 1,
        cmd: "session.open",
        args: { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
      });
      const { result } = renderHook(() => useEnqueueSources(), { wrapper: clientWrapper(transport) });
      expect(result.current.map((s) => s.id)).toEqual(["clip-bunny", "clip-ambient", "clip-live", "clip-radio"]);
      expect(result.current.filter((s) => s.source.kind === "file" && s.source.path === "/media/demo/big-buck-bunny.mkv")).toHaveLength(1);
    } finally {
      transport.dispose();
    }
  });

  it("desktop shell: clips are hidden (the real app opens real files) — the honest empty seam", () => {
    const transport = new StubTransport();
    const hook = withDesktopShell(() => renderHook(() => useEnqueueSources(), { wrapper: clientWrapper(transport) }));
    expect(hook.result.current).toEqual([]);
  });
});

describe("enqueueEntries — the shared menu shape (both consumers)", () => {
  const SOURCES: readonly EnqueueSource[] = [
    { id: "a", label: "A clip", source: { kind: "file", path: "/a.mkv" }, title: "A clip" },
    { id: "b", label: "B stream", source: { kind: "url", url: "https://b.example/x" }, title: null },
  ];

  it("no sources → the honest disabled empty row; no separator, no footer", () => {
    const entries = enqueueEntries([], vi.fn(), "Nothing to add yet", "Open media", vi.fn());
    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({ id: "add-empty", labelKey: "Nothing to add yet", disabled: true });
  });

  it("sources → one row each (file/url icons), a separator, then the open-media footer", () => {
    const entries = enqueueEntries(SOURCES, vi.fn(), "Nothing to add yet", "Open media", vi.fn());
    const shape = entries.map((entry) =>
      "labelKey" in entry ? `${entry.id}|${entry.labelKey}|${entry.icon ?? "-"}` : `sep:${entry.id}`,
    );
    expect(shape).toEqual([
      "a|A clip|open-file",
      "b|B stream|network-stream",
      "sep:add-sep",
      "add-open-media|Open media|open-file",
    ]);
  });

  it("activating a source dispatches playLater with that source AND title; the footer opens media", () => {
    const playLater = vi.fn();
    const onOpenMedia = vi.fn();
    const entries = enqueueEntries(SOURCES, playLater, "Nothing to add yet", "Open media", onOpenMedia);
    const items = entries.filter((entry): entry is Extract<MenuEntry<string>, { labelKey: string }> => "labelKey" in entry);
    items[0]!.onSelect?.();
    expect(playLater).toHaveBeenCalledWith({ kind: "file", path: "/a.mkv" }, "A clip");
    items[1]!.onSelect?.();
    expect(playLater).toHaveBeenCalledWith({ kind: "url", url: "https://b.example/x" }, null);
    items[2]!.onSelect?.(); // the footer (after the separator)
    expect(onOpenMedia).toHaveBeenCalledTimes(1);
    expect(playLater).toHaveBeenCalledTimes(2); // the footer never enqueues
  });
});

// j14 #1 verification (k08 landed the migration): BOTH consumers build
// their add menus through the shared module — the retired ~45-line inline
// i04 copy (clip mapping, shell hiding, recents, dedup) must stay dead.
describe("enqueueSources — cross-consumer guard (j14 #1)", () => {
  const dirUrl = import.meta.url.slice(0, import.meta.url.lastIndexOf("/") + 1);
  const toPath = (url: string): string => url.replace(/^file:\/\//, "");
  const readSource = async (relative: string): Promise<string> => {
    const fs = (await import("node:fs")) as { readFileSync(path: string, enc: "utf8"): string };
    return fs.readFileSync(toPath(`${dirUrl}${relative}`), "utf8");
  };

  it("the queue panel and the MiniPlayer both consume the shared seam — no inline copy", async () => {
    const consumers: ReadonlyArray<[string, string]> = [
      ["PlaylistPanel", await readSource("PlaylistPanel.tsx")],
      ["MiniPlayer", await readSource("../player/variants/MiniPlayer.tsx")],
    ];
    for (const [name, src] of consumers) {
      expect(src, `${name}: imports the shared module`).toContain("enqueueSources");
      expect(src, `${name}: uses the hook`).toContain("useEnqueueSources(");
      expect(src, `${name}: uses the entry builder`).toContain("enqueueEntries(");
      // the retired inline-copy pieces must stay retired:
      expect(src.includes("DEMO_CLIPS"), `${name}: inline clip mapping`).toBe(false);
      expect(src.includes("recentOpenings"), `${name}: inline recents read`).toBe(false);
      expect(src.includes("isDemoTransport"), `${name}: inline transport sniffing`).toBe(false);
    }
  });
});
