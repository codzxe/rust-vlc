// Bookmarks dialog (issue #32, `ui-dialog-bookmarks`) — create / jump /
// delete over the session's bookmark list.
//
// FLOW (all through the client transport, wire `media.*-bookmark`):
//   * mount (per-open): `media.get-bookmarks` → the list;
//   * create: `media.add-bookmark` with the ENGINE-REPORTED position
//     (µs on the wire — schema $defs/Timestamp "Microseconds since media
//     start"; the seam's seconds → µs here, ×1e6) + optional label;
//   * jump: the row's primary action dispatches `playback.seek` ABSOLUTE
//     through the player state seam (`actions.seek` — the same funnel as
//     the seekbar and the Go-to-Time dialog; wire µs → seam seconds,
//     ÷1e6); the dialog stays open (management surface, VLC parity);
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
// FOCUS (w26.i09, h19 P4 pattern): body before header in the DOM (the
// header paints first via `order: -1`) so the surface trap's initial
// focus lands on the label input, not the ✕.
//
// ERRORS (w26.k09, j09 P2): these direct client sends bypass the
// provider's dispatch, so the player's lastError surface never sees them
// — load/add/delete failures surface HERE as an honest inline error line
// (role="alert"), and a failed add never destroys the user's typed label
// (it clears on success only).
//
// I18N (w26.k09, j09 P2): every user-visible string resolves through
// useDialogsMsg (central catalog → the w26.k09 lane keys; §4.5 seam).

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
import { useDialogsMsg } from "./useDialogsMsg";
import "./dialogs.css";

/**
 * Wire µs per seam second — the schema's unit contract
 * (schemas/commands.schema.json $defs/Timestamp: "Microseconds since media
 * start"; `media.add-bookmark` arg `time` and `Bookmark.time` both carry
 * it). The player state seam speaks SECONDS (context.tsx PlayerState);
 * every crossing lives next to this constant (j09 P1).
 */
