// w26.b10 — sorting model tests (pure; `Sorting Options.svg` rows 1–16).
// Natural order, honest unknowns (no metadata → no reorder), stability.

import { describe, expect, it } from "vitest";
import type { PlaylistEntry } from "../PlaylistProvider";
import {
  DEFAULT_SORT_KEY,
  SORT_KEYS,
  entryDisplayTitle,
  formatDurationClock,
  sortLabelKey,
  sortPlaylistEntries,
  sortUsesMeta,
  type PlaylistEntryMeta,
} from "./sorting";

const entry = (id: number, title: string | null, path: string | null, durationSeconds: number | null = null): PlaylistEntry => ({
  id,
  title,
  source: path === null ? null : { kind: "file", path },
  durationSeconds,
});

const ENTRIES: PlaylistEntry[] = [
  entry(1, "Episode 10", "/tv/e10.mkv"),
  entry(2, "Episode 2", "/tv/e2.mkv"),
  entry(3, null, "/tv/Episode 1.mp4"),
  entry(4, "episode 2", "/tv/e2b.mkv"),
];

describe("SORT_KEYS — frame data (Sorting Options.svg rows 1–16)", () => {
  it("lists the 16 frame options in row order", () => {
    expect(SORT_KEYS).toHaveLength(16);
    expect(SORT_KEYS[0]).toBe("unwatched-first");
    expect(SORT_KEYS[4]).toBe("title-asc");
    expect(SORT_KEYS[15]).toBe("disc-number-desc");
  });

  it("frame default is row 1 (Unwatched First)", () => {
    expect(DEFAULT_SORT_KEY).toBe("unwatched-first");
  });

  it("every key maps to its lane i18n key", () => {
    expect(sortLabelKey("title-asc")).toBe("playlist.options.sort.title-asc");
  });
});

describe("sortPlaylistEntries — natural title/URI order", () => {
  it("title-asc sorts naturally: Episode 2 before Episode 10 (never lexical)", () => {
    const titles = sortPlaylistEntries(ENTRIES, "title-asc").map((e) => entryDisplayTitle(e, ""));
    // Entry 3 has no title — its source basename "Episode 1.mp4" is the key.
    expect(titles).toEqual(["Episode 1.mp4", "Episode 2", "episode 2", "Episode 10"]);
  });

  it("title-desc inverts the natural order", () => {
    const titles = sortPlaylistEntries(ENTRIES, "title-desc").map((e) => entryDisplayTitle(e, ""));
    expect(titles).toEqual(["Episode 10", "episode 2", "Episode 2", "Episode 1.mp4"]);
  });

  it("uri-asc/uri-desc sort by source location (e2 < e2b < e10, naturally)", () => {
    const uris = sortPlaylistEntries(ENTRIES, "uri-asc").map((e) => e.source?.kind === "file" ? e.source.path : "");
    expect(uris).toEqual(["/tv/e2.mkv", "/tv/e2b.mkv", "/tv/e10.mkv", "/tv/Episode 1.mp4"]);
    const urisDesc = sortPlaylistEntries(ENTRIES, "uri-desc").map((e) => e.source?.kind === "file" ? e.source.path : "");
    expect(urisDesc).toEqual([...uris].reverse());
  });
});

describe("sortPlaylistEntries — honest unknowns (no metadata seam)", () => {
  it("watched/unwatched sorts are stable no-ops without metadata", () => {
    expect(sortPlaylistEntries(ENTRIES, "unwatched-first").map((e) => e.id)).toEqual([1, 2, 3, 4]);
    expect(sortPlaylistEntries(ENTRIES, "watched-first").map((e) => e.id)).toEqual([1, 2, 3, 4]);
  });

  it("artist/album/track/disc sorts keep queue order when metadata is absent", () => {
    for (const key of ["artist-asc", "album-desc", "track-number-asc", "disc-number-desc"] as const) {
      expect(sortPlaylistEntries(ENTRIES, key).map((e) => e.id)).toEqual([1, 2, 3, 4]);
    }
  });

  it("newest/oldest fall back to queue position (append order)", () => {
    expect(sortPlaylistEntries(ENTRIES, "oldest-first").map((e) => e.id)).toEqual([1, 2, 3, 4]);
    expect(sortPlaylistEntries(ENTRIES, "newest-first").map((e) => e.id)).toEqual([4, 3, 2, 1]);
  });
});

