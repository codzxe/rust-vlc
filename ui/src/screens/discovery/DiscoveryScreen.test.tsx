// /discovery tests (card 7.D1, issue #42) — pins the honest-capability
// contract of the discovery panel:
//   * the Solstice data-backdrop veil is GONE (W25 §0 retirement): the
//     root stays clean of backdrop tiers for the screen's whole life
//     cycle, mount and unmount alike;
//   * the W26 structure: page heading + 15px-class section headers
//     (SectionHeader grammar) + the r14 panel cards;
//   * the HONEST initial state: no fabricated volumes, no commands sent,
//     the quiet empty state and the recorded network verdict visible;
//   * the live event plumbing through the REAL subscribe seam: the
//     catalog's devices.volume-added/-removed events fold into rows
//     (label + kind meta + id, count chip tracks the table);
//   * the pure fold (applyVolumeEvent): re-add replaces, remove drops,
//     unknown-id removal is a no-op;
//   * untrusted labels (V7) render as text, never markup;
//   * w26.c6 — the volume-row context menu (folder context menu.svg): the
//     frame's 6 pinned rows open on right-click; "Add file…" fires the
//     shell open-media event (the one real seam) and the five no-seam rows
//     render honestly disabled.

import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport, Event } from "../../client/transport";
import { OPEN_MEDIA_EVENT } from "../open-media";
import { applyVolumeEvent, DiscoveryScreen } from "./DiscoveryScreen";

/** The DOM Event (the catalog `Event` type is imported as a spy shape). */
type DomEvent = globalThis.Event;

/** Transport double with a test-controlled event emitter (the sanctioned
 * seam: production code only ever sees subscribe/send). */
class EmitterTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: Event): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return { gen: 1, requestId: request.requestId, result: { completed: { kind: "unit" } } };
  }
}

const renderScreen = () => {
  const transport = new EmitterTransport();
  const view = render(
    <MemoryRouter initialEntries={["/discovery"]}>
      <Routes>
        <Route path="/discovery" element={<DiscoveryScreen />} />
      </Routes>
    </MemoryRouter>,
    { wrapper: (ui) => <ClientProvider transport={transport}>{ui.children}</ClientProvider> },
  );
  return { transport, view };
};

afterEach(() => {
  delete document.documentElement.dataset.backdrop;
});

describe("applyVolumeEvent (pure fold)", () => {
  const added = { kind: "devices.volume-added" as const, id: "usb-1", label: "Sandisk", volumeKind: "removable" as const };

  it("adds a row and re-add replaces it (label/kind may change)", () => {
    const empty = new Map();
    const one = applyVolumeEvent(empty, added);
    expect(one.get("usb-1")).toEqual({ id: "usb-1", label: "Sandisk", volumeKind: "removable" });

    const renamed = applyVolumeEvent(one, {
      kind: "devices.volume-added",
      id: "usb-1",
      label: "Backup",
      volumeKind: "fixed",
    });
    expect(renamed.size).toBe(1);
    expect(renamed.get("usb-1")).toEqual({ id: "usb-1", label: "Backup", volumeKind: "fixed" });
  });

  it("removal drops the row; an unknown id is a no-op", () => {
    const one = applyVolumeEvent(new Map(), added);
    const gone = applyVolumeEvent(one, { kind: "devices.volume-removed", id: "usb-1" });
    expect(gone.size).toBe(0);
    expect(applyVolumeEvent(gone, { kind: "devices.volume-removed", id: "ghost" }).size).toBe(0);
  });

  it("preserves insertion order across separate ids", () => {
    let table = applyVolumeEvent(new Map(), added);
    table = applyVolumeEvent(table, {
      kind: "devices.volume-added",
      id: "cd-0",
      label: "Disc",
      volumeKind: "optical",
    });
    expect([...table.keys()]).toEqual(["usb-1", "cd-0"]);
  });
});

