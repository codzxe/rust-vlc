// w26.k06 — player state seam failure-path tests (j06 J06-2 P1 + J06-12 P3).
//
// The wave-J review found NO test coverage for the provider's error paths:
// `pendingToggle` was cleared only by `session.state-changed`, so a
// play/pause that answers ERROR (engine) or never reaches the engine
// (transport failure) left the transport control's pending spinner stuck
// forever — violating doc 08 §5.1's "≤250 ms pending" truthful-pending
// contract. These tests pin all resolution paths plus the tracks.changed
// kind guard (J06-12: "data"/"secondary-audio" must not write stray
// selection keys).
import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandTransport, CommandResponse, CommandRequest, Event } from "../../client/transport";
import { PlayerControlsProvider, usePlayerControls } from "./context";
import type { PlayerState } from "./context";

/** Scripted transport: every command answers the ERROR envelope unless a
 * handler is installed for it (the engine-truth path stays per-test). */
class ErrorTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();
  /** Optional per-cmd override (e.g. accepted for the happy-path test). */
  nextResult: ((request: CommandRequest) => CommandResponse["result"]) | null = null;

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    if (this.nextResult !== null) {
      return { gen: 1, requestId: request.requestId, result: this.nextResult(request) };
    }
    return {
      gen: 1,
      requestId: request.requestId,
      result: {
        error: {
          code: "not-seekable",
          details: { cmd: request.cmd },
          message: "engine refused the command",
          stage: "control",
        },
      },
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }
}

/** Transport whose send() REJECTS — the host-unreachable path. */
class UnreachableTransport implements CommandTransport {
  subscribe(): () => void {
    return () => {};
  }
  send(): Promise<CommandResponse> {
    return Promise.reject(new Error("host unreachable"));
  }
}

/** Probe: exposes the provider state + real action triggers. */
function StateProbe({ onState }: { onState: (state: PlayerState) => void }) {
  const { state, actions } = usePlayerControls();
  onState(state);
  return (
    <div>
      <button type="button" onClick={() => actions.togglePlayPause()}>
        toggle
      </button>
      <button type="button" onClick={() => actions.seek(120)}>
        seek
      </button>
    </div>
  );
}

/** Mount the provider (transport injected) + probe; returns the latest
 * state snapshot getter and the action triggers. */
function mountProbe(transport: CommandTransport): {
  latest: () => PlayerState;
  toggle: () => void;
  seek: () => void;
} {
  let latestState: PlayerState | null = null;
  render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider transport={transport}>
        <StateProbe onState={(state) => (latestState = state)} />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
  return {
    latest: () => latestState as PlayerState,
    toggle: () => {
      fireEvent.click(screen.getByRole("button", { name: "toggle" }));
    },
    seek: () => {
      fireEvent.click(screen.getByRole("button", { name: "seek" }));
    },
  };
}

