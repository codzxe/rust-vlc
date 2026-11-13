// Card 4.D2 — /media-info route body tests: the screen mounts the shared
// panel under the i18n heading; honest empty state via the scaffold
// MockTransport (nothing exposed, nothing open).
//
// w26.i18 (h19 §5): the empty state must carry a REAL path to the
// populated body — the "Open media" action firing the shell-wide
// open-media event — and ride the r14 Card grammar (never a bare line).

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { DemoTransport } from "../../client/demo-transport";
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

describe("MediaInfoScreen — the real DemoTransport (w26.k17, j17 P1-1 regression)", () => {
  it("bunny opened through the demo transport renders Duration 10:35 — the wire is µs", async () => {
    // The exact live-verified repro this fix closes: pre-k17 the demo spoke
    // seconds into µs wire fields while this screen formatted ms, so the
    // 10:35 film rendered "0:00". One unit contract now: the wire carries
    // 635 000 000 µs; formatDurationUs renders 10:35. This test FAILS on
    // the old code at the duration assertion (renders 0:00).
    vi.useFakeTimers();
    const transport = new DemoTransport();
    try {
      // The dev shell's open-media flow: session.open for the bunny clip,
      // then the fixed opening choreography (info burst at 320 ms).
      await act(async () => {
        await transport.send({
          args: { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
          cmd: "session.open",
          requestId: 1,
        });
      });
      await act(async () => {
        vi.advanceTimersByTime(400);
      });
      // Late navigation to /media-info: the provider re-seeds from the
      // session-scoped media.get-info probe (j17 §3's verified path).
      vi.useRealTimers();
      const { container } = render(
        <ClientProvider transport={transport}>
          <MediaInfoScreen />
        </ClientProvider>,
      );
      await waitFor(() => {
        expect(container.querySelector('[data-state="ready"]')).not.toBeNull();
      });
      expect(container.querySelector('[data-field="duration"]')?.textContent).toBe(
        "Duration10:35",
      );
    } finally {
      transport.dispose();
      vi.useRealTimers();
    }
  });
});
