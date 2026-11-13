// /open-network-stream tests (W26 Figma rebuild, w26.b15) — pins:
//   * dialog semantics: role=dialog named by the header title, focus trap,
//     Escape + scrim + ✕ + Cancel all exit to /player without opening;
//   * the frame anatomy: Network URL textarea (label + helper + placeholder),
//     Playlist Name (optional) input, footer Cancel + Open actions;
//   * the URL contract (http/https only, honest error, nothing sent) and the
//     open wiring (`session.open` with the url source, then /player);
//   * recent streams persist to localStorage and refill the field;
//   * a named open records a LOCAL saved playlist (the frame helper copy
//     "The playlist will be saved and displayed with this title" made real
//     through the b09 playlistsStore seam).

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
import { resetPlaylistsForTests } from "../playlist/playlistsStore";
import { NetworkStreamScreen, isValidNetworkUrl } from "./NetworkStreamScreen";

class SpyTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  subscribe(): () => void {
    return () => {};
  }
  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return { gen: 1, requestId: request.requestId, result: { completed: { kind: "unit" } } };
  }
}

const renderScreen = () => {
  const transport = new SpyTransport();
  const view = render(
    <MemoryRouter initialEntries={["/open-network-stream"]}>
      <Routes>
        <Route path="/open-network-stream" element={<NetworkStreamScreen />} />
        <Route path="/player" element={<p data-testid="player-route">player</p>} />
      </Routes>
    </MemoryRouter>,
    { wrapper: (ui) => <ClientProvider transport={transport}>{ui.children}</ClientProvider> },
  );
  return { transport, view };
};

describe("isValidNetworkUrl (pure)", () => {
  it("accepts http/https URLs and rejects everything else", () => {
    expect(isValidNetworkUrl("https://example.com/stream.m3u8")).toBe(true);
    expect(isValidNetworkUrl("http://localhost:8080/x")).toBe(true);
    expect(isValidNetworkUrl("ftp://example.com/x")).toBe(false);
    expect(isValidNetworkUrl("not a url")).toBe(false);
    expect(isValidNetworkUrl("")).toBe(false);
  });
});