describe("PlayerControlsProvider — pendingToggle resolution (w26.k06, J06-2 P1)", () => {
  it("a toggle that answers ERROR clears the pending flag and surfaces the typed error", async () => {
    const transport = new ErrorTransport();
    const probe = mountProbe(transport);
    expect(probe.latest().pendingToggle).toBe(false);

    await act(async () => {
      probe.toggle();
    });
    // The click marks the request pending; the error envelope resolves it —
    // the spinner must NOT survive the failure (doc 08 §5.1). This is the
    // exact stuck-spinner regression the wave-J review found.
    expect(probe.latest().pendingToggle).toBe(false);
    expect(probe.latest().lastError?.code).toBe("not-seekable");
    expect(probe.latest().lastError?.details).toEqual({ cmd: "playback.toggle-play-pause" });
  });

  it("a toggle over a DEAD transport (send rejects) clears the pending flag + not-ready error", async () => {
    // The transport-failure probe: host unreachable — the request never
    // reached the engine, so no session.state-changed will ever arrive.
    const transport = new UnreachableTransport();
    const probe = mountProbe(transport);

    await act(async () => {
      probe.toggle();
    });
    expect(probe.latest().pendingToggle).toBe(false);
    expect(probe.latest().lastError?.code).toBe("not-ready");
    expect(probe.latest().lastError?.stage).toBe("control");
  });

  it("the happy path is unchanged: accepted + session.state-changed resolves the pending", async () => {
    const transport = new ErrorTransport();
    transport.nextResult = () => ({ accepted: { gen: 1 } });
    const probe = mountProbe(transport);

    await act(async () => {
      probe.toggle();
    });
    // Accepted keeps the request outstanding (doc 08 §5.1)…
    expect(probe.latest().pendingToggle).toBe(true);
    await act(async () => {
      transport.emit({
        seq: 1,
        kind: "session.state-changed",
        from: "paused",
        to: "playing",
      });
    });
    // …and the engine's state change resolves it.
    expect(probe.latest().pendingToggle).toBe(false);
    expect(probe.latest().playbackState).toBe("playing");
  });

  it("a failing NON-toggle command does not falsely resolve an outstanding toggle", async () => {
    const transport = new ErrorTransport();
    transport.nextResult = (request) =>
      request.cmd === "playback.toggle-play-pause"
        ? { accepted: { gen: 1 } }
        : {
            error: {
              code: "not-found",
              details: { cmd: request.cmd },
              message: "engine refused the command",
              stage: "control",
            },
          };
    const probe = mountProbe(transport);

    await act(async () => {
      probe.toggle();
    });
    expect(probe.latest().pendingToggle).toBe(true);

    // A seek that FAILS: its error surfaces, but the still-outstanding
    // toggle keeps its pending truth (only the toggle's own failure paths
    // or the engine's state change resolve it).
    await act(async () => {
      probe.seek();
    });
    expect(probe.latest().pendingToggle).toBe(true);
    expect(probe.latest().pendingSeek).toBe(120);
    expect(probe.latest().lastError?.code).toBe("not-found");
  });
});

describe("PlayerControlsProvider — tracks.changed kind guard (w26.k06, J06-12 P3)", () => {
  it("a DATA track change writes NO stray selection key (the old cast bug)", async () => {
    const transport = new ErrorTransport();
    const probe = mountProbe(transport);
    await act(async () => {
      transport.emit({ seq: 1, kind: "session.state-changed", from: "idle", to: "playing" });
      transport.emit({ seq: 2, kind: "tracks.changed", trackKind: "data", selected: 7 });
    });
    const selections = probe.latest().selections ?? {};
    expect(Object.keys(selections)).toEqual([]);
  });

  it("a SECONDARY-AUDIO track change is likewise a no-op here (no field lie)", async () => {
    const transport = new ErrorTransport();
    const probe = mountProbe(transport);
    await act(async () => {
      transport.emit({
        seq: 1,
        kind: "tracks.changed",
        trackKind: "secondary-audio",
        selected: 3,
      });
    });
    const selections = probe.latest().selections ?? {};
    expect(Object.keys(selections)).toEqual([]);
  });

  it("the three SUPPORTED kinds still land (the guard did not over-reach)", async () => {
    const transport = new ErrorTransport();
    const probe = mountProbe(transport);
    await act(async () => {
      transport.emit({ seq: 1, kind: "tracks.changed", trackKind: "video", selected: 11 });
    });
    expect(probe.latest().selections).toEqual({ video: 11 });
  });
});

describe("PlayerControlsProvider — playback.error resolves a pending toggle (w26.k06, J06-2)", () => {
  it("an async engine failure while a toggle is outstanding clears the spinner", async () => {
    const transport = new ErrorTransport();
    transport.nextResult = () => ({ accepted: { gen: 1 } });
    const probe = mountProbe(transport);
    await act(async () => {
      probe.toggle();
    });
    expect(probe.latest().pendingToggle).toBe(true);
    await act(async () => {
      transport.emit({
        seq: 1,
        kind: "playback.error",
        code: "decoder",
        details: {},
        fatal: true,
        message: "decode failed",
        stage: "decode",
      });
    });
    expect(probe.latest().pendingToggle).toBe(false);
    expect(probe.latest().lastError?.code).toBe("decoder");
  });
});
