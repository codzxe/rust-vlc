// Go-to-Time dialog (issue #32, `ui-dialog-goto-time`) — the VLC-parity
// "Jump to time" surface, driven by the shared player state seam.
//
// FLOW: time input (mm:ss / hh:mm:ss / bare seconds, parseTimeInput) →
// `playback.seek` with an ABSOLUTE target through
// PlayerControlsProvider's `actions.seek` (client transport — the same
// funnel the seekbar uses, doc 08 §5.2 truthful-pending). The dialog
// closes on submit; seek truth arrives via `timing.seek-completed` /
// `timing.seek-failed` (the pending contract lives in the seam, not here).
//
// HOTKEY: keymaps/default.toml's `jump-to-time` (Ctrl+T, ui_action =
// "goto-time", when = "has-media") — PlayerScreen matches the
// modifier-combo through keymap.ts's DIALOG_KEYMAP seed + open this
// dialog. The engine side of a goto is just the existing seek; no new
// command exists (nor is one needed).
//
// SKIN (w26.a18, Figma dialog grammar — design/figma/Open network
// stream.svg / Keyboard Shortcuts.svg): dark scrim (click closes), rx14
// panel + hairline border, 50px #090909 title-bar strip (title 15px
// semibold + ✕, fill contrast — no hairline), the Time row label-left /
// control-right with a tabular mono input, and the footer grammar — quiet
// Cancel left, TINTED confirm right (Button primitive variant "tinted":
// #371F06 fill + orange ink, the frames' Save).
// FOCUS (w26.i09, h19 P4 pattern): body before header in the DOM (the
// header paints first via `order: -1`) so the surface trap's initial
// focus lands on the Time input — Enter on a fresh dialog submits the
// seek instead of dismissing the window.
//
// I18N (w26.k09, j09 P2): every user-visible string resolves through
// useDialogsMsg (central catalog → the w26.k09 lane keys; §4.5 seam).

import { useId, useState } from "react";
import type { FormEvent } from "react";
import { Button } from "../../../components/button/Button";
import { IconButton } from "../../../components/button/IconButton";
import { TextField } from "../../../components/form/TextField";
import { Dialog } from "../../../components/surface/Surface";
import { formatSeekTime } from "../../../components/slider";
import { usePlayerControls } from "../context";
import { CloseGlyph } from "./dialogIcons";
import { parseTimeInput } from "./parseTimeInput";
import { useDialogsMsg } from "./useDialogsMsg";
import "./dialogs.css";

/**
 * The Go-to-Time dialog. Mount per-open (fresh state — the input prefills
 * with the engine-reported position at open time, VLC parity): render as
 * `{open ? <GotoTimeDialog open onClose={…} /> : null}`.
 */
export function GotoTimeDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { state, actions } = usePlayerControls();
  const m = useDialogsMsg();
  // Prefill: the engine-reported position, captured at mount (= open).
  const [value, setValue] = useState(() => formatSeekTime(state.position));
  const [error, setError] = useState<string | undefined>(undefined);
  const titleId = useId();
  const formId = useId();

  const onSubmit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const seconds = parseTimeInput(value);
    if (seconds === null) {
      setError(m("player.dialog.goto.invalid"));
      return;
    }
    // Duration cross-check — engine-reported truth only (null = unknown
    // duration, e.g. live streams: no ceiling is invented).
    const duration = state.durationSeconds;
    if (duration !== null && seconds > duration) {
      setError(
        m("player.dialog.goto.beyond-end").replace(
          "{duration}",
          formatSeekTime(duration),
        ),
      );
      return;
    }
    actions.seek(seconds);
    onClose();
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      labelledBy={titleId}
      className="player-dialog__surface"
      closeOnOutsideClick
    >
      <div className="player-dialog player-dialog--goto">
        {/* Body first in the DOM (focus order — see module header); the
         * header paints above it via `order: -1`. */}
        <div className="player-dialog__body">
          <p className="player-dialog__subtitle">
            {m("player.dialog.goto.subtitle")}
          </p>

          <form id={formId} className="player-dialog__form" onSubmit={onSubmit}>
            <TextField
              label={m("player.dialog.goto.label")}
              placeholder={m("player.dialog.goto.placeholder")}
              value={value}
              error={error}
              spellCheck={false}
              autoComplete="off"
              /* The grammar includes ":" (mm:ss) and numeric keypads have
               * none — text keeps the colon reachable (j09 P3). */
              inputMode="text"
              onChange={(event) => {
                setValue(event.target.value);
                if (error !== undefined) setError(undefined);
              }}
            />
          </form>

          {/* Engine-reported context, honest per doc 08 §5: position is the
              seam's engine truth; duration shows "--:--" when unknown. */}
          <p className="player-dialog__hint" data-hint="position">
            {formatSeekTime(state.position)} /{" "}
            {state.durationSeconds === null ? "--:--" : formatSeekTime(state.durationSeconds)}
          </p>
        </div>
        <div className="player-dialog__header">
          <h2 id={titleId} className="player-dialog__title">
            {m("player.dialog.goto.title")}
          </h2>
          {/* Figma dialog grammar: the title bar carries the quiet ✕
              (Escape and scrim click also close — the Dialog owns both). */}
          <IconButton
            className="player-dialog__close"
            label={m("player.dialog.goto.close")}
            title={m("player.dialog.goto.close")}
            size="sm"
            onClick={onClose}
          >
            <CloseGlyph size={16} />
          </IconButton>
        </div>

        {/* Footer grammar (frame: quiet Cancel left, tinted confirm right). */}
        <div className="player-dialog__footer">
          <Button onClick={onClose}>{m("player.dialog.goto.cancel")}</Button>
          <Button type="submit" variant="tinted" form={formId}>
            {m("player.dialog.goto.action")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
