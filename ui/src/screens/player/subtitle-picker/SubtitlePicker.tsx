// SubtitlePicker (FX9-A06, wave FX9, plan F3.1) — the GUI subtitle track
// picker: the "add subtitle file" flow + the sidecar rows.
//
// THE WIRE'S TRUTH: the picker half-existed unfed — the canvas submenu
// (canvasTrackSubmenu("subtitle", …), PlayerScreen) listed ZERO tracks
// because no engine lane reported subtitle tracks and no subtitle.add-file
// affordance existed anywhere. FX9-A01 (committed) routes the five
// subtitle verbs through the real CLI host, so track_snapshot now appends
// the consumer's sidecar rows (origin "external-subtitle") — this surface
// renders what the seams feed:
//   * the "add subtitle file" entry — the hidden file-input flow
//     (NewPlaylistDialog's file-input pattern) dispatching the catalog's
//     `subtitle.add-file` { path, encoding: null, select: true } through
//     the player context's ONE dispatch lane;
//   * the sidecar rows — the same engine truth the audio/video submenu
//     rows render (tracksOfKind / selectedTrackId / trackDisplayName over
//     state.tracks — A01's snapshot enrichment feeds them through the
//     snapshot / tracks.list read-back);
//   * the selected sidecar reflected in the radio (state.selections).
//
// INTEGRATION (the tools/equalizer pattern): the presence probe is
// usePlayerControlsOrNull — inside the player composition the picker
// dispatches for real through the same wire the transport controls ride;
// bare mounts (dev harness, jsdom trees without the player providers)
// degrade honestly: the honest unavailable face + the entry disabled with
// the REGISTERED `player.controls.reason.unknown` title — every key this
// module resolves is a registered i18n catalog key (a lost run crashed the
// title on an unregistered one). The TRIGGER SEAM follows
// `lumen:open-media` / `lumen:open-tools`: any surface calls
// openSubtitleFilePicker() (or dispatches the SUBTITLE_PICKER_EVENT window
// event) and the mounted picker opens its file input. PlayerScreen mounts
// SubtitlePickerHost — the zero-chrome listener half (its canvas context
// menu's "Add subtitles" row is the visible entry); the full surface below
// is for hosts that want rows + entry in one place.
//
// HONESTY: the web file input yields the picked file's NAME (no
// host-filesystem path exists in the browser shell — the AddSubtitlesDialog
// note); the dev shell's transport synthesizes the sidecar from it. A typed
// refusal (no session, an unsupported sidecar format) surfaces through
// the provider's lastError (the codebase's error surface), never silently.

import { useEffect, useId, useRef } from "react";
import type { ChangeEvent, ReactNode, RefObject } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import {
  selectedTrackId,
  trackDisplayName,
  tracksOfKind,
  usePlayerControlsOrNull,
} from "../context";

/**
 * The cross-surface CustomEvent name (never rename — the
 * `lumen:open-media` / `lumen:open-tools` pattern): any surface asks for
 * the add-file flow by dispatching it; the mounted picker owns the hidden
 * file input and opens it.
 */
export const SUBTITLE_PICKER_EVENT = "lumen:subtitle-add-file";

/** Open the file flow from any surface (the openToolsDialog pattern). */
export function openSubtitleFilePicker(): void {
  window.dispatchEvent(new CustomEvent(SUBTITLE_PICKER_EVENT));
}

/** The sidecar formats the FX9 lane dispatches (the CLI host's
 * SidecarFormat::from_path table — srt, vtt, ass/ssa, sup, idx), as the
 * file input's accept filter. */
const SIDECAR_ACCEPT = ".srt,.vtt,.ass,.ssa,.sup,.idx";

/** The add-file entry's label key (REGISTERED — the catalog's
 * "Add subtitles"). */
const ADD_FILE_LABEL_KEY: MessageKey = "playlist.subtitles.open";
/** The no-context disabled reason (REGISTERED). */
const REASON_UNKNOWN_KEY: MessageKey = "player.controls.reason.unknown";
/** The no-media disabled reason (REGISTERED). */
const REASON_NO_MEDIA_KEY: MessageKey = "player.controls.reason.no-media";
/** The rows' radiogroup name + the auto/disable row keys (REGISTERED —
 * the canvas submenu's own rows). */
const TRACK_GROUP_KEY: MessageKey = "player.track.subtitle";
const TRACK_AUTO_KEY: MessageKey = "player.track.auto";
const TRACK_DISABLE_KEY: MessageKey = "player.track.disable";
const TRACK_UNKNOWN_KEY: MessageKey = "player.track.unknown";

/**
 * The hidden file input — the one flow both the host and the full surface
 * ride (NewPlaylistDialog's pattern: a programmatic click opens the
 * picker, the change event reads the file, the input is cleared so a
 * re-pick of the same file still fires onChange).
 */
function SubtitleFileInput({
  id,
  inputRef,
  label,
  onChange,
}: {
  id: string;
  inputRef: RefObject<HTMLInputElement | null>;
  label: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
}): ReactNode {
  return (
    <input
      ref={inputRef}
      id={id}
      type="file"
      accept={SIDECAR_ACCEPT}
      aria-label={label}
      /* The picker owns no stylesheet this wave (the FX9 wiring lane) —
       * the player tree's visually-hidden clip utility hosts the input; a
       * dedicated class is the subtitle-picker CSS lane's followup. */
      className="lumen-player__sr-note"
      onChange={onChange}
    />
  );
}

