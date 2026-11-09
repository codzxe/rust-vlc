// DemoTransport bookmark semantics (issue #32) — the simulation mirrors
// the engine-side BookmarkList contract: creation-ordered ids from 1, no
// id reuse after delete, per-media reset on open, typed refusals for
// negative positions and unknown ids, and the commands listed for the
// CustomizePanels capability gate.
import { afterEach, describe, expect, it } from "vitest";
import { DemoTransport } from "./demo-transport";
import type { CommandResponse } from "./transport";

const transport = new DemoTransport();

afterEach(() => {
  transport.dispose();
});

const completed = (response: CommandResponse): CommandResponse["result"] =>
  response.result;

describe("DemoTransport — media.*-bookmark commands (issue #32)", () => {
  it("the trio is listed (capability gate)", async () => {
    const listing = await transport.send({
      cmd: "system.list-commands",
      requestId: 1,
    });
    if (!("completed" in listing.result)) throw new Error("expected completed");
    if (listing.result.completed.kind !== "command-listings") {
      throw new Error("expected command-listings");
    }
    const ids = listing.result.completed.commands.map((c) => c.id);
    expect(ids).toContain("media.get-bookmarks");
    expect(ids).toContain("media.add-bookmark");
    expect(ids).toContain("media.remove-bookmark");
  });

  it("add assigns ids from 1, remove does not renumber, get returns the truth", async () => {
    const a = completed(
      await transport.send({
        args: { time: 10_000_000, label: "Intro" },
        cmd: "media.add-bookmark",
        requestId: 2,
      }),
    );
    const b = completed(
      await transport.send({
        args: { time: 65_000_000, label: null },
        cmd: "media.add-bookmark",
        requestId: 3,
      }),
    );
    if (a.kind !== "bookmark-added" || b.kind !== "bookmark-added") {
      throw new Error("expected bookmark-added results");
    }
    expect(a.bookmark.id).toBe(1);
    expect(b.bookmark.id).toBe(2);
    expect(a.bookmark.label).toBe("Intro");
    expect(b.bookmark.label).toBeNull();

    const removed = completed(
      await transport.send({
        args: { id: 1 },
        cmd: "media.remove-bookmark",
        requestId: 4,
      }),
    );
    if (removed.kind !== "bookmarks") throw new Error("expected bookmarks result");
    expect(removed.bookmarks).toHaveLength(1);
    expect(removed.bookmarks[0]?.id).toBe(2);
  });

  it("removing an unknown id is a typed not-found refusal", async () => {
    const result = completed(
      await transport.send({
        args: { id: 99 },
        cmd: "media.remove-bookmark",
        requestId: 5,
      }),
    );
    if (!("error" in result)) throw new Error("expected an error envelope");
    expect(result.error.code).toBe("not-found");
  });

  it("a negative position is refused invalid-argument", async () => {
    const result = completed(
      await transport.send({
        args: { time: -1, label: null },
        cmd: "media.add-bookmark",
        requestId: 6,
      }),
    );
    if (!("error" in result)) throw new Error("expected an error envelope");
    expect(result.error.code).toBe("invalid-argument");
  });

  it("opening a new media resets the list and the numbering (per-media)", async () => {
    await transport.send({
      args: { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
      cmd: "session.open",
      requestId: 7,
    });
    await transport.send({
      args: { time: 5_000_000, label: "one" },
      cmd: "media.add-bookmark",
      requestId: 8,
    });
    // A second open clears it.
    await transport.send({
      args: { source: { kind: "file", path: "/media/demo/ambient-loop.flac" } },
      cmd: "session.open",
      requestId: 9,
    });
    const list = completed(
      await transport.send({ cmd: "media.get-bookmarks", requestId: 10 }),
    );
    if (list.kind !== "bookmarks") throw new Error("expected bookmarks result");
    expect(list.bookmarks).toEqual([]);
    // Numbering restarted.
    const next = completed(
      await transport.send({
        args: { time: 1_000_000, label: null },
        cmd: "media.add-bookmark",
        requestId: 11,
      }),
    );
    if (next.kind !== "bookmark-added") throw new Error("expected bookmark-added");
    expect(next.bookmark.id).toBe(1);
  });
});
