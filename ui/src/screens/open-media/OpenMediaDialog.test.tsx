// Open-media dialog flow tests (the decoupled-trigger + ONE-funnel
// contracts, unchanged by the W26 w26.b13 redesign; fixer pass w26.k16).
//
// Pins the open-media surface's behavior:
//   * single click on a list item OPENS it (session.open with the real
//     SourceSpec) and closes the dialog (the old double-click trap is gone);
//   * the dialog carries a visible Close affordance; Escape closes;
//   * the paste-a-path input opens through the same seam, with honest
//     validation; a PLAYLIST path loads the list instead (playlist.load —
//     the 4.D1 handoff, issue #3);
//   * source locations render in full (never ellipsized away);
//   * source rows wear the honest kind glyphs (file / network stream);
//   * w26.k16 (j16 P2-3): initial focus lands on the FIRST FIELD, never the
//     ✕ glyph — the w26.i17 fix the two sibling dialogs already had;
//   * w26.k16 (j16 P3-10): the demo clips render only for the DEMO transport
//     (a browser host with a real transport sees none; the desktop shell
//     never does);
//   * w26.k16 (j16 P3-17): rows are plain activation buttons (no
//     focus-as-selection, no aria-pressed) and the footer Open is the
//     paste-a-path form's submit button — always a live control.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { MemoryRouter, useLocation } from "react-router-dom";
import { ClientProvider } from "../../client/ClientProvider";
import { DemoTransport } from "../../client/demo-transport";
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

/** The demo clips render only where isDemoTransport() is true (w26.k16,
 * j16 P3-10) — this double keeps `instanceof DemoTransport` while recording
 * the wire. DISPOSE after use: the demo clock + opening choreography run on
 * real timers (enqueueSources.test.tsx precedent). */
class RecordingDemoTransport extends DemoTransport {
  readonly sent: CommandRequest[] = [];

