// Bookmarks dialog (issue #32, `ui-dialog-bookmarks`) — create / jump /
// delete over the session's bookmark list.
//
// FLOW (all through the client transport, wire `media.*-bookmark`):
//   * mount (per-open): `media.get-bookmarks` → the list;
//   * create: `media.add-bookmark` with the ENGINE-REPORTED position
//     (µs on the wire; the seam's seconds → µs here) + optional label;
//   * jump: the row's primary action dispatches `playback.seek` ABSOLUTE
//     through the player state seam (`actions.seek` — the same funnel as
//     the seekbar and the Go-to-Time dialog); the dialog stays open
//     (management surface, VLC parity);
//   * delete: `media.remove-bookmark` per row; the response's updated
//     list is the truth, never a local guess.
//
// HOTKEY: keymaps/default.toml's `bookmarks-dialog` (Ctrl+B, ui_action =
// "bookmarks") through keymap.ts's DIALOG_KEYMAP seed.
//
// SKIN (w26.a18, Figma dialog grammar — design/figma/Open network
// stream.svg / Keyboard Shortcuts.svg): dark scrim (click closes), rx14
// panel + hairline border, 50px #090909 title-bar strip (title 15px
// semibold + ✕), rows label-left / control-right, r6 row hovers,
// tabular timecodes and the list in NATURAL order (naturalSort.ts —
// "1, 2, 10", never lexical). The Add action rides the Button
// primitive's "tinted" dialog-confirm variant (the frames' Save).
//
// COPY NOTE: local module copy — ui/src/i18n/messages.ts is owned by the
// i18n agent (wave ownership map); central keys are the recorded
// follow-up (same note as GotoTimeDialog).

import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Button } from "../../../components/button/Button";
import { IconButton } from "../../../components/button/IconButton";
import { TextField } from "../../../components/form/TextField";
import { Dialog } from "../../../components/surface/Surface";
import { formatSeekTime } from "../../../components/slider";
import { useClient } from "../../../client/ClientProvider";
import type { Bookmark } from "../../../generated/commands";
import { usePlayerControls } from "../context";
import { CloseGlyph } from "./dialogIcons";
import { compareBookmarks } from "./naturalSort";
import "./dialogs.css";

/** The dialog's English copy (see COPY NOTE — central i18n is follow-up). */
export const BOOKMARKS_COPY = {
  title: "Bookmarks",
  subtitle: "Saved positions in the current media.",
  label: "Label (optional)",
  placeholder: "Name this bookmark",
  add: "Add bookmark",
  jump: (time: string) => `Jump to ${time}`,
  deleteBookmark: (time: string) => `Delete bookmark at ${time}`,
  unnamed: "Unnamed bookmark",
  empty: "No bookmarks yet — add the current position.",
  close: "Close",
} as const;

/**
 * The Bookmarks dialog. Mount per-open (fresh list fetch), like
 * GotoTimeDialog: `{open ? <BookmarksDialog open onClose={…} /> : null}`.
 */
