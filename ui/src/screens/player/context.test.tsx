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

/**
 * w29.desktop-probe — the REAL desktop engine host's cold-start face,
 * mirrored from the Rust sources (not a mock's guess). The desktop shell
 * drives the CLI host (desktop → lumen-cli, ARCH-5) behind
 * `lumen_dispatch`, and with no session open it answers the player's
 * mount probes:
 *   - `system.get-state-snapshot` → control/unsupported-for-host — the
 *     served table has no snapshot row (apps/cli/src/dispatch.rs SERVED,
 *     doc 05 §5.3's first-vertical subset);
 *   - `media.get-info` / `media.get-current-time` → session/not-ready
 *     "no session open" (apps/cli/src/host.rs `media_info`/`current_time`
 *     — the facts are session-scoped);
 *   - any session command (`playback.play` & friends) → session/not-ready
 *     "no session open" (`submit_and_await`).
 * These are the exact shapes the TauriTransport delivers on a cold start.
 */
class DesktopHostTransport implements CommandTransport {
  private readonly listeners = new Set<(event: Event) => void>();

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    const error = (
      stage: "control" | "session",
      code: "not-ready" | "unsupported-for-host",
      message: string,
    ): CommandResponse => ({
      gen: 0,
      requestId: request.requestId,
      result: { error: { code, details: {}, message, stage } },
    });
    switch (request.cmd) {
      case "system.get-state-snapshot":
        // The CLI host's unserved-command refusal, verbatim shape
        // (dispatch.rs `unserved`): stage control, code
        // unsupported-for-host, the command named.
        return error(
          "control",
          "unsupported-for-host",
          "`system.get-state-snapshot` is not served by this build's command-line host (not in the first vertical's served set (see `lumen commands`))",
        );
      default:
        // media.get-info / media.get-current-time / playback.* with no
        // session: the honest not-ready.
        return error("session", "not-ready", "no session open");
    }
  }

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
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

describe("PlayerControlsProvider — desktop cold-start probes (w29.desktop-probe)", () => {
  // User report on the v0.1.1 desktop build: a "Playback error —
  // session: no session open (not-ready)" chip on the EMPTY player at
  // every cold start. Root cause: the mount probes' error handling was
  // written against the browser DemoTransport's idle face
  // (media.get-info → not-found); the real desktop host answers
  // not-ready "no session open" (and has no snapshot row at all), and
  // those STATE answers landed in the lastError surface as if they were
  // playback failures.

  it("the desktop host's idle answers surface NO playback error (states, not failures)", async () => {
    const transport = new DesktopHostTransport();
    const probe = mountProbe(transport);
    // Let the three mount probes (snapshot, current-time, info) resolve.
    await act(async () => {
      await Promise.resolve();
    });
    await act(async () => {
      await Promise.resolve();
    });
    expect(probe.latest().lastError).toBeNull();
    // The honest idle view stands — the empty-state hero renders.
    expect(probe.latest().playbackState).toBe("idle");
  });

  it("the demo transport's old idle face stays exempt (not-found regression pin)", async () => {
    // The browser dev shell's cold start: the snapshot + current-time
    // probes complete and media.get-info answers not-found ("no media
    // open" — the shape the pre-w29 guard was written against). The
    // generalization must keep that case exempt too.
    const transport = new ErrorTransport();
    transport.nextResult = (request) => {
      if (request.cmd === "media.get-info") {
        return {
          error: {
            code: "not-found",
            details: {},
            message: "demo transport: no media open",
            stage: "input",
          },
        };
      }
      if (request.cmd === "media.get-current-time") {
        return {
          completed: {
            kind: "current-time",
            position: 0,
            time: 0,
            buffered: [],
          },
        };
      }
      if (request.cmd === "system.get-state-snapshot") {
        return {
          completed: {
            kind: "state-snapshot",
            snapshot: {
              state: "idle",
              time: 0,
              duration: { kind: "unknown" },
              volume: 0.5,
              muted: false,
              tracks: [],
              selections: {},
            },
          },
        } as unknown as CommandResponse["result"];
      }
      return {
        error: {
          code: "not-seekable",
          details: { cmd: request.cmd },
          message: "engine refused the command",
          stage: "control",
        },
      };
    };
    const probe = mountProbe(transport);
    await act(async () => {
      await Promise.resolve();
    });
    expect(probe.latest().lastError).toBeNull();
    expect(probe.latest().playbackState).toBe("idle");
  });

  it("the guard does not over-reach: a failed USER toggle still surfaces the typed error", async () => {
    const transport = new DesktopHostTransport();
    const probe = mountProbe(transport);
    await act(async () => {
      await Promise.resolve();
    });
    // A play with no session (e.g. a keymap binding before the button's
    // idle-disable lands) is a real failed action — the chip answers.
    await act(async () => {
      probe.toggle();
    });
    expect(probe.latest().lastError?.stage).toBe("session");
    expect(probe.latest().lastError?.code).toBe("not-ready");
    expect(probe.latest().lastError?.message).toBe("no session open");
    expect(probe.latest().pendingToggle).toBe(false);
  });
});
