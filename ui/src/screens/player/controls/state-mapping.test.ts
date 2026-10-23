// Card 4.C2 — the doc 08 §5.1 state-mapping contract, parametrized per
// engine state. The table IS the spec: these tests pin every row.
import { describe, expect, it } from "vitest";
import type { PlaybackState } from "../../../client/transport";
import {
  controlCommandOf,
  mapTransportControls,
  seekDisableReason,
  type TransportMappingInputs,
} from "./state-mapping";

const inputs = (overrides: Partial<TransportMappingInputs>): TransportMappingInputs => ({
  playbackState: "playing",
  seekable: null,
  durationSeconds: 100,
  pendingToggle: false,
  ...overrides,
});

describe("mapTransportControls — doc 08 §5.1 PlayButton table", () => {
  const cases: ReadonlyArray<{
    readonly state: PlaybackState;
    readonly glyph: "play" | "pause";
    readonly disabled: boolean;
    readonly pending: boolean;
    readonly restart: boolean;
    readonly busy: boolean;
  }> = [
    // idle: disabled ("no media open"), play glyph at rest.
    { state: "idle", glyph: "play", disabled: true, pending: false, restart: false, busy: false },
    // opening: pending (session.open accepted) — never optimistic play.
    { state: "opening", glyph: "play", disabled: true, pending: true, restart: false, busy: false },
    // playing: pause glyph.
    { state: "playing", glyph: "pause", disabled: false, pending: false, restart: false, busy: false },
    // paused: play glyph.
    { state: "paused", glyph: "play", disabled: false, pending: false, restart: false, busy: false },
    // buffering: [PROPOSAL] disabled-with-progress-hint (doc table silent).
    { state: "buffering", glyph: "pause", disabled: true, pending: false, restart: false, busy: true },
    // ended: play glyph + restart affordance.
    { state: "ended", glyph: "play", disabled: false, pending: false, restart: true, busy: false },
    // stopped: [PROPOSAL] doc omits the column — restart affordance.
    { state: "stopped", glyph: "play", disabled: false, pending: false, restart: true, busy: false },
    // error: play + error surface elsewhere (§5.4); retry stays possible.
    { state: "error", glyph: "play", disabled: false, pending: false, restart: false, busy: false },
  ];

  it.each(cases)("$state → glyph=$glyph disabled=$disabled pending=$pending", (row) => {
    const mapped = mapTransportControls(inputs({ playbackState: row.state }));
    expect(mapped.play).toEqual({
      glyph: row.glyph,
      disabled: row.disabled,
      pending: row.pending,
      restart: row.restart,
      busy: row.busy,
    });
    expect(mapped.playback).toBe(row.state);
  });

  it("pendingToggle marks an accepted play/pause in playing (≤250ms pending)", () => {
    const mapped = mapTransportControls(inputs({ playbackState: "playing", pendingToggle: true }));
    expect(mapped.play.pending).toBe(true);
    expect(mapped.play.glyph).toBe("pause");
  });

  it("pendingToggle marks an accepted play/pause in paused (idem)", () => {
    const mapped = mapTransportControls(inputs({ playbackState: "paused", pendingToggle: true }));
    expect(mapped.play.pending).toBe(true);
  });
});

describe("mapTransportControls — stop button", () => {
  it.each<PlaybackState>(["idle", "opening", "stopped", "ended", "error"])(
    "%s → stop disabled (nothing active to stop)",
    (state) => {
      expect(mapTransportControls(inputs({ playbackState: state })).stop.disabled).toBe(true);
    },
  );
  it.each<PlaybackState>(["playing", "paused", "buffering"])(
    "%s → stop enabled",
    (state) => {
      expect(mapTransportControls(inputs({ playbackState: state })).stop.disabled).toBe(false);
    },
  );
});

describe("mapTransportControls — seek (doc 08 §5.2/§5.3)", () => {
  it("disabled when duration is unknown", () => {
    const mapped = mapTransportControls(inputs({ playbackState: "playing", durationSeconds: null }));
    expect(mapped.seek.disabled).toBe(true);
    expect(seekDisableReason(inputs({ playbackState: "playing", durationSeconds: null }))).toBe(
      "player.controls.reason.no-media",
    );
  });

  it("disabled-with-reason when the engine REPORTS non-seekable (never guesswork)", () => {
    const i = inputs({ playbackState: "playing", seekable: false });
    expect(mapTransportControls(i).seek.disabled).toBe(true);
    expect(seekDisableReason(i)).toBe("player.controls.reason.not-seekable");
  });

  it("enabled while playing with known duration and no non-seekable report", () => {
    const mapped = mapTransportControls(inputs({ playbackState: "playing" }));
    expect(mapped.seek.disabled).toBe(false);
  });

  it("aria-busy (busy) while buffering — knob never hidden, §5.2", () => {
    const mapped = mapTransportControls(inputs({ playbackState: "buffering", durationSeconds: 100 }));
    expect(mapped.seek.busy).toBe(true);
    expect(mapped.seek.disabled).toBe(false);
  });
});

describe("controlCommandOf — keymap seam (data-driven)", () => {
  it("maps the owned no-arg playback commands", () => {
    expect(controlCommandOf({ command: "playback.toggle-play-pause" })).toBe(
      "playback.toggle-play-pause",
    );
    expect(controlCommandOf({ command: "playback.stop" })).toBe("playback.stop");
  });
  it("returns null for anything else (shell's onBinding keeps it)", () => {
    expect(controlCommandOf({})).toBeNull();
    expect(controlCommandOf({ command: "audio.set-mute" })).toBeNull();
  });
});
