// Card 4.C2 — the control bar: transport cluster, seekbar cluster, volume/
// mute, track menus (doc 08 §5). Every control invokes a real command via the
// client seam, reflects engine-reported state only, and surfaces typed
// errors (§5.4) instead of swallowing them.

import {
  Button,
  Dropdown,
  IconButton,
  Icon,
  Seekbar,
  VolumeSlider,
  formatSeekTime,
} from "../../../components";
import { useMsg } from "../../../i18n/messages";
import type { DropdownOption } from "../../../components";
import {
  selectedTrackId,
  trackDisplayName,
  tracksOfKind,
  usePlayerControls,
} from "../context";
import { mapTransportControls, seekDisableReason } from "./state-mapping";
import "./controls.css";

export interface ControlBarProps {
  /** Host label context (reserved; the bar is engine-state driven). */
  readonly className?: string;
}

export function ControlBar({ className = "" }: ControlBarProps) {
  const { state } = usePlayerControls();
  return (
    <div
      className={["lumen-controls", className].filter(Boolean).join(" ")}
      data-playback-state={state.playbackState}
      data-engine-reported={state.engineReported}
    >
      <TransportCluster />
      <SeekbarCluster />
      <VolumeCluster />
      <TrackMenuCluster />
      <ErrorSurface />
    </div>
  );
}

/** Play/pause toggle + stop (doc 08 §5.1 mapping). */
function TransportCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const mapped = mapTransportControls(state);

  const playLabel =
    mapped.play.glyph === "pause" ? m("player.controls.pause") : m("player.controls.play");
  const label = mapped.play.busy
    ? `${playLabel} — ${m("player.controls.buffering-hint")}`
    : mapped.play.restart
      ? `${playLabel} — ${m("player.controls.restart")}`
      : playLabel;

  return (
    <div
      className="lumen-controls__transport"
      role="group"
      aria-label={m("player.controls.group-transport")}
    >
      <IconButton
        data-control="play-pause"
        data-glyph={mapped.play.glyph}
        data-restart={mapped.play.restart || undefined}
        label={label}
        loading={mapped.play.pending}
        disabled={mapped.play.disabled}
        onClick={actions.togglePlayPause}
      >
        <Icon name={mapped.play.glyph} />
      </IconButton>
      <IconButton
        data-control="stop"
        label={m("player.controls.stop")}
        disabled={mapped.stop.disabled}
        onClick={actions.stop}
      >
        <Icon name="stop" />
      </IconButton>
    </div>
  );
}

/** Seekbar + honest time labels (doc 08 §5.2 accepted-vs-completed). */
function SeekbarCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

  return (
    <div className="lumen-controls__seek">
      <span className="lumen-controls__time" data-time="position">
        {formatSeekTime(state.position)}
      </span>
      <Seekbar
        data-control="seekbar"
        aria-busy={mapped.seek.busy || undefined}
        aria-label={
          reason !== null
            ? `${m("player.controls.seekbar")} (${m(reason)})`
            : m("player.controls.seekbar")
        }
        value={state.position}
        duration={state.durationSeconds ?? 0}
        buffered={state.buffered}
        pending={state.pendingSeek ?? undefined}
        disabled={mapped.seek.disabled}
        onCommit={actions.seek}
      />
      <span className="lumen-controls__time" data-time="duration">
        {state.durationSeconds === null
          ? m("player.controls.duration-unknown")
          : formatSeekTime(state.durationSeconds)}
      </span>
    </div>
  );
}

/** Volume slider + mute toggle (engine-reported, never optimistic). */
function VolumeCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const muted = state.muted === true;
  const volume = state.volume;
  const effectiveMuted = muted || volume === 0;
  const iconName = effectiveMuted
    ? "volume-mute"
    : (volume ?? 0) < 0.5
      ? "volume-low"
      : "volume-high";

  return (
    <div
      className="lumen-controls__volume"
      role="group"
      aria-label={m("player.controls.group-volume")}
    >
      <IconButton
        data-control="mute"
        data-muted={String(muted)}
        label={muted ? m("player.controls.unmute") : m("player.controls.mute")}
        disabled={state.muted === null}
        onClick={() => actions.setMute(!muted)}
      >
        <Icon name={iconName} />
      </IconButton>
      <VolumeSlider
        data-control="volume"
        aria-label={m("player.controls.volume")}
        value={volume ?? 0}
        disabled={volume === null}
        onCommit={actions.setVolume}
      />
    </div>
  );
}

/** Track menus (A/V/S) from the real tracks.list result — real ids/labels. */
function TrackMenuCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();

  const menus: ReadonlyArray<{
    readonly kind: "video" | "audio" | "subtitle";
    readonly labelKey: "player.track.video" | "player.track.audio" | "player.track.subtitle";
  }> = [
    { kind: "video", labelKey: "player.track.video" },
    { kind: "audio", labelKey: "player.track.audio" },
    { kind: "subtitle", labelKey: "player.track.subtitle" },
  ];

  return (
    <div
      className="lumen-controls__tracks"
      role="group"
      aria-label={m("player.controls.group-tracks")}
    >
      {menus.map(({ kind, labelKey }) => {
        const tracks = tracksOfKind(state.tracks, kind);
        const selected = selectedTrackId(state.tracks, state.selections, kind);
        // Disabled while opening (doc §5.5 TrackMenu disabled-while-loading)
        // or when no track list has been reported (honest, never fake).
        // Unreported = no track list and no successful engine query at all:
        // render an honest Unknown, not a fabricated "Disabled".
        const unreported = !state.engineReported && tracks.length === 0;
        const disabled = state.playbackState === "opening" || unreported;
        const options: DropdownOption[] = [
          { value: "auto", labelKey: m("player.track.auto") },
          { value: "none", labelKey: m("player.track.disable") },
          ...tracks.map((track, index) => ({
            value: `id:${track.id}`,
            labelKey: trackDisplayName(track, m("player.track.unknown"), index),
          })),
        ];
        if (unreported) {
          options.push({ value: "unknown", labelKey: m("player.track.unknown") });
        }
        const value = unreported ? "unknown" : selected === null ? "none" : `id:${selected}`;
        return (
          <Dropdown
            key={kind}
            label={m(labelKey)}
            options={options}
            value={value}
            disabled={disabled}
            onChange={(next: string) => {
              if (next === "auto") {
                actions.selectTrack(kind, { kind: "auto" });
              } else if (next === "none") {
                actions.selectTrack(kind, { kind: "none" });
              } else if (next.startsWith("id:")) {
                actions.selectTrack(kind, { kind: "id", id: Number(next.slice(3)) });
              }
            }}
          />
        );
      })}
    </div>
  );
}

/** Typed-error surface (doc 08 §5.4): stage-named, dismissible, honest. */
function ErrorSurface() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  if (state.lastError === null) return null;
  const error = state.lastError;
  return (
    <div className="lumen-controls__error" role="status" data-error-code={error.code}>
      <span className="lumen-controls__error-title">{m("player.error.title")}</span>
      <span className="lumen-controls__error-detail">
        {`${error.stage}: ${error.message} (${error.code})`}
      </span>
      <Button data-control="dismiss-error" size="sm" onClick={actions.dismissError}>
        {m("player.error.dismiss")}
      </Button>
    </div>
  );
}