describe("sortPlaylistEntries — metadata seam", () => {
  const meta: Record<number, PlaylistEntryMeta> = {
    1: { watched: true, artist: "Bee", album: "Zeta", trackNumber: 3, discNumber: 2, addedAt: 400 },
    2: { watched: false, artist: "Adele", album: "Alpha", trackNumber: 10, discNumber: 1, addedAt: 300 },
    3: { artist: "Cee", album: "Beta", trackNumber: 2, addedAt: 200 },
    4: { watched: true, trackNumber: 1, addedAt: 100 },
  };
  const resolve = (e: PlaylistEntry): PlaylistEntryMeta | undefined => meta[e.id];

  it("unwatched-first puts unwatched before watched", () => {
    expect(sortPlaylistEntries(ENTRIES, "unwatched-first", resolve).map((e) => e.id)).toEqual([2, 3, 1, 4]);
  });

  it("watched-first inverts it", () => {
    expect(sortPlaylistEntries(ENTRIES, "watched-first", resolve).map((e) => e.id)).toEqual([1, 4, 2, 3]);
  });

  it("artist-asc sorts by artist; unknown artist stays last (honest)", () => {
    expect(sortPlaylistEntries(ENTRIES, "artist-asc", resolve).map((e) => e.id)).toEqual([2, 1, 3, 4]);
  });

  it("artist-desc reverses known artists, unknown still last", () => {
    expect(sortPlaylistEntries(ENTRIES, "artist-desc", resolve).map((e) => e.id)).toEqual([3, 1, 2, 4]);
  });

  it("album-asc / album-desc order by album name", () => {
    expect(sortPlaylistEntries(ENTRIES, "album-asc", resolve).map((e) => e.id)).toEqual([2, 3, 1, 4]);
    expect(sortPlaylistEntries(ENTRIES, "album-desc", resolve).map((e) => e.id)).toEqual([1, 3, 2, 4]);
  });

  it("track-number-asc sorts numerically (2 before 10)", () => {
    expect(sortPlaylistEntries(ENTRIES, "track-number-asc", resolve).map((e) => e.id)).toEqual([4, 3, 1, 2]);
    expect(sortPlaylistEntries(ENTRIES, "track-number-desc", resolve).map((e) => e.id)).toEqual([2, 1, 3, 4]);
  });

  it("disc-number sorts numerically; unknown last", () => {
    expect(sortPlaylistEntries(ENTRIES, "disc-number-asc", resolve).map((e) => e.id)).toEqual([2, 1, 3, 4]);
    expect(sortPlaylistEntries(ENTRIES, "disc-number-desc", resolve).map((e) => e.id)).toEqual([1, 2, 3, 4]);
  });

  it("newest/oldest order by addedAt when the seam reports it", () => {
    expect(sortPlaylistEntries(ENTRIES, "newest-first", resolve).map((e) => e.id)).toEqual([1, 2, 3, 4]);
    expect(sortPlaylistEntries(ENTRIES, "oldest-first", resolve).map((e) => e.id)).toEqual([4, 3, 2, 1]);
  });
});

describe("sortPlaylistEntries — contract details", () => {
  it("never mutates the input array", () => {
    const input = [...ENTRIES];
    sortPlaylistEntries(ENTRIES, "title-asc");
    expect(ENTRIES).toEqual(input);
  });

  it("ties keep queue order (total stability)", () => {
    const dupes = [entry(1, "Same", "/a"), entry(2, "Same", "/b"), entry(3, "Same", "/c")];
    expect(sortPlaylistEntries(dupes, "title-desc").map((e) => e.id)).toEqual([1, 2, 3]);
  });
});

describe("helpers", () => {
  it("formatDurationClock renders zero-padded h:mm:ss (thumbnail.svg 00:02:10)", () => {
    expect(formatDurationClock(130)).toBe("00:02:10");
    expect(formatDurationClock(3725)).toBe("01:02:05");
    expect(formatDurationClock(null)).toBeNull();
    expect(formatDurationClock(-5)).toBeNull();
  });

  it("entryDisplayTitle: title → source basename → unknown", () => {
    expect(entryDisplayTitle(ENTRIES[0]!, "?")).toBe("Episode 10");
    expect(entryDisplayTitle(ENTRIES[2]!, "?")).toBe("Episode 1.mp4");
    expect(entryDisplayTitle(entry(9, null, null), "?")).toBe("?");
  });

  it("sortUsesMeta flags the metadata-dependent keys", () => {
    expect(sortUsesMeta("artist-asc")).toBe(true);
    expect(sortUsesMeta("unwatched-first")).toBe(true);
    expect(sortUsesMeta("title-asc")).toBe(false);
  });
});
