// Card 4.D1 — /playlist route body tests (Solstice v2 APP-tier pass):
// the page veils the backdrop to "calm" while mounted (cleaned up on
// unmount), the queue card anchors under the page heading, the panel hides
// its duplicate h2 in page mode, and the EMPTY STATE TEACHES — its primary
// CTA dispatches the shell-wide `lumen:open-media` CustomEvent (the
// AppShell owns the single listener; regression for the audit defect
// "0 items state should teach").

import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
import { PlaylistScreen } from "./PlaylistScreen";

class NullTransport implements CommandTransport {
  subscribe(): () => void {
    return () => {};
  }
  async send(request: CommandRequest): Promise<CommandResponse> {
    return { gen: 1, requestId: request.requestId, result: { completed: { kind: "unit" } } };
  }
}

const renderScreen = () =>
  render(
    <ClientProvider transport={new NullTransport()}>
      <PlaylistScreen />
    </ClientProvider>,
  );

describe("PlaylistScreen (/playlist route body)", () => {
  afterEach(() => {
    delete document.documentElement.dataset.backdrop;
  });

  it("veils the backdrop to calm while mounted and cleans up on unmount", () => {
    const { unmount } = renderScreen();
    expect(document.documentElement.dataset.backdrop).toBe("calm");
    unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("renders ONE page heading — the panel's own h2 is hidden in page mode", () => {
    renderScreen();
    expect(screen.getAllByRole("heading", { name: "Playlist" }).length).toBe(1);
  });

  it("anchors heading and queue card on the same column (page column + card classes)", () => {
    const { container } = renderScreen();
    expect(container.querySelector(".lumen-playlist-page")).not.toBeNull();
    expect(container.querySelector(".lumen-playlist-page__title")).not.toBeNull();
    expect(container.querySelector(".lumen-playlist-page__panel.lumen-playlist")).not.toBeNull();
  });

  it("empty state teaches: the Open media CTA dispatches the shell open-media event", () => {
    renderScreen();
    const onOpenMedia = vi.fn();
    window.addEventListener("lumen:open-media", onOpenMedia);
    fireEvent.click(screen.getByTestId("playlist-empty-open-media"));
    window.removeEventListener("lumen:open-media", onOpenMedia);
    expect(onOpenMedia).toHaveBeenCalledTimes(1);
  });

  it("empty state keeps the honest copy (no invented queue data)", () => {
    renderScreen();
    expect(screen.getByText("Nothing in the queue")).toBeInTheDocument();
    expect(screen.getByText("Files you open are listed here in order.")).toBeInTheDocument();
    expect(
      screen.getByText("Pick a file or drop it anywhere — playlists arrive with the desktop build."),
    ).toBeInTheDocument();
  });
});