  override async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return super.send(request);
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
    const transport = new RecordingDemoTransport();
    try {
      renderDialog(true, () => {}, transport);
      expect(screen.getByRole("heading", { name: "Open media" })).toBeInTheDocument();
      const bunny = screen.getByRole("button", { name: /Big Buck Bunny/ });
      expect(bunny.textContent).toContain("/media/demo/big-buck-bunny.mkv");
      const radio = screen.getByRole("button", { name: /Night radio/ });
      expect(radio.textContent).toContain("https://ice1.somafm.com/groovesalad-128-mp3");
      // title attribute carries the untruncated string for hover/AT.
      expect(
        screen.getByTitle("https://demo.lumen.dev/live/edge-hsl.m3u8"),
      ).toBeInTheDocument();
    } finally {
      transport.dispose(); // the demo clock + choreography timers
    }
  });

  it("single click OPENS the clip: session.open with the real source, dialog closes, routes to /player", async () => {
    const transport = new RecordingDemoTransport();
    const onClose = vi.fn();
    try {
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
    } finally {
      transport.dispose();
    }
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

describe("OpenMediaDialog — first-field focus (w26.k16, j16 P2-3 — the w26.i17 family fix)", () => {
  it("initial focus sits on the path field, never the ✕ glyph", () => {
    renderDialog(true, () => {}, new RecordingTransport());
    const dialog = screen.getByRole("dialog", { name: "Open media" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    // The trap's first-focusable is the ✕ button; the screen moves initial
    // focus to the FIRST FIELD so no ring paints on the close glyph at rest
    // and Enter at rest submits instead of dismissing (the same fix the
    // netstream and shortcuts dialogs already carry).
    expect(document.activeElement).toBe(screen.getByLabelText("File path or URL"));
    expect(document.activeElement).not.toBe(screen.getByRole("button", { name: "Close" }));
  });

  it("re-opening (the AppShell toggles `open` in place) moves focus back to the path field", () => {
    const transport = new RecordingTransport();
    const view = render(
      <ClientProvider transport={transport}>
        <MemoryRouter initialEntries={["/player"]}>
          <OpenMediaDialog open={false} onClose={() => {}} />
        </MemoryRouter>
      </ClientProvider>,
    );
    expect(screen.queryByRole("dialog")).toBeNull();
    view.rerender(
      <ClientProvider transport={transport}>
        <MemoryRouter initialEntries={["/player"]}>
          <OpenMediaDialog open onClose={() => {}} />
        </MemoryRouter>
      </ClientProvider>,
    );
    expect(document.activeElement).toBe(screen.getByLabelText("File path or URL"));
  });
});

describe("OpenMediaDialog — demo-clip gating (w26.k16, j16 P3-10)", () => {
  it("a browser host with a NON-demo transport sees no demo clips (no fake /media/demo rows)", () => {
    // The old shell-only gate would have offered the four simulation
    // fixtures to any browser-context host — the exact "dead controls lying
    // about their effect" pattern the gating fix closes.
    renderDialog(true, () => {}, new RecordingTransport());
    expect(screen.queryByRole("heading", { name: "Demo clips" })).toBeNull();
    expect(screen.queryByRole("button", { name: /Big Buck Bunny/ })).toBeNull();
  });

  it("the demo transport in the browser shell still lists the clips", () => {
    const transport = new RecordingDemoTransport();
    try {
      renderDialog(true, () => {}, transport);
      expect(screen.getByRole("heading", { name: "Demo clips" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Big Buck Bunny/ })).toBeInTheDocument();
    } finally {
      transport.dispose();
    }
  });

  it("the desktop shell hides the clips even for the demo transport", () => {
    // jsdom is a browser context: the desktop probe reads the Tauri
    // internals — stubbed here (enqueueSources.test.tsx precedent).
    const transport = new RecordingDemoTransport();
    const host = globalThis as { __TAURI_INTERNALS__?: unknown };
    host.__TAURI_INTERNALS__ = {
      invoke: () => Promise.resolve({}),
      transformCallback: () => 0,
    };
    try {
      renderDialog(true, () => {}, transport);
      expect(screen.queryByRole("heading", { name: "Demo clips" })).toBeNull();
      expect(screen.queryByRole("button", { name: /Big Buck Bunny/ })).toBeNull();
    } finally {
      delete host.__TAURI_INTERNALS__;
      transport.dispose();
    }
  });
});

describe("OpenMediaDialog — plain activation rows + live footer Open (w26.k16, j16 P3-17)", () => {
  it("rows carry no aria-pressed and focusing one mutates nothing", () => {
    const transport = new RecordingDemoTransport();
    try {
      renderDialog(true, () => {}, transport);
      const bunny = screen.getByRole("button", { name: /Big Buck Bunny/ });
      bunny.focus();
      // Focus-as-selection was a nonstandard conflation: rows used to light
      // up (aria-pressed) on focus and tabbing through enabled the footer
      // Open — plain activation buttons do none of that.
      expect(bunny).not.toHaveAttribute("aria-pressed");
      expect(transport.sent).toHaveLength(0);
      // The footer confirm is ALWAYS live (it submits the pasted path).
      expect(screen.getByRole("button", { name: "Open" })).toBeEnabled();
    } finally {
      transport.dispose();
    }
  });

  it("footer Open submits the pasted path through the same session.open seam", async () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    fireEvent.change(screen.getByLabelText("File path or URL"), {
      target: { value: "https://example.com/night.mp3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
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

  it("footer Open with a blank path is honestly invalid (live confirm, no fabricated source)", () => {
    const transport = new RecordingTransport();
    const onClose = vi.fn();
    renderDialog(true, onClose, transport);
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    expect(screen.getByText("Enter a file path or an http(s) URL")).toBeInTheDocument();
    expect(transport.sent.filter((r) => r.cmd === "session.open")).toHaveLength(0);
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("OpenMediaDialog — source-kind affordances (W26)", () => {
  it("file sources wear the open-file glyph, stream sources the network-stream glyph", () => {
    expect(sourceIcon({ kind: "file", path: "/media/demo/big-buck-bunny.mkv" })).toBe("open-file");
    expect(sourceIcon({ kind: "url", url: "https://ice1.somafm.com/groovesalad-128-mp3" })).toBe(
      "network-stream",
    );
    // every pickable row carries its glyph (decorative, aria-hidden)
    const transport = new RecordingDemoTransport();
    try {
      renderDialog(true, () => {}, transport);
      const bunny = screen.getByRole("button", { name: /Big Buck Bunny/ });
      expect(bunny.querySelector(".open-media__item-icon svg")).not.toBeNull();
      expect(bunny.querySelector(".open-media__item-icon svg")).toHaveAttribute(
        "aria-hidden",
        "true",
      );
    } finally {
      transport.dispose();
    }
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