const US_PER_SECOND = 1_000_000;

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
  const m = useDialogsMsg();
  const [bookmarks, setBookmarks] = useState<readonly Bookmark[]>([]);
  const [labelValue, setLabelValue] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | undefined>(undefined);
  const titleId = useId();
  const nextRequestId = useRef(0);
  const requestId = (): number => {
    nextRequestId.current += 1;
    return nextRequestId.current;
  };

  // Mount = open: fetch the engine's list once. A failed load surfaces as
  // the honest inline error (the empty copy would claim "no bookmarks"
  // about a list we never saw — the error line replaces it).
  useEffect(() => {
    let cancelled = false;
    void client
      .send({ cmd: "media.get-bookmarks", requestId: requestId() })
      .then((response) => {
        if (cancelled) return;
        const result = response.result;
        if ("error" in result) {
          setError(m("player.dialog.bookmarks.load-failed"));
        } else if (
          "completed" in result &&
          result.completed.kind === "bookmarks"
        ) {
          setBookmarks(result.completed.bookmarks);
        }
        setLoaded(true);
      })
      .catch(() => {
        if (cancelled) return;
        setError(m("player.dialog.bookmarks.load-failed"));
        setLoaded(true);
      });
    return () => {
      cancelled = true;
    };
    // Mount-time fetch only — the list refreshes through the command
    // responses below, never a re-render guess. `m` is the mount render's
    // resolver; the catalogs it reads are module-level constants.
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
    // the seam speaks seconds, the wire µs (schema $defs/Timestamp).
    const timeUs = Math.round(state.position * US_PER_SECOND);
    setError(undefined);
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
        if ("error" in result) {
          // Honest failure: the typed error surfaces inline and the
          // user's label survives for a retry — never a silent loss.
          setError(m("player.dialog.bookmarks.add-failed"));
          return;
        }
        if ("completed" in result && result.completed.kind === "bookmark-added") {
          const added = result.completed.bookmark;
          setBookmarks((prev) => [...prev, added]);
          // Clear the label ONLY on success (j09 P2: a failed add must
          // not destroy what the user typed).
          setLabelValue("");
        }
        // Other outcomes ("accepted"): no fabricated row — the pending
        // contract lives in the seam, not here.
      })
      .catch(() => {
        setError(m("player.dialog.bookmarks.add-failed"));
      });
  };

  const onDelete = (bookmark: Bookmark): void => {
    setError(undefined);
    void client
      .send({
        args: { id: bookmark.id },
        cmd: "media.remove-bookmark",
        requestId: requestId(),
      })
      .then((response) => {
        const result = response.result;
        if ("error" in result) {
          setError(m("player.dialog.bookmarks.delete-failed"));
          return;
        }
        if ("completed" in result && result.completed.kind === "bookmarks") {
          const next = result.completed.bookmarks;
          setBookmarks(next);
        }
      })
      .catch(() => {
        // Transport failure — the row stays (the list is never guessed).
        setError(m("player.dialog.bookmarks.delete-failed"));
      });
  };

  const onJump = (bookmark: Bookmark): void => {
    // Wire µs → seam seconds for the shared seek funnel (schema contract).
    actions.seek(bookmark.time / US_PER_SECOND);
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
        {/* Body first in the DOM (focus order — see module header); the
         * header paints above it via `order: -1`. */}
        <div className="player-dialog__body">
          <p className="player-dialog__subtitle">
            {m("player.dialog.bookmarks.subtitle")}
          </p>

          <form className="player-dialog__form" onSubmit={onAdd}>
            <TextField
              label={m("player.dialog.bookmarks.label")}
              placeholder={m("player.dialog.bookmarks.placeholder")}
              value={labelValue}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => {
                setLabelValue(event.target.value);
              }}
            />
            <Button type="submit" variant="tinted" className="player-dialog__form-action">
              {m("player.dialog.bookmarks.add")}
            </Button>
          </form>

          {error !== undefined ? (
            <p className="player-dialog__error" role="alert">
              {error}
            </p>
          ) : null}

          {sortedBookmarks.length === 0 ? (
            <p className="player-dialog__empty">
              {/* The empty copy asserts "no bookmarks yet" — suppressed
               * while an error line explains what actually happened. */}
              {loaded && error === undefined ? m("player.dialog.bookmarks.empty") : ""}
            </p>
          ) : (
            <ul className="player-dialog__list">
              {sortedBookmarks.map((bookmark) => {
                const time = formatSeekTime(bookmark.time / US_PER_SECOND);
                const rowLabel =
                  bookmark.label ?? m("player.dialog.bookmarks.unnamed");
                // Accessible name carries time AND label (j09 P3): two
                // bookmarks at the same timecode stay distinguishable.
                const jumpLabel = m("player.dialog.bookmarks.jump")
                  .replace("{time}", time)
                  .replace("{label}", rowLabel);
                return (
                  <li key={bookmark.id} className="player-dialog__row">
                    <button
                      type="button"
                      className="player-dialog__row-main"
                      aria-label={jumpLabel}
                      onClick={() => onJump(bookmark)}
                    >
                      <span className="player-dialog__row-time">{time}</span>
                      <span className="player-dialog__row-label">{rowLabel}</span>
                    </button>
                    <IconButton
                      className="player-dialog__row-delete"
                      label={m("player.dialog.bookmarks.delete").replace(
                        "{time}",
                        time,
                      )}
                      title={m("player.dialog.bookmarks.delete").replace(
                        "{time}",
                        time,
                      )}
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
        <div className="player-dialog__header">
          <h2 id={titleId} className="player-dialog__title">
            {m("player.dialog.bookmarks.title")}
          </h2>
          <IconButton
            className="player-dialog__close"
            label={m("player.dialog.bookmarks.close")}
            title={m("player.dialog.bookmarks.close")}
            size="sm"
            onClick={onClose}
          >
            <CloseGlyph size={16} />
          </IconButton>
        </div>
      </div>
    </Dialog>
  );
}