describe("DiscoveryScreen (/discovery)", () => {
  it("never sets a backdrop tier (Solstice veil retired) across its life cycle", () => {
    const { view } = renderScreen();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
    view.unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("renders the W26 structure: page heading + section headers + panel cards", () => {
    renderScreen();
    expect(screen.getByRole("heading", { level: 1, name: "Discovery" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "Local volumes" })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: "Network services (UPnP)" }),
    ).toBeInTheDocument();
    // Both lanes render the r14 panel card (thumbnail.svg grammar).
    expect(screen.getByTestId("discovery-card")).toBeInTheDocument();
    expect(screen.getByTestId("discovery-network")).toBeInTheDocument();
  });

  it("renders the honest initial state: empty volumes + the network verdict, sends nothing", () => {
    const { transport } = renderScreen();
    expect(screen.getByTestId("discovery-volumes-empty")).toBeInTheDocument();
    expect(screen.getByTestId("discovery-network")).toBeInTheDocument();
    // The honest network verdict is visible (status names the follow-up).
    expect(screen.getByText(/SSDP\/UPnP search is implemented but not reachable/)).toBeInTheDocument();
    // The quiet empty state carries its title + explanatory body.
    expect(screen.getByText("No volumes reported yet")).toBeInTheDocument();
    expect(
      screen.getByText(/Volume watching arrives with the platform integrations/),
    ).toBeInTheDocument();
    // Count chip reads zero.
    expect(screen.getByText("0")).toBeInTheDocument();
    // No device rows exist and no command was sent — the screen is
    // read-only event plumbing, never a fabricated snapshot.
    expect(screen.queryAllByTestId("discovery-volume")).toHaveLength(0);
    expect(transport.sent).toHaveLength(0);
  });

  it("folds live devices.volume-added events into rows through the real subscribe seam", () => {
    const { transport } = renderScreen();
    act(() => {
      transport.emit({
        kind: "devices.volume-added",
        id: "usb-7",
        label: "Field Kit",
        volumeKind: "removable",
        seq: 1,
      });
      transport.emit({
        kind: "devices.volume-added",
        id: "nas-1",
        label: "Attic Share",
        volumeKind: "network",
        seq: 2,
      });
    });
    const rows = screen.getAllByTestId("discovery-volume");
    expect(rows).toHaveLength(2);
    expect(screen.getByText("Field Kit")).toBeInTheDocument();
    expect(screen.getByText("Removable")).toBeInTheDocument();
    expect(screen.getByText("Attic Share")).toBeInTheDocument();
    expect(screen.getByText("Network mount")).toBeInTheDocument();
    // ids render as the row's opaque identity.
    expect(screen.getByText("usb-7")).toBeInTheDocument();
    expect(screen.getByText("nas-1")).toBeInTheDocument();
    // Count chip tracks the table.
    expect(screen.getByText("2")).toBeInTheDocument();
    // The empty state is gone.
    expect(screen.queryByTestId("discovery-volumes-empty")).toBeNull();
  });

  it("drops the row when devices.volume-removed arrives; other events are ignored", () => {
    const { transport } = renderScreen();
    act(() => {
      transport.emit({
        kind: "devices.volume-added",
        id: "usb-7",
        label: "Field Kit",
        volumeKind: "removable",
        seq: 1,
      });
    });
    // An unrelated catalog event must not touch the table.
    act(() => {
      transport.emit({ kind: "playback.rate-changed", rate: 1.25, seq: 2 });
    });
    expect(screen.getAllByTestId("discovery-volume")).toHaveLength(1);
    act(() => {
      transport.emit({ kind: "devices.volume-removed", id: "usb-7", seq: 3 });
    });
    expect(screen.queryAllByTestId("discovery-volume")).toHaveLength(0);
    expect(screen.getByTestId("discovery-volumes-empty")).toBeInTheDocument();
  });

  it("renders an untrusted label as text, never markup (V7)", () => {
    const { transport } = renderScreen();
    act(() => {
      transport.emit({
        kind: "devices.volume-added",
        id: "evil",
        label: "<img src=x onerror=alert(1)> USB",
        volumeKind: "removable",
        seq: 1,
      });
    });
    // The label text is present (escaped), and no img element exists.
    expect(screen.getByText("<img src=x onerror=alert(1)> USB")).toBeInTheDocument();
    expect(document.querySelector("img")).toBeNull();
  });
});

describe("DiscoveryScreen — volume-row context menu (w26.c6, folder context menu.svg)", () => {
  /** Emit one removable volume and right-click its row; returns the menu. */
  const openVolumeMenu = async () => {
    const { transport } = renderScreen();
    act(() => {
      transport.emit({
        kind: "devices.volume-added",
        id: "usb-1",
        label: "Field Kit",
        volumeKind: "removable",
        seq: 1,
      });
    });
    const row = screen.getByTestId("discovery-volume");
    // The ContextMenu wrapper sits inside the <li>; target it so the
    // contextmenu event reaches its handler (an ancestor dispatch would
    // bubble past it, not through it).
    fireEvent.contextMenu(row.querySelector<HTMLElement>(".lumen-context-menu")!, { clientX: 60, clientY: 40 });
    return await screen.findByRole("menu", { name: "Volume actions" });
  };

  it("opens the frame's 6 rows in pinned order (plus · folder · subtitles · audio · refresh · info)", async () => {
    const menu = await openVolumeMenu();
    const labels = [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')].map((item) => item.textContent);
    expect(labels).toEqual(["Add file…", "Add folder…", "Add subtitle tracks", "Add audio tracks", "Refresh", "Info"]);
    // WAI-ARIA: opening a menu moves focus onto the first item.
    expect(menu.querySelector<HTMLElement>('[role="menuitem"]')).toHaveFocus();
  });

  it("Add file… fires the shell-wide open-media event (the one real seam)", async () => {
    const menu = await openVolumeMenu();
    const seen: string[] = [];
    const onEvent = (event: DomEvent): void => {
      seen.push(event.type);
    };
    window.addEventListener(OPEN_MEDIA_EVENT, onEvent);
    try {
      fireEvent.click(within(menu).getByText("Add file…"));
    } finally {
      window.removeEventListener(OPEN_MEDIA_EVENT, onEvent);
    }
    expect(seen).toEqual([OPEN_MEDIA_EVENT]);
    // Activation dismisses the menu (the menu API's onDismissAll).
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("no-seam rows render honestly disabled (folder/subtitles/audio/refresh/info)", async () => {
    const menu = await openVolumeMenu();
    for (const label of ["Add folder…", "Add subtitle tracks", "Add audio tracks", "Refresh", "Info"]) {
      const item = [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
        (row) => row.textContent === label,
      )!;
      expect(item, label).toHaveAttribute("aria-disabled", "true");
    }
  });

  it("Escape closes the menu (menu API)", async () => {
    const menu = await openVolumeMenu();
    fireEvent.keyDown(within(menu).getByText("Add file…"), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });
});
