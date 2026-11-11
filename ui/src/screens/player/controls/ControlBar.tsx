// w26.a14 — the bottom control bar, rebuilt to the W26 Figma frames
// (design/figma/Player.svg "Controls" group, the Controls.svg variant sheet,
// and the component frames play.svg / "Current Time.svg" / "Duration.svg" /
// volume.svg / Button.png; W25 figma-fidelity-spec §2 "Player" family).
// THREE groups across the ~44px chrome strip (player.css owns the strip):
//
//   LEFT    stop · ORANGE PLAY PILL (dark on-accent glyph — the hero
//           control; play.svg 54×28 r6 accent pill, the frame's muted
//           type=pause pill for the pause glyph) · next (a real
//           `playlist.next` through the playlist context PlayerScreen
//           mounts around this bar)
//   CENTER  current timecode (tabular, 12px tertiary) · slim seekbar (the
//           w26.a04 slider API: 4px pill track, accent fill, honest
//           pending/buffer) · remaining timecode
//   RIGHT   total duration · volume icon + mini level slider (the frame's
//           green→amber→red signal gradient revealed to the level — the
//           "level pill" the spec calls out) · video/audio/subtitle track
//           menus (icon faces, honest engine-reported options) ·
//           fullscreen (Fullscreen API on the player host) · cast
//           (honestly not-available — no surface yet) · gear (Settings
//           route) · app menu (real navigation + the shell open-media seam)
//
// Behavior contract UNCHANGED (state-mapping.ts is the doc 08 §5 mapping —
// play/pause/stop/seek/volume/mute/track wiring below is a restyle of the
// same dispatch, never a semantic change). New controls are additive seams
// on the same honest pattern: disabled-with-reason, never fabricated state.

import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Button,
  Dropdown,
  IconButton,
  Icon,
  Seekbar,
  VolumeSlider,
  formatSeekTime,
} from "../../../components";
import { MenuPopup } from "../../../components/menu/MenuPopup";
import type { DropdownOption, IconName, MenuEntry } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import {
  selectedTrackId,
  trackDisplayName,
  tracksOfKind,
  usePlayerControls,
} from "../context";
import { usePlaylist } from "../../playlist";
import { mapTransportControls, seekDisableReason } from "./state-mapping";
import "./controls.css";

/**
 * w26.a14 local copy — genuinely new keys (CONTRACT §4.5), registered for
 * the Lead in ui/src/i18n/keys/w26.a14.ts. Until the post-wave merge these
 * read from this module copy (the established dialogs COPY-NOTE pattern,
 * w26.a18 — TOOLS_DIALOG_COPY). Every other label below flows through
 * useMsg() + existing MessageKeys.
 */
const CONTROLS_COPY = {
  /** Player.svg Frame 10 glyph — next in the queue. */
  next: "Next",
  /** Fullscreen toggle (entering). */
  fullscreen: "Enter fullscreen",
  /** Fullscreen toggle (exiting). */
  exitFullscreen: "Exit fullscreen",
  /** Remote playback; no surface exists yet (honest disable). */
  cast: "Cast",
  /** aria name of the output cluster (fullscreen/cast/gear/menu). */
  groupOutput: "Player controls",
} as const;

/** Track-menu icon faces (Player.svg right cluster: audio bars + cc card;
 * the video kind has no bar glyph in the frames — `pip`'s screen-with-video
 * geometry is the closest honest glyph from the shared Icon.svg catalog). */
const TRACK_ICONS: Record<"video" | "audio" | "subtitle", IconName> = {
  video: "pip",
  audio: "audio",
  subtitle: "subtitles",
};

/** Live document fullscreen truth (never cached — doc 08 §4 honest state). */
const fullscreenActive = (): boolean => (document.fullscreenElement ?? null) !== null;

/** Host capability (jsdom and embedded hosts expose no Fullscreen API). */
const fullscreenSupported = (): boolean =>
  typeof document !== "undefined" &&
  typeof document.documentElement.requestFullscreen === "function" &&
  typeof document.exitFullscreen === "function";

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
      <SeekCluster />
      <RightCluster />
      <ErrorSurface />
    </div>
  );
}