export function BookmarksDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const client = useClient();
  const { state, actions } = usePlayerControls();
  const [bookmarks, setBookmarks] = useState<readonly Bookmark[]>([]);
  const [labelValue, setLabelValue] = useState("");
  const [loaded, setLoaded] = useState(false);
  const titleId = useId();
  const nextRequestId = useRef(0);
  const requestId = (): number => {
    nextRequestId.current += 1;
    return nextRequestId.current;
  };

  // Mount = open: fetch the engine's list once. Errors surface as the
  // honest empty list + loaded flag (the row area shows the empty state;
  // typed errors stay on the player's error surface).
  useEffect(() => {
    let cancelled = false;
    void client
      .send({ cmd: "media.get-bookmarks", requestId: requestId() })
      .then((response) => {
        if (cancelled) return;
        const result = response.result;
        if ("completed" in result && result.completed.kind === "bookmarks") {
          const initial = result.completed.bookmarks;
          setBookmarks(initial);
        }
        setLoaded(true);
      })
      .catch(() => {
        if (!cancelled) setLoaded(true);
      });
    return () => {
      cancelled = true;
    };
    // Mount-time fetch only — the list refreshes through the command
    // responses below, never a re-render guess.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The rendered order (w26.a18): natural sort — time ascending, labels
  // digit-aware, creation id last. A copy, never the engine array itself.
  const sortedBookmarks = useMemo(
    () => [...bookmarks].sort(compareBookmarks),
    [bookmarks],
  );

  const onAdd = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    // The engine-reported position is the truth (never a guessed clock);
    // the seam speaks seconds, the wire µs.
    const timeUs = Math.round(state.position * 1000);
    void client
      .send({
        args: {
          time: timeUs,
          label: labelValue.trim().length > 0 ? labelValue.trim() : null,
        },
        cmd: "media.add-bookmark",
        requestId: requestId(),
      })
      .then((response) => {
        // Hoist the narrowed result — narrowing does not cross the
        // setState updater closure boundary.
        const result = response.result;
        if ("completed" in result && result.completed.kind === "bookmark-added") {
          const added = result.completed.bookmark;
          setBookmarks((prev) => [...prev, added]);
        }
        // Errors: the list stays what it was — never a fabricated row.
      })
      .catch(() => {});
    setLabelValue("");
  };

  const onDelete = (bookmark: Bookmark): void => {
    void client
      .send({
        args: { id: bookmark.id },
        cmd: "media.remove-bookmark",
        requestId: requestId(),
      })
      .then((response) => {
        const result = response.result;
        if ("completed" in result && result.completed.kind === "bookmarks") {
          const next = result.completed.bookmarks;
          setBookmarks(next);
        }
      })
      .catch(() => {});
  };

  const onJump = (bookmark: Bookmark): void => {
    // µs wire → seconds for the shared seek funnel.
    actions.seek(bookmark.time / 1000);
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      labelledBy={titleId}
      className="player-dialog__surface"
      closeOnOutsideClick
    >
      <div className="player-dialog player-dialog--bookmarks">
        <div className="player-dialog__header">
          <h2 id={titleId} className="player-dialog__title">
            {BOOKMARKS_COPY.title}
          </h2>
          <IconButton
            className="player-dialog__close"
            label={BOOKMARKS_COPY.close}
            title={BOOKMARKS_COPY.close}
            size="sm"
            onClick={onClose}
          >
            <CloseGlyph size={16} />
          </IconButton>
        </div>

        <div className="player-dialog__body">
          <p className="player-dialog__subtitle">{BOOKMARKS_COPY.subtitle}</p>

          <form className="player-dialog__form" onSubmit={onAdd}>
            <TextField
              label={BOOKMARKS_COPY.label}
              placeholder={BOOKMARKS_COPY.placeholder}
              value={labelValue}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => {
                setLabelValue(event.target.value);
              }}
            />
            <Button type="submit" variant="tinted" className="player-dialog__form-action">
              {BOOKMARKS_COPY.add}
            </Button>
          </form>

          {sortedBookmarks.length === 0 ? (
            <p className="player-dialog__empty">
              {loaded ? BOOKMARKS_COPY.empty : ""}
            </p>
          ) : (
            <ul className="player-dialog__list">
              {sortedBookmarks.map((bookmark) => {
                const time = formatSeekTime(bookmark.time / 1000);
                return (
                  <li key={bookmark.id} className="player-dialog__row">
                    <button
                      type="button"
                      className="player-dialog__row-main"
                      aria-label={BOOKMARKS_COPY.jump(time)}
                      onClick={() => onJump(bookmark)}
                    >
                      <span className="player-dialog__row-time">{time}</span>
                      <span className="player-dialog__row-label">
                        {bookmark.label ?? BOOKMARKS_COPY.unnamed}
                      </span>
                    </button>
                    <IconButton
                      className="player-dialog__row-delete"
                      label={BOOKMARKS_COPY.deleteBookmark(time)}
                      title={BOOKMARKS_COPY.deleteBookmark(time)}
                      size="sm"
                      onClick={() => onDelete(bookmark)}
                    >
                      <CloseGlyph size={12} />
                    </IconButton>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </Dialog>
  );
}
