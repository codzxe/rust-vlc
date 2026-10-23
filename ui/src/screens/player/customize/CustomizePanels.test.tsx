// Card 4.F3 — customize panels tests. AC proof:
//  * capability-gating matrix: available → rendered; not-exposed → hidden;
//  * field-command bindings (dispatch spies on the transport);
//  * all-unsupported → panel hidden (no dead panels);
//  * slider live-vs-committed on color fields (live never dispatches).
import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import { fireEvent } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
} from "../../../client/transport";
import type { CommandListing, CommandResult } from "../../../generated/commands";
import { CustomizePanels } from "./CustomizePanels";
import { fieldVisible, panelVisible, useCommandCapabilities } from "./capabilities";
import { CUSTOMIZE_PANELS } from "./panels";

const listing = (id: string): CommandListing => ({
  id,
  class: "s",
  requires: [],
  since: "0.1.0",
});

const NOT_EXPOSED = (cmd: string) => ({
  error: {
    code: "feature-not-exposed" as const,
    details: { cmd },
    message: "stub: not simulated",
    stage: "control" as const,
  },
});

/**
 * Scriptable transport: `commands` is the capability listing (what
 * `system.list-commands` reports); `complete` scripts command completions;
 * every sent request is recorded for dispatch spies.
 */
class StubTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();

  constructor(
    private readonly commands: readonly CommandListing[],
    private readonly complete?: (request: CommandRequest) => CommandResult | null,
  ) {}

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    if (request.cmd === "system.list-commands") {
      return {
        gen: 1,
        requestId: request.requestId,
        result: { completed: { kind: "command-listings", commands: [...this.commands] } },
      };
    }
    const completed = this.complete?.(request) ?? null;
    return {
      gen: 1,
      requestId: request.requestId,
      result: completed ? { completed } : NOT_EXPOSED(request.cmd),
    };
  }
}

const renderCustomize = (transport: CommandTransport) =>
  render(
    <ClientProvider transport={transport}>
      <CustomizePanels />
    </ClientProvider>,
  );

const CATALOG_COMMANDS = [
  "video.set-aspect-ratio",
  "video.set-crop",
  "video.set-zoom",
  "playback.set-rate",
  "playback.seek",
  "playback.stop",
  "playlist.set-repeat",
  "playlist.set-shuffle",
];

describe("capability gating (AC: available → rendered; not-exposed → hidden)", () => {
  it("renders fields whose commands the engine lists", async () => {
    const stub = new StubTransport(CATALOG_COMMANDS.map(listing));
    renderCustomize(stub);
    expect(await screen.findByRole("radiogroup", { name: "Aspect ratio" })).toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "Crop" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Zoom" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Playback speed" })).toBeInTheDocument();
  });

  it("hides fields whose commands are not exposed (crop absent, colors absent)", async () => {
    const stub = new StubTransport([listing("video.set-aspect-ratio")]);
    renderCustomize(stub);
    expect(await screen.findByRole("radiogroup", { name: "Aspect ratio" })).toBeInTheDocument();
    // crop + zoom not listed → hidden; whole colors/HDR panels absent too.
    expect(screen.queryByRole("radiogroup", { name: "Crop" })).not.toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Brightness" })).not.toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Playback speed" })).not.toBeInTheDocument();
  });

  it("renders the video-colors sliders only when color commands are listed", async () => {
    const stub = new StubTransport(
      ["video.set-brightness", "video.set-contrast", "video.set-saturation", "video.set-hue", "video.set-gamma"].map(listing),
    );
    renderCustomize(stub);
    expect(await screen.findByRole("slider", { name: "Brightness" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Contrast" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Saturation" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Hue" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Gamma" })).toBeInTheDocument();
    // forward-declared colors listed → panel visible; essential not listed
    expect(screen.queryByRole("slider", { name: "Playback speed" })).not.toBeInTheDocument();
  });

  it("hides a panel entirely when ALL its fields are unsupported (HDR)", async () => {
    const stub = new StubTransport(CATALOG_COMMANDS.map(listing));
    renderCustomize(stub);
    await screen.findByRole("heading", { name: "Crop & aspect ratio" });
    expect(screen.queryByRole("heading", { name: "HDR options" })).not.toBeInTheDocument();
    expect(screen.queryByRole("radiogroup", { name: "Tone mapping" })).not.toBeInTheDocument();
  });

  it("renders the HDR panel when tone-mapping commands are listed", async () => {
    const stub = new StubTransport(
      ["video.set-tone-mapping", "video.set-tone-mapping-strength"].map(listing),
    );
    renderCustomize(stub);
    expect(await screen.findByRole("heading", { name: "HDR options" })).toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "Tone mapping" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Tone-mapping strength" })).toBeInTheDocument();
  });

  it("renders no panels against the scaffold MockTransport (empty listing) — honest note only", async () => {
    renderCustomize(new MockTransport());
    await waitFor(() => {
      expect(screen.getByText(/No customize controls are supported/)).toBeInTheDocument();
    });
    expect(screen.queryByRole("heading", { name: "Essential" })).not.toBeInTheDocument();
    expect(screen.queryByRole("slider")).not.toBeInTheDocument();
  });

  it("pure helpers: fieldVisible/panelVisible encode the gating matrix", () => {
    const caps = {
      available: new Set<string>(["video.set-aspect-ratio"]),
      resolved: true,
    };
    const crop = CUSTOMIZE_PANELS.find((p) => p.id === "crop-aspect");
    expect(crop).toBeDefined();
    expect(fieldVisible(crop!.fields[0]!, caps)).toBe(true);
    expect(fieldVisible(crop!.fields[1]!, caps)).toBe(false);
    expect(panelVisible(crop!, caps)).toBe(true);
    const hdr = CUSTOMIZE_PANELS.find((p) => p.id === "hdr");
    expect(hdr).toBeDefined();
    expect(panelVisible(hdr!, caps)).toBe(false);
  });

  it("useCommandCapabilities resolves ids from system.list-commands", async () => {
    const stub = new StubTransport([listing("playback.stop")]);
    const { result } = renderHook(() => useCommandCapabilities(stub));
    await waitFor(() => expect(result.current.resolved).toBe(true));
    expect(result.current.available.has("playback.stop")).toBe(true);
    expect(result.current.available.has("video.set-crop")).toBe(false);
  });
});

