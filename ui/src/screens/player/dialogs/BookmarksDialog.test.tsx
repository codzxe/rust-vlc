// Bookmarks dialog tests (issue #32, `ui-dialog-bookmarks`).
//
// Pins the management contract at the wire seam: mount fetches
// `media.get-bookmarks`; Add sends `media.add-bookmark` with the
// engine-reported position (seconds → µs) + optional label and appends
// the returned bookmark; Jump dispatches `playback.seek` ABSOLUTE through
// the player state seam; Delete sends `media.remove-bookmark` and adopts
// the response's updated list; empty state is honest.
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../../client/ClientProvider";
import type { CommandTransport } from "../../../client/transport";
import type { CommandRequest, CommandResponse, Event } from "../../../client/transport";
import type { Bookmark } from "../../../generated/commands";
import { PlayerControlsProvider } from "../context";
import { BookmarksDialog, BOOKMARKS_COPY } from "./BookmarksDialog";

/**
 * Transport with a real in-memory bookmark list (mirrors the engine-side
 * BookmarkList semantics: creation-ordered ids from 1, no id reuse).
 */
class BookmarkTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  private bookmarks: Bookmark[] = [];
  private nextId = 1;

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    const respond = (result: CommandResponse["result"]): CommandResponse => ({
      gen: this.sent.length,
      requestId: request.requestId,
      result,
    });
    switch (request.cmd) {
      case "media.get-bookmarks":
        return respond({
          completed: { kind: "bookmarks", bookmarks: [...this.bookmarks] },
        });
      case "media.add-bookmark": {
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

  it("Add sends media.add-bookmark with the engine-reported position in µs + the label", async () => {
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
