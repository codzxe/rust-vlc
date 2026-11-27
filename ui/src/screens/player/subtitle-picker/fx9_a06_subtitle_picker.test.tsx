// FX9-A06 (wave FX9, plan F3.1) — the GUI subtitle track picker pins.
//
// The wave finding: the picker half-existed unfed — the canvas submenu
// listed ZERO subtitle tracks (no engine lane reported any) and NO
// subtitle.add-file affordance existed anywhere while the generated verb
// layer already carried the command. FX9-A01 (committed) routes the five
// subtitle verbs through the real CLI host and enriches track_snapshot
// with the consumer's sidecar rows; this file pins the UI half:
//   * the add-file flow — the hidden file input dispatches
//     `subtitle.add-file` { path, encoding: null, select: true } through
//     the player context's ONE dispatch lane (the catalog's §2.8 shape);
//   * the sidecar rows — rendered from state.tracks exactly like the
//     audio/video submenu rows (A01's snapshot enrichment feeds them);
//   * the selection — `tracks.set-subtitle-track` on row activation, the
//     selected sidecar reflected in the radio (state.selections truth);
//   * refusal surfacing — a typed subtitle.add-file refusal lands in the
//     provider's lastError (the codebase's error surface), never silently;
//   * the no-context fallback — a bare mount degrades its entry disabled
//     with the REGISTERED `player.controls.reason.unknown` title (every
//     key the picker resolves is registered — asserted below);
//   * the canvas submenu — sidecar rows + the radio + the add-file entry,
//     the row's trigger seam reaching PlayerScreen's mounted host.
//   * the dev shell's transport — the subtitle.add-file case mirroring
//     the CLI host's answer shapes (the accept receipt, the typed
//     refusals) + the sidecar track state feeding the tracks read-back.
//
// Mount contract (verified): the provider's mount-time
// system.get-state-snapshot probe is ASYNC — every test that asserts a
// media-open-dependent UI awaits flush() after mount and before asserting
// enabled/rows state (the fx4_a08/fx5_a05 equalizer suites' convention).
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import { DemoTransport } from "../../../client/demo-transport";
import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
} from "../../../client/transport";
import { messages } from "../../../i18n/messages";
import type { StateSnapshot, TrackInfo } from "../../../generated/commands";
import { PlayerControlsProvider, usePlayerControls } from "../context";
import type { PlayerState } from "../context";
import { PlayerScreen } from "../PlayerScreen";
import { SubtitlePicker } from "./SubtitlePicker";

/** Recording transport (the fx4_a08_eq_dispatch.test convention): every
 * command is recorded; results are per-test via `respond`; `emit` reaches
 * the subscribers (the context.test ErrorTransport convention). */
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

  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }
}

/** The provider-state probe (context.test's StateProbe convention). */
function StateProbe({ onState }: { onState: (state: PlayerState) => void }) {
  const { state } = usePlayerControls();
  onState(state);
  return null;
}

/** The sidecar row A01's snapshot enrichment feeds state.tracks (the CLI
 * host's sidecar_tracks shape: external origin, the file's basename). */
const SIDECAR_ROW: TrackInfo = {
  id: 65_280,
  kind: "subtitle",
  isSelected: true,
  isDefault: false,
  origin: "external-subtitle",
  language: null,
  name: "movie.srt",
  codec: { description: null, fourcc: "srt", profile: null, level: null },
};

/** A playing-session snapshot whose tracks carry the sidecar row and
 * whose selection already reports it (the enriched read-back). */
const snapshotWithSidecar = (): StateSnapshot => ({
  audioDelayMs: 0,
  devices: [],
  duration: { kind: "known", time: 635 * 1_000_000 },
  muted: false,
  playlist: { current: null, itemCount: 0, repeat: "off", shuffle: false },
  rate: 1,
  selections: { subtitle: SIDECAR_ROW.id },
  state: "playing",
  subtitle: {},
  subtitleDelayMs: 0,
  time: 0,
  tracks: [SIDECAR_ROW],
  videoDelayMs: 0,
  volume: 0.8,
});

/** A transport whose mount-time probes answer the playing snapshot. */
const snapshotTransport = (): RecordingTransport => {
  const transport = new RecordingTransport();
  transport.respond = (request) =>
    request.cmd === "system.get-state-snapshot"
      ? { completed: { kind: "state-snapshot", snapshot: snapshotWithSidecar() } }
      : { completed: { kind: "unit" } };
  return transport;
};

