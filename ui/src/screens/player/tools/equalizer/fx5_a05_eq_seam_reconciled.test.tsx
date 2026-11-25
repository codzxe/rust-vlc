// FX5-A05 (wave FX5, plan F1.1c) — the EQ dispatch seam's reconciliation
// pins, the UI half of the catalog/host verb pair (FX5-A02: the
// audio.set-eq / audio.set-balance catalog ids; FX5-A03: the host routing
// through audio_controls()).
//
// FX4-A08 landed the seam on the wave contract — verb id `audio.set-eq`,
// payload { lowDb, midDb, highDb, preampDb } (the engine setter
// AudioControls::set_eq's shape, camelCase per the wire convention), the
// SAME single transport lane volume/mute ride, and the honest-refusal
// surfacing. The landed catalog (verified this wave: crates/commands/src/
// catalog/command.rs `AudioSetEq { low_db, mid_db, high_db, preamp_db }`,
// whose doc states the field mapping is "one-to-one with the
// { lowDb, midDb, highDb, preampDb } the UI dispatches") matches the
// contract exactly — zero shape drift to reconcile; these pins make any
// FUTURE drift fail the central vitest run loudly (the reconciliation
// would then be the ONE construction site FX4-A08's risk #1 named:
// EqSetRequest in context.tsx + the matching args in the dispatch
// tests).
//
// The remaining UI-side gap is ONLY the central `cargo xtask codegen`
// refresh — the generated union (ui/src/generated/commands.ts) does not
// carry the command yet — so the eq requests are read through the
// seam's typed forward declaration (EqSetRequest — context.tsx) with the
// same documented cast the context's own dispatch makes.
//
// Transport: the fx4_a08_eq_dispatch.test RecordingTransport convention
// (recorded sends, scripted results).
import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../../client/ClientProvider";
import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
} from "../../../../client/transport";
import { PlayerControlsProvider, usePlayerControls } from "../../context";
import type { EqSetRequest, PlayerControls, PlayerState } from "../../context";
import { EqualizerTool } from "./EqualizerTool";

/** Recording transport (the fx4_a08_eq_dispatch.test convention): every
 * command is recorded; results are per-test via `respond`. */
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

/** The provider probe (context.test's StateProbe convention), exposing
 * the full controls (actions ride the same lane the tool's commits do). */
function StateProbe({ onControls }: { onControls: (controls: PlayerControls) => void }) {
  const controls = usePlayerControls();
  onControls(controls);
  return null;
}

/** Mount the tool inside the REAL player wiring (ClientProvider +
 * PlayerControlsProvider — the same chain PlayerScreen mounts). */
function mountTool(transport: RecordingTransport, onControls?: (c: PlayerControls) => void) {
  return render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider transport={transport}>
        {onControls ? <StateProbe onControls={onControls} /> : null}
        <EqualizerTool defaultOpen />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
}

/**
 * The recorded `audio.set-eq` request, viewed through the seam's typed
 * forward declaration (EqSetRequest, context.tsx): FX5-A02's catalog id
 * has landed, but the request still rides the CommandRequest union until
 * the central codegen refresh carries it there natively — the same
 * documented cast the context's own dispatch makes. `null` for every
 * non-eq send (the mount-time probes legitimately dispatch
 * system.get-state-snapshot / media.get-*); the discriminator check is
 * on the runtime cmd string.
 */
const eqRequestOf = (request: CommandRequest): EqSetRequest | null => {
  const candidate = request as unknown as EqSetRequest;
  return candidate.cmd === "audio.set-eq" ? candidate : null;
};

/** Only the audio.set-eq requests. */
const eqRequests = (transport: RecordingTransport): EqSetRequest[] =>
  transport.sent
    .map(eqRequestOf)
    .filter((request): request is EqSetRequest => request !== null);

/** The recorded request's wire verb id, widened off the generated union —
 * the honest-absence pin below reads the raw cmd without pretending the UI
 * types a verb it does not carry. */
const wireCmd = (request: CommandRequest): string => request.cmd;

const flush = async (): Promise<void> => {
  await act(async () => {});
};

