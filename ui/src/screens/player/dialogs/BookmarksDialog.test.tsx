// Bookmarks dialog tests (issue #32, `ui-dialog-bookmarks`).
//
// Pins the management contract at the wire seam: mount fetches
// `media.get-bookmarks`; Add sends `media.add-bookmark` with the
// engine-reported position (seam seconds → wire µs — the schema's
// $defs/Timestamp unit, pinned at a NONZERO position since w26.k09: the
// j09 review's zero-only adds were unit-agnostic and hid a 1000× ms/µs
// bug) + optional label and appends the returned bookmark; Jump dispatches
// `playback.seek` ABSOLUTE through the player state seam (wire µs → seam
// seconds); Delete sends `media.remove-bookmark` and adopts the response's
// updated list; empty state is honest; load/add/delete failures surface
// inline and never destroy the user's typed label (j09 P2).
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandTransport } from "../../../client/transport";
import type { CommandRequest, CommandResponse, Event } from "../../../client/transport";
import type { Bookmark } from "../../../generated/commands";
import { PlayerControlsProvider } from "../context";
import { BookmarksDialog } from "./BookmarksDialog";
import { BOOKMARKS_COPY } from "./useDialogsMsg";

/**
 * Transport with a real in-memory bookmark list (mirrors the engine-side
 * BookmarkList semantics: creation-ordered ids from 1, no id reuse). The
 * optional seed lets tests pin the RENDERED order against an engine list
 * that arrives out of order; the failure flags open the j09-P2 error
 * paths (typed error responses + transport rejection).
 */
class BookmarkTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  /** When set, `media.get-bookmarks` answers a typed error. */
  failLoad = false;
  /** When set, `media.add-bookmark` answers a typed error. */
  failAdds = false;
  /** When set, `media.remove-bookmark` rejects (transport down). */
  rejectRemoves = false;
  private readonly listeners = new Set<(event: Event) => void>();
  private bookmarks: Bookmark[];
  private nextId: number;

  constructor(seed: Bookmark[] = []) {
    this.bookmarks = [...seed];
    this.nextId = seed.reduce((max, b) => Math.max(max, b.id), 0) + 1;
  }

  /** Push an engine event into every subscriber (seam position truth). */
  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }

  async send(request: CommandRequest): Promise<CommandResponse> {
    if (request.cmd === "media.remove-bookmark" && this.rejectRemoves) {
      throw new Error("command transport unavailable");
    }
    this.sent.push(request);
    const respond = (result: CommandResponse["result"]): CommandResponse => ({
      gen: this.sent.length,
      requestId: request.requestId,
      result,
    });
    switch (request.cmd) {
      case "media.get-bookmarks":
        if (this.failLoad) {
          return respond({
            error: {
              code: "not-ready",
              details: {},
              message: "engine not ready",
              stage: "control",
            },
          });
        }
        return respond({
          completed: { kind: "bookmarks", bookmarks: [...this.bookmarks] },
        });
      case "media.add-bookmark": {
        if (this.failAdds) {
          return respond({
            error: {
              code: "not-ready",
              details: {},
              message: "engine not ready",
              stage: "control",
            },
          });
        }
        const bookmark: Bookmark = {
          id: this.nextId,
          time: request.args.time,
          label: request.args.label ?? null,
        };
        this.nextId += 1;
        this.bookmarks.push(bookmark);
        return respond({ completed: { kind: "bookmark-added", bookmark } });
      }
      case "media.remove-bookmark": {
        const before = this.bookmarks.length;
        this.bookmarks = this.bookmarks.filter((b) => b.id !== request.args.id);
        if (this.bookmarks.length === before) {
          return respond({
            error: {
              code: "not-found",
              details: { id: String(request.args.id) },
              message: "no such bookmark",
              stage: "input",
            },
          });
        }
        return respond({
          completed: { kind: "bookmarks", bookmarks: [...this.bookmarks] },
        });
      }
      default:
        return respond({ completed: { kind: "unit" } });
    }
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}

function renderDialog(transport: CommandTransport) {
  render(
    <ClientProvider transport={transport}>
      <PlayerControlsProvider transport={transport}>
        <BookmarksDialog open onClose={vi.fn()} />
      </PlayerControlsProvider>
    </ClientProvider>,
  );
}

