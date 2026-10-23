// Card 4.C2 — control bar integration: command dispatch per control (spy
// assertions via a fake transport), truthful pending-seek, track menus from
// real TrackInfo fixtures, honest error surfacing (doc 08 §5.4).
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import type {
  CommandRequest,
  CommandResponse,
  Event,
  PlaybackState,
  TrackInfo,
} from "../../../client/transport";
import { PlayerControlsProvider } from "../context";
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

const track = (
  id: number,
  kind: TrackInfo["kind"],
  name: string | null,
  language: string | null,
  isSelected = false,
): TrackInfo => ({
  id,
  kind,
  isSelected,
  isDefault: false,
  origin: "stream",
  codec: {},
  name,
  language,
});

const snapshotResult = (
  state: PlaybackState,
  extra: Partial<{
    volume: number;
    muted: boolean;
    time: number;
    tracks: TrackInfo[];
    selections: Record<string, number | null>;
    durationTime: number | null;
  }> = {},
): CommandResponse["result"] => ({
  completed: {
    kind: "state-snapshot",
    snapshot: {
      audioDelayMs: 0,
      devices: [],
      duration: { kind: "known", time: extra.durationTime ?? 100 },
      muted: extra.muted ?? false,
      playlist: {},
      rate: 1,
      selections: extra.selections ?? {},
      state,
      subtitle: {},
      subtitleDelayMs: 0,
      time: extra.time ?? 0,
      tracks: extra.tracks ?? [],
      videoDelayMs: 0,
      volume: extra.volume ?? 0.5,
    },
  },
} as unknown as CommandResponse["result"]);

function renderBar(transport: FakeTransport | MockTransport) {
  return render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider>
        <ControlBar />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
}

const playPauseButton = (): HTMLElement => screen.getByRole("button", { name: /play|pause/i });

const click = (el: Element): boolean => fireEvent.click(el);

/** Flush pending async command completions (act-wrapped microtask drain). */
const flush = async (): Promise<void> => {
  await act(async () => {});
};

describe("transport cluster — every control invokes a real command", () => {
  it("play/pause toggle dispatches playback.toggle-play-pause when enabled", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing");
    renderBar(fake);
    await flush();
    click(playPauseButton());
    expect(fake.sent.some((r) => r.cmd === "playback.toggle-play-pause")).toBe(true);
  });

  it("stop dispatches playback.stop when enabled (playing)", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing");
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Stop" }));
    expect(fake.sent.some((r) => r.cmd === "playback.stop")).toBe(true);
  });

  it("stop is inert while idle — no command dispatched", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    const stop = screen.getByRole("button", { name: "Stop" });
    expect(stop).toBeDisabled();
    click(stop);
    expect(fake.sent.some((r) => r.cmd === "playback.stop")).toBe(false);
  });

  it("idle: play disabled with the no-media reason (doc 08 §5.1)", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    const play = playPauseButton();
    expect(play).toBeDisabled();
    expect(play.getAttribute("data-glyph")).toBe("play");
  });

  it("buffering: play disabled with progress hint, volume still live (§5.3)", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing");
    renderBar(fake);
    await flush();
    act(() => {
      fake.emit({ kind: "session.state-changed", from: "playing", to: "playing", seq: 1 });
      fake.emit({ kind: "session.buffering", progress: 0.5, seq: 2 });
    });
    const play = playPauseButton();
    expect(play).toBeDisabled();
    expect(play.getAttribute("aria-label")).toContain("Buffering");
    // Control bar remains interactive: mute still dispatches.
    click(screen.getByRole("button", { name: "Mute" }));
    expect(fake.sent.some((r) => r.cmd === "audio.set-mute")).toBe(true);
    await flush();
  });

  it("session.state-changed flips the glyph (idle → playing → pause glyph)", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    expect(playPauseButton().getAttribute("data-glyph")).toBe("play");
    act(() => {
      fake.emit({ kind: "session.state-changed", from: "idle", to: "playing", seq: 1 });
    });
    expect(playPauseButton().getAttribute("data-glyph")).toBe("pause");
    expect(screen.getByRole("button", { name: /pause/i })).toBeEnabled();
  });

  it("ended: restart affordance on the play button (doc §5.1)", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    act(() => {
      fake.emit({ kind: "session.state-changed", from: "playing", to: "ended", seq: 1 });
    });
    const play = playPauseButton();
    expect(play.getAttribute("data-restart")).toBe("true");
    expect(play.getAttribute("aria-label")).toContain("Restart");
  });
});

