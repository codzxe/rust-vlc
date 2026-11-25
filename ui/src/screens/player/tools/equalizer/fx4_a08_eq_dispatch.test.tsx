// FX4-A08 (wave FX4, plan F1.1) — the EQ panel's REAL IPC dispatch pins.
//
// The wave-15 audit finding (W15-A01/R1/A09): the equalizer UI was a
// 10-band pretense with faders interactive-but-LOCAL-ONLY — no command ever
// left the panel (no audio.set-eq command existed). This file pins the fix:
// the panel is the engine's 3-band truth (Eq3Band) and every fader commit /
// preset pick dispatches the REAL `audio.set-eq` verb through the SAME
// transport lane the working player controls ride (PlayerControlsProvider's
// single dispatch — ControlBar's volume/mute/transport use the identical
// seam). The transport is a recording fake (the ControlBar.test FakeTransport
// convention); assertions are on the dispatched verb ids + payload shapes.
import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../../client/ClientProvider";
import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
} from "../../../../client/transport";
import { PlayerControlsProvider, usePlayerControls } from "../../context";
import type { PlayerState } from "../../context";
import { EqualizerTool } from "./EqualizerTool";
import { EQ_PRESETS } from "./presets";

/** Recording transport (the ControlBar.test FakeTransport convention):
 * every command is recorded; results are per-test via `respond`. */
class RecordingTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  respond: (request: CommandRequest) => CommandResponse["result"] = () => ({
    completed: { kind: "unit" },
  });

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return {
      gen: this.sent.length,
      requestId: request.requestId,
      result: this.respond(request),
    };
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}

/** The provider-state probe (context.test's StateProbe convention). */
function StateProbe({ onState }: { onState: (state: PlayerState) => void }) {
  const { state } = usePlayerControls();
  onState(state);
  return null;
}

/** Mount the tool inside the REAL player wiring (ClientProvider +
 * PlayerControlsProvider — the same chain PlayerScreen mounts). */
function mountTool(transport: RecordingTransport, onState?: (s: PlayerState) => void) {
  return render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider transport={transport}>
        {onState ? <StateProbe onState={onState} /> : null}
        <EqualizerTool defaultOpen />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
}

/** Only the audio.set-eq requests (the provider's mount-time probes
 * legitimately dispatch system.get-state-snapshot / media.get-*). */
const eqRequests = (transport: RecordingTransport): CommandRequest[] =>
  transport.sent.filter((request) => request.cmd === "audio.set-eq");

const flush = async (): Promise<void> => {
  await act(async () => {});
};