/** Mount the picker inside the REAL player wiring (ClientProvider +
 * PlayerControlsProvider — the same chain PlayerScreen mounts). */
function mountPicker(
  transport: RecordingTransport,
  onState?: (state: PlayerState) => void,
) {
  return render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider transport={transport}>
        {onState ? <StateProbe onState={onState} /> : null}
        <SubtitlePicker />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
}

/** The sidecar-file pick (jsdom File — the picked NAME is the web shell's
 * honest path answer; the NewPlaylistDialog input pattern). */
const pickSidecar = (name: string): void => {
  fireEvent.change(screen.getByLabelText("Add subtitles", { selector: "input" }), {
    target: { files: [new File([], name)] },
  });
};

const addFileRequests = (transport: RecordingTransport): CommandRequest[] =>
  transport.sent.filter((request) => request.cmd === "subtitle.add-file");

const wireCmd = (request: CommandRequest): string => request.cmd;

const flush = async (): Promise<void> => {
  await act(async () => {});
};

describe("FX9-A06 — the subtitle picker's add-file flow (wave FX9, F3.1)", () => {
  it("fx9_a06_add_file_dispatches_subtitle_add_file_with_the_catalog_payload", async () => {
    // THE payload pin: the catalog's §2.8 shape — the picked file's path,
    // the explicit no-override encoding (null) and select-on-arrival
    // (true), riding the ONE dispatch lane with a monotonic requestId.
    const transport = snapshotTransport();
    mountPicker(transport);
    // The mount-time snapshot probe is ASYNC — flush before the flow.
    await flush();
    fireEvent.click(screen.getByRole("button", { name: "Add subtitles" }));
    pickSidecar("movie.srt");
    await flush();
    const adds = addFileRequests(transport);
    expect(adds).toHaveLength(1);
    expect(adds[0]).toMatchObject({
      cmd: "subtitle.add-file",
      args: { path: "movie.srt", encoding: null, select: true },
    });
    expect(typeof adds[0]?.requestId).toBe("number");
    // The class-A accept's read-back: the lane re-queries tracks.list
    // (A01's enriched rows) AFTER the add.
    const cmds = transport.sent.map(wireCmd);
    expect(cmds).toContain("tracks.list");
    expect(cmds.indexOf("subtitle.add-file")).toBeLessThan(cmds.lastIndexOf("tracks.list"));
  });

  it("fx9_a06_sidecar_rows_render_when_tracks_list_carries_them", async () => {
    // A01's snapshot enrichment feeds state.tracks — the sidecar row
    // renders exactly like the audio/video submenu rows render (one
    // radio per engine-reported row).
    const transport = snapshotTransport();
    mountPicker(transport);
    await flush();
    const row = screen.getByRole("radio", { name: "movie.srt" });
    expect(row).toHaveAttribute("data-origin", "external-subtitle");
    expect(row).toHaveAttribute("aria-checked", "true");
    // The radio group + the frame-fixed action rows (the submenu grammar).
    expect(screen.getByRole("radiogroup", { name: "Subtitle track" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Automatic" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Disabled" })).toBeInTheDocument();
    // The entry is enabled with the session up.
    expect(screen.getByRole("button", { name: "Add subtitles" })).toBeEnabled();
  });

  it("fx9_a06_selection_dispatches_the_select_verb_and_reflects_the_radio", async () => {
    const transport = snapshotTransport();
    mountPicker(transport);
    await flush();
    // Engine truth at mount: the snapshot's selection checks the sidecar.
    expect(screen.getByRole("radio", { name: "movie.srt" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("radio", { name: "Disabled" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    // Row activation dispatches the real select verb — the exact
    // selector shapes (auto / none / the sidecar's id).
    fireEvent.click(screen.getByRole("radio", { name: "Disabled" }));
    await flush();
    fireEvent.click(screen.getByRole("radio", { name: "Automatic" }));
    await flush();
    fireEvent.click(screen.getByRole("radio", { name: "movie.srt" }));
    await flush();
    const selects = transport.sent.filter((r) => r.cmd === "tracks.set-subtitle-track");
    expect(selects).toHaveLength(3);
    expect(selects[0]).toMatchObject({ args: { selector: { kind: "none" } } });
    expect(selects[1]).toMatchObject({ args: { selector: { kind: "auto" } } });
    expect(selects[2]).toMatchObject({
      args: { selector: { kind: "id", id: SIDECAR_ROW.id } },
    });
    // The radio reflects ENGINE truth (tracks.changed), never the click:
    // the selection event moves it, and moving it back moves it back.
    transport.emit({ seq: 1, kind: "tracks.changed", trackKind: "subtitle", selected: null });
    await flush();
    expect(screen.getByRole("radio", { name: "Disabled" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("radio", { name: "movie.srt" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    transport.emit({
      seq: 2,
      kind: "tracks.changed",
      trackKind: "subtitle",
      selected: SIDECAR_ROW.id,
    });
    await flush();
    expect(screen.getByRole("radio", { name: "movie.srt" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("fx9_a06_add_file_refusal_surfaces_the_typed_error", async () => {
    // A failed subtitle.add-file surfaces through the provider's
    // lastError (the codebase's error surface — ControlBar's ErrorSurface
    // renders from it), never silently; the refusal is NOT a mount-time
    // state probe, and its error stays the surfaced one (the tracks
    // read-back never rides a refused add).
    const transport = snapshotTransport();
    let latestState: PlayerState | null = null;
    mountPicker(transport, (state) => (latestState = state));
    const latest = (): PlayerState => latestState as PlayerState;
    await flush();
    transport.respond = (request) =>
      request.cmd === "subtitle.add-file"
        ? {
            error: {
              code: "not-ready",
              details: { cmd: "subtitle.add-file" },
              message: "no session open",
              stage: "session",
            },
          }
        : request.cmd === "system.get-state-snapshot"
          ? { completed: { kind: "state-snapshot", snapshot: snapshotWithSidecar() } }
          : { completed: { kind: "unit" } };
    fireEvent.click(screen.getByRole("button", { name: "Add subtitles" }));
    pickSidecar("movie.srt");
    await flush();
    expect(latest().lastError?.code).toBe("not-ready");
    expect(latest().lastError?.stage).toBe("session");
    expect(latest().lastError?.details).toMatchObject({ cmd: "subtitle.add-file" });
    // The refused add never re-queries (the refusal stays the surfaced
    // truth — no tracks.list dispatch after it).
    const cmds = transport.sent.map(wireCmd);
    expect(cmds).not.toContain("tracks.list");
  });

  it("fx9_a06_without_player_context_the_entry_degrades_disabled", () => {
    // Bare mounts (jsdom trees without the player providers) degrade
    // honestly: the entry disabled with the REGISTERED reason key's
    // title — every key the picker resolves must be registered (a lost
    // run crashed the title on an unregistered key).
    for (const key of [
      "player.controls.reason.unknown",
      "player.controls.reason.no-media",
      "player.track.subtitle",
      "player.track.auto",
      "player.track.disable",
      "player.track.unknown",
      "playlist.subtitles.open",
    ] as const) {
      expect(typeof messages[key], key).toBe("string");
    }
    render(<SubtitlePicker />);
    const entry = screen.getByRole("button", { name: "Add subtitles" });
    expect(entry).toBeDisabled();
    expect(entry).toHaveAttribute("title", "unavailable");
    // The honest unavailable face — no rows to render (never a fabricated
    // list), the reason row states it.
    expect(screen.queryByRole("radio")).toBeNull();
    expect(screen.getByText("unavailable")).toBeInTheDocument();
    // The disabled entry is inert — no crash, no dispatch attempt.
    fireEvent.click(entry);
    expect(screen.queryByRole("radio")).toBeNull();
  });
});

describe("FX9-A06 — the canvas submenu carries the sidecar lane (PlayerScreen)", () => {
  /** Right-click the canvas region; returns the opened menu (the
   * PlayerScreen.test openCanvasMenu convention). */
  const openCanvasMenu = async (): Promise<HTMLElement> => {
    const canvas = screen.getByRole("region", { name: "Video canvas" });
    fireEvent.contextMenu(canvas, { clientX: 300, clientY: 200 });
    return await screen.findByRole("menu", { name: "Player actions" });
  };

  it("fx9_a06_canvas_submenu_lists_sidecar_rows_radio_and_add_file_entry", async () => {
    // The canvas submenu is the picker's shipped surface: the sidecar row
    // (radio, engine-selected), the fixed action rows, and the add-file
    // entry whose trigger seam (openSubtitleFilePicker) reaches the
    // mounted SubtitlePickerHost's hidden input.
    const transport = snapshotTransport();
    render(
      <ClientProvider transport={transport}>
        <PlayerScreen />
      </ClientProvider>,
    );
    // PlayerScreen mounts its own PlayerControlsProvider — the probe is
    // async there too: flush before the menu reads the state.
    await flush();

    const menu = await openCanvasMenu();
    fireEvent.click(within(menu).getByText("Subtitle"));
    const submenu = await screen.findByRole("menu", { name: "Subtitle" });
    expect(
      [...submenu.querySelectorAll<HTMLElement>(".lumen-menu__label")].map(
        (label) => label.textContent,
      ),
    ).toEqual(["Automatic", "Disabled", "movie.srt", "Add subtitles"]);

    // The sidecar row is a radio reflecting the engine's selection.
    const sidecar = within(submenu).getByRole("menuitemradio", { name: "movie.srt" });
    expect(sidecar).toHaveAttribute("aria-checked", "true");

    // Row activation dispatches the real select verb (the id selector).
    fireEvent.click(sidecar);
    await flush();
    const selects = transport.sent.filter((r) => r.cmd === "tracks.set-subtitle-track");
    expect(selects).toHaveLength(1);
    expect(selects[0]).toMatchObject({
      args: { selector: { kind: "id", id: SIDECAR_ROW.id } },
    });

    // The add-file entry: the trigger seam opens the mounted host's
    // hidden input (the lumen:open-media pattern — a window CustomEvent).
    const input = screen.getByLabelText("Add subtitles", { selector: "input" });
    const clickSpy = vi.spyOn(input, "click");
    const menuAgain = await openCanvasMenu();
    fireEvent.click(within(menuAgain).getByText("Subtitle"));
    const submenuAgain = await screen.findByRole("menu", { name: "Subtitle" });
    fireEvent.click(within(submenuAgain).getByText("Add subtitles"));
    expect(clickSpy).toHaveBeenCalledTimes(1);
    // The picked file dispatches subtitle.add-file through the player
    // context — the full flow, pinned on the shipped surface.
    fireEvent.change(input, { target: { files: [new File([], "episode.srt")] } });
    await flush();
    const adds = addFileRequests(transport);
    expect(adds).toHaveLength(1);
    expect(adds[0]).toMatchObject({
      cmd: "subtitle.add-file",
      args: { path: "episode.srt", encoding: null, select: true },
    });
  });
});

describe("FX9-A06 — the dev shell mirrors the CLI host's subtitle lane", () => {
  // The demo-bookmarks.test convention: direct transport sends — the
  // answer shapes mirror the committed CLI host (FX9-A01/A03). A fresh
  // transport per test (per-test session state; dispose in afterEach).
  let transport: DemoTransport;

  beforeEach(() => {
    transport = new DemoTransport();
  });

  afterEach(() => {
    transport.dispose();
  });

  const raw = async (request: CommandRequest): Promise<CommandResponse["result"]> =>
    (await transport.send(request)).result;

  it("fx9_a06_demo_transport_answers_the_cli_host_shapes", async () => {
    // The capability listing carries the now-served verb.
    const listing = await raw({ cmd: "system.list-commands", requestId: 1 });
    if (!("completed" in listing) || listing.completed.kind !== "command-listings") {
      throw new Error("expected command-listings");
    }
    expect(listing.completed.commands.map((c) => c.id)).toContain("subtitle.add-file");

    // No session: the CLI host's typed no-session face (Stage::Session,
    // NotReady — host.rs no_session_open).
    const noSession = await raw({
      args: { encoding: null, path: "/subs/movie.srt", select: true },
      cmd: "subtitle.add-file",
      requestId: 2,
    });
    if (!("error" in noSession)) throw new Error("expected an error envelope");
    expect(noSession.error.code).toBe("not-ready");
    expect(noSession.error.stage).toBe("session");

    // A session, but an unknown sidecar format: the host's typed
    // unsupported face (Stage::Demux — SidecarFormat::from_path).
    await transport.send({
      args: { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
      cmd: "session.open",
      requestId: 3,
    });
    const noFormat = await raw({
      args: { encoding: null, path: "/subs/movie.txt", select: true },
      cmd: "subtitle.add-file",
      requestId: 4,
    });
    if (!("error" in noFormat)) throw new Error("expected an error envelope");
    expect(noFormat.error.code).toBe("unsupported");
    expect(noFormat.error.stage).toBe("demux");

    // A supported sidecar: the class-A ACCEPT receipt (host.rs
    // subtitle_add_file — no payload to echo).
    const accepted = await raw({
      args: { encoding: null, path: "/subs/movie.srt", select: true },
      cmd: "subtitle.add-file",
      requestId: 5,
    });
    if (!("accepted" in accepted)) throw new Error("expected an accepted receipt");
    expect(accepted.accepted.session).toBe(1);
  });

  it("fx9_a06_demo_transport_sidecar_state_feeds_the_tracks_read_back", async () => {
    await transport.send({
      args: { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
      cmd: "session.open",
      requestId: 1,
    });
    // The arrival truth is the tracks read-back (A01's track_snapshot
    // enrichment): tracks.list + the snapshot carry the sidecar row.
    const add = await raw({
      args: { encoding: null, path: "/subs/movie.srt", select: true },
      cmd: "subtitle.add-file",
      requestId: 2,
    });
    if (!("accepted" in add)) throw new Error("expected an accepted receipt");

    const list = await raw({ cmd: "tracks.list", requestId: 3 });
    if (!("completed" in list) || list.completed.kind !== "track-list") {
      throw new Error("expected a track-list result");
    }
    const sidecar = list.completed.tracks.find((track) => track.id === 65_280);
    expect(sidecar).toMatchObject({
      kind: "subtitle",
      origin: "external-subtitle",
      name: "movie.srt",
      isSelected: true,
    });
    // One active subtitle lane: the fixture's embedded subtitle row (id 5,
    // selected in the seed) is deselected by the selected sidecar.
    const embedded = list.completed.tracks.find((track) => track.id === 5);
    expect(embedded?.kind).toBe("subtitle");
    expect(embedded?.isSelected).toBe(false);

    // The snapshot read-back carries the same rows (A01's enrichment).
    const snapshot = await raw({ cmd: "system.get-state-snapshot", requestId: 4 });
    if (!("completed" in snapshot) || snapshot.completed.kind !== "state-snapshot") {
      throw new Error("expected a state-snapshot result");
    }
    expect(
      snapshot.completed.snapshot.tracks.some(
        (track) => track.origin === "external-subtitle" && track.id === 65_280,
      ),
    ).toBe(true);
    expect(snapshot.completed.snapshot.selections.subtitle).toBe(65_280);

    // A second sidecar with select=false registers pickable but leaves
    // the active lane untouched (the next handle from the base).
    await transport.send({
      args: { encoding: null, path: "/subs/extra.vtt", select: false },
      cmd: "subtitle.add-file",
      requestId: 5,
    });
    const listTwo = await raw({ cmd: "tracks.list", requestId: 6 });
    if (!("completed" in listTwo) || listTwo.completed.kind !== "track-list") {
      throw new Error("expected a track-list result");
    }
    const extra = listTwo.completed.tracks.find((track) => track.id === 65_281);
    expect(extra).toMatchObject({ origin: "external-subtitle", name: "extra.vtt" });
    expect(extra?.isSelected).toBe(false);
    expect(
      listTwo.completed.tracks.find((track) => track.id === 65_280)?.isSelected,
    ).toBe(true);

    // Per-session sidecars: a new open starts the numbering over.
    await transport.send({
      args: { source: { kind: "file", path: "/media/demo/ambient-loop.flac" } },
      cmd: "session.open",
      requestId: 7,
    });
    const fresh = await raw({ cmd: "tracks.list", requestId: 8 });
    if (!("completed" in fresh) || fresh.completed.kind !== "track-list") {
      throw new Error("expected a track-list result");
    }
    expect(
      fresh.completed.tracks.filter((track) => track.origin === "external-subtitle"),
    ).toEqual([]);
  });
});