describe("NetworkStreamScreen (/open-network-stream)", () => {
  beforeEach(() => {
    window.localStorage.clear();
    resetPlaylistsForTests();
  });

  afterEach(() => {
    window.localStorage.clear();
    resetPlaylistsForTests();
  });

  it("renders the frame anatomy: titled dialog, ✕ close, URL + playlist fields, footer actions", () => {
    renderScreen();
    expect(screen.getByRole("dialog", { name: "Open network stream" })).toBeInTheDocument();
    // j16 P3-15: the dialog family's heading convention — h2 title (the
    // AppShell owns the page h1), h3 section heads, like shortcuts/open-media.
    expect(screen.getByRole("heading", { name: "Open network stream", level: 2 })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Recent streams", level: 3 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close" })).toBeInTheDocument();
    expect(screen.getByLabelText("Network URL")).toBeInTheDocument();
    expect(screen.getByLabelText("Playlist Name (optional)")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open stream" })).toBeInTheDocument();
    // standard footer action row (Cancel left, Open right)
    expect(document.querySelector(".network-stream__actions")).not.toBeNull();
    expect(screen.getByTestId("network-stream-card")).not.toBeNull();
    // w26.i17 fix hooks: the section rule + the re-skinned Cancel exist.
    expect(document.querySelector(".network-stream__rule")).not.toBeNull();
    expect(document.querySelector(".network-stream__cancel")).not.toBeNull();
  });

  it("initial focus sits on the URL field, never the ✕ glyph (w26.i17, h19 shame #4)", () => {
    renderScreen();
    const dialog = screen.getByRole("dialog", { name: "Open network stream" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    // The trap's first-focusable is the ✕ button; the screen moves initial
    // focus to the FIRST FIELD so no ring paints on the close glyph at rest
    // and Enter at rest can never dismiss the dialog.
    expect(document.activeElement).toBe(screen.getByLabelText("Network URL"));
    expect(document.activeElement).not.toBe(screen.getByRole("button", { name: "Close" }));
  });

  it("Escape closes the dialog and exits to /player without opening anything", () => {
    const { transport } = renderScreen();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(screen.getByTestId("player-route")).toBeInTheDocument();
    expect(transport.sent.length).toBe(0);
  });

  it("a scrim click closes the dialog and exits to /player without opening anything", () => {
    const { transport } = renderScreen();
    fireEvent.click(screen.getByRole("dialog").parentElement!);
    expect(screen.getByTestId("player-route")).toBeInTheDocument();
    expect(transport.sent.length).toBe(0);
  });

  it("rejects an invalid URL with the honest error and sends nothing", () => {
    const { transport } = renderScreen();
    fireEvent.change(screen.getByLabelText("Network URL"), { target: { value: "nope" } });
    fireEvent.submit(screen.getByLabelText("Network URL").closest("form")!);
    expect(screen.getByText("Enter a valid http:// or https:// URL")).toBeInTheDocument();
    // j16 P2-4: the notice is ANNOUNCED — aria-describedby alone does not
    // fire on dynamic insertion, so the error carries role="alert".
    expect(screen.getByTestId("network-stream-error")).toHaveAttribute("role", "alert");
    expect(transport.sent.some((r) => r.cmd === "session.open")).toBe(false);
  });

  it("submits: session.open with the url source, then routes to /player", async () => {
    const { transport } = renderScreen();
    const field = screen.getByLabelText("Network URL");
    fireEvent.change(field, { target: { value: "https://example.com/live.m3u8" } });
    fireEvent.submit(field.closest("form")!);
    await waitFor(() => expect(screen.getByTestId("player-route")).toBeInTheDocument());
    const open = transport.sent.find((r) => r.cmd === "session.open");
    expect(open && "args" in open ? open.args : null).toEqual({
      source: { kind: "url", url: "https://example.com/live.m3u8" },
    });
    // the stream lands in the persisted recents
    expect(JSON.parse(window.localStorage.getItem("lumen.streams.recent.v1")!)).toEqual([
      "https://example.com/live.m3u8",
    ]);
  });

  it("records a named open into the local playlist library (the helper copy made real)", async () => {
    const { transport } = renderScreen();
    fireEvent.change(screen.getByLabelText("Network URL"), {
      target: { value: "https://example.com/live.m3u8" },
    });
    fireEvent.change(screen.getByLabelText("Playlist Name (optional)"), {
      target: { value: "Favorite TV Channels" },
    });
    fireEvent.submit(screen.getByLabelText("Network URL").closest("form")!);
    await waitFor(() => expect(screen.getByTestId("player-route")).toBeInTheDocument());
    const saved = JSON.parse(
      window.localStorage.getItem("lumen.playlists.v1")!,
    ) as { playlists: { title: string; itemNames: string[] }[] };
    expect(
      saved.playlists.some(
        (p) => p.title === "Favorite TV Channels" && p.itemNames.includes("https://example.com/live.m3u8"),
      ),
    ).toBe(true);
    expect(transport.sent.some((r) => r.cmd === "session.open")).toBe(true);
  });

  it("Cancel exits to /player without opening anything", () => {
    const { transport } = renderScreen();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByTestId("player-route")).toBeInTheDocument();
    expect(transport.sent.length).toBe(0);
  });

  it("renders persisted recents and refills the field on click", () => {
    window.localStorage.setItem(
      "lumen.streams.recent.v1",
      JSON.stringify(["https://a.example/1", "https://b.example/2"]),
    );
    renderScreen();
    expect(screen.getByText("https://a.example/1")).toBeInTheDocument();
    expect(screen.getByText("https://b.example/2")).toBeInTheDocument();
    fireEvent.click(screen.getByText("https://b.example/2"));
    expect((screen.getByLabelText("Network URL") as HTMLTextAreaElement).value).toBe("https://b.example/2");
  });

  it("shows the honest empty recents state", () => {
    renderScreen();
    expect(screen.getByText("No recent streams")).toBeInTheDocument();
  });
});