/** The add-file flow's shared half — the hidden input + the
 * `subtitle.add-file` dispatch through the player context (the presence
 * probe: no provider above → the flow degrades to a no-op, never a crash
 * — the EqualizerTool rule). */
function useSubtitleAddFile(): {
  fileInput: ReactNode;
  label: string;
  openPicker: () => void;
} {
  const m = useMsg();
  const controls = usePlayerControlsOrNull();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const fileInputId = useId();
  const label = m(ADD_FILE_LABEL_KEY);

  const openPicker = (): void => {
    fileInputRef.current?.click();
  };

  const onFileChange = (event: ChangeEvent<HTMLInputElement>): void => {
    const file = event.target.files?.[0] ?? null;
    // same file re-picked stays a change event (NewPlaylistDialog rule)
    event.target.value = "";
    // A cancelled dialog fires no change event; an empty pick is a no-op.
    if (file === null) return;
    controls?.actions.addSubtitleFile(file.name);
  };

  const fileInput = (
    <SubtitleFileInput
      id={fileInputId}
      inputRef={fileInputRef}
      label={label}
      onChange={onFileChange}
    />
  );
  return { fileInput, label, openPicker };
}

/** The trigger-seam listener (the ToolsDialogHost pattern) — any
 * surface's openSubtitleFilePicker() reaches the mounted picker. */
function useSubtitlePickerEvent(openPicker: () => void): void {
  useEffect(() => {
    const onOpen = (): void => openPicker();
    window.addEventListener(SUBTITLE_PICKER_EVENT, onOpen);
    return () => window.removeEventListener(SUBTITLE_PICKER_EVENT, onOpen);
  }, [openPicker]);
}

export interface SubtitlePickerProps {
  /** Extra classes on the picker root. */
  className?: string;
}

/**
 * The picker's full surface — the subtitle track rows (radios: automatic,
 * disable, one per engine-reported row, sidecars included) + the add-file
 * entry. Standalone mount (hosts, tests); PlayerScreen wires the flow host
 * instead — its canvas context menu carries the visible rows + entry.
 */
export function SubtitlePicker({ className = "" }: SubtitlePickerProps): ReactNode {
  const m = useMsg();
  const controls = usePlayerControlsOrNull();
  const { fileInput, label, openPicker } = useSubtitleAddFile();
  useSubtitlePickerEvent(openPicker);

  // The rows' engine truth — the same data path the canvas submenu's
  // audio/video rows ride (mirror of canvasTrackSubmenu's helpers).
  const state = controls?.state ?? null;
  const noContext = controls === null;
  const rows = state === null ? [] : tracksOfKind(state.tracks, "subtitle");
  const selected =
    state === null ? null : selectedTrackId(state.tracks, state.selections, "subtitle");
  const unreported = state !== null && !state.engineReported && rows.length === 0;
  const noMedia =
    state !== null &&
    (state.playbackState === "idle" ||
      state.playbackState === "opening" ||
      unreported);
  const disabledReason = noContext
    ? m(REASON_UNKNOWN_KEY)
    : noMedia
      ? m(REASON_NO_MEDIA_KEY)
      : null;

  return (
    <div
      className={["subtitle-picker", className].filter(Boolean).join(" ")}
      data-subtitle-picker
    >
      {noContext ? (
        /* Bare mount (no PlayerControlsProvider above): the honest
         * unavailable face — no rows to render, the entry inert. */
        <p data-subtitle-picker-reason>{m(REASON_UNKNOWN_KEY)}</p>
      ) : noMedia ? (
        <p data-subtitle-picker-reason>{m(REASON_NO_MEDIA_KEY)}</p>
      ) : (
        <div role="radiogroup" aria-label={m(TRACK_GROUP_KEY)}>
          <button
            type="button"
            role="radio"
            aria-checked={false}
            data-track="auto"
            onClick={() => controls?.actions.selectTrack("subtitle", { kind: "auto" })}
          >
            {m(TRACK_AUTO_KEY)}
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={selected === null}
            data-track="none"
            onClick={() => controls?.actions.selectTrack("subtitle", { kind: "none" })}
          >
            {m(TRACK_DISABLE_KEY)}
          </button>
          {rows.map((track, index) => (
            <button
              key={track.id}
              type="button"
              role="radio"
              aria-checked={selected === track.id}
              data-track={track.id}
              data-origin={track.origin}
              onClick={() =>
                controls?.actions.selectTrack("subtitle", { kind: "id", id: track.id })
              }
            >
              {trackDisplayName(track, m(TRACK_UNKNOWN_KEY), index)}
            </button>
          ))}
        </div>
      )}
      <button
        type="button"
        data-subtitle-add-file
        disabled={disabledReason !== null}
        title={disabledReason ?? undefined}
        onClick={openPicker}
      >
        {label}
      </button>
      {fileInput}
    </div>
  );
}

/**
 * SubtitlePickerHost — the LISTENER half (the ToolsDialogHost pattern):
 * PlayerScreen's zero-chrome mount. The canvas context menu's
 * "Add subtitles" row (openSubtitleFilePicker()) is the visible entry;
 * this host owns the hidden file input, the trigger listener and — through
 * the player context — the `subtitle.add-file` dispatch. Full variant
 * only: the miniplayer surface exposes no subtitle entry (the
 * ToolsDialogHost rule).
 */
export function SubtitlePickerHost(): ReactNode {
  const { fileInput, openPicker } = useSubtitleAddFile();
  useSubtitlePickerEvent(openPicker);
  return fileInput;
}