describe("BookmarksDialog — create / jump / delete", () => {
  it("initial focus lands on the label input, never the header ✕ (h19 P4)", () => {
    // w26.i09: body renders before the header in the DOM (the header
    // paints first via `order: -1`), so the surface trap's first-focusable
    // lands on the label field, not the close chrome.
    renderDialog(new BookmarkTransport());
    expect(screen.getByLabelText(BOOKMARKS_COPY.label)).toHaveFocus();
    expect(
      screen.getByRole("button", { name: BOOKMARKS_COPY.close }),
    ).not.toHaveFocus();
  });

  it("mount fetches the engine's list (media.get-bookmarks) and renders rows", async () => {
    const transport = new BookmarkTransport();
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    // Empty truth first — the honest empty state, not a spinner guess.
    expect(screen.getByText(BOOKMARKS_COPY.empty)).toBeInTheDocument();

    // Add two bookmarks; rows render with time + label.
    fireEvent.change(screen.getByLabelText(BOOKMARKS_COPY.label), {
      target: { value: "Intro" },
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      expect(screen.getByText("Intro")).toBeInTheDocument();
    });
    fireEvent.change(screen.getByLabelText(BOOKMARKS_COPY.label), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      expect(screen.getByText(BOOKMARKS_COPY.unnamed)).toBeInTheDocument();
    });
  });

  it("Add at position 0 sends media.add-bookmark with time 0 µs + the label", async () => {
    const transport = new BookmarkTransport();
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    fireEvent.change(screen.getByLabelText(BOOKMARKS_COPY.label), {
      target: { value: "Chorus" },
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      const add = transport.sent.find((r) => r.cmd === "media.add-bookmark");
      expect(add?.args).toEqual({ time: 0, label: "Chorus" });
    });
  });

  it("wire unit is µs per the schema — 90 s adds as 90 000 000, renders 1:30, jumps back to 90 s (j09 P1 regression)", async () => {
    // The j09 P1: the dialog computed ms (position × 1000) against a µs
    // schema — latent only because every prior add ran at position 0
    // (unit-agnostic). This pin runs at a NONZERO position and closes the
    // loop end-to-end: seam seconds → wire µs → engine echo → rendered
    // timecode → jump → seam seconds.
    const transport = new BookmarkTransport();
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    // Seam truth: the engine reports position 90 (seconds per the seam
    // contract). The time-tick carries the µs twin too, so the pin holds
    // whichever side of the k17 family-wide seam fix converts it.
    transport.emit({
      gen: 1,
      seq: 1,
      kind: "timing.time-tick",
      position: 90,
      time: 90_000_000,
    });
    fireEvent.change(screen.getByLabelText(BOOKMARKS_COPY.label), {
      target: { value: "Chorus" },
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      const add = transport.sent.find((r) => r.cmd === "media.add-bookmark");
      // 90 s → 90 000 000 µs on the wire ($defs/Timestamp), NOT 90 000.
      expect(add?.args).toEqual({ time: 90_000_000, label: "Chorus" });
    });
    // The engine-echoed bookmark (time = the wire µs we sent) renders as
    // the SAME seconds — the row reads 1:30, never 25:00:00.
    const jump = await screen.findByRole("button", {
      name: BOOKMARKS_COPY.jump("1:30", "Chorus"),
    });
    fireEvent.click(jump);
    await waitFor(() => {
      const seek = transport.sent.find((r) => r.cmd === "playback.seek");
      // Wire µs → seam seconds for the shared seek funnel: 90, not 90 000.
      expect(seek?.args).toEqual({ target: { kind: "absolute", time: 90 } });
    });
  });

  it("Jump dispatches playback.seek ABSOLUTE seconds through the player seam (dialog stays open)", async () => {
    const transport = new BookmarkTransport();
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    // Create one at the current position, then jump.
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Jump to 0:00/ })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /Jump to 0:00/ }));
    await waitFor(() => {
      const seek = transport.sent.find((r) => r.cmd === "playback.seek");
      expect(seek?.args).toEqual({ target: { kind: "absolute", time: 0 } });
    });
    // Management surface: the dialog does NOT close on jump (VLC parity).
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("Delete sends media.remove-bookmark and adopts the response's updated list", async () => {
    const transport = new BookmarkTransport();
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      expect(screen.getByText(BOOKMARKS_COPY.unnamed)).toBeInTheDocument();
    });
    fireEvent.click(
      screen.getByRole("button", { name: BOOKMARKS_COPY.deleteBookmark("0:00") }),
    );
    await waitFor(() => {
      const remove = transport.sent.find((r) => r.cmd === "media.remove-bookmark");
      expect(remove?.args).toEqual({ id: 1 });
    });
    await waitFor(() => {
      expect(screen.getByText(BOOKMARKS_COPY.empty)).toBeInTheDocument();
    });
  });

  it("rows render in natural order — time ascending, labels digit-aware (never lexical)", async () => {
    // Engine truth arrives scrambled (µs per the schema): 5:00 first, two
    // bookmarks at 2:00 whose labels would sort "Part 10" before "Part 2"
    // lexically.
    const transport = new BookmarkTransport([
      { id: 3, time: 300_000_000, label: "Part 10" },
      { id: 1, time: 120_000_000, label: "Part 10" },
      { id: 2, time: 120_000_000, label: "Part 2" },
      { id: 4, time: 10_000_000, label: null },
    ]);
    renderDialog(transport);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Jump to 0:10/ })).toBeInTheDocument();
    });
    const rows = screen
      .getAllByRole("button", { name: /Jump to/ })
      .map((button) => button.getAttribute("aria-label"));
    // Accessible names carry time AND label (j09 P3: same-time rows stay
    // distinguishable to screen readers).
    expect(rows).toEqual([
      "Jump to 0:10 — Unnamed bookmark",
      "Jump to 2:00 — Part 2",
      "Jump to 2:00 — Part 10",
      "Jump to 5:00 — Part 10",
    ]);
    // The 2:00 tie breaks naturally: "Part 2" before "Part 10".
    const partRows = screen.getAllByText(/Part/);
    expect(partRows.map((node) => node.textContent)).toEqual(["Part 2", "Part 10", "Part 10"]);
  });

  it("a failed add surfaces the inline error and KEEPS the user's typed label (j09 P2)", async () => {
    const transport = new BookmarkTransport();
    transport.failAdds = true;
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    fireEvent.change(screen.getByLabelText(BOOKMARKS_COPY.label), {
      target: { value: "Keeper" },
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    // Honest failure: the error line announces itself…
    expect(await screen.findByRole("alert")).toHaveTextContent(BOOKMARKS_COPY.addFailed);
    // …the typed label SURVIVES for a retry (never silently cleared)…
    expect(screen.getByLabelText(BOOKMARKS_COPY.label)).toHaveValue("Keeper");
    // …and no fabricated row joined the list.
    expect(screen.queryByText("Keeper")).not.toBeInTheDocument();
  });

  it("a failed load surfaces the error instead of the misleading empty copy (j09 P2)", async () => {
    const transport = new BookmarkTransport();
    transport.failLoad = true;
    renderDialog(transport);
    expect(await screen.findByRole("alert")).toHaveTextContent(BOOKMARKS_COPY.loadFailed);
    // "No bookmarks yet" would claim a truth about a list we never saw.
    expect(screen.queryByText(BOOKMARKS_COPY.empty)).not.toBeInTheDocument();
  });

  it("a failed delete (transport down) surfaces the inline error and keeps the row (j09 P2)", async () => {
    const transport = new BookmarkTransport();
    renderDialog(transport);
    await waitFor(() => {
      expect(transport.sent.filter((r) => r.cmd === "media.get-bookmarks")).toHaveLength(1);
    });
    fireEvent.click(screen.getByRole("button", { name: BOOKMARKS_COPY.add }));
    await waitFor(() => {
      expect(screen.getByText(BOOKMARKS_COPY.unnamed)).toBeInTheDocument();
    });
    transport.rejectRemoves = true;
    fireEvent.click(
      screen.getByRole("button", { name: BOOKMARKS_COPY.deleteBookmark("0:00") }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(BOOKMARKS_COPY.deleteFailed);
    // The list is never guessed on failure — the row stays.
    expect(screen.getByText(BOOKMARKS_COPY.unnamed)).toBeInTheDocument();
  });

  it("Escape closes (Dialog focus trap owns the key)", () => {
    const transport = new BookmarkTransport();
    const onClose = vi.fn();
    render(
      <ClientProvider transport={transport}>
        <PlayerControlsProvider transport={transport}>
          <BookmarksDialog open onClose={onClose} />
        </PlayerControlsProvider>
      </ClientProvider>,
    );
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledWith("escape");
  });
});