/** stop · play pill · next (doc 08 §5.1 mapping; Player.svg Frame 8→play→10). */
function TransportCluster() {
  const { state, actions } = usePlayerControls();
  const playlist = usePlaylist();
  const m = useMsg();
  const mapped = mapTransportControls(state);

  const playLabel =
    mapped.play.glyph === "pause" ? m("player.controls.pause") : m("player.controls.play");
  const label = mapped.play.busy
    ? `${playLabel} — ${m("player.controls.buffering-hint")}`
    : mapped.play.restart
      ? `${playLabel} — ${m("player.controls.restart")}`
      : playLabel;

  // Next — a real playlist.next through the playlist context (the provider
  // PlayerScreen mounts around this bar). Honestly disabled with a reason
  // while the queue is empty or the engine exposes no playlist surface.
  const nextDisabled = playlist.notExposed || playlist.entries.length === 0;
  const nextLabel = nextDisabled
    ? `${CONTROLS_COPY.next} — ${m(
        playlist.notExposed
          ? "player.controls.reason.unknown"
          : "player.controls.reason.no-media",
      )}`
    : CONTROLS_COPY.next;

  return (
    <div
      className="lumen-controls__transport"
      role="group"
      aria-label={m("player.controls.group-transport")}
    >
      <IconButton
        data-control="stop"
        label={m("player.controls.stop")}
        disabled={mapped.stop.disabled}
        onClick={actions.stop}
      >
        <Icon name="stop" />
      </IconButton>
      <IconButton
        className="lumen-controls__play"
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
        data-control="next"
        label={nextLabel}
        disabled={nextDisabled}
        onClick={() => playlist.next()}
      >
        <Icon name="next" />
      </IconButton>
    </div>
  );
}

/** current timecode · slim seekbar · remaining (doc 08 §5.2/§5.3 honest). */
function SeekCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

  // Remaining is honest arithmetic over engine truth only: duration known →
  // duration − position; unknown → the same "--:--" unknown as the total.
  const remaining =
    state.durationSeconds === null || state.durationSeconds <= 0
      ? m("player.controls.duration-unknown")
      : `−${formatSeekTime(state.durationSeconds - state.position)}`;

  return (
    <div className="lumen-controls__seek">
      <span className="lumen-controls__time" data-time="position">
        {formatSeekTime(state.position)}
      </span>
      <Seekbar
        data-control="seekbar"
        /* A duration-less seekbar is an unplayed track — data-empty lets the
         * slider CSS hide the phantom zero-position thumb (calm track). */
        data-empty={
          state.durationSeconds === null || state.durationSeconds <= 0 ? "" : undefined
        }
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
      <span className="lumen-controls__time" data-time="remaining">
        {remaining}
      </span>
    </div>
  );
}

/** The right group: total duration · volume · track menus · output controls. */
function RightCluster() {
  const { state } = usePlayerControls();
  const m = useMsg();
  return (
    <div className="lumen-controls__right">
      <span className="lumen-controls__time" data-time="duration">
        {state.durationSeconds === null
          ? m("player.controls.duration-unknown")
          : formatSeekTime(state.durationSeconds)}
      </span>
      <VolumeCluster />
      <TrackCluster />
      <OutputCluster />
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
        className="lumen-controls__volume-slider"
      />
    </div>
  );
}

/** Track menus (A/V/S) from the real tracks.list result — real ids/labels. */
function TrackCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();

  // v2 brief §4.3 — the honest no-media trigger. With no media open (or
  // while the engine is still opening / has reported nothing) the trigger
  // shows the track kind plus an em-dash (visually its icon face — the
  // value text is available to assistive tech) and carries the no-media
  // reason in its accessible name. The honest menu options (Automatic /
  // Disabled-off / real track ids) exist only once media is open — tracks
  // are NEVER fabricated.
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
        // No honest track data to show: no media open, still opening, or the
        // engine has not reported any track list at all (doc §5.5
        // disabled-while-loading — never a fabricated list).
        const unreported = !state.engineReported && tracks.length === 0;
        const noMedia =
          state.playbackState === "idle" || state.playbackState === "opening" || unreported;
        const options: DropdownOption[] = noMedia
          ? [{ value: "unavailable", labelKey: `${m(labelKey)} —` }]
          : [
              { value: "auto", labelKey: m("player.track.auto") },
              { value: "none", labelKey: m("player.track.disable") },
              ...tracks.map((track, index) => ({
                value: `id:${track.id}`,
                labelKey: trackDisplayName(track, m("player.track.unknown"), index),
              })),
            ];
        const value = noMedia ? "unavailable" : selected === null ? "none" : `id:${selected}`;
        return (
          <div
            key={kind}
            className="lumen-controls__track"
            data-track-kind={kind}
            data-empty={noMedia ? "" : undefined}
          >
            {/* The frame's icon face (audio bars / cc card); the dropdown
                value span stays in the DOM for assistive tech + tests. */}
            <Icon name={TRACK_ICONS[kind]} className="lumen-controls__track-icon" />
            <Dropdown
              label={
                noMedia
                  ? `${m(labelKey)} — ${m("player.controls.reason.no-media")}`
                  : m(labelKey)
              }
              options={options}
              value={value}
              disabled={noMedia}
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
          </div>
        );
      })}
    </div>
  );
}

