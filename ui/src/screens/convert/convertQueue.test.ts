// convertQueue.test.ts — pins the pure job-queue model (card 7.E3):
// the lifecycle state machine, the bounded queue, the honest cancel
// (partial progress kept), the total-reducer no-ops on illegal events,
// the engine-lane probe against REAL catalog shapes, and the percent
// formatter.

import { describe, expect, it } from "vitest";
import type { CommandListing } from "../../generated/commands";
import type { ConvertJob, ConvertQueueEvent } from "./convertQueue";
import {
  MAX_CONVERT_QUEUE_JOBS,
  canEnqueue,
  cancellable,
  convertQueueReduce,
  formatProgressLabel,
  laneCommandsOf,
  nextJobId,
  sourceBasename,
} from "./convertQueue";

const run = (queue: readonly ConvertJob[], ...events: ConvertQueueEvent[]): readonly ConvertJob[] =>
  events.reduce((acc, event) => convertQueueReduce(acc, event), queue);

const listing = (id: string): CommandListing => ({
  class: "s",
  deprecated: null,
  id,
  requires: [],
  since: "0.1.0",
});

describe("convertQueueReduce — lifecycle", () => {
  it("enqueues a queued job with a monotonic id and no progress", () => {
    const queue = run([], { kind: "enqueue", sourcePath: "/a/bunny.mkv", target: "mp4" });
    expect(queue).toHaveLength(1);
    expect(queue[0]).toMatchObject({
      id: 1,
      sourcePath: "/a/bunny.mkv",
      target: "mp4",
      state: "queued",
      progress: null,
      detail: null,
    });
    const two = run(queue, { kind: "enqueue", sourcePath: "/a/tone.wav", target: "wav" });
    expect(two[1]!.id).toBe(2);
    expect(nextJobId(two)).toBe(3);
  });

  it("runs the full happy path: start → progress → complete", () => {
    const queue = run(
      [],
      { kind: "enqueue", sourcePath: "/a/x.flac", target: "wav" },
      { kind: "start", id: 1 },
      { kind: "progress", id: 1, fraction: 0.25 },
      { kind: "progress", id: 1, fraction: 0.9 },
      { kind: "complete", id: 1 },
    );
    expect(queue[0]).toMatchObject({ state: "completed", progress: 1 });
  });

  it("cancel of a running job keeps the last progress — the honest partial", () => {
    const queue = run(
      [],
      { kind: "enqueue", sourcePath: "/a/x.flac", target: "wav" },
      { kind: "start", id: 1 },
      { kind: "progress", id: 1, fraction: 0.4 },
      { kind: "cancel", id: 1 },
    );
    expect(queue[0]).toMatchObject({ state: "cancelled", progress: 0.4 });
  });

  it("fail records the reason; block records the not-exposed reason", () => {
    const failed = run(
      [],
      { kind: "enqueue", sourcePath: "/a/x.mp4", target: "mp4" },
      { kind: "start", id: 1 },
      { kind: "fail", id: 1, reason: "engine: unsupported codec" },
    );
    expect(failed[0]).toMatchObject({ state: "failed", detail: "engine: unsupported codec" });
    const blocked = run(
      [],
      { kind: "enqueue", sourcePath: "/a/x.mp4", target: "mp4" },
      { kind: "block", id: 1, reason: "feature-not-exposed" },
    );
    expect(blocked[0]).toMatchObject({ state: "blocked", detail: "feature-not-exposed" });
  });

  it("is total: unknown ids and illegal transitions are no-ops, never throws", () => {
    const queue = run([], { kind: "enqueue", sourcePath: "/a/x", target: "wav" });
    // unknown id
    expect(run(queue, { kind: "start", id: 99 })).toBe(queue);
    // complete without start (still queued)
    expect(run(queue, { kind: "complete", id: 1 })).toBe(queue);
    expect(run(queue, { kind: "progress", id: 1, fraction: 0.5 })).toBe(queue);
    // block after the job left queued
    const running = run(queue, { kind: "start", id: 1 });
    expect(run(running, { kind: "block", id: 1, reason: "x" })).toBe(running);
    // cancel of a terminal job
    const done = run(running, { kind: "complete", id: 1 });
    expect(run(done, { kind: "cancel", id: 1 })).toBe(done);
  });

  it("clamps dirty progress fractions into 0..1", () => {
    const queue = run(
      [],
      { kind: "enqueue", sourcePath: "/a/x", target: "wav" },
      { kind: "start", id: 1 },
      { kind: "progress", id: 1, fraction: 7 },
    );
    expect(queue[0]!.progress).toBe(1);
    const negative = run(
      [],
      { kind: "enqueue", sourcePath: "/a/x", target: "wav" },
      { kind: "start", id: 1 },
      { kind: "progress", id: 1, fraction: -0.5 },
    );
    expect(negative[0]!.progress).toBe(0);
  });
});

