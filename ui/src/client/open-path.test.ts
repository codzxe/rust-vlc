import { describe, expect, it } from "vitest";
import type { SourceSpec } from "./transport";
import { classifyOpenSource, isPlaylistPath, openPathRequest, openPathRoute } from "./open-path";

// Issue #3 (4.H1 handoffs b + d): the ONE UI-side file-open funnel. A
// media source opens a session; a playlist file loads the list
// (`playlist.load {path, append: false}` — parse lives engine-side,
// crates/playlists, per 4.D1's handoff). Everything travels over the
// SAME transport command pattern; there is no second mechanism.

const file = (path: string): SourceSpec => ({ kind: "file", path });
const url = (href: string): SourceSpec => ({ kind: "url", url: href });

describe("isPlaylistPath — the suffix classifier", () => {
  it("admits every playlist format the engine's playlists crate reads", () => {
    for (const path of [
      "/tmp/queue.m3u",
      "/tmp/queue.M3U8",
      "/tmp/radio.pls",
      "/tmp/library.xspf",
      "/tmp/album.cue",
      "C:\\Users\\me\\list.M3u",
    ]) {
      expect(isPlaylistPath(path), path).toBe(true);
    }
  });

  it("refuses media, doubled suffixes, and extensionless paths (never a guess)", () => {
    for (const path of [
      "/tmp/movie.avi",
      "/tmp/queue.m3u.bak",
      "/tmp/queue.",
      "/tmp/noext",
      "/tmp/.m3u",
    ]) {
      expect(isPlaylistPath(path), path).toBe(false);
    }
  });
});

describe("classifyOpenSource — what opening means", () => {
  it("URLs are always media (a stream opens a session)", () => {
    expect(classifyOpenSource(url("https://ice.example/stream"))).toEqual({
      kind: "media",
      source: url("https://ice.example/stream"),
    });
  });

  it("playlist files load the list; media files open a session", () => {
    expect(classifyOpenSource(file("/tmp/queue.m3u"))).toEqual({
      kind: "playlist",
      path: "/tmp/queue.m3u",
    });
    expect(classifyOpenSource(file("/tmp/movie.mkv"))).toEqual({
      kind: "media",
      source: file("/tmp/movie.mkv"),
    });
  });
});

describe("openPathRequest — the wire request", () => {
  it("a playlist file becomes playlist.load with append: false (an open replaces the queue)", () => {
    expect(openPathRequest(file("/media/queue.xspf"), 12)).toEqual({
      requestId: 12,
      cmd: "playlist.load",
      args: { path: "/media/queue.xspf", append: false },
    });
  });

  it("a media file becomes session.open with the source verbatim", () => {
    expect(openPathRequest(file("/media/movie.mkv"), 13)).toEqual({
      requestId: 13,
      cmd: "session.open",
      args: { source: file("/media/movie.mkv") },
    });
  });

  it("a URL becomes session.open (never a playlist load)", () => {
    expect(openPathRequest(url("https://demo.example/live"), 14)).toEqual({
      requestId: 14,
      cmd: "session.open",
      args: { source: url("https://demo.example/live") },
    });
  });

  it("echoes the caller's request id in both branches", () => {
    expect(openPathRequest(file("/a.m3u"), 99).requestId).toBe(99);
    expect(openPathRequest(file("/a.mkv"), 98).requestId).toBe(98);
  });
});

describe("openPathRoute — where the app goes after opening", () => {
  it("playlist files route to the playlist screen; media routes to the player", () => {
    expect(openPathRoute(file("/tmp/queue.m3u"))).toBe("/playlist");
    expect(openPathRoute(file("/tmp/movie.mkv"))).toBe("/player");
    expect(openPathRoute(url("https://demo.example/live"))).toBe("/player");
  });
});
