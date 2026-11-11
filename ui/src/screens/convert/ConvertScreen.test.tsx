// /convert tests (card 7.E3, issue #44) — pins the capability honesty:
//   * the target picker (the W26 menu API) offers EXACTLY the engine's two
//     real operations (WAV record / MP4-remux) with their honest notes;
//   * the screen probes the command catalog (`system.list-commands`) for a
//     conversion lane; with the REAL namespaces (no convert.* / record.*
//     commands — today's truth) the honest unavailable banner renders;
//   * a Start enqueues a job that transitions to `blocked` with the typed
//     not-exposed reason — NEVER a simulated progress bar, and no request
//     is fabricated beyond the catalog probe;
//   * the Browse action is the honest disabled-from-data stub (no native
//     picker command exists — the extensions.tsx precedent);
//   * the queue bound refuses the 9th job; terminal rows show no cancel;
//   * a transport that lists convert.* commands (the future lane) gets a
//     queued job and no banner — the engine-binding seam's other branch.
// The full lifecycle arithmetic (start/progress/complete/fail/cancel) is
// pinned by convertQueue.test.ts against the pure reducer.

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
import type { CommandListing } from "../../generated/commands";
import { ConvertScreen } from "./ConvertScreen";

const listing = (id: string): CommandListing => ({
  class: "s",
  deprecated: null,
  id,
  requires: [],
  since: "0.1.0",
});

/** The namespaces that exist in the engine catalog TODAY (no lane). */
const REAL_TODAY: readonly string[] = [
  "session.open",
  "playback.play",
  "tracks.list",
  "audio.set-volume",
  "media.get-info",
  "playlist.next",
  "system.list-commands",
];

/** A catalog-serving spy: records every request, answers the listing. */
class CatalogTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];

  constructor(private readonly ids: readonly string[]) {}

  subscribe(): () => void {
    return () => {};
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    if (request.cmd === "system.list-commands") {
      return {
        gen: 1,
        requestId: request.requestId,
        result: {
          completed: {
            kind: "command-listings",
            commands: this.ids.map(listing),
          },
        },
      };
    }
    return { gen: 1, requestId: request.requestId, result: { completed: { kind: "unit" } } };
  }
}

const renderScreen = (ids: readonly string[] = REAL_TODAY) => {
  const transport = new CatalogTransport(ids);
  render(
    <MemoryRouter initialEntries={["/convert"]}>
      <Routes>
        <Route path="/convert" element={<ConvertScreen />} />
        <Route path="/player" element={<p data-testid="player-route">player</p>} />
      </Routes>
    </MemoryRouter>,
    { wrapper: (ui) => <ClientProvider transport={transport}>{ui.children}</ClientProvider> },
  );
  return { transport };
};

/** Open the target dropdown (menu API) and return its listbox. */
const openTargets = (): HTMLElement => {
  fireEvent.click(screen.getByRole("button", { name: "Target" }));
  return screen.getByRole("listbox", { name: "Target" });
};

afterEach(() => {
  delete document.documentElement.dataset.backdrop;
});