describe("field-command bindings (dispatch spies)", () => {
  it("aspect selection dispatches video.set-aspect-ratio with the ratio payload", async () => {
    const stub = new StubTransport([listing("video.set-aspect-ratio")]);
    renderCustomize(stub);
    fireEvent.click(await screen.findByRole("radio", { name: "16:9" }));
    await act(async () => {});
    const sent = stub.sent.filter((r) => r.cmd === "video.set-aspect-ratio");
    expect(sent).toHaveLength(1);
    expect(sent[0]!.args).toEqual({ ratio: { kind: "ratio", w: 16, h: 9 } });
  });

  it("crop selection dispatches video.set-crop with the none/ratio payload", async () => {
    const stub = new StubTransport([listing("video.set-crop")]);
    renderCustomize(stub);
    fireEvent.click(await screen.findByRole("radio", { name: "None" }));
    await act(async () => {});
    const sent = stub.sent.filter((r) => r.cmd === "video.set-crop");
    expect(sent).toHaveLength(1);
    expect(sent[0]!.args).toEqual({ crop: { kind: "none" } });
  });

  it("seek quick actions dispatch relative playback.seek targets (±10 s / ±60 s)", async () => {
    const stub = new StubTransport([listing("playback.seek"), listing("playback.stop")]);
    renderCustomize(stub);
    fireEvent.click(await screen.findByRole("button", { name: "-10 s" }));
    fireEvent.click(screen.getByRole("button", { name: "+10 s" }));
    fireEvent.click(screen.getByRole("button", { name: "-60 s" }));
    fireEvent.click(screen.getByRole("button", { name: "+60 s" }));
    fireEvent.click(screen.getByRole("button", { name: "Stop" }));
    await act(async () => {});
    const seeks = stub.sent.filter((r) => r.cmd === "playback.seek");
    expect(seeks.map((r) => (r.args as { target: { deltaMs: number } }).target.deltaMs)).toEqual([
      -10_000, 10_000, -60_000, 60_000,
    ]);
    expect(stub.sent.some((r) => r.cmd === "playback.stop")).toBe(true);
  });

  it("repeat selection dispatches playlist.set-repeat; applied-repeat confirms the engine mode", async () => {
    const stub = new StubTransport(
      [listing("playlist.set-repeat")],
      (request) =>
        request.cmd === "playlist.set-repeat"
          ? { kind: "applied-repeat", mode: (request.args as { mode: "off" | "one" | "all" }).mode }
          : null,
    );
    renderCustomize(stub);
    const one = await screen.findByRole("radio", { name: "One" });
    fireEvent.click(one);
    await waitFor(() => expect(one).toBeChecked());
    const sent = stub.sent.filter((r) => r.cmd === "playlist.set-repeat");
    expect(sent).toHaveLength(1);
    expect(sent[0]!.args).toEqual({ mode: "one" });
  });

  it("shuffle quick action dispatches playlist.set-shuffle and flips on the engine report", async () => {
    const stub = new StubTransport(
      [listing("playlist.set-shuffle")],
      (request) =>
        request.cmd === "playlist.set-shuffle"
          ? { kind: "applied-bool", value: (request.args as { enabled: boolean }).enabled }
          : null,
    );
    renderCustomize(stub);
    const shuffle = await screen.findByRole("button", { name: "Shuffle" });
    expect(shuffle).not.toHaveAttribute("aria-pressed");
    fireEvent.click(shuffle);
    await waitFor(() => expect(shuffle).toHaveAttribute("aria-pressed", "true"));
    fireEvent.click(shuffle);
    await waitFor(() => expect(shuffle).toHaveAttribute("aria-pressed", "false"));
    const sent = stub.sent.filter((r) => r.cmd === "playlist.set-shuffle");
    expect(sent.map((r) => (r.args as { enabled: boolean }).enabled)).toEqual([true, false]);
  });

  it("engine events update confirmed state (rate-changed, shuffle-set)", async () => {
    const stub = new StubTransport([listing("playback.set-rate"), listing("playlist.set-shuffle")]);
    renderCustomize(stub);
    const speed = await screen.findByRole("slider", { name: "Playback speed" });
    expect(speed).toHaveAttribute("aria-valuenow", "1");
    act(() => {
      stub.emit({
        seq: 1,
        kind: "playback.rate-changed",
        rate: 2,
      });
    });
    await waitFor(() => expect(speed).toHaveAttribute("aria-valuenow", "2"));
  });
});

