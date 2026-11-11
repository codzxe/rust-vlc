// Open-media dialog flow tests (the decoupled-trigger + ONE-funnel
// contracts, unchanged by the W26 w26.b13 redesign).
//
// Pins the open-media surface's behavior:
//   * single click on a list item OPENS it (session.open with the real
//     SourceSpec) and closes the dialog (the old double-click trap is gone);
//   * the dialog carries a visible Close affordance; Escape closes;
//   * the paste-a-path input opens through the same seam, with honest
//     validation; a PLAYLIST path loads the list instead (playlist.load —
//     the 4.D1 handoff, issue #3);
//   * source locations render in full (never ellipsized away);
//   * source rows wear the honest kind glyphs (file / network stream).

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { MemoryRouter, useLocation } from "react-router-dom";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandTransport } from "../../client/transport";
import type { CommandRequest, CommandResponse, Event } from "../../client/transport";
import {
  OpenMediaDialog,
  OPEN_MEDIA_EVENT,
  parseSourceInput,
  sourceIcon,
} from "./OpenMediaDialog";

/** Transport that records every sent command and answers successfully. */
class RecordingTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return { gen: this.sent.length, requestId: request.requestId, result: { completed: { kind: "unit" } } };
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}

/** Exposes the router location for navigation assertions. */
let currentPath = "";
function LocationProbe(): null {
  currentPath = useLocation().pathname;
  return null;
}

function renderDialog(open: boolean, onClose: () => void, transport: CommandTransport) {
  return render(
    <ClientProvider transport={transport}>
      <MemoryRouter initialEntries={["/player"]}>
        <LocationProbe />
        <OpenMediaDialog open={open} onClose={onClose} />
      </MemoryRouter>
    </ClientProvider>,
  );
}

describe("parseSourceInput — the paste-a-path contract", () => {
  it("http(s) and other scheme'd strings open as url sources", () => {
    expect(parseSourceInput("https://example.com/a.m3u8")).toEqual({
      kind: "url",
      url: "https://example.com/a.m3u8",
    });
    expect(parseSourceInput("  HTTP://Example.com/x  ")).toEqual({
      kind: "url",
      url: "HTTP://Example.com/x",
    });
  });

  it("everything else is a file path; blank input is not a source", () => {
    expect(parseSourceInput("/media/demo/big-buck-bunny.mkv")).toEqual({
      kind: "file",
      path: "/media/demo/big-buck-bunny.mkv",
    });
    expect(parseSourceInput("   ")).toBeNull();
    expect(parseSourceInput("")).toBeNull();
  });
});

