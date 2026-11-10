// /discovery tests (card 7.D1, issue #42) — pins the honest-capability
// contract of the discovery panel:
//   * the calm-backdrop veil contract (mount/unmount, like siblings);
//   * the HONEST initial state: no fabricated volumes, no commands sent,
//     the empty state and the recorded network verdict visible;
//   * the live event plumbing through the REAL subscribe seam: the
//     catalog's devices.volume-added/-removed events fold into rows
//     (label + kind chip + id, count chip tracks the table);
//   * the pure fold (applyVolumeEvent): re-add replaces, remove drops,
//     unknown-id removal is a no-op;
//   * untrusted labels (V7) render as text, never markup.

import { act, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport, Event } from "../../client/transport";
import { applyVolumeEvent, DiscoveryScreen } from "./DiscoveryScreen";

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
  it("veils the backdrop to calm while mounted and cleans up on unmount", () => {
    const { view } = renderScreen();
    expect(document.documentElement.dataset.backdrop).toBe("calm");
    view.unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("renders the honest initial state: empty volumes + the network verdict, sends nothing", () => {
    const { transport } = renderScreen();
    expect(screen.getByTestId("discovery-volumes-empty")).toBeInTheDocument();
    expect(screen.getByTestId("discovery-network")).toBeInTheDocument();
    // The honest network verdict is visible (status names the follow-up).
    expect(screen.getByText(/SSDP\/UPnP search is implemented but not reachable/)).toBeInTheDocument();
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