describe("ConvertScreen (/convert)", () => {
  it("retires the Solstice backdrop veil — mounting touches no data-backdrop tier", () => {
    const view = render(
      <MemoryRouter initialEntries={["/convert"]}>
        <Routes>
          <Route path="/convert" element={<ConvertScreen />} />
        </Routes>
      </MemoryRouter>,
      {
        wrapper: (ui) => (
          <ClientProvider transport={new CatalogTransport(REAL_TODAY)}>{ui.children}</ClientProvider>
        ),
      },
    );
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
    view.unmount();
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("renders the working-screen form with EXACTLY the two real targets and honest notes", async () => {
    renderScreen();
    expect(screen.getByText("Convert / Save")).toBeInTheDocument();
    expect(screen.getByLabelText("Source file")).toBeInTheDocument();
    // the trigger shows the wav default; opening the menu (menu API) lists
    // exactly two options — the engine's real capability set, no MP3/MKV lies
    expect(screen.getByRole("button", { name: "Target" })).toHaveTextContent("Record to WAV");
    const list = openTargets();
    const options = within(list).getAllByRole("option");
    expect(options).toHaveLength(2);
    expect(within(list).getByRole("option", { name: "Record to WAV", selected: true })).toBeInTheDocument();
    expect(within(list).getByRole("option", { name: "Remux to MP4" })).toBeInTheDocument();
    // the wav note shows first; switching flips the note (the honest per-
    // target description)
    expect(screen.getByTestId("convert-target-note").textContent).toMatch(/uncompressed/i);
    fireEvent.click(within(list).getByRole("option", { name: "Remux to MP4" }));
    expect(screen.getByTestId("convert-target-note").textContent).toMatch(/without re-encoding/i);
    // standard action row
    expect(screen.getByRole("button", { name: "Convert" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back to player" })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId("convert-lane-note")).toBeInTheDocument());
  });

  it("the Browse action is the honest disabled stub — no dead enabled control, no fabricated request", () => {
    const { transport } = renderScreen();
    const browse = screen.getByRole("button", { name: "Browse" });
    expect(browse).toBeDisabled();
    expect(browse).toHaveAttribute("data-wiring-stub");
    // the gate reason is wired to the action for AT
    const gate = document.getElementById("convert-browse-gate");
    expect(gate?.textContent).toMatch(/not exposed in this build/i);
    expect(browse.getAttribute("aria-describedby")).toContain("convert-browse-gate");
    expect(transport.sent.filter((r) => r.cmd !== "system.list-commands")).toHaveLength(0);
  });

  it("probes the command catalog once on mount and shows the honest unavailable banner", async () => {
    const { transport } = renderScreen();
    await waitFor(() =>
      expect(
        transport.sent.filter((r) => r.cmd === "system.list-commands"),
      ).toHaveLength(1),
    );
    expect(screen.getByTestId("convert-lane-note").textContent).toMatch(
      /no conversion commands/i,
    );
  });

  it("refuses an empty source with the honest error and enqueues nothing", () => {
    const { transport } = renderScreen();
    fireEvent.submit(screen.getByLabelText("Source file").closest("form")!);
    expect(screen.getByText("Enter a source file path")).toBeInTheDocument();
    expect(screen.getByText("No conversions queued")).toBeInTheDocument();
    expect(transport.sent.filter((r) => r.cmd !== "system.list-commands")).toHaveLength(0);
  });

  it("enqueues a job that is BLOCKED with the typed reason — never a fabricated request", async () => {
    const { transport } = renderScreen();
    fireEvent.change(screen.getByLabelText("Source file"), {
      target: { value: "/media/demo/bunny.mkv" },
    });
    const list = openTargets();
    fireEvent.click(within(list).getByRole("option", { name: "Remux to MP4" }));
    fireEvent.submit(screen.getByLabelText("Source file").closest("form")!);
    // the row: name, target, blocked status, honest reason
    expect(screen.getByText("bunny.mkv")).toBeInTheDocument();
    expect(screen.getByTestId("convert-job-status-1").textContent).toBe("Unavailable in this build");
    expect(screen.getByTestId("convert-job-detail-1").textContent).toMatch(
      /not exposed by this build/i,
    );
    // terminal row → NO cancel affordance
    expect(screen.queryByTestId("convert-job-cancel-1")).toBeNull();
    // the only wire traffic is the catalog probe — no invented command id
    expect(transport.sent.map((r) => r.cmd)).toEqual(["system.list-commands"]);
    // the form cleared for the next job
    expect((screen.getByLabelText("Source file") as HTMLInputElement).value).toBe("");
  });

  it("refuses the 9th job with the honest bound error", () => {
    renderScreen();
    const form = screen.getByLabelText("Source file").closest("form")!;
    for (let i = 1; i <= 8; i += 1) {
      fireEvent.change(screen.getByLabelText("Source file"), {
        target: { value: `/media/demo/take-${i}.flac` },
      });
      fireEvent.submit(form);
    }
    expect(screen.getAllByRole("listitem")).toHaveLength(8);
    fireEvent.change(screen.getByLabelText("Source file"), {
      target: { value: "/media/demo/one-too-many.flac" },
    });
    fireEvent.submit(form);
    expect(screen.getByText(/queue is full/i)).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(8);
  });

  it("a transport that lists convert.* commands gets a QUEUED job and no banner (the seam's other branch)", async () => {
    renderScreen([...REAL_TODAY, "convert.start"]);
    await waitFor(() => expect(screen.queryByTestId("convert-lane-note")).toBeNull());
    fireEvent.change(screen.getByLabelText("Source file"), {
      target: { value: "/media/demo/bunny.mkv" },
    });
    fireEvent.submit(screen.getByLabelText("Source file").closest("form")!);
    expect(screen.getByTestId("convert-job-status-1").textContent).toBe("Queued");
    // queued is cancellable — the lifecycle affordance renders
    expect(screen.getByTestId("convert-job-cancel-1")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("convert-job-cancel-1"));
    expect(screen.getByTestId("convert-job-status-1").textContent).toBe("Cancelled");
  });

  it("Back to player exits to /player without opening anything", () => {
    const { transport } = renderScreen();
    fireEvent.click(screen.getByRole("button", { name: "Back to player" }));
    expect(screen.getByTestId("player-route")).toBeInTheDocument();
    expect(transport.sent.filter((r) => r.cmd !== "system.list-commands")).toHaveLength(0);
  });
});