describe("convertQueueReduce — the bound", () => {
  it("refuses the (MAX+1)-th row (no-op) and canEnqueue flips at the bound", () => {
    let queue: readonly ConvertJob[] = [];
    for (let i = 0; i < MAX_CONVERT_QUEUE_JOBS; i += 1) {
      queue = run(queue, { kind: "enqueue", sourcePath: `/a/x${i}`, target: "wav" });
    }
    expect(queue).toHaveLength(MAX_CONVERT_QUEUE_JOBS);
    expect(canEnqueue(queue)).toBe(false);
    const over = run(queue, { kind: "enqueue", sourcePath: "/a/one-too-many", target: "wav" });
    expect(over).toBe(queue);
    expect(over).toHaveLength(MAX_CONVERT_QUEUE_JOBS);
    expect(MAX_CONVERT_QUEUE_JOBS).toBe(8);
  });

  it("cancellable admits exactly queued and running", () => {
    expect(cancellable("queued")).toBe(true);
    expect(cancellable("running")).toBe(true);
    for (const state of ["completed", "failed", "cancelled", "blocked"] as const) {
      expect(cancellable(state)).toBe(false);
    }
  });
});

describe("laneCommandsOf — the engine-lane probe", () => {
  it("finds convert/record namespaced commands in a listing", () => {
    const found = laneCommandsOf([
      listing("session.open"),
      listing("convert.start"),
      listing("record.stop"),
      listing("playback.play"),
    ]);
    expect(found.map((l) => l.id)).toEqual(["convert.start", "record.stop"]);
  });

  it("returns empty for the REAL 64-command engine catalog namespaces — the honest today", () => {
    // The namespaces that exist in crates/commands today (doc 05 §2):
    // session, playback, tracks, audio, video, media, playlist, config,
    // plugins, system. No convert.* / record.* row exists.
    const realNamespaces = [
      "session.open",
      "playback.play",
      "tracks.list",
      "audio.set-volume",
      "video.take-screenshot",
      "media.get-info",
      "playlist.next",
      "config.get-setting",
      "plugins.list",
      "system.list-commands",
    ];
    expect(laneCommandsOf(realNamespaces.map(listing))).toEqual([]);
  });

  it("returns empty for an empty or failed listing (absence is never proof)", () => {
    expect(laneCommandsOf([])).toEqual([]);
  });
});

describe("formatProgressLabel", () => {
  it("formats fractions as rounded percents, clamped", () => {
    expect(formatProgressLabel(0)).toBe("0%");
    expect(formatProgressLabel(0.25)).toBe("25%");
    expect(formatProgressLabel(0.999)).toBe("100%");
    expect(formatProgressLabel(1.5)).toBe("100%");
    expect(formatProgressLabel(-1)).toBe("0%");
  });
});

describe("sourceBasename", () => {
  it("takes the final segment across both separator kinds", () => {
    expect(sourceBasename("/media/demo/bunny.mkv")).toBe("bunny.mkv");
    expect(sourceBasename("C:\\Media\\bunny.mkv")).toBe("bunny.mkv");
    expect(sourceBasename("bunny.mkv")).toBe("bunny.mkv");
    expect(sourceBasename("/media/dir/")).toBe("");
  });
});
