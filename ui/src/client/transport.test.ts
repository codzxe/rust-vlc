import { describe, expect, it } from "vitest";
import { MockTransport } from "./transport";

describe("MockTransport (1.D4 wire types)", () => {
  it("answers system.list-commands with an empty completed listing", async () => {
    const transport = new MockTransport();
    const response = await transport.send({ requestId: 1, cmd: "system.list-commands" });
    expect(response.requestId).toBe(1);
    expect(response.result).toEqual({
      completed: { kind: "command-listings", commands: [] },
    });
  });

  it("returns the typed feature-not-exposed error for unsimulated commands", async () => {
    const transport = new MockTransport();
    const response = await transport.send({ requestId: 2, cmd: "session.close" });
    expect(response.result).toMatchObject({
      error: { code: "feature-not-exposed", stage: "control" },
    });
  });

  it("records issued requests for test assertions", async () => {
    const transport = new MockTransport();
    await transport.send({ requestId: 3, cmd: "playback.play" });
    expect(transport.sent.map((r) => r.cmd)).toEqual(["playback.play"]);
  });

  it("supports subscribe/unsubscribe without side effects", () => {
    const transport = new MockTransport();
    const seen: number[] = [];
    const unsubscribe = transport.subscribe(() => seen.push(1));
    unsubscribe();
    unsubscribe(); // idempotent
    expect(seen).toEqual([]);
  });
});