describe("slider live-vs-committed (doc 08 §5)", () => {
  it("speed slider: live drag never dispatches; commit dispatches playback.set-rate once", async () => {
    const stub = new StubTransport(
      [listing("playback.set-rate")],
      (request) =>
        request.cmd === "playback.set-rate"
          ? { kind: "applied-rate", rate: (request.args as { rate: number }).rate }
          : null,
    );
    renderCustomize(stub);
    const slider = await screen.findByRole("slider", { name: "Playback speed" });
    // jsdom rects are zero-width: any large clientX quantizes to max (4).
    fireEvent.pointerDown(slider, { pointerId: 1, clientX: 999 });
    fireEvent.pointerMove(slider, { pointerId: 1, clientX: 999 });
    await act(async () => {});
    expect(stub.sent.filter((r) => r.cmd === "playback.set-rate")).toHaveLength(0);
    expect(screen.getByText("4.00x")).toBeInTheDocument(); // live readout
    fireEvent.pointerUp(slider, { pointerId: 1, clientX: 999 });
    await act(async () => {});
    const sent = stub.sent.filter((r) => r.cmd === "playback.set-rate");
    expect(sent).toHaveLength(1);
    expect(sent[0]!.args).toEqual({ rate: 4 });
    // engine confirmation becomes the committed value
    await waitFor(() => expect(slider).toHaveAttribute("aria-valuenow", "4"));
  });

  it("color slider: live preview fires without dispatch; commit dispatches exactly once", async () => {
    const stub = new StubTransport([listing("video.set-brightness")]);
    renderCustomize(stub);
    const slider = await screen.findByRole("slider", { name: "Brightness" });
    fireEvent.pointerDown(slider, { pointerId: 1, clientX: 999 });
    fireEvent.pointerMove(slider, { pointerId: 1, clientX: 999 });
    await act(async () => {});
    // Forward-declared command: cast keeps the spy honest pre-codegen.
    const brightnessSends = () => stub.sent.filter((r) => (r.cmd as string) === "video.set-brightness");
    expect(brightnessSends()).toHaveLength(0);
    expect(screen.getByText("100")).toBeInTheDocument(); // live readout at max
    fireEvent.pointerUp(slider, { pointerId: 1, clientX: 999 });
    await act(async () => {});
    const sent = brightnessSends();
    expect(sent).toHaveLength(1);
    expect((sent[0] as { args: unknown }).args).toEqual({ value: 100 });
    // stub completes with feature-not-exposed → committed stays honest (0)
    expect(slider).toHaveAttribute("aria-valuenow", "0");
  });

  it("zoom slider commit dispatches video.set-zoom; applied-zoom confirms", async () => {
    const stub = new StubTransport(
      [listing("video.set-zoom")],
      (request) =>
        request.cmd === "video.set-zoom"
          ? { kind: "applied-zoom", factor: (request.args as { factor: number }).factor }
          : null,
    );
    renderCustomize(stub);
    const slider = await screen.findByRole("slider", { name: "Zoom" });
    fireEvent.pointerDown(slider, { pointerId: 1, clientX: 999 });
    fireEvent.pointerUp(slider, { pointerId: 1, clientX: 999 });
    await act(async () => {});
    const sent = stub.sent.filter((r) => r.cmd === "video.set-zoom");
    expect(sent).toHaveLength(1);
    expect(sent[0]!.args).toEqual({ factor: 4 });
    await waitFor(() => expect(slider).toHaveAttribute("aria-valuenow", "4"));
  });
});
