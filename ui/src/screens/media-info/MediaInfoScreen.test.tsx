// Card 4.D2 — /media-info route body tests: the screen mounts the shared
// panel under the i18n heading; honest empty state via the scaffold
// MockTransport (nothing exposed, nothing open).
//
// w26.i18 (h19 §5): the empty state must carry a REAL path to the
// populated body — the "Open media" action firing the shell-wide
// open-media event — and ride the r14 Card grammar (never a bare line).

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { MediaInfo } from "../../client/transport";
import { OPEN_MEDIA_EVENT } from "../open-media";
import { MediaInfoScreen } from "./MediaInfoScreen";
import { fullMediaInfo } from "./fixtures";
import type { CommandResponse, CommandTransport } from "../../client/transport";
import type { CommandRequest } from "../../client/transport";

class FixtureTransport implements CommandTransport {
  constructor(private readonly info: MediaInfo) {}
  subscribe(): () => void {
    return () => {};
  }
  async send(request: CommandRequest): Promise<CommandResponse> {
    return {
      gen: 1,
      requestId: request.requestId,
      result: { completed: { kind: "media-info", info: this.info } },
    };
  }
}

function renderScreen(transport: CommandTransport) {
  return render(
    <ClientProvider transport={transport}>
      <MediaInfoScreen />
    </ClientProvider>,
  );
}

describe("MediaInfoScreen (/media-info route body)", () => {
  it("renders the screen heading and the fixture-driven sections", async () => {
    const { container } = renderScreen(new FixtureTransport(fullMediaInfo));
    expect(screen.getByRole("heading", { level: 1, name: "Media information" })).toBeTruthy();
    await waitFor(() => {
      expect(screen.getByText("Matroska (MKV)")).toBeTruthy();
    });
    expect(container.querySelector('[data-screen="media-info"]')).not.toBeNull();
    expect(container.querySelector('[data-mi-section="metadata"]')).not.toBeNull();
  });

  it("renders the honest empty state when nothing is open", async () => {
    const nothingOpen: CommandTransport = {
      subscribe: () => () => {},
      async send(request) {
        return {
          gen: 1,
          requestId: request.requestId,
          result: {
            error: { code: "not-found", details: {}, message: "no session", stage: "session" },
          },
        };
      },
    };
    const { container } = renderScreen(nothingOpen);
    await waitFor(() => {
      expect(container.querySelector('[data-state="empty"]')).not.toBeNull();
    });
    expect(screen.getByText("No media open")).toBeTruthy();
    // w26.i18: the empty state rides the r14 Card grammar — the route has a
    // visible body even before any media is open (h19 §5).
    expect(container.querySelector('[data-testid="media-info-empty-card"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="media-info-empty-card"]')?.classList).toContain(
      "lumen-card",
    );
  });

  it("empty state: Open media fires the shell-wide open-media event (the one real seam)", async () => {
    const nothingOpen: CommandTransport = {
      subscribe: () => () => {},
      async send(request) {
        return {
          gen: 1,
          requestId: request.requestId,
          result: {
            error: { code: "not-found", details: {}, message: "no session", stage: "session" },
          },
        };
      },
    };
    renderScreen(nothingOpen);
    const cta = await screen.findByTestId("media-info-open");
    expect(cta.textContent).toBe("Open media");
    const seen: string[] = [];
    const onEvent = (event: Event): void => {
      seen.push(event.type);
    };
    window.addEventListener(OPEN_MEDIA_EVENT, onEvent);
    try {
      fireEvent.click(cta);
    } finally {
      window.removeEventListener(OPEN_MEDIA_EVENT, onEvent);
    }
    expect(seen).toEqual([OPEN_MEDIA_EVENT]);
  });
});