describe("OpenMediaDialog — v2 §4.1 single-click open flow", () => {
  it("renders the clip catalog with FULL source locations (no ellipsized paths)", () => {
    renderDialog(true, () => {}, new RecordingTransport());
    expect(screen.getByRole("heading", { name: "Open media" })).toBeInTheDocument();
    const bunny = screen.getByRole("button", { name: /Big Buck Bunny/ });
    expect(bunny.textContent).toContain("/media/demo/big-buck-bunny.mkv");
    const radio = screen.getByRole("button", { name: /Night radio/ });
    expect(radio.textContent).toContain("https://ice1.somafm.com/groovesalad-128-mp3");
    // title attribute carries the untruncated string for hover/AT.
    expect(
      screen.getByTitle("https://demo.lumen.dev/live/edge-hsl.m3u8"),
    ).toBeInTheDocument();
  });

  it("single click OPENS the clip: session.open with the real source, dialog closes, routes to /player", async () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    fireEvent.click(screen.getByRole("button", { name: /Big Buck Bunny/ }));
    await waitFor(() => {
      const open = transport.sent.find((r) => r.cmd === "session.open");
      expect(open?.args).toEqual({
        source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" },
      });
    });
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
      expect(currentPath).toBe("/player");
    });
  });

  it("carries a visible Close affordance that closes without opening anything", () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(transport.sent.filter((r) => r.cmd === "session.open")).toHaveLength(0);
  });

  it("Escape closes (Dialog focus trap owns the key)", () => {
    const onClose = vi.fn();
    renderDialog(true, onClose, new RecordingTransport());
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledWith("escape");
  });

  it("the paste-a-path input opens a URL through the same session.open seam", async () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    const input = screen.getByLabelText("File path or URL");
    fireEvent.change(input, { target: { value: "https://example.com/night.mp3" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => {
      const open = transport.sent.find((r) => r.cmd === "session.open");
      expect(open?.args).toEqual({
        source: { kind: "url", url: "https://example.com/night.mp3" },
      });
    });
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  it("a pasted PLAYLIST path loads the list (playlist.load, append: false) and routes to /playlist — the 4.D1 handoff through the ONE funnel (issue #3)", async () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    const input = screen.getByLabelText("File path or URL");
    fireEvent.change(input, { target: { value: "/media/queues/chill.m3u" } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    await waitFor(() => {
      const load = transport.sent.find((r) => r.cmd === "playlist.load");
      expect(load?.args).toEqual({ path: "/media/queues/chill.m3u", append: false });
    });
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
      expect(currentPath).toBe("/playlist");
    });
    // One funnel, one kind: a playlist open never opens a session too.
    expect(transport.sent.filter((r) => r.cmd === "session.open")).toHaveLength(0);
  });

  it("blank path input is honestly invalid — no fabricated source, no close", () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    const input = screen.getByLabelText("File path or URL");
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.submit(input.closest("form") as HTMLFormElement);
    expect(screen.getByText("Enter a file path or an http(s) URL")).toBeInTheDocument();
    expect(transport.sent.filter((r) => r.cmd === "session.open")).toHaveLength(0);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("closed dialog renders nothing (no ghost scrim)", () => {
    const { container } = renderDialog(false, () => {}, new RecordingTransport());
    expect(container.querySelector(".lumen-scrim")).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("OpenMediaDialog — the decoupled trigger contract", () => {
  it("OPEN_MEDIA_EVENT is the stable cross-agent event name", () => {
    // The AppShell listener and every CTA dispatch exactly this string —
    // renaming it breaks the shell-wide open flow (cross-agent contract).
    expect(OPEN_MEDIA_EVENT).toBe("lumen:open-media");
  });
});

describe("OpenMediaDialog — source-kind affordances (W26)", () => {
  it("file sources wear the open-file glyph, stream sources the network-stream glyph", () => {
    expect(sourceIcon({ kind: "file", path: "/media/demo/big-buck-bunny.mkv" })).toBe("open-file");
    expect(sourceIcon({ kind: "url", url: "https://ice1.somafm.com/groovesalad-128-mp3" })).toBe(
      "network-stream",
    );
    // every pickable row carries its glyph (decorative, aria-hidden)
    renderDialog(true, () => {}, new RecordingTransport());
    const bunny = screen.getByRole("button", { name: /Big Buck Bunny/ });
    expect(bunny.querySelector(".open-media__item-icon svg")).not.toBeNull();
    expect(bunny.querySelector(".open-media__item-icon svg")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });
});

describe("OpenMediaDialog — modal layer source contract (w26.a08 handoff)", () => {
  // jsdom never cascades the real stylesheet (vitest stubs .css imports),
  // so layering is pinned at the SOURCE level (node:fs, Button.test.tsx
  // precedent). Since w26.a08 the shared .lumen-scrim primitive carries
  // z-index 20 — above the player stage chrome and the menubar popup layer
  // (menu.css z-index 10). This suite guards the RETIREMENT: the host sheet
  // must not reintroduce a :has() scrim override or a local z-index (the
  // Solstice-era patch that used to live here).
  const { dirname } = import.meta as ImportMeta & { dirname?: string };
  const raw = readFileSync(`${dirname}/open-media.css`, "utf8");

  // Comments are prose (they may mention the retired pattern); rules only.
  const css = raw.replace(/\/\*[\s\S]*?\*\//g, "");

  it("carries no :has() scrim override — the shared primitive owns the scrim", () => {
    expect(css).not.toMatch(/:has\(/);
  });

  it("carries no local z-index — the primitive's z-index 20 wins by construction", () => {
    expect(css).not.toMatch(/z-index/);
  });
});
