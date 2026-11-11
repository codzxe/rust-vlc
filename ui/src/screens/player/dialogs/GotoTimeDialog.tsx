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
// stream.svg / Keyboard Shortcuts.svg): dark scrim (click closes), r12
// panel + hairline border, title-bar strip (title 15px semibold + ✕),
// the Time row label-left / control-right with a tabular mono input,
// and the footer grammar — quiet action left, primary action right.
//
// COPY NOTE: local module copy — ui/src/i18n/messages.ts is owned by the
// i18n agent (wave ownership map), so central keys are the recorded
// follow-up for this dialog's strings; the vocabulary mirrors VLC's
// "Go to time" phrasing meanwhile.

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
import "./dialogs.css";

/** The dialog's English copy (see COPY NOTE — central i18n is follow-up). */
export const GOTO_COPY = {
  title: "Go to time",
  subtitle: "Jump to a specific time in the current media.",
  label: "Time",
  placeholder: "mm:ss or seconds (1:30, 90)",
  invalid: "Enter a time as seconds (90) or mm:ss (1:30)",
  beyondEnd: (duration: string) => `That time is past the end (${duration})`,
  action: "Go",
  cancel: "Cancel",
  close: "Close",
} as const;

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
  // Prefill: the engine-reported position, captured at mount (= open).
  const [value, setValue] = useState(() => formatSeekTime(state.position));
  const [error, setError] = useState<string | undefined>(undefined);
  const titleId = useId();
  const formId = useId();

  const onSubmit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const seconds = parseTimeInput(value);
    if (seconds === null) {
      setError(GOTO_COPY.invalid);
      return;
    }
    // Duration cross-check — engine-reported truth only (null = unknown
    // duration, e.g. live streams: no ceiling is invented).
    const duration = state.durationSeconds;
    if (duration !== null && seconds > duration) {
      setError(GOTO_COPY.beyondEnd(formatSeekTime(duration)));
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
        <div className="player-dialog__header">
          <h2 id={titleId} className="player-dialog__title">
            {GOTO_COPY.title}
          </h2>
          {/* Figma dialog grammar: the title bar carries the quiet ✕
              (Escape and scrim click also close — the Dialog owns both). */}
          <IconButton
            className="player-dialog__close"
            label={GOTO_COPY.close}
            title={GOTO_COPY.close}
            size="sm"
            onClick={onClose}
          >
            <CloseGlyph size={16} />
          </IconButton>
        </div>

        <div className="player-dialog__body">
          <p className="player-dialog__subtitle">{GOTO_COPY.subtitle}</p>

          <form id={formId} className="player-dialog__form" onSubmit={onSubmit}>
            <TextField
              label={GOTO_COPY.label}
              placeholder={GOTO_COPY.placeholder}
              value={value}
              error={error}
              spellCheck={false}
              autoComplete="off"
              inputMode="numeric"
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

        {/* Footer grammar (frame: quiet action left, primary action right). */}
        <div className="player-dialog__footer">
          <Button onClick={onClose}>{GOTO_COPY.cancel}</Button>
          <Button type="submit" variant="primary" form={formId}>
            {GOTO_COPY.action}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