/** fullscreen · cast · gear (Settings) · app menu. */
function OutputCluster() {
  const m = useMsg();
  return (
    <div
      className="lumen-controls__output"
      role="group"
      aria-label={CONTROLS_COPY.groupOutput}
    >
      <FullscreenButton />
      <IconButton
        data-control="cast"
        /* Honest not-available: no cast surface exists in the engine yet —
         * the control renders, disabled with its reason (doc 08 §4.4). */
        label={`${CONTROLS_COPY.cast} — ${m("player.controls.reason.unknown")}`}
        disabled
      >
        <Icon name="tv" />
      </IconButton>
      <IconButton
        data-control="settings"
        label={m("menu.view.settings")}
        onClick={useNavigateToSettings()}
      >
        <Icon name="settings" />
      </IconButton>
      <AppMenuButton />
    </div>
  );
}

/** Fullscreen toggle on the player host (PlayerScreen's [data-screen]
 * root), state synced only from fullscreenchange events (doc 08 §4.3). */
function FullscreenButton() {
  const m = useMsg();
  const hostRef = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(fullscreenActive);

  useEffect(() => {
    const onChange = (): void => setActive(fullscreenActive());
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, []);

  const supported = fullscreenSupported();
  const base = active ? CONTROLS_COPY.exitFullscreen : CONTROLS_COPY.fullscreen;
  return (
    <div className="lumen-controls__fs" ref={hostRef}>
      <IconButton
        data-control="fullscreen"
        data-fullscreen={String(active)}
        label={supported ? base : `${base} — ${m("player.controls.reason.unknown")}`}
        disabled={!supported}
        onClick={() => {
          if (fullscreenActive()) {
            void document.exitFullscreen();
            return;
          }
          const host =
            hostRef.current?.closest("[data-screen='player']") ?? document.documentElement;
          void host.requestFullscreen().catch(() => {
            /* A refused request never flips state — the fullscreenchange
             * listener stays the only source of truth (doc 08 §4). */
          });
        }}
      >
        <Icon name={active ? "minimize" : "fullscreen"} />
      </IconButton>
    </div>
  );
}

/** The Settings gear — a real navigation into the Settings screen. */
function useNavigateToSettings(): () => void {
  const navigate = useNavigate();
  return () => navigate("/settings");
}

/** The app menu (layout.v1 [[menu]] id="app"): real actions only — the
 * shell-wide open-media event and route navigations. Compact menu grammar. */
function AppMenuButton() {
  const m = useMsg();
  const navigate = useNavigate();
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);

  // Outside pointer dismiss (the MenuPopup pattern's caller half — MenuBar
  // carries the same listener).
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent): void => {
      if (rootRef.current !== null && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const entries: MenuEntry<MessageKey>[] = [
    {
      id: "open-media",
      labelKey: "player.empty.open",
      icon: "open-file",
      onSelect: () => window.dispatchEvent(new CustomEvent("lumen:open-media")),
    },
    {
      id: "open-network-stream",
      labelKey: "menu.file.open-network-stream",
      icon: "network-stream",
      onSelect: () => navigate("/open-network-stream"),
    },
    { id: "sep-navigation", kind: "separator" },
    {
      id: "keyboard-shortcuts",
      labelKey: "menu.help.keyboard-shortcuts",
      icon: "keyboard-shortcuts",
      onSelect: () => navigate("/keyboard-shortcuts"),
    },
    {
      id: "settings",
      labelKey: "menu.view.settings",
      icon: "settings",
      onSelect: () => navigate("/settings"),
    },
  ];

  const close = (): void => setOpen(false);

  return (
    <div className="lumen-controls__menu" ref={rootRef}>
      <IconButton
        data-control="menu"
        label={m("player.menu.app")}
        aria-haspopup="menu"
        aria-expanded={open}
        className={open ? "lumen-icon-host--open" : undefined}
        onClick={() => setOpen((prev) => !prev)}
      >
        <Icon name="menu" />
      </IconButton>
      {open ? (
        <MenuPopup
          entries={entries}
          resolveLabel={m}
          idPrefix="lumen-controls-menu"
          density="compact"
          className="lumen-controls__menu-popup"
          ariaLabel={m("player.menu.app")}
          onRequestClose={close}
          onDismissAll={close}
        />
      ) : null}
    </div>
  );
}

/** Typed-error surface (doc 08 §5.4): stage-named, dismissible, honest.
 * W26 restyle: a compact r10 panel chip (surface-panel + hairline border)
 * floating above the bar's right edge — never a wide strip. */
function ErrorSurface() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  if (state.lastError === null) return null;
  const error = state.lastError;
  return (
    <div className="lumen-controls__error" role="status" data-error-code={error.code}>
      <Icon name="about" className="lumen-controls__error-icon" />
      <span className="lumen-controls__error-detail">
        <span className="lumen-controls__error-title">{m("player.error.title")}</span>
        {`${error.stage}: ${error.message} (${error.code})`}
      </span>
      <Button data-control="dismiss-error" size="sm" onClick={actions.dismissError}>
        {m("player.error.dismiss")}
      </Button>
    </div>
  );
}