describe("seekbar cluster — accepted vs completed (doc 08 §5.2)", () => {
  const seekSetup = (): FakeTransport => {
    const fake = new FakeTransport();
    fake.respond = (request) => {
      if (request.cmd === "media.get-current-time") {
        return {
          completed: {
            kind: "current-time",
            position: 10,
            time: 10,
            buffered: [{ start: 0, end: 50 }],
          },
        };
      }
      if (request.cmd === "system.get-state-snapshot") {
        return snapshotResult("playing", { time: 10 });
      }
      return { completed: { kind: "unit" } };
    };
    return fake;
  };

  it("keyboard commit dispatches playback.seek absolute and enters pending", async () => {
    const fake = seekSetup();
    renderBar(fake);
    await flush();
    const seekbar = screen.getByRole("slider", { name: "Seek" });
    fireEvent.keyDown(seekbar, { key: "End" });
    const seek = fake.sent.find((r) => r.cmd === "playback.seek");
    expect(seek).toBeDefined();
    expect(seek?.args).toEqual({ target: { kind: "absolute", time: 100 } });
    // Truthful pending: request outstanding until the engine confirms.
    expect(seekbar.getAttribute("data-pending")).toBe("true");
  });

  it("timing.seek-completed clears pending (commit → set; confirm → cleared)", async () => {
    const fake = seekSetup();
    renderBar(fake);
    await flush();
    const seekbar = screen.getByRole("slider", { name: "Seek" });
    fireEvent.keyDown(seekbar, { key: "End" });
    expect(seekbar.getAttribute("data-pending")).toBe("true");
    act(() => {
      fake.emit({ kind: "timing.seek-completed", position: 100, seekKind: "command", seq: 3 });
    });
    expect(seekbar.getAttribute("data-pending")).toBeNull();
    expect(seekbar.getAttribute("aria-valuenow")).toBe("100");
  });

  it("time-tick catching up to the target clears pending (tolerance)", async () => {
    const fake = seekSetup();
    renderBar(fake);
    await flush();
    const seekbar = screen.getByRole("slider", { name: "Seek" });
    fireEvent.keyDown(seekbar, { key: "End" });
    expect(seekbar.getAttribute("data-pending")).toBe("true");
    act(() => {
      fake.emit({ kind: "timing.time-tick", position: 100, time: 100, seq: 4 });
    });
    expect(seekbar.getAttribute("data-pending")).toBeNull();
  });

  it("timing.seek-failed clears pending AND surfaces the typed error", async () => {
    const fake = seekSetup();
    renderBar(fake);
    await flush();
    const seekbar = screen.getByRole("slider", { name: "Seek" });
    fireEvent.keyDown(seekbar, { key: "End" });
    act(() => {
      fake.emit({
        kind: "timing.seek-failed",
        error: { code: "not-seekable", details: {}, message: "cannot seek", stage: "demux" },
        seq: 5,
      });
    });
    expect(seekbar.getAttribute("data-pending")).toBeNull();
    expect(screen.getByRole("status").getAttribute("data-error-code")).toBe("not-seekable");
  });

  it("renders engine-reported buffered ranges honestly", async () => {
    renderBar(seekSetup());
    await flush();
    const buffer = document.querySelector(".lumen-seekbar__buffer");
    expect(buffer).not.toBeNull();
    expect(buffer?.getAttribute("data-range-start")).toBe("0");
    expect(buffer?.getAttribute("data-range-end")).toBe("50");
  });

  it("seek disabled with reason while the engine reports non-seekable", async () => {
    const fake = new FakeTransport();
    fake.respond = (request) => {
      if (request.cmd === "media.get-info") {
        return {
          completed: {
            kind: "media-info",
            info: {
              chapters: [],
              duration: { kind: "known", time: 100 },
              isLive: false,
              metadata: {},
              seekable: false,
              source: { kind: "file", path: "/tmp/a.mkv" },
              tracks: [],
            },
          },
        };
      }
      return snapshotResult("playing");
    };
    renderBar(fake);
    await flush();
    const seekbar = screen.getByRole("slider", { name: /not seekable/ });
    expect(seekbar).toHaveAttribute("aria-disabled", "true");
  });
});

describe("volume/mute — engine-reported, never optimistic", () => {
  it("mute toggle dispatches audio.set-mute(true) from the unmuted engine state", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { muted: false });
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Mute" }));
    const mute = fake.sent.find((r) => r.cmd === "audio.set-mute");
    expect(mute?.args).toEqual({ muted: true });
  });

  it("unmute dispatches audio.set-mute(false) from the muted engine state", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { muted: true });
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Unmute" }));
    const mute = fake.sent.find((r) => r.cmd === "audio.set-mute");
    expect(mute?.args).toEqual({ muted: false });
  });

  it("keyboard commit dispatches audio.set-volume with the committed value", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { volume: 0.5 });
    renderBar(fake);
    await flush();
    const volume = screen.getByRole("slider", { name: "Volume" });
    fireEvent.keyDown(volume, { key: "ArrowUp" });
    const set = fake.sent.find((r) => r.cmd === "audio.set-volume");
    expect(set).toBeDefined();
    expect(set?.args).toEqual({ volume: 0.51 });
  });

  it("volume slider reflects the engine completion, not optimism", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { volume: 0.5 });
    renderBar(fake);
    await flush();
    const volume = screen.getByRole("slider", { name: "Volume" });
    expect(volume.getAttribute("aria-valuenow")).toBe("0.5");
    // Engine completes with a DIFFERENT value than any UI prediction — that
    // completion is what the UI reflects.
    fake.respond = () => ({ completed: { kind: "audio-volume", volume: 0.4 } });
    fireEvent.keyDown(volume, { key: "ArrowUp" });
    await flush();
    expect(volume.getAttribute("aria-valuenow")).toBe("0.4");
  });

  it("volume is honestly disabled before any engine report", () => {
    const fake = new FakeTransport();
    fake.respond = () => ({ error: { code: "feature-not-exposed", details: {}, message: "no", stage: "control" } });
    renderBar(fake);
    expect(screen.getByRole("slider", { name: "Volume" })).toHaveAttribute("aria-disabled", "true");
  });
});

