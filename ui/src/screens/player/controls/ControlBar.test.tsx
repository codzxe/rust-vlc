// Card 4.C2 — control bar integration: command dispatch per control (spy
// assertions via a fake transport), truthful pending-seek, track menus from
// real TrackInfo fixtures, honest error surfacing (doc 08 §5.4).
//
// w26.a14 — updated to the W26 single-row anatomy (Player.svg "Controls"):
// previous · play pill · next | timecode · seekbar | total · volume wedge ·
// audio · subtitles · gear · queue popovers · app menu (w26.gfix-C: g04
// P1-1 previous-not-stop; P1-4 interactive timecodes; P1-6 the frame's
// right cluster — fullscreen/cast are gone from every control-bar frame).
//
// w26.f13 — re-anchored with the app-menu rebuild (e12 F5): the menu now
// carries menu.png's 12 frame rows (comfortable, 375px, accelerators from
// keymap DATA); the Video/Audio/Subtitle track submenus that lived in it
// since w26.a13 moved OUT (menu.png has no track rows) — their frame home
// is the CANVAS context menu (right-click menu.svg rows), so the track
// tests below drive the SAME honest fixtures through PlayerScreen's canvas
// menu. Every dispatch assertion is the same command + args as before; the
// two-timecode anatomy (e02 F6) gets its own pin below.
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
import { PlaylistProvider } from "../../playlist";
import { PlayerScreen } from "../PlayerScreen";
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
    playlist: { shuffle?: boolean; repeat?: "off" | "all" | "one"; current?: number | null };
  }> = {},
): CommandResponse["result"] => ({
  completed: {
    kind: "state-snapshot",
    snapshot: {
      audioDelayMs: 0,
      devices: [],
      duration: { kind: "known", time: extra.durationTime ?? 100 },
      muted: extra.muted ?? false,
      playlist: extra.playlist ?? {},
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

// PlayerScreen mounts the same provider chain around the bar — the playlist
// context is what `next` dispatches through (playlist.next).
function renderBar(transport: FakeTransport | MockTransport) {
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

const playPauseButton = (): HTMLElement => screen.getByRole("button", { name: /play|pause/i });

const click = (el: Element): boolean => fireEvent.click(el);

/** Opens the app menu and a track submenu; returns the submenu popup. */
const openAppMenu = (): HTMLElement => {
  click(screen.getByRole("button", { name: "Application menu" }));
  return screen.getByRole("menu", { name: "Application menu" });
};

/**
 * w26.f13 re-anchor: the track selectors' menu home is the canvas context
 * menu (right-click menu.svg's Video/Audio/Subtitle rows — PlayerScreen's
 * canvasTrackSubmenu, the same engine-truth builder the app menu used).
 * Opens the canvas menu and the named track submenu; returns the submenu.
 */
const openCanvasTrackMenu = (kindName: string): HTMLElement => {
  fireEvent.contextMenu(screen.getByRole("region", { name: "Video canvas" }), {
    clientX: 300,
    clientY: 200,
  });
  click(screen.getByRole("menuitem", { name: kindName }));
  return screen.getByRole("menu", { name: kindName });
};

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

  it("previous dispatches playlist.prev when the queue has entries", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing");
    renderBar(fake);
    await flush();
    act(() => {
      fake.emit({ seq: 1, kind: "playlist.changed", change: { kind: "items-added", ids: [7, 8] } });
    });
    click(screen.getByRole("button", { name: "Previous" }));
    expect(fake.sent.some((r) => r.cmd === "playlist.prev")).toBe(true);
  });

  it("previous is inert while the queue is empty — no command dispatched", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    const previous = screen.getByRole("button", { name: /Previous — no media open/ });
    expect(previous).toBeDisabled();
    click(previous);
    expect(fake.sent.some((r) => r.cmd === "playlist.prev")).toBe(false);
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

describe("track menus (canvas context-menu submenus, w26.f13 re-anchor) — real ids, real labels, honest unknowns", () => {
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

  // PlayerScreen mounts the same provider chain around the bar + the canvas
  // menu; the track submenus now live only there (see the file header).
  const renderScreen = (transport: FakeTransport) =>
    render(
      <ClientProvider transport={transport}>
        <PlayerScreen />
      </ClientProvider>,
    );

  it("v2 §4.3 — no media open: each submenu holds ONLY the disabled no-media reason (never fabricated options)", async () => {
    // The pre-W26 bar showed the no-media truth on disabled dropdown
    // triggers; the W26 anatomy carries the same honesty into the canvas
    // menu: the Video/Audio/Subtitle submenus exist and open, but their
    // single option is the inert "no media open" reason — no "Automatic",
    // no "Disabled", no invented track names anywhere.
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("idle");
    renderScreen(fake);
    await flush();
    fireEvent.contextMenu(screen.getByRole("region", { name: "Video canvas" }), {
      clientX: 300,
      clientY: 200,
    });
    for (const name of ["Video", "Audio", "Subtitle"]) {
      const parent = screen.getByRole("menuitem", { name });
      expect(parent).toHaveAttribute("aria-haspopup", "true");
      click(parent);
      const submenu = screen.getByRole("menu", { name });
      const options = within(submenu).getAllByRole("menuitem");
      expect(options).toHaveLength(1);
      expect(options[0]).toHaveAttribute("aria-disabled", "true");
      expect(options[0]?.textContent).toBe("no media open");
      expect(within(submenu).queryByRole("menuitemradio")).toBeNull();
    }
  });

  it("v2 §4.3 — with media open the honest menu options return (auto/none/real ids)", async () => {
    const fake = tracksSetup();
    renderScreen(fake);
    await flush();
    const submenu = openCanvasTrackMenu("Audio");
    expect(within(submenu).getAllByRole("menuitemradio")).toHaveLength(5); // auto + disable + 3 audio
    expect(
      within(submenu).getByRole("menuitemradio", { name: "Automatic" }),
    ).toBeInTheDocument();
    expect(within(submenu).getByRole("menuitemradio", { name: "Disabled" })).toBeInTheDocument();
  });

  it("lists real tracks per kind from the tracks.list result", async () => {
    const fake = tracksSetup();
    renderScreen(fake);
    await flush();
    // One open menu; hovering/clicking between the track parents swaps the
    // submenu in place (MenuPopup's openSubmenu switch).
    fireEvent.contextMenu(screen.getByRole("region", { name: "Video canvas" }), {
      clientX: 300,
      clientY: 200,
    });
    click(screen.getByRole("menuitem", { name: "Audio" }));
    const audio = screen.getByRole("menu", { name: "Audio" });
    expect(within(audio).getByRole("menuitemradio", { name: "English 5.1" })).toBeInTheDocument();
    expect(within(audio).getByRole("menuitemradio", { name: "Commentaire" })).toBeInTheDocument();
    expect(within(audio).getAllByRole("menuitemradio")).toHaveLength(5); // auto + disable + 3 audio
    // The video menu lists the video track, never the audio ones.
    click(screen.getByRole("menuitem", { name: "Video" }));
    const video = screen.getByRole("menu", { name: "Video" });
    expect(within(video).getByRole("menuitemradio", { name: "1080p" })).toBeInTheDocument();
    expect(within(video).getAllByRole("menuitemradio")).toHaveLength(3); // auto + disable + 1 video
    expect(within(video).queryByRole("menuitemradio", { name: "English 5.1" })).toBeNull();
  });

  it("subtitle menu only lists subtitle tracks", async () => {
    renderScreen(tracksSetup());
    await flush();
    const submenu = openCanvasTrackMenu("Subtitle");
    expect(within(submenu).getAllByRole("menuitemradio")).toHaveLength(3); // auto + disable + 1 subtitle
    expect(within(submenu).getByRole("menuitemradio", { name: "English SDH" })).toBeInTheDocument();
    expect(within(submenu).queryByRole("menuitemradio", { name: "English 5.1" })).toBeNull();
  });

  it("renders an honest 'Unknown' label — never an invented title", async () => {
    renderScreen(tracksSetup());
    await flush();
    const submenu = openCanvasTrackMenu("Audio");
    expect(within(submenu).getByRole("menuitemradio", { name: /Unknown 3/ })).toBeInTheDocument();
  });

  it("marks the engine-selected track (aria-checked radio)", async () => {
    renderScreen(tracksSetup());
    await flush();
    const submenu = openCanvasTrackMenu("Audio");
    expect(
      within(submenu).getByRole("menuitemradio", { name: "Commentaire" }).getAttribute("aria-checked"),
    ).toBe("true");
    expect(
      within(submenu).getByRole("menuitemradio", { name: "English 5.1" }).getAttribute("aria-checked"),
    ).toBe("false");
  });

  it("switching dispatches tracks.set-audio-track with the real id", async () => {
    const fake = tracksSetup();
    renderScreen(fake);
    await flush();
    const submenu = openCanvasTrackMenu("Audio");
    click(within(submenu).getByRole("menuitemradio", { name: "English 5.1" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-audio-track");
    expect(select?.args).toEqual({ selector: { kind: "id", id: 1 } });
  });

  it("'Disable' dispatches the none selector", async () => {
    const fake = tracksSetup();
    renderScreen(fake);
    await flush();
    const submenu = openCanvasTrackMenu("Subtitle");
    click(within(submenu).getByRole("menuitemradio", { name: "Disabled" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-subtitle-track");
    expect(select?.args).toEqual({ selector: { kind: "none" } });
  });

  it("'Automatic' dispatches the auto selector", async () => {
    const fake = tracksSetup();
    renderScreen(fake);
    await flush();
    const submenu = openCanvasTrackMenu("Video");
    click(within(submenu).getByRole("menuitemradio", { name: "Automatic" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-video-track");
    expect(select?.args).toEqual({ selector: { kind: "auto" } });
  });

  it("tracks.changed event updates the selected marker", async () => {
    const fake = tracksSetup();
    renderScreen(fake);
    await flush();
    act(() => {
      fake.emit({ kind: "tracks.changed", selected: 1, trackKind: "audio", seq: 5 });
    });
    const submenu = openCanvasTrackMenu("Audio");
    expect(
      within(submenu).getByRole("menuitemradio", { name: "English 5.1" }).getAttribute("aria-checked"),
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

describe("app menu — menu.png grammar (w26.f13, e12 F5)", () => {
  /** Regex faces: rows with accelerators compose their accessible name from
   * label + shortcut text, so exact-name queries would miss. */
  it("opens the frame's 12 rows + 1 separator in pinned order (menu.png)", () => {
    renderBar(new FakeTransport());
    const menu = openAppMenu();
    expect(
      [...menu.querySelectorAll<HTMLElement>(".lumen-menu__label")].map((label) => label.textContent),
    ).toEqual([
      "Open File(s)",
      "Open Folder",
      "Open Network Stream",
      "Open Disc",
      "Open Capture Device",
      "Convert",
      "Open Recent",
      "Profile",
      "Keyboard Shortcuts",
      "Customize Player",
      "Settings",
      "About VLC",
    ]);
    expect(within(menu).getAllByRole("separator")).toHaveLength(1);
  });

  it("accelerators are keymap DATA in the frame spelling (Ctrl + O … Shift + F1)", () => {
    renderBar(new FakeTransport());
    const menu = openAppMenu();
    // HOTKEY_BINDINGS rows: media.open-file/-folder/-network-stream/-disc/
    // -capture-device/convert + window.preferences/window.about — exactly the
    // eight combos menu.png pins, in row order.
    expect(
      [...menu.querySelectorAll<HTMLElement>(".lumen-menu__accelerator")].map((acc) => acc.textContent),
    ).toEqual([
      "Ctrl + O",
      "Ctrl + F",
      "Ctrl + N",
      "Ctrl + D",
      "Ctrl + C",
      "Ctrl + R",
      "Ctrl + P",
      "Shift + F1",
    ]);
  });

  it("honest gates: no-seam rows inert, routable rows and submenu parents active", () => {
    renderBar(new FakeTransport());
    const menu = openAppMenu();
    for (const label of ["Open Folder", "Open Disc", "Open Capture Device"]) {
      expect(within(menu).getByRole("menuitem", { name: new RegExp(`^${label}`) })).toHaveAttribute(
        "aria-disabled",
        "true",
      );
    }
    for (const label of [
      "Open File(s)",
      "Open Network Stream",
      "Convert",
      "Open Recent",
      "Profile",
      "Keyboard Shortcuts",
      "Customize Player",
      "Settings",
      "About VLC",
    ]) {
      const face = label.replace(/[()]/g, "\\$&");
      expect(
        within(menu).getByRole("menuitem", { name: new RegExp(`^${face}`) }),
      ).not.toHaveAttribute("aria-disabled");
    }
  });

  it("Open Recent opens onto the honest empty reason — never a fabricated history", () => {
    // FakeTransport is not the demo transport: nothing remembers openings.
    renderBar(new FakeTransport());
    const menu = openAppMenu();
    click(within(menu).getByRole("menuitem", { name: "Open Recent" }));
    const submenu = screen.getByRole("menu", { name: "Open Recent" });
    const rows = within(submenu).getAllByRole("menuitem");
    expect(rows).toHaveLength(1);
    expect(rows[0]).toHaveAttribute("aria-disabled", "true");
    expect(rows[0]?.textContent).toBe("Nothing opened yet");
  });

  it("Profile flyout holds the honest unavailable row (no profile engine seam)", () => {
    renderBar(new FakeTransport());
    const menu = openAppMenu();
    click(within(menu).getByRole("menuitem", { name: "Profile" }));
    const submenu = screen.getByRole("menu", { name: "Profile" });
    const rows = within(submenu).getAllByRole("menuitem");
    expect(rows).toHaveLength(1);
    expect(rows[0]).toHaveAttribute("aria-disabled", "true");
    expect(rows[0]?.textContent).toBe("unavailable");
  });

  it("Open File(s) dispatches the shell-wide open-media event", () => {
    renderBar(new FakeTransport());
    let heard = false;
    const onOpenMedia = (): void => {
      heard = true;
    };
    window.addEventListener("lumen:open-media", onOpenMedia);
    try {
      const menu = openAppMenu();
      click(within(menu).getByRole("menuitem", { name: /^Open File\(s\)/ }));
      expect(heard).toBe(true);
    } finally {
      window.removeEventListener("lumen:open-media", onOpenMedia);
    }
  });

  it("Customize Player fires the lumen:open-customize seam (f09's dialog)", () => {
    renderBar(new FakeTransport());
    let heard = false;
    const onOpenCustomize = (): void => {
      heard = true;
    };
    window.addEventListener("lumen:open-customize", onOpenCustomize);
    try {
      const menu = openAppMenu();
      click(within(menu).getByRole("menuitem", { name: /^Customize Player/ }));
      expect(heard).toBe(true);
    } finally {
      window.removeEventListener("lumen:open-customize", onOpenCustomize);
    }
  });

  it("Escape closes and restores focus to the trigger (e12 F8)", () => {
    renderBar(new FakeTransport());
    const trigger = screen.getByRole("button", { name: "Application menu" });
    click(trigger);
    const menu = screen.getByRole("menu", { name: "Application menu" });
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});

describe("seek cluster — the frame's two timecodes (w26.f13, e02 F6)", () => {
  it("renders exactly current + duration; the third (remaining) face is gone", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { time: 10, durationTime: 100 });
    renderBar(fake);
    await flush();
    const times = [...document.querySelectorAll<HTMLElement>(".lumen-controls__time")];
    // Player.svg anatomy: position | seekbar | … | duration — no remaining.
    expect(times.map((time) => time.getAttribute("data-time"))).toEqual(["position", "duration"]);
    expect(times[0]?.textContent).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    expect(times[1]?.textContent).toBe("00:01:40");
  });

  it("both readouts are real buttons (Current_Time.svg/Duration.svg hover + click faces)", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { time: 10, durationTime: 100 });
    renderBar(fake);
    await flush();
    const position = screen.getByRole("button", { name: "Current time" });
    const duration = screen.getByRole("button", { name: "Total duration" });
    expect(position).toBeEnabled();
    expect(duration).toBeEnabled();
  });

  it("duration click toggles the remaining '-HH:MM:SS' face (Duration.svg state=click)", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { time: 10, durationTime: 100 });
    renderBar(fake);
    await flush();
    const duration = screen.getByRole("button", { name: "Total duration" });
    expect(duration.textContent).toBe("00:01:40");
    click(duration);
    const remaining = screen.getByRole("button", { name: "Remaining time" });
    expect(remaining.textContent).toBe("-00:01:30");
    expect(remaining).toHaveAttribute("aria-pressed", "true");
    click(remaining);
    expect(screen.getByRole("button", { name: "Total duration" }).textContent).toBe("00:01:40");
  });

  it("position click opens the Go-to-Time surface (Current_Time.svg state=click)", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", { time: 10, durationTime: 100 });
    renderBar(fake);
    await flush();
    const position = screen.getByRole("button", { name: "Current time" });
    click(position);
    // The edit face: the dialog (prefilled with this position) + the
    // caret-carrying editing state on the readout.
    expect(screen.getByText("Go to time")).toBeInTheDocument();
    expect(position).toHaveAttribute("data-editing", "");
  });

  it("timecodes are inert with the honest reason while no media is open", () => {
    const fake = new FakeTransport();
    renderBar(fake);
    expect(screen.getByRole("button", { name: /Current time/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /Total duration/ })).toBeDisabled();
  });
});

describe("right cluster — Controls.svg Frame 23 (w26.gfix-C, g04 P1-6) + the frame's settings panels (w26.i03, h04 §2/§7)", () => {
  it("renders the frame's triggers: audio · subtitles · gear · queue (+ the f13 app menu)", () => {
    renderBar(new MockTransport());
    for (const name of ["Audio tracks", "Subtitle tracks", "Settings", "Queue", "Application menu"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    // No frame shows fullscreen/cast on the control bar (g04 P1-6).
    expect(screen.queryByRole("button", { name: /fullscreen/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /cast/i })).toBeNull();
  });

  it("audio trigger opens the settings panel: tab header + engine-truth rows + the quiet footer", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing", {
      tracks: [
        track(1, "audio", "English 5.1", "en"),
        track(2, "audio", "Commentaire", "fr", true),
      ],
      selections: { audio: 2 },
    });
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Audio tracks" }));
    const panel = screen.getByRole("dialog", { name: "Audio tracks" });
    // The 3-tab header (Controls.svg "audio" variant: Video · Audio · Subtitles).
    expect(within(panel).getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "Video",
      "Audio",
      "Subtitles",
    ]);
    expect(within(panel).getByRole("tab", { name: "Audio" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    // The frame's section header over the engine-truth radio rows.
    expect(within(panel).getByText("Embedded Tracks")).toBeInTheDocument();
    expect(within(panel).getByRole("radio", { name: "Automatic" })).toBeInTheDocument();
    expect(within(panel).getByRole("radio", { name: "English 5.1" })).toBeInTheDocument();
    // The engine-reported selection is checked, never optimistic.
    expect(
      within(panel).getByRole("radio", { name: "Commentaire" }).getAttribute("aria-checked"),
    ).toBe("true");
    // The frame's Output Device section renders its honest unavailable face.
    expect(within(panel).getByText("Output Device")).toBeInTheDocument();
    expect(within(panel).getByText("unavailable")).toBeInTheDocument();
    // Button.png's quiet footer (the panel's 360×32 r4 button) is real.
    expect(within(panel).getByRole("button", { name: "Show Advanced Settings" })).toBeInTheDocument();
    click(within(panel).getByRole("radio", { name: "English 5.1" }));
    const select = fake.sent.find((r) => r.cmd === "tracks.set-audio-track");
    expect(select?.args).toEqual({ selector: { kind: "id", id: 1 } });
  });

  it("idle: the track panels open onto the honest no-media reason", () => {
    renderBar(new MockTransport());
    click(screen.getByRole("button", { name: "Subtitle tracks" }));
    const panel = screen.getByRole("dialog", { name: "Subtitle tracks" });
    // Subtitles tab active: Embedded Tracks + the frame's External Tracks section.
    expect(within(panel).getByRole("tab", { name: "Subtitles" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(within(panel).getByText("Embedded Tracks")).toBeInTheDocument();
    expect(within(panel).getAllByText("no media open")).toHaveLength(1);
    expect(within(panel).getByText("External Tracks")).toBeInTheDocument();
    // No fabricated rows anywhere.
    expect(within(panel).queryByRole("radio")).toBeNull();
    // The footer still routes to the real tools surface.
    expect(within(panel).getByRole("button", { name: "Show Advanced Settings" })).toBeInTheDocument();
  });

  it("queue trigger lists the real queue, marks the current item, plays the pick", async () => {
    const fake = new FakeTransport();
    fake.respond = () => snapshotResult("playing");
    renderBar(fake);
    await flush();
    act(() => {
      fake.emit({
        seq: 1,
        kind: "playlist.changed",
        change: { kind: "items-added", ids: [7, 8] },
      });
      fake.emit({ seq: 2, kind: "playlist.changed", change: { kind: "current-changed", id: 7 } });
    });
    click(screen.getByRole("button", { name: "Queue" }));
    const panel = screen.getByRole("dialog", { name: "Queue" });
    // The Queue · Bookmarks · Chapters tab header (Controls.svg queue variant).
    expect(within(panel).getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "Queue",
      "Bookmarks",
      "Chapters",
    ]);
    // The frame's "4 items" meta row, over the real count.
    expect(within(panel).getByText("2 items")).toBeInTheDocument();
    // Honest unknown titles (no playlist.list seam) — never fabricated.
    const rows = within(panel).getAllByRole("radio");
    expect(rows).toHaveLength(2);
    expect(rows[0]?.textContent).toBe("Unknown title");
    expect(rows[0]?.getAttribute("aria-checked")).toBe("true");
    click(rows[1] as HTMLElement);
    const play = fake.sent.find((r) => r.cmd === "playlist.play-item");
    expect(play?.args).toEqual({ id: 8 });
  });

  it("idle queue: the honest empty reason — never a fabricated list", () => {
    renderBar(new MockTransport());
    click(screen.getByRole("button", { name: "Queue" }));
    const panel = screen.getByRole("dialog", { name: "Queue" });
    expect(within(panel).getByText("Nothing in the queue")).toBeInTheDocument();
    expect(within(panel).queryByRole("radio")).toBeNull();
  });

  it("gear opens the 4-tab settings panel; Playback carries the real shuffle/repeat seams", async () => {
    const fake = new FakeTransport();
    fake.respond = () =>
      snapshotResult("playing", {
        tracks: [track(2, "audio", "Commentaire", "fr", true)],
        selections: { audio: 2 },
        playlist: { shuffle: false, repeat: "off" },
      });
    renderBar(fake);
    await flush();
    click(screen.getByRole("button", { name: "Settings" }));
    const panel = screen.getByRole("dialog", { name: "Settings" });
    // The 4-tab header (the "playback"/"* settings" variants add Playback).
    expect(within(panel).getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "Playback",
      "Video",
      "Audio",
      "Subtitles",
    ]);
    // Audio tab active (the "audio settings" variant): the Defaults
    // label/value rows over the same engine data + the track section.
    expect(within(panel).getByText("Defaults")).toBeInTheDocument();
    expect(within(panel).getByText("Audio track")).toBeInTheDocument();
    // "Commentaire" paints twice over the same engine truth: the Defaults
    // value row AND the checked radio row.
    expect(within(panel).getAllByText("Commentaire")).toHaveLength(2);
    expect(within(panel).getByText("Embedded Tracks")).toBeInTheDocument();
    // Switch to Playback: the real seams — Toggle.png switch + the
    // Values.png segmented control (h04 §8/§9 flow-coherence gaps).
    click(within(panel).getByRole("tab", { name: "Playback" }));
    click(within(panel).getByRole("switch", { name: "Shuffle" }));
    expect(fake.sent.some((r) => r.cmd === "playlist.set-shuffle")).toBe(true);
    click(within(panel).getByRole("radio", { name: "All" }));
    const repeat = fake.sent.find((r) => r.cmd === "playlist.set-repeat");
    expect(repeat?.args).toEqual({ mode: "all" });
  });

  it("panel Escape restores focus to its trigger (e12 F8 pattern)", () => {
    renderBar(new MockTransport());
    const trigger = screen.getByRole("button", { name: "Queue" });
    click(trigger);
    const panel = screen.getByRole("dialog", { name: "Queue" });
    fireEvent.keyDown(panel, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});
