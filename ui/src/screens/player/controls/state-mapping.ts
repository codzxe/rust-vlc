// Card 4.C2 — engine-state → control mapping (doc 08 §5.1 PlayButton table,
// the contract; §5.2 SeekBar; §5.3 buffering). NEVER guess: every row below
// cites the doc, and doc-silent cells carry [PROPOSAL].
//
// Doc 08 §5.1 table (columns = engine states; the generated PlaybackState
// spells "loading" as "opening" and adds "stopped", which the doc omits):
//   idle:     disabled ("no media open"), play glyph
//   opening:  pending (session.open accepted) — play glyph only after
//             loading→playing is OBSERVED (never optimistic)
//   playing:  pause glyph; pending while playback.play/pause accepted ≤250ms
//   paused:   play glyph; pending idem
//   buffering: doc table silent for glyphs/pending → [PROPOSAL] disabled +
//             progress hint (aria-busy); volume/track stay live (§5.3)
//   ended:    play glyph + restart affordance
//   error:    play glyph + typed-error surface (§5.4)
//   stopped:  [PROPOSAL] doc omits the column — play glyph + restart, stop
//             disabled (nothing is playing to stop)

import type { PlaybackState } from "../../../client/transport";

export type PlayGlyph = "play" | "pause";

/** Mapped state of the play/pause toggle button (doc 08 §5.1). */
export interface PlayButtonState {
  readonly glyph: PlayGlyph;
  /** Button non-interactive (idle: no media; buffering: [PROPOSAL]). */
  readonly disabled: boolean;
  /** §5.1 pending visual (0.7 + spinner) — accepted request outstanding. */
  readonly pending: boolean;
  /** Ended/stopped → the affordance restarts playback (doc §5.1). */
  readonly restart: boolean;
  /** aria-busy progress hint while buffering (§5.3 task mapping). */
  readonly busy: boolean;
}

export interface StopButtonState {
  readonly disabled: boolean;
}

export interface SeekControlState {
  /** Disabled-with-reason per §5.3 — non-seekable only when REPORTED. */
  readonly disabled: boolean;
  /** aria-busy while buffering (doc §5.2 — knob never hidden). */
  readonly busy: boolean;
}

export interface TransportControlStates {
  readonly playback: PlaybackState;
  readonly play: PlayButtonState;
  readonly stop: StopButtonState;
  readonly seek: SeekControlState;
}

export interface TransportMappingInputs {
  readonly playbackState: PlaybackState;
  /** Engine-reported seekable flag; null = not reported (don't guess). */
  readonly seekable: boolean | null;
  /** Known finite duration (seconds); null = unknown. */
  readonly durationSeconds: number | null;
  /** Outstanding play/pause request (truthful pending, §5.1). */
  readonly pendingToggle: boolean;
}

/**
 * The doc 08 §5.1 mapping (pure — parametrized tests pin every row).
 */
export function mapTransportControls(input: TransportMappingInputs): TransportControlStates {
  const { playbackState: playback, seekable, durationSeconds, pendingToggle } = input;

  // Play/pause toggle — §5.1 table rows.
  let play: PlayButtonState = { glyph: "play", disabled: true, pending: false, restart: false, busy: false };
  switch (playback) {
    case "idle":
      play = { glyph: "play", disabled: true, pending: false, restart: false, busy: false };
      break;
    case "opening":
      // Pending comes from the accepted session.open; the glyph stays "play"
      // but the button is inert until the loading→playing transition is
      // observed (doc: session.open is never optimistic).
      play = { glyph: "play", disabled: true, pending: true, restart: false, busy: false };
      break;
    case "playing":
      play = { glyph: "pause", disabled: false, pending: pendingToggle, restart: false, busy: false };
      break;
    case "paused":
      play = { glyph: "play", disabled: false, pending: pendingToggle, restart: false, busy: false };
      break;
    case "buffering":
      // [PROPOSAL] doc table silent: disabled-with-progress-hint; §5.3 keeps
      // volume/track interactive — only the transport toggle is inert.
      play = { glyph: "pause", disabled: true, pending: false, restart: false, busy: true };
      break;
    case "ended":
      play = { glyph: "play", disabled: false, pending: false, restart: true, busy: false };
      break;
    case "stopped":
      // [PROPOSAL] doc omits the column: restart affordance like ended.
      play = { glyph: "play", disabled: false, pending: false, restart: true, busy: false };
      break;
    case "error":
      // §5.1: "play + error toast" — enabled so the user can retry; the
      // typed error itself renders in the error surface (§5.4).
      play = { glyph: "play", disabled: false, pending: false, restart: false, busy: false };
      break;
  }

  // Stop — enabled only while playback exists to stop
  // (playing/paused/buffering [PROPOSAL]; idle/opening/stopped/ended/error
  // have nothing active).
  const stop: StopButtonState = {
    disabled: !(playback === "playing" || playback === "paused" || playback === "buffering"),
  };

  // Seek — §5.2/§5.3: disabled only when there is nothing to seek in, or the
  // engine REPORTED non-seekable media (never guesswork).
  const seek: SeekControlState = {
    disabled:
      durationSeconds === null ||
      durationSeconds <= 0 ||
      seekable === false ||
      playback === "idle" ||
      playback === "opening" ||
      playback === "stopped" ||
      playback === "error",
    busy: playback === "buffering",
  };

  return { playback: playback as PlaybackState, play, stop, seek };
}

/** i18n key for the disabled-with-reason hint (§5.3 honest reasons). */
export type SeekDisableReason =
  | "player.controls.reason.no-media"
  | "player.controls.reason.not-seekable"
  | "player.controls.reason.unknown";

export function seekDisableReason(input: TransportMappingInputs): SeekDisableReason | null {
  const mapped = mapTransportControls(input);
  if (!mapped.seek.disabled) return null;
  if (input.seekable === false) return "player.controls.reason.not-seekable";
  if (
    input.playbackState === "idle" ||
    input.durationSeconds === null ||
    input.durationSeconds <= 0
  ) {
    return "player.controls.reason.no-media";
  }
  return "player.controls.reason.unknown";
}

/**
 * Keymap seam: bindings whose `command` names a no-argument command owned by
 * the control bar dispatch here (doc 05 §8.1 — recipes/commands come from
 * keymap DATA). The seed keymap has no play/pause binding yet — this stays
 * data-driven so the loaded keymap lights it up without code changes
 * [PROPOSAL — recorded in ui/NEXT-STEPS.md].
 */
export type PlayerControlCommand =
  | "playback.play"
  | "playback.pause"
  | "playback.toggle-play-pause"
  | "playback.stop";

export function controlCommandOf(
  binding: Readonly<{ command?: string }>,
): PlayerControlCommand | null {
  switch (binding.command) {
    case "playback.play":
    case "playback.pause":
    case "playback.toggle-play-pause":
    case "playback.stop":
      return binding.command;
    default:
      return null;
  }
}