describe("FX4-A08 — the EQ panel calls the real IPC verb (wave FX4, F1.1)", () => {
  it("fx4_a08_eq_panel_is_the_engine_three_band_truth", () => {
    const transport = new RecordingTransport();
    mountTool(transport);
    // Preamp + exactly the engine's Eq3Band trio, frequencies in the
    // visible labels — the retired 10-band pretense is gone.
    const labels = ["Preamp", "Low (250 Hz)", "Mid (1 kHz)", "High (4 kHz)"];
    for (const label of labels) {
      expect(screen.getByRole("slider", { name: label })).toHaveAttribute(
        "aria-orientation",
        "vertical",
      );
    }
    expect(screen.getAllByRole("slider")).toHaveLength(4);
    // The ±20 dB UI range sits inside the engine's ±40 dB runtime bound.
    for (const slider of screen.getAllByRole("slider")) {
      expect(slider).toHaveAttribute("aria-valuemin", "-20");
      expect(slider).toHaveAttribute("aria-valuemax", "20");
    }
  });

  it("fx4_a08_eq_fader_commit_dispatches_audio_set_eq", async () => {
    const transport = new RecordingTransport();
    mountTool(transport);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    await flush();
    const eq = eqRequests(transport);
    expect(eq).toHaveLength(1);
    // The exact verb id + payload: the engine setter's shape (low/mid/high
    // dB + preamp dB), camelCase per the wire convention.
    expect(eq[0]).toMatchObject({
      cmd: "audio.set-eq",
      args: { lowDb: 0.1, midDb: 0, highDb: 0, preampDb: 0 },
    });
    expect(typeof eq[0]?.requestId).toBe("number");
  });

  it("fx4_a08_eq_request_ids_are_monotonic_across_commits", async () => {
    // doc 05 §4: request ids are the anti-replay monotonic counter — the
    // pre-wave audit found hardcoded 1/2 ids elsewhere (W15-A10); the eq
    // lane inherits the provider's counter and must never regress to a
    // constant.
    const transport = new RecordingTransport();
    mountTool(transport);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    fireEvent.keyDown(screen.getByRole("slider", { name: "Mid (1 kHz)" }), {
      key: "ArrowDown",
    });
    await flush();
    const eq = eqRequests(transport);
    expect(eq).toHaveLength(2);
    expect((eq[1]?.requestId ?? 0) > (eq[0]?.requestId ?? 0)).toBe(true);
  });

  it("fx4_a08_eq_preset_pick_dispatches_audio_set_eq", async () => {
    const transport = new RecordingTransport();
    mountTool(transport);
    fireEvent.click(screen.getByRole("button", { name: "Equalizer preset" }));
    fireEvent.click(screen.getByRole("option", { name: "Full bass" }));
    await flush();
    const eq = eqRequests(transport);
    expect(eq).toHaveLength(1);
    const fullBass = EQ_PRESETS.find((preset) => preset.id === "full-bass");
    expect(eq[0]).toMatchObject({
      cmd: "audio.set-eq",
      args: {
        lowDb: fullBass?.gains[0],
        midDb: fullBass?.gains[1],
        highDb: fullBass?.gains[2],
        preampDb: 0,
      },
    });
  });

  it("fx4_a08_eq_tool_no_longer_carries_staged_markers", () => {
    // The dead-widget finding inverted: the pre-wave tool carried
    // data-staged-tool / data-pending-command + an in-body note naming an
    // awaited command. The verb is dispatched now — every staging marker
    // must stay retired (a regression to decorative state fails here).
    const transport = new RecordingTransport();
    const { container } = mountTool(transport);
    const tool = container.querySelector('[data-tool="equalizer"]');
    expect(tool).not.toBeNull();
    expect(tool).not.toHaveAttribute("data-staged-tool");
    expect(tool).not.toHaveAttribute("data-pending-command");
    expect(container.querySelector(".eq-tool__staged")).toBeNull();
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("fx4_a08_eq_engine_refusal_surfaces_not_swallowed", async () => {
    // A typed engine refusal (e.g. a band beyond the ±40 dB runtime bound)
    // must surface through the provider's lastError — never silently
    // swallowed (the provider's own §5.4 contract; the tool keeps the
    // faders as the editing surface and the error chip tells the truth).
    const transport = new RecordingTransport();
    let latest: PlayerState | null = null;
    mountTool(transport, (state) => (latest = state));
    // Only the eq verb answers the typed refusal (the mount-time probes
    // answer completed) — so the surfaced error is unambiguously the eq one.
    transport.respond = (request) =>
      request.cmd === "audio.set-eq"
        ? {
            error: {
              code: "invalid-argument",
              details: { cmd: "audio.set-eq" },
              message: "band gain beyond the runtime bound",
              stage: "control",
            },
          }
        : { completed: { kind: "unit" } };
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    await flush();
    expect(latest?.lastError?.code).toBe("invalid-argument");
    expect(latest?.lastError?.details).toMatchObject({ cmd: "audio.set-eq" });
  });

  it("fx4_a08_eq_without_player_context_falls_back_to_the_local_seam", () => {
    // Bare mounts (the dev harness, jsdom test trees without the player
    // composition) get no provider — usePlayerControlsOrNull returns null
    // and the tool keeps its LOCAL editing contract: the onChange seam
    // fires, nothing crashes, no dispatch is attempted.
    const onChange = vi.fn();
    render(<EqualizerTool defaultOpen onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    expect(screen.getAllByText("0.1 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      preamp: 0,
      gains: [0.1, 0, 0],
      preset: "none",
    });
  });
});
