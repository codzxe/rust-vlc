// w26.a14 — the bottom control bar, composed to the W26 Figma frames.
// Authorities: design/figma/Player.svg "Controls" group (the 1920×54 strip
// → the app's ~44px chrome strip), the component frames play.svg /
// "Current Time.svg" / "Duration.svg" / volume.svg, and the measured anatomy
// in docs/waves/w26/analysis/menus-control-parts.md §B1/§B6–§B9 (values
// cited per rule in controls.css). ONE row, three groups:
//
//   LEFT    stop · ORANGE PLAY PILL 54×28 r6 (dark on-accent glyph — the
//           hero; play.svg's pause type grays the pill — accent is the
//           "play" affordance) · next (a real playlist.next through the
//           playlist context PlayerScreen mounts around this bar)
//   CENTER  current timecode (≈15px tabular) · seekbar — the w26.a04
//           slider API with the frame-measured anatomy: 28px r6 surface
//           track, played fill r4 accent-tinted inset 2, and the playhead
//           as an 8×24 r4 accent BAR (never a circular thumb) · remaining
//           timecode (Duration.svg's "-00:23:30" click face)
//   RIGHT   total duration · volume icon + the 89px level wedge
//           (volume.svg: green→yellow→orange→red gradient anchored to the
//           FULL track, revealed to the level — the spec §2 "small green
//           level pill") · fullscreen (Fullscreen API on the player host) ·
//           cast (honestly not-available — no surface yet) · app menu
//           (real navigation + the shell open-media seam + the
//           Video/Audio/Subtitle track SUBMENUS — the track-selector
//           dropdowns moved off the row into the menu per the w26.a13
//           controls-strip finding; types.ts "dynamic submenus (tracks…)
//           arrive as data at runtime") · gear (Settings route)
//
// Behavior contract UNCHANGED (state-mapping.ts is the doc 08 §5 mapping —
// play/pause/stop/seek/volume/mute/track wiring below is a restyle of the
// same dispatch, never a semantic change). New controls are additive seams
// on the same honest pattern: disabled-with-reason, never fabricated state.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { Button, IconButton, Icon, Seekbar, VolumeSlider } from "../../../components";
import { MenuPopup } from "../../../components/menu/MenuPopup";
import { identityResolveLabel } from "../../../components/menu/types";
import type { IconName, MenuEntry } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import {
  selectedTrackId,
  trackDisplayName,
  tracksOfKind,
  usePlayerControls,
} from "../context";
import { usePlaylist } from "../../playlist";
import { openToolsDialog } from "../dialogs";
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
  /** aria name of the output cluster (fullscreen/cast/menu/gear). */
  groupOutput: "Player controls",
} as const;

/** Track-submenu faces (Player.svg right cluster glyphs: the cc card reads
 * "subtitles"; the audio bars read "audio"; the video kind has no bar glyph
 * in the frames — `pip`'s screen-with-video geometry is the closest honest
 * glyph from the shared Icon.svg catalog). */
const TRACK_KINDS: ReadonlyArray<{
  readonly kind: "video" | "audio" | "subtitle";
  readonly labelKey: "player.track.video" | "player.track.audio" | "player.track.subtitle";
  readonly icon: IconName;
}> = [
  { kind: "video", labelKey: "player.track.video", icon: "pip" },
  { kind: "audio", labelKey: "player.track.audio", icon: "audio" },
  { kind: "subtitle", labelKey: "player.track.subtitle", icon: "subtitles" },
];

/** Live document fullscreen truth (never cached — doc 08 §4 honest state). */
const fullscreenActive = (): boolean => (document.fullscreenElement ?? null) !== null;

/**
 * Frame timecode format — "Current Time.svg"/"Duration.svg" read
 * "00:02:10"/"00:25:40": zero-padded HH:MM:SS on tabular figures (the CSS
 * sets font-variant-numeric). The shared slider's formatSeekTime (compact
 * m:ss) stays the scrub aria formatter — the bar's visible readouts use the
 * frame format. Negative inputs clamp (the remaining readout handles its
 * own minus sign).
 */