describe("track menus — real ids, real labels, honest unknowns", () => {
  const FIXTURE: TrackInfo[] = [
    track(1, "audio", "English 5.1", "en"),
    track(2, "audio", "Commentaire", "fr", true),
    track(3, "audio", null, null), // honest Unknown
    track(7, "subtitle", "English SDH", "en"),
    track(8, "video", "1080p", null),
  ];

  const tracksSetup = (): FakeTransport => {
    const fake = new FakeTransport();
    fake.respond = (request) => {
      if (request.cmd === "tracks.list") {
        return { completed: { kind: "track-list", tracks: FIXTURE } };
      }
      return snapshotResult("playing", { tracks: FIXTURE, selections: { audio: 2 } });
    };
    return fake;
  };

  it("lists real tracks per kind from the tracks.list result", async () => {
    renderBar(tracksSetup());
    await flush();
    click(screen.getByRole("button", { name: "Audio track" }));
    const list = screen.getByRole("listbox", { name: "Audio track" });
    expect(within(list).getByRole("option", { name: "Commentaire" })).toBeInTheDocument();
    expect(within(list).getAllByRole("option")).toHaveLength(5); // auto + disable + 3 audio
  });

  it("subtitle menu only lists subtitle tracks", async () => {
    renderBar(tracksSetup());
    await flush();
    click(screen.getByRole("button", { name: "Subtitle track" }));
    const list = screen.getByRole("listbox", { name: "Subtitle track" });
    expect(within(list).getAllByRole("option")).toHaveLength(3); // auto + disable + 1 subtitle
    expect(within(list).getByRole("option", { name: "English SDH" })).toBeInTheDocument();
  });

  it("renders an honest 'Unknown' label — never an invented title", async () => {
    renderBar(tracksSetup());
    await flush();
    click(screen.getByRole("button", { name: "Audio track" }));
    const list = screen.getByRole("listbox", { name: "Audio track" });
    expect(within(list).getByRole("option", { name: /Unknown 3/ })).toBeInTheDocument();
  });

  it("marks the engine-selected track (aria-selected)", async () => {
    renderBar(tracksSetup());
    await flush();
    click(screen.getByRole("button", { name: "Audio track" }));
    const list = screen.getByRole("listbox", { name: "Audio track" });
    expect(
      within(list).getByRole("option", { name: "Commentaire" }).getAttribute("aria-selected"),
    ).toBe("true");
  });

  it("switching dispatches tracks.set-audio-track with the real id", async () => {
    const fake = tracksSetup();
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Audio track" }));
    click(screen.getByRole("option", { name: "English 5.1" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-audio-track");
    expect(select?.args).toEqual({ selector: { kind: "id", id: 1 } });
  });

  it("'Disable' dispatches the none selector", async () => {
    const fake = tracksSetup();
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Subtitle track" }));
    click(screen.getByRole("option", { name: "Disabled" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-subtitle-track");
    expect(select?.args).toEqual({ selector: { kind: "none" } });
  });

  it("'Automatic' dispatches the auto selector", async () => {
    const fake = tracksSetup();
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Video track" }));
    click(screen.getByRole("option", { name: "Automatic" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-video-track");
    expect(select?.args).toEqual({ selector: { kind: "auto" } });
  });

  it("tracks.changed event updates the selected marker", async () => {
    const fake = tracksSetup();
    renderBar(fake);
    await flush();
    act(() => {
      fake.emit({ kind: "tracks.changed", selected: 1, trackKind: "audio", seq: 5 });
    });
    click(screen.getByRole("button", { name: "Audio track" }));
    const list = screen.getByRole("listbox", { name: "Audio track" });
    expect(
      within(list).getByRole("option", { name: "English 5.1" }).getAttribute("aria-selected"),
    ).toBe("true");
  });
});

describe("error surfacing — typed errors, never swallowed (§5.4)", () => {
  it("MockTransport feature-not-exposed is surfaced honestly and dismissable", async () => {
    renderBar(new MockTransport());
    const status = await screen.findByRole("status");
    expect(status.getAttribute("data-error-code")).toBe("feature-not-exposed");
    expect(status.textContent).toContain("control");
    click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("playback.error event surfaces a typed error; fatal marks engine error", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    act(() => {
      fake.emit({
        kind: "playback.error",
        code: "decoder",
        details: {},
        fatal: true,
        message: "decoder failed",
        stage: "decode",
        seq: 9,
      });
    });
    const bar = document.querySelector(".lumen-controls");
    expect(bar?.getAttribute("data-playback-state")).toBe("error");
    expect(screen.getByRole("status").getAttribute("data-error-code")).toBe("decoder");
  });
});
