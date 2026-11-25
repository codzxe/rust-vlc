// FX4-A08 (wave FX4, plan F1.1) — the volume slider + mute button's REAL
// IPC pins. Wave-15 verified this wiring REAL end-to-end (ControlBar:477-482
// → context.tsx's `audio.set-volume`/`audio.set-mute` dispatch → the client
// transport → lumen_dispatch); these tests are the fix-wave's regression
// guards: if the slider ever regresses to dead local state (an onCommit
// detached, a payload reshaped, the round-trip broken), the lane fails here.
// Transport: the ControlBar.test FakeTransport convention (recorded sends,
// scripted results).
import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { secondsToUs } from "../../../client/wire-time";
import type {
  CommandRequest,
  CommandResponse,
  Event,
  PlaybackState,
} from "../../../client/transport";
import { PlayerControlsProvider } from "../context";
import { PlaylistProvider } from "../../playlist";
import { ControlBar } from "./ControlBar";

class FakeTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  respond: (request: CommandRequest) => CommandResponse["result"] = () => ({
    completed: { kind: "unit" },
  });

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return { gen: this.sent.length, requestId: request.requestId, result: this.respond(request) };
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }
}

/** An engine snapshot answer (the ControlBar.test snapshotResult helper,
 * seconds in, µs Timestamps on the wire). */
const snapshotResult = (
  state: PlaybackState,
  volume: number,
  muted: boolean,
): CommandResponse["result"] =>
  ({
    completed: {
      kind: "state-snapshot",
      snapshot: {
        audioDelayMs: 0,
        devices: [],
        duration: { kind: "known", time: secondsToUs(100) },
        muted,
        playlist: {},
        rate: 1,
        selections: {},
        state,
        subtitle: {},
        subtitleDelayMs: 0,
        time: secondsToUs(0),
        tracks: [],
        videoDelayMs: 0,
        volume,
      },
    },
  }) as unknown as CommandResponse["result"];

// The provider chain PlayerScreen mounts around the bar.
function renderBar(transport: FakeTransport) {
  return render(
    <ClientProvider transport={transport}>
      <PlaylistProvider>
        <PlayerControlsProvider>
          <ControlBar />
        </PlayerControlsProvider>
      </PlaylistProvider>
    </ClientProvider>,
  );
}

const flush = async (): Promise<void> => {
  await act(async () => {});
};

describe("FX4-A08 — volume/mute call the real IPC verbs (wave FX4, F1.1)", () => {
  it("fx4_a08_volume_slider_commit_dispatches_audio_set_volume", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", 0.5, false);
    renderBar(fake);
    await flush();
    fireEvent.keyDown(screen.getByRole("slider", { name: "Volume" }), {
      key: "ArrowUp",
    });
    await flush();
    const set = fake.sent.find((request) => request.cmd === "audio.set-volume");
    // The catalog's exact verb id + payload: args.volume a plain 0…1 wire
    // scalar (f32), exactly the committed value — never a percent, never a
    // reshaped field.
    expect(set).toBeDefined();
    expect(set?.args).toEqual({ volume: 0.51 });
    expect(typeof set?.requestId).toBe("number");
  });

  it("fx4_a08_volume_round_trips_from_engine_reported_state", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", 0.5, false);
    renderBar(fake);
    await flush();
    const slider = screen.getByRole("slider", { name: "Volume" });
    // The engine's snapshot report drives the fader (no dead local state).
    expect(slider.getAttribute("aria-valuenow")).toBe("0.5");
    // The completion — not optimism — is what the fader reflects next.
    fake.respond = () => ({ completed: { kind: "audio-volume", volume: 0.4 } });
    fireEvent.keyDown(slider, { key: "ArrowUp" });
    await flush();
    expect(slider.getAttribute("aria-valuenow")).toBe("0.4");
  });

  it("fx4_a08_mute_button_dispatches_audio_set_mute", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", 0.5, false);
    renderBar(fake);
    await flush();
    fireEvent.click(screen.getByRole("button", { name: "Mute" }));
    await flush();
    const mute = fake.sent.find((request) => request.cmd === "audio.set-mute");
    expect(mute).toBeDefined();
    expect(mute?.args).toEqual({ muted: true });
  });

  it("fx4_a08_unmute_dispatches_audio_set_mute_false", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", 0.5, true);
    renderBar(fake);
    await flush();
    fireEvent.click(screen.getByRole("button", { name: "Unmute" }));
    await flush();
    const mute = fake.sent.find((request) => request.cmd === "audio.set-mute");
    expect(mute).toBeDefined();
    expect(mute?.args).toEqual({ muted: false });
  });

  it("fx4_a08_volume_and_toggle_ride_the_single_transport_lane", async () => {
    // The wave-15 UI truth: every working control rides the ONE dispatch
    // lane (the transport behind lumen_dispatch). Volume and the transport
    // toggle must appear on the SAME recorded transport — never a second
    // side channel for audio.
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", 0.5, false);
    renderBar(fake);
    await flush();
    fireEvent.click(screen.getByRole("button", { name: /pause/i }));
    fireEvent.keyDown(screen.getByRole("slider", { name: "Volume" }), {
      key: "ArrowUp",
    });
    await flush();
    const cmds = fake.sent.map((request) => request.cmd);
    expect(cmds).toContain("playback.toggle-play-pause");
    expect(cmds).toContain("audio.set-volume");
  });
});