const formatTimecode = (seconds: number): string => {
  const total = Math.max(0, Math.floor(seconds));
  const pad = (value: number): string => String(value).padStart(2, "0");
  return `${pad(Math.floor(total / 3600))}:${pad(Math.floor((total % 3600) / 60))}:${pad(total % 60)}`;
};

/** Bar icon sizes (Player.svg "Controls" group, measured): the transport
 * glyphs are small and quiet (~10px tall bodies), the right-cluster glyphs
 * larger (~17–18px). */
const TRANSPORT_ICON_SIZE = 14;
const OUTPUT_ICON_SIZE = 18;

/** Host capability (jsdom and embedded hosts expose no Fullscreen API). */
const fullscreenSupported = (): boolean =>
  typeof document !== "undefined" &&
  typeof document.documentElement.requestFullscreen === "function" &&
  typeof document.exitFullscreen === "function";

/**
 * Router-presence seam for the bar's links (gear → Settings, the app menu's
 * route entries). The app mounts /player inside <App/>'s Router, so the
 * real bar navigates in-app; bare mounts — embedded hosts and jsdom test
 * trees without a Router, where `useNavigate` throws — fall back to
 * document navigation instead of crashing the bar. The component split
 * keeps every hook call unconditional (rules-of-hooks).
 */
function NavigationSeam({
  children,
}: {
  children: (navigate: (to: string) => void) => ReactNode;
}): ReactNode {
  return useInRouterContext() ? (
    <RoutedNavigation>{children}</RoutedNavigation>
  ) : (
    <>{children((to) => window.location.assign(to))}</>
  );
}

function RoutedNavigation({
  children,
}: {
  children: (navigate: (to: string) => void) => ReactNode;
}): ReactNode {
  const navigate = useNavigate();
  return <>{children((to) => navigate(to))}</>;
}

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
        <Icon name="stop" size={TRANSPORT_ICON_SIZE} />
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
        <Icon name="next" size={TRANSPORT_ICON_SIZE} />
      </IconButton>
    </div>
  );
}

/** current timecode · seekbar · remaining (doc 08 §5.2/§5.3 honest). */
function SeekCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

  // Remaining is honest arithmetic over engine truth only: duration known →
  // duration − position; unknown → the same "--:--" unknown as the total
  // (Duration.svg's click face is the minus-prefixed remaining reading).
  const remaining =
    state.durationSeconds === null || state.durationSeconds <= 0
      ? m("player.controls.duration-unknown")
      : `−${formatTimecode(state.durationSeconds - state.position)}`;

  return (
    <div className="lumen-controls__seek">
      <span className="lumen-controls__time" data-time="position">
        {formatTimecode(state.position)}
      </span>
      <Seekbar
        data-control="seekbar"
        /* A duration-less seekbar is an unplayed track — data-empty lets the
         * slider CSS hide the phantom zero-position playhead (calm track). */
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

/** The right group: total duration · volume · output controls. */
function RightCluster() {
  const { state } = usePlayerControls();
  const m = useMsg();
  return (
    <div className="lumen-controls__right">
      <span className="lumen-controls__time" data-time="duration">
        {state.durationSeconds === null
          ? m("player.controls.duration-unknown")
          : formatTimecode(state.durationSeconds)}
      </span>
      <VolumeCluster />
      <OutputCluster />
    </div>
  );
}

/** Volume icon + the 89px level wedge (engine-reported, never optimistic). */
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
        <Icon name={iconName} size={OUTPUT_ICON_SIZE} />
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

/** fullscreen · cast · app menu (+ track submenus) · gear (Settings). */
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
        <Icon name="tv" size={OUTPUT_ICON_SIZE} />
      </IconButton>
      <NavigationSeam>
        {(navigate) => (
          <>
            <AppMenuButton navigate={navigate} />
            {/* w26.c5: gear opens the audio/video tools dialog (frame: Player.svg
                right cluster); Settings stays reachable via the app menu. */}
            <IconButton
              data-control="settings"
              label={m("menu.view.settings")}
              onClick={() => openToolsDialog("audio")}
            >
              <Icon name="settings" size={OUTPUT_ICON_SIZE} />
            </IconButton>
          </>
        )}
      </NavigationSeam>
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
        <Icon name={active ? "minimize" : "fullscreen"} size={OUTPUT_ICON_SIZE} />
      </IconButton>
    </div>
  );
}