describe("FX5-A05 — the EQ seam reconciled to the catalog verbs (wave FX5, F1.1c)", () => {
  it("fx5_a05_eq_request_envelope_is_the_reconciled_wave_contract", async () => {
    // THE shape-drift oracle: the exact envelope — verb id, the four dB
    // fields, nothing else. If FX5-A02's catalog payload lands with
    // different field names, this fails the central run and the
    // reconciliation is EqSetRequest (context.tsx), the one seam.
    const transport = new RecordingTransport();
    mountTool(transport);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    await flush();
    const eq = eqRequests(transport);
    expect(eq).toHaveLength(1);
    expect(eq[0]).toEqual({
      args: { lowDb: 0.1, midDb: 0, highDb: 0, preampDb: 0 },
      cmd: "audio.set-eq",
      requestId: eq[0]?.requestId,
    });
    expect(typeof eq[0]?.requestId).toBe("number");
  });

  it("fx5_a05_eq_and_volume_ride_the_one_transport_lane_monotonic", async () => {
    // The reconciled verb joins the working audio family on the ONE
    // dispatch lane (the single transport.send behind lumen_dispatch)
    // with the shared monotonic requestId counter — never a second side
    // channel for audio.
    const transport = new RecordingTransport();
    let latestControls: PlayerControls | null = null;
    mountTool(transport, (controls) => (latestControls = controls));
    const controls = (): PlayerControls => latestControls as PlayerControls;
    await flush();
    controls().actions.setVolume(0.5);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Mid (1 kHz)" }), {
      key: "ArrowDown",
    });
    await flush();
    const cmds = transport.sent.map(wireCmd);
    expect(cmds).toContain("audio.set-volume");
    expect(cmds).toContain("audio.set-eq");
    const volume = transport.sent.find(
      (request) => wireCmd(request) === "audio.set-volume",
    );
    const eq = eqRequests(transport);
    expect(eq).toHaveLength(1);
    expect(typeof volume?.requestId).toBe("number");
    expect((eq[0]?.requestId ?? 0) > (volume?.requestId ?? 0)).toBe(true);
  });

  it("fx5_a05_eq_completed_answer_smears_no_player_state", async () => {
    // The success posture with the landed routing (FX5-A03): the verb's
    // completed answers own NO state field in the seam — the real
    // `audio-eq` read-back (CommandResult::AudioEq — FX5-A02's catalog,
    // viewed through the documented forward-declaration cast until the
    // codegen refresh) lands in applyCompleted's default no-op branch,
    // and even an applied-bool answer (the shape a naive kind-match
    // would smear onto `muted`) stays off the state — the guard routes
    // applied-bool to audio.set-mute ONLY. No error surfaces; the
    // faders stay the editing surface and a later commit dispatches
    // again on the lane.
    const transport = new RecordingTransport();
    let latestState: PlayerState | null = null;
    mountTool(transport, (c) => (latestState = c.state));
    const latest = (): PlayerState => latestState as PlayerState;
    // Commit 1 answers with the REAL landed shape — the chain's own
    // read-back (host.rs audio_set_eq: AudioEq { eq: AudioEqBands }).
    transport.respond = (request) =>
      eqRequestOf(request) !== null
        ? ({
            completed: {
              kind: "audio-eq",
              eq: { lowDb: 0.1, midDb: 0, highDb: 0, preampDb: 0 },
            },
          } as unknown as CommandResponse["result"])
        : { completed: { kind: "unit" } };
    fireEvent.keyDown(screen.getByRole("slider", { name: "High (4 kHz)" }), {
      key: "ArrowUp",
    });
    await flush();
    expect(latest().lastError).toBeNull();
    expect(latest().muted).toBeNull();
    // Commit 2 answers with the DANGEROUS kind: an applied-bool the
    // naive kind-match in applyCompleted would write onto `muted`.
    transport.respond = (request) =>
      eqRequestOf(request) !== null
        ? { completed: { kind: "applied-bool", value: true } }
        : { completed: { kind: "unit" } };
    fireEvent.keyDown(screen.getByRole("slider", { name: "High (4 kHz)" }), {
      key: "ArrowDown",
    });
    await flush();
    expect(eqRequests(transport)).toHaveLength(2);
    expect(latest().muted).toBeNull();
    expect(latest().lastError).toBeNull();
  });

  it("fx5_a05_eq_unserved_refusal_surfaces_not_swallowed", async () => {
    // The honest-refusal face for any host that does not serve the verb
    // (the pre-FX5-A03 shipped host's dispatch catch-all, verbatim
    // class — apps/cli/src/dispatch.rs: control/unsupported-for-host,
    // the command + owner in details, the audio.* owner branch): the
    // provider must SURFACE it through lastError — the eq verb is no
    // mount-time state probe, so its refusal is a failure the error chip
    // tells the truth about, never swallowed.
    const transport = new RecordingTransport();
    let latestState: PlayerState | null = null;
    mountTool(transport, (c) => (latestState = c.state));
    const latest = (): PlayerState => latestState as PlayerState;
    transport.respond = (request) =>
      eqRequestOf(request) !== null
        ? {
            error: {
              code: "unsupported-for-host",
              details: {
                command: "audio.set-eq",
                owner: "audio device control joins with the real backends (ADR-005)",
              },
              message:
                "`audio.set-eq` is not served by this build's command-line host (audio device control joins with the real backends (ADR-005))",
              stage: "control",
            },
          }
        : { completed: { kind: "unit" } };
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    await flush();
    expect(latest().lastError?.code).toBe("unsupported-for-host");
    expect(latest().lastError?.stage).toBe("control");
    expect(latest().lastError?.details).toMatchObject({ command: "audio.set-eq" });
    // The seam stays alive — the refusal is surfaced, the editing surface
    // keeps dispatching (a later commit still reaches the wire).
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    await flush();
    expect(eqRequests(transport)).toHaveLength(2);
  });

  it("fx5_a05_balance_verb_is_honestly_absent_from_the_player_seam", async () => {
    // FX5-A02 also added `audio.set-balance` to the catalog this wave
    // (and FX5-A03 routes it); the player domain carries NO balance
    // control to wire it to (verified: Volume.tsx is the volume slider,
    // the context's actions carry volume/mute/eq only, and the pre-wave
    // balance slider was retired from the settings area). Pin the honest
    // absence: exercising the audio family dispatches set-volume +
    // set-eq and NEVER set-balance, and the action surface carries no
    // setBalance member — a balance UI that dispatches the real verb is
    // a future assignment's surface, not a pretense bolted onto this
    // seam.
    const transport = new RecordingTransport();
    let latestControls: PlayerControls | null = null;
    mountTool(transport, (controls) => (latestControls = controls));
    const controls = (): PlayerControls => latestControls as PlayerControls;
    await flush();
    controls().actions.setVolume(0.5);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Low (250 Hz)" }), {
      key: "ArrowUp",
    });
    await flush();
    const cmds = transport.sent.map(wireCmd);
    expect(cmds).toContain("audio.set-volume");
    expect(cmds).toContain("audio.set-eq");
    expect(cmds).not.toContain("audio.set-balance");
    expect("setBalance" in controls().actions).toBe(false);
  });
});
