// /open-network-stream tests (Solstice v2 craft pass) — pins the audit fixes:
//   * calm backdrop while mounted, cleaned up on unmount (brief §1);
//   * the URL form validates (http/https only) and submits on Enter through
//     `session.open`, then routes to /player (the demo-transport flow);
//   * a secondary "Back to player" affordance always exits to /player;
//   * recent streams persist to localStorage and refill the field;
//   * the Open action is a standard-size primary button in an action row
//     (class hook, not a full-width slab).

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
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
  });

  afterEach(() => {
    window.localStorage.clear();
    delete document.documentElement.dataset.backdrop;
  });

  it("veils the backdrop to calm while mounted and cleans up on unmount (v2 §1)", () => {
    const { view } = renderScreen();
    expect(document.documentElement.dataset.backdrop).toBe("calm");
    view.unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("renders the anchored form: label, field, standard primary Open, secondary Back", () => {
    renderScreen();
    expect(screen.getByLabelText("Network URL")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open stream" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back to player" })).toBeInTheDocument();
    // standard-size action row (not a stretched slab)
    expect(document.querySelector(".network-stream__actions")).not.toBeNull();
    expect(screen.getByTestId("network-stream-card")).not.toBeNull();
  });

  it("rejects an invalid URL with the honest error and sends nothing", () => {
    const { transport } = renderScreen();
    fireEvent.change(screen.getByLabelText("Network URL"), { target: { value: "nope" } });
    fireEvent.submit(screen.getByLabelText("Network URL").closest("form")!);
    expect(screen.getByText("Enter a valid http:// or https:// URL")).toBeInTheDocument();
    expect(transport.sent.some((r) => r.cmd === "session.open")).toBe(false);
  });

  it("submits on Enter: session.open with the url source, then routes to /player", async () => {
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

  it("Back to player exits to /player without opening anything", () => {
    const { transport } = renderScreen();
    fireEvent.click(screen.getByRole("button", { name: "Back to player" }));
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
    expect((screen.getByLabelText("Network URL") as HTMLInputElement).value).toBe("https://b.example/2");
  });

  it("shows the honest empty recents state", () => {
    renderScreen();
    expect(screen.getByText("No recent streams")).toBeInTheDocument();
  });
});