/**
 * The app menu (layout.v1 [[menu]] id="app"): real actions only — the
 * shell-wide open-media event, route navigations, and the three engine-truth
 * track submenus (Video/Audio/Subtitle; types.ts "dynamic submenus (tracks,
 * devices, chapters) arrive as data at runtime"). Labels are pre-resolved
 * through useMsg (dynamic track names arrive as already-resolved strings —
 * the Dropdown identity pattern), so the popup resolves with the identity
 * seam.
 */
function AppMenuButton({ navigate }: { readonly navigate: (to: string) => void }) {
  const m = useMsg();
  const { state, actions } = usePlayerControls();
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

  const trackMenus: MenuEntry<string>[] = TRACK_KINDS.map(({ kind, labelKey, icon }): MenuEntry<string> => {
    const tracks = tracksOfKind(state.tracks, kind);
    const selected = selectedTrackId(state.tracks, state.selections, kind);
    // No honest track data to show: no media open, still opening, or the
    // engine has not reported any track list at all (doc §5.5 — never a
    // fabricated list). The submenu still opens — onto its single disabled
    // no-media reason row (v2 §4.3's honest-unknown pattern, restated as a
    // menu: discoverable, labeled, inert).
    const unreported = !state.engineReported && tracks.length === 0;
    const noMedia =
      state.playbackState === "idle" || state.playbackState === "opening" || unreported;
    if (noMedia) {
      return {
        id: `track-${kind}`,
        labelKey: m(labelKey),
        icon,
        submenu: [
          {
            id: `track-${kind}-no-media`,
            labelKey: m("player.controls.reason.no-media"),
            disabled: true,
          },
        ],
      };
    }
    return {
      id: `track-${kind}`,
      labelKey: m(labelKey),
      icon,
      submenu: [
        {
          id: `track-${kind}-auto`,
          labelKey: m("player.track.auto"),
          kind: "radio",
          /* Mirrors the pre-W26 dropdown truth: "Automatic" is an action,
           * never the engine-reported selection (selections arrive as ids). */
          checked: false,
          onSelect: () => actions.selectTrack(kind, { kind: "auto" }),
        },
        {
          id: `track-${kind}-none`,
          labelKey: m("player.track.disable"),
          kind: "radio",
          checked: selected === null,
          onSelect: () => actions.selectTrack(kind, { kind: "none" }),
        },
        ...tracks.map((track, index): MenuEntry<string> => ({
          id: `track-${kind}-id-${track.id}`,
          labelKey: trackDisplayName(track, m("player.track.unknown"), index),
          kind: "radio",
          checked: selected === track.id,
          onSelect: () => actions.selectTrack(kind, { kind: "id", id: track.id }),
        })),
      ],
    };
  });

  const entries: MenuEntry<string>[] = [
    {
      id: "open-media",
      labelKey: m("player.empty.open"),
      icon: "open-file",
      onSelect: () => window.dispatchEvent(new CustomEvent("lumen:open-media")),
    },
    {
      id: "open-network-stream",
      labelKey: m("menu.file.open-network-stream"),
      icon: "network-stream",
      onSelect: () => navigate("/open-network-stream"),
    },
    { id: "sep-tracks", kind: "separator" },
    ...trackMenus,
    { id: "sep-navigation", kind: "separator" },
    {
      id: "keyboard-shortcuts",
      labelKey: m("menu.help.keyboard-shortcuts"),
      icon: "keyboard-shortcuts",
      onSelect: () => navigate("/keyboard-shortcuts"),
    },
    {
      id: "settings",
      labelKey: m("menu.view.settings"),
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
        <Icon name="menu" size={OUTPUT_ICON_SIZE} />
      </IconButton>
      {open ? (
        <MenuPopup
          entries={entries}
          resolveLabel={identityResolveLabel}
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
 * W26 restyle: a compact r12 panel chip (surface-panel + hairline border)
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
