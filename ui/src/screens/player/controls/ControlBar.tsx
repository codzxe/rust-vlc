// w26.a14 — the bottom control bar, composed to the W26 Figma frames.
// Authorities: design/figma/Player.svg "Controls" group (the 1920×54 strip
// → the app's ~44px chrome strip), the component frames play.svg /
// "Current Time.svg" / Duration.svg / volume.svg, the Controls.svg variant
// sheet (12 popover states), and the measured anatomy in
// docs/waves/w26/analysis/menus-control-parts.md §B1/§B6–§B9 (values cited
// per rule in controls.css). ONE row, three groups:
//
//   LEFT    previous · ORANGE PLAY PILL 54×28 r6 (dark on-accent glyph —
//           the hero; play.svg's pause type grays the pill — accent is the
//           "play" affordance) · next. w26.gfix-C (g04 P1-1): the left slot
//           is SKIP-TO-PREVIOUS (Player.svg Frame 8 / Controls.svg), NOT
//           stop — stop stays on the canvas context menu where
//           right-click_menu.svg actually shows it.
//   CENTER  current timecode (≈15px tabular; Current_Time.svg: hover chip
//           + the click face = the editable timecode — the Go-to-Time
//           dialog, orange caret while open) · seekbar — the w26.a04 slider
//           API with the frame-measured anatomy: 28px r6 surface track,
//           played fill r4 accent-tinted inset 2, and the playhead as an
//           8×24 r4 accent BAR (never a circular thumb).
//           w26.f13 (e02 F6): exactly two timecodes (00:02:10 | seekbar |
//           00:25:40). w26.gfix-C (g04 P1-4): both readouts are now real
//           buttons — the duration click face is Duration.svg's "-00:23:30"
//           remaining toggle.
//   RIGHT   total duration (remaining-toggle button) · volume icon + the
//           89px level wedge (volume.svg: green→yellow→orange→red gradient
//           anchored to the FULL track, revealed to the level) · audio ·
//           subtitles · gear · queue — the Controls.svg Frame 23 right
//           cluster. w26.i03 (h04 §2/§7, replacing the gfix-C MenuPopup
//           mounts): each trigger opens the frame's SETTINGS PANEL — a
//           400 r10 #0E0E0E@95% tabbed panel floating ABOVE the strip
//           (never over its own trigger — h04 defect #1), 50px #090909 tab
//           header with the #D37918 active underline, section headers,
//           engine-truth radio rows, and Button.png's 360×32 r4 quiet
//           footer ("Show Advanced Settings" → the real tools surface).
//           The app menu (w26.f13 ruling: it lives in the bar until f08
//           relocates the trigger to the shell title bar) stays as the
//           cluster's documented extra — menu.png grammar, MenuPopup.
//
// Behavior contract UNCHANGED (state-mapping.ts is the doc 08 §5 mapping —
// play/pause/seek/volume/mute wiring below is a restyle of the same
// dispatch, never a semantic change). New controls are additive seams on
// the same honest pattern: disabled-with-reason, never fabricated state.

import { useCallback, useEffect, useRef, useState } from "react";
import { memo } from "react";
import type { ReactNode } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { Button, IconButton, Icon, Seekbar, SegmentedControl, VolumeSlider } from "../../../components";
import { MenuPopup } from "../../../components/menu/MenuPopup";
import { identityResolveLabel } from "../../../components/menu/types";
import type { MenuEntry } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as k07Keys } from "../../../i18n/keys/w26.k07";
import { buildProfileFlyout, createProfile, selectProfile, useMediaProfiles } from "../../../components/menu";
import { useClient } from "../../../client/ClientProvider";
import type { PlaybackState } from "../../../client/transport";
import { isDemoTransport } from "../../../client/demo-transport";
import { openPathRequest, openPathRoute } from "../../../client/open-path";
import { HOTKEY_BINDINGS } from "../../settings/areas/hotkeys/generated/bindings";
import { usePlayerControls } from "../context";
import type { PlayerControls } from "../context";
import { selectedTrackId, trackDisplayName, tracksOfKind } from "../context";
import { usePlaylist } from "../../playlist";
import { formatDuration } from "../../playlist";
import { CustomizeToolbarItems } from "../customize/toolbarItems";
import { GotoTimeDialog, openToolsDialog } from "../dialogs";
import { mapTransportControls, seekDisableReason } from "./state-mapping";
import { SettingsPanel } from "./SettingsPanel";
import type { SettingsPanelTab } from "./SettingsPanel";
import {
  PanelRadioGroup,
  PanelReasonRow,
  PanelSection,
  PanelToggleRow,
  PanelValueRow,
  QuietButton,
} from "./SettingsPanel";
import "./controls.css";

/**
 * w26.k07 lane keys (the queue meta composition, j07 P2-4) — contract §4.5:
 * catalog first once the Lead registers them, lane copy until then. This is
 * the LIVE pre-registration seam (the b15 useNetworkStreamMsg pattern),
 * typed over the WHOLE k07 key union — unlike the dead f13/gfixC/i03/i16
 * resolvers this lane deleted (j07 P1: every one of those keys was already
 * registered in messages.ts, so their fallbacks were unreachable and the
 * LaneMessageKey union lied about the catalog merge). After the Lead
 * registers the k07 keys, both paths return the same string and the next
 * cleanup pass can delete this seam the same way.
 */
const k07Catalog: Readonly<Record<string, string>> = k07Keys;

type K07LaneKey = keyof typeof k07Keys;

const k07Resolver =
  (m: (key: MessageKey) => string) =>
  (key: K07LaneKey): string =>
    m(key as MessageKey) ?? k07Catalog[key];

/**
 * Binding-id → accelerator display string from keymap DATA (HOTKEY_BINDINGS
 * is the generated keymaps/default.toml excerpt — doc 05 §8.1: menus render
 * the binding's key, never a hardcoded string). Frame spelling "Ctrl + O"
 * (menu.png): modifiers re-joined with spaces, plain keys as-is.
 * (PlayerScreen's canvas-menu helper, verbatim — that copy is not exported.)
 */
const acceleratorOf = (bindingId: string): string | undefined => {
  const binding = HOTKEY_BINDINGS.find((entry) => entry.id === bindingId);
  if (binding === undefined) return undefined;
  return binding.key
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean)
    .join(" + ");
};

/** The customize surface's open seam — PlayerScreen exports this name as
 * PLAYER_OPEN_CUSTOMIZE_EVENT; the string is the frozen cross-component
 * contract ("never rename"), restated here because importing PlayerScreen
 * from this sibling would be a module cycle. */
const OPEN_CUSTOMIZE_EVENT = "lumen:open-customize";

/**
 * Frame timecode format — "Current Time.svg"/"Duration.svg" read
 * "00:02:10"/"00:25:40": zero-padded HH:MM:SS on tabular figures (the CSS
 * sets font-variant-numeric). The shared slider's formatSeekTime (compact
 * m:ss) stays the scrub aria formatter — the bar's visible readouts use the
 * frame format.
 */
const formatTimecode = (seconds: number): string => {
  const total = Math.max(0, Math.floor(seconds));
  const pad = (value: number): string => String(value).padStart(2, "0");
  return `${pad(Math.floor(total / 3600))}:${pad(Math.floor((total % 3600) / 60))}:${pad(total % 60)}`;
};

/** Bar icon sizes (Player.svg "Controls" group, measured): the transport
 * glyphs are ~10px-tall bodies (w26.gfix-C, g04 P2-1: the pre-fix 14px
 * render measured ~60–75% of frame — 20px on the 24-unit viewBox + 1.5
 * stroke ≈ the frame bodies), the play pill's triangle 22px, the
 * right-cluster glyphs ~17–18px. */
const TRANSPORT_ICON_SIZE = 20;
const PLAY_ICON_SIZE = 22;
const OUTPUT_ICON_SIZE = 18;

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
  // w26.k07 (j07 P2-5): stable identity — the bare-mount fallback feeds the
  // memo'd AppMenuButton as a prop, so a per-tick root re-render must not
  // mint a fresh navigate (the routed arm's useNavigate is already stable).
  const fallbackNavigate = useCallback((to: string) => window.location.assign(to), []);
  return useInRouterContext() ? (
    <RoutedNavigation>{children}</RoutedNavigation>
  ) : (
    <>{children(fallbackNavigate)}</>
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
  const { state, actions } = usePlayerControls();
  return (
    <div
      className={["lumen-controls", className].filter(Boolean).join(" ")}
      data-playback-state={state.playbackState}
      data-engine-reported={state.engineReported}
    >
      {/* w26.k07 (j07 P2-5): the transport cluster takes PRIMITIVE props so
       * the memo below holds across position ticks (the only per-tick state
       * change — the root + the seek/duration readouts re-render, the
       * transport cluster does not). */}
      <TransportCluster
        playbackState={state.playbackState}
        pendingToggle={state.pendingToggle}
        onTogglePlayPause={actions.togglePlayPause}
      />
      {/* w26.i05's cross-lane wiring (h07 defect 1's last mile, taken
       * verbatim from its lane log): the saved customize band rendered in
       * the real strip, directly after the transport — frame -7's position.
       * Renders nothing while the band is empty (frame -1's clean
       * baseline), so the untouched bar's DOM is unchanged. */}
      <CustomizeToolbarItems />
      <SeekCluster />
      <RightCluster />
      <ErrorSurface />
    </div>
  );
}

/**
 * The engine-truth radio rows for one track kind (w26.i03: the same honest
 * content the gfix-C menus carried, as PanelRadioRow descriptors): no honest
 * track data → empty (the caller renders the no-media reason row); else
 * Automatic / Disabled / real engine ids, the engine-reported selection
 * checked (never optimistic).
 */
interface TrackRow {
  readonly key: string;
  readonly label: string;
  readonly checked: boolean;
  readonly onSelect: () => void;
}

function trackRows(
  kind: "video" | "audio" | "subtitle",
  controls: PlayerControls,
  m: (key: MessageKey) => string,
): TrackRow[] {
  const { state, actions } = controls;
  const tracks = tracksOfKind(state.tracks, kind);
  const selected = selectedTrackId(state.tracks, state.selections, kind);
  const unreported = !state.engineReported && tracks.length === 0;
  const noMedia = state.playbackState === "idle" || state.playbackState === "opening" || unreported;
  if (noMedia) {
    return [];
  }
  return [
    {
      key: `${kind}-auto`,
      label: m("player.track.auto"),
      /* "Automatic" is an action, never the reported selection. */
      checked: false,
      onSelect: () => actions.selectTrack(kind, { kind: "auto" }),
    },
    {
      key: `${kind}-none`,
      label: m("player.track.disable"),
      checked: selected === null,
      onSelect: () => actions.selectTrack(kind, { kind: "none" }),
    },
    ...tracks.map((track, index): TrackRow => ({
      key: `${kind}-id-${track.id}`,
      label: trackDisplayName(track, m("player.track.unknown"), index),
      checked: selected === track.id,
      onSelect: () => actions.selectTrack(kind, { kind: "id", id: track.id }),
    })),
  ];
}

/** previous · play pill · next (Player.svg Frame 8 → play → Frame 10).
 * w26.k07 (j07 P2-5): memo'd on primitive props — the play mapping reads
 * only playbackState/pendingToggle (state-mapping.ts §5.1), so a position
 * tick never re-renders this cluster. Its other inputs (the playlist
 * context, useMsg) change only on playlist/locale events, never per tick. */
const TransportCluster = memo(function TransportCluster({
  playbackState,
  pendingToggle,
  onTogglePlayPause,
}: {
  readonly playbackState: PlaybackState;
  readonly pendingToggle: boolean;
  readonly onTogglePlayPause: () => void;
}) {
  const playlist = usePlaylist();
  const m = useMsg();
  // The cluster renders only the PLAY mapping; seekable/durationSeconds feed
  // the SEEK mapping (state-mapping.ts), which this cluster does not render.
  const mapped = mapTransportControls({
    playbackState,
    seekable: null,
    durationSeconds: null,
    pendingToggle,
  });

  const playLabel =
    mapped.play.glyph === "pause" ? m("player.controls.pause") : m("player.controls.play");
  const label = mapped.play.busy
    ? `${playLabel} — ${m("player.controls.buffering-hint")}`
    : mapped.play.restart
      ? `${playLabel} — ${m("player.controls.restart")}`
      : playLabel;

  // Previous/next — real playlist.prev/next through the playlist context
  // (the provider PlayerScreen mounts around this bar). Honestly disabled
  // with a reason while the queue is empty or the engine exposes no
  // playlist surface.
  const queueEmpty = playlist.notExposed || playlist.entries.length === 0;
  const prevLabel = queueEmpty
    ? `${m("menu.playback.previous")} — ${m(
        playlist.notExposed ? "player.controls.reason.unknown" : "player.controls.reason.no-media",
      )}`
    : m("menu.playback.previous");
  const nextLabel = queueEmpty
    ? `${m("player.controls.next")} — ${m(
        playlist.notExposed ? "player.controls.reason.unknown" : "player.controls.reason.no-media",
      )}`
    : m("player.controls.next");

  return (
    <div
      className="lumen-controls__transport"
      role="group"
      aria-label={m("player.controls.group-transport")}
    >
      <IconButton
        data-control="previous"
        label={prevLabel}
        disabled={queueEmpty}
        onClick={() => playlist.prev()}
      >
        <Icon name="previous" size={TRANSPORT_ICON_SIZE} />
      </IconButton>
      <IconButton
        className="lumen-controls__play"
        data-control="play-pause"
        data-glyph={mapped.play.glyph}
        data-restart={mapped.play.restart || undefined}
        label={label}
        loading={mapped.play.pending}
        disabled={mapped.play.disabled}
        onClick={onTogglePlayPause}
      >
        <Icon name={mapped.play.glyph} size={PLAY_ICON_SIZE} />
      </IconButton>
      <IconButton
        data-control="next"
        label={nextLabel}
        disabled={queueEmpty}
        onClick={() => playlist.next()}
      >
        <Icon name="next" size={TRANSPORT_ICON_SIZE} />
      </IconButton>
    </div>
  );
});

/** current timecode · seekbar (doc 08 §5.2/§5.3 honest).
 * w26.f13 (e02 F6): Player.svg shows exactly TWO timecodes — position left
 * of the bar, the total in the right cluster.
 * w26.gfix-C (g04 P1-4): the position readout is a real button —
 * Current_Time.svg's hover chip (76×28 r6 #090909) and the click face (the
 * editable timecode: #EAEAEA ink + the 1px #D37918 caret) ported as the
 * button's hover/editing states, the edit surface being the existing
 * Go-to-Time dialog (prefilled with this position, VLC parity). */
function SeekCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);
  const [editing, setEditing] = useState(false);

  return (
    <div className="lumen-controls__seek">
      <button
        type="button"
        className="lumen-controls__time"
        data-time="position"
        data-editing={editing ? "" : undefined}
        aria-label={
          reason !== null
            ? `${m("player.controls.time-current")} (${m(reason)})`
            : m("player.controls.time-current")
        }
        disabled={mapped.seek.disabled}
        onClick={() => setEditing(true)}
      >
        {formatTimecode(state.position)}
      </button>
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
      {editing ? <GotoTimeDialog open onClose={() => setEditing(false)} /> : null}
    </div>
  );
}

/** The right group: total duration · volume · output controls. */
function RightCluster() {
  return (
    <div className="lumen-controls__right">
      <DurationTimecode />
      <VolumeCluster />
      <OutputCluster />
    </div>
  );
}

/** Duration.svg: default face = the total "00:25:40"; the click face is the
 * "-00:23:30" REMAINING toggle (aria-pressed carries the state). Disabled
 * with the honest unknown face while no duration is reported. */
function DurationTimecode() {
  const { state } = usePlayerControls();
  const m = useMsg();
  const [remaining, setRemaining] = useState(false);
  const known = state.durationSeconds !== null && state.durationSeconds > 0;
  const face = !known
    ? m("player.controls.duration-unknown")
    : remaining
      ? `-${formatTimecode(Math.max(0, state.durationSeconds - state.position))}`
      : formatTimecode(state.durationSeconds);
  return (
    <button
      type="button"
      className="lumen-controls__time"
      data-time="duration"
      data-remaining={remaining || undefined}
      aria-label={m(remaining ? "player.controls.time-remaining" : "player.controls.time-duration")}
      aria-pressed={known ? remaining : undefined}
      disabled={!known}
      onClick={() => setRemaining((prev) => !prev)}
    >
      {face}
    </button>
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

/** audio · subtitles · gear · queue (+ the app menu — w26.f13 ruling).
 * w26.gfix-C (g04 P1-6): the cluster is Controls.svg Frame 23's composition
 * — the frame's three tool triggers + queue replace the pre-fix
 * fullscreen/cast/menu-then-gear row (fullscreen stays reachable through
 * the keymap's F binding PlayerScreen wires; cast never had a surface).
 * w26.i03 (h04 §2/§7): the popovers are the frame's SETTINGS PANELS, not
 * menus — see the header comment and SettingsPanel.tsx. */
function OutputCluster() {
  const m = useMsg();
  // w26.k07 (j07 P2-4): the queue meta composition's lane seam — the ONLY
  // live pre-registration resolver left in this file (see k07Resolver).
  const km = k07Resolver(m);
  const controls = usePlayerControls();
  const playlist = usePlaylist();

  // The tab descriptors (Controls.svg Frame 241: icon + label). The audio
  // and subtitles triggers share the 3-tab track panel ("audio"/"subtitles"
  // variants: Video · Audio · Subtitles); the gear carries the 4-tab
  // settings panel ("playback"/"* settings" variants add Playback); the
  // queue trigger carries the Queue · Bookmarks · Chapters panel.
  const trackTabs: SettingsPanelTab[] = [
    { id: "video", label: m("player.canvas.menu.video"), icon: "tv" },
    { id: "audio", label: m("player.canvas.menu.audio"), icon: "audio" },
    { id: "subtitle", label: m("player.controls.tab.subtitles"), icon: "subtitles" },
  ];
  const settingsTabs: SettingsPanelTab[] = [
    { id: "playback", label: m("menu.playback"), icon: "playback" },
    ...trackTabs,
  ];
  const queueTabs: SettingsPanelTab[] = [
    { id: "queue", label: m("player.controls.queue"), icon: "queue" },
    { id: "bookmarks", label: m("player.controls.tab.bookmarks"), icon: "bookmark" },
    { id: "chapters", label: m("player.controls.tab.chapters"), icon: "chapters" },
  ];

  /** The Playback tab — the REAL playback seams the bar owns (playlist
   * context): Shuffle as a Toggle.png switch, Repeat as the Values.png
   * segmented control. Honest disabled while the engine has not reported. */
  const renderPlaybackTab = (): ReactNode => (
    <div className="lumen-controls__panel-section">
      <PanelToggleRow
        label={m("menu.playback.shuffle")}
        checked={playlist.shuffle === true}
        disabled={playlist.shuffle === null}
        onChange={(next) => playlist.setShuffle(next)}
      />
      <div className="lumen-controls__panel-field">
        <span className="lumen-controls__panel-field-label">{m("menu.playback.repeat")}</span>
        <SegmentedControl
          className="lumen-controls__panel-segmented"
          label={m("menu.playback.repeat")}
          options={[
            { value: "off", label: m("player.controls.repeat.off") },
            { value: "all", label: m("player.controls.repeat.all") },
            { value: "one", label: m("player.controls.repeat.one") },
          ]}
          value={playlist.repeat ?? ""}
          disabled={playlist.repeat === null}
          onChange={(mode) => playlist.setRepeat(mode as "off" | "all" | "one")}
        />
      </div>
    </div>
  );

  /** One track-kind tab — the "audio"/"subtitles" panel anatomy: Embedded
   * Tracks section (+ the kind's second frame section), and on the gear's
   * settings panel the Defaults label/value rows over the same engine data
   * ("audio settings" variant: Audio Track → "1", Audio Language →
   * "English"). Sections separate by SPACING only — the variant panels
   * carry no divider lines (verified: zero hairline rects/lines inside the
   * panel x-range in any Controls.svg variant group). */
  const renderTrackTab = (
    kind: "video" | "audio" | "subtitle",
    withDefaults: boolean,
  ): ReactNode => {
    const rows = trackRows(kind, controls, m);
    const tracks = tracksOfKind(controls.state.tracks, kind);
    const selected = selectedTrackId(controls.state.tracks, controls.state.selections, kind);
    const selectedTrack = tracks.find((track) => track.id === selected) ?? null;
    const noMedia = rows.length === 0;
    // w26.k07 (j07 P2-2): the rows ride ONE radiogroup named by the section
    // title (roving tabindex + arrow keys — SettingsPanel's PanelRadioGroup).
    const trackSection = (
      <PanelSection title={m("player.controls.section.embedded-tracks")}>
        {noMedia ? (
          <PanelReasonRow reason={m("player.controls.reason.no-media")} />
        ) : (
          <PanelRadioGroup label={m("player.controls.section.embedded-tracks")} options={rows} />
        )}
      </PanelSection>
    );
    return (
      <>
        {withDefaults && kind !== "video" ? (
          <PanelSection title={m("player.controls.section.defaults")}>
            <PanelValueRow
              label={m(`player.track.${kind}`)}
              value={
                noMedia
                  ? m("player.controls.reason.no-media")
                  : selectedTrack !== null
                    ? trackDisplayName(selectedTrack, m("player.track.unknown"), 0)
                    : m("player.track.disable")
              }
            />
            <PanelValueRow
              label={m(
                kind === "subtitle"
                  ? "player.controls.row.subtitle-language"
                  : "player.controls.row.audio-language",
              )}
              value={selectedTrack?.language ?? m("player.controls.reason.unknown")}
            />
          </PanelSection>
        ) : null}
        {trackSection}
        {kind === "audio" ? (
          /* No output-device seam exists in any transport — the frame's
           * section renders its honest unavailable face, never a fabricated
           * device list (doc 08 §4.4). The frame's right-aligned rescan
           * glyph (audio variant, Vector 19/20 x1870–1890) rides the header
           * inert — no seam to wire. */
          <PanelSection
            title={m("player.controls.section.output-device")}
            trailingIcon="refresh"
          >
            <PanelReasonRow reason={m("player.controls.reason.unknown")} />
          </PanelSection>
        ) : null}
        {kind === "subtitle" ? (
          /* External sidecar tracks have no engine seam either (the Add
           * Subtitles flow is the playlist dialogs lane's surface). */
          <PanelSection title={m("player.controls.section.external-tracks")}>
            <PanelReasonRow reason={m("player.controls.reason.unknown")} />
          </PanelSection>
        ) : null}
      </>
    );
  };

  /** The Queue tab — the playlist context's real entries (click plays; the
   * engine-reported current row checked, the entry's duration right-aligned
   * — the queue variant's "The Wand_S01E11.mp4 … 23:06" rows), the frame's
   * "4 items - 2h 3m" meta row, or the honest empty reason; the docked queue
   * panel itself stays the canvas menu's "Playlist" seam
   * (PLAYER_TOGGLE_SIDEBAR_EVENT). */
  const renderQueueTab = (): ReactNode => {
    const queueEmpty = playlist.notExposed || playlist.entries.length === 0;
    if (queueEmpty) {
      return (
        <PanelReasonRow
          reason={m(
            playlist.notExposed ? "player.controls.reason.unknown" : "player.controls.queue-empty",
          )}
        />
      );
    }
    // "4 items - 2h 3m": the count + the queue's known total (nulls are
    // skipped — a partial total would read as the whole truth). w26.k07
    // (j07 P2-4): the composition rides i18n keys (w26.k07.ts) — unit
    // grammar, join and word order are localizable, never hardcoded English.
    const knownSeconds = playlist.entries
      .map((entry) => entry.durationSeconds)
      .filter((seconds): seconds is number => seconds !== null);
    const total = knownSeconds.reduce((sum, seconds) => sum + seconds, 0);
    const count = String(playlist.entries.length);
    const meta =
      knownSeconds.length === playlist.entries.length && total > 0
        ? km("player.controls.queue-meta-total")
            .replace("{count}", count)
            .replace(
              "{duration}",
              km("player.controls.queue-total")
                .replace("{hours}", String(Math.floor(total / 3600)))
                .replace("{minutes}", String(Math.floor((total % 3600) / 60))),
            )
        : km("player.controls.queue-meta").replace("{count}", count);
    // The queue rows are one radiogroup (j07 P2-2) named by the tab — arrow
    // keys move focus + play the picked entry (click parity).
    const queueOptions = playlist.entries.map((entry) => ({
      key: `queue-${entry.id}`,
      label: entry.title ?? m("playlist.entry.unknown-title"),
      checked: playlist.currentId === entry.id,
      value: formatDuration(entry.durationSeconds) ?? undefined,
      onSelect: () => playlist.playItem(entry.id),
    }));
    return (
      <>
        <div className="lumen-controls__panel-meta">{meta}</div>
        <PanelRadioGroup label={m("player.controls.queue")} options={queueOptions} />
      </>
    );
  };

  /** Bookmarks/Chapters have no bar-context seam (the bookmarks engine data
   * lives behind the tools/dialogs surfaces, w26.c5) — honest faces. */
  const renderUnavailableTab = (): ReactNode => (
    <PanelReasonRow reason={m("player.controls.reason.unknown")} />
  );

  /** The shared footer — the frame's two faces (all values measured on the
   * variant sheets): the 3-tab TRACK panels end in Button.png's quiet
   * button (360×32 r4 #090909, "video"/"audio"/"subtitles" variants); the
   * gear's 4-tab SETTINGS panel ends in the same content BARE (no 360×32
   * rect exists in the "audio settings" footer zone — just the ⓘ + label
   * 27px above the panel bottom) and the Playback tab carries NO footer
   * ("playback" variant has no footer label at all). Both open the real
   * A/V tools surface (w26.c5). */
  const renderTrackFooter = (tab: string, close: () => void): ReactNode => (
    <QuietButton
      label={m("player.controls.advanced-settings")}
      onClick={() => {
        close();
        openToolsDialog(tab === "video" ? "video" : "audio");
      }}
    />
  );
  const renderSettingsFooter = (tab: string, close: () => void): ReactNode =>
    tab === "playback" ? null : (
      <QuietButton
        bare
        label={m("player.controls.advanced-settings")}
        onClick={() => {
          close();
          openToolsDialog(tab === "video" ? "video" : "audio");
        }}
      />
    );

  return (
    <div
      className="lumen-controls__output"
      role="group"
      aria-label={m("player.controls.group-output")}
    >
      <PanelPopover
        dataControl="audio"
        label={m("player.controls.audio-tracks")}
        idPrefix="lumen-controls-audio"
        initialTab="audio"
        tabs={trackTabs}
        renderTab={(tab) => renderTrackTab(tab as "video" | "audio" | "subtitle", false)}
        renderFooter={renderTrackFooter}
      >
        <Icon name="audio" size={OUTPUT_ICON_SIZE} />
      </PanelPopover>
      <PanelPopover
        dataControl="subtitles"
        label={m("player.controls.subtitle-tracks")}
        idPrefix="lumen-controls-subtitles"
        initialTab="subtitle"
        tabs={trackTabs}
        renderTab={(tab) => renderTrackTab(tab as "video" | "audio" | "subtitle", false)}
        renderFooter={renderTrackFooter}
      >
        <Icon name="subtitles" size={OUTPUT_ICON_SIZE} />
      </PanelPopover>
      <PanelPopover
        dataControl="settings"
        label={m("menu.view.settings")}
        idPrefix="lumen-controls-settings"
        initialTab="audio"
        tabs={settingsTabs}
        renderTab={(tab) =>
          tab === "playback"
            ? renderPlaybackTab()
            : renderTrackTab(tab as "video" | "audio" | "subtitle", true)
        }
        renderFooter={renderSettingsFooter}
      >
        <Icon name="settings" size={OUTPUT_ICON_SIZE} />
      </PanelPopover>
      <PanelPopover
        dataControl="queue"
        label={m("player.controls.queue")}
        idPrefix="lumen-controls-queue"
        initialTab="queue"
        tabs={queueTabs}
        renderTab={(tab) => (tab === "queue" ? renderQueueTab() : renderUnavailableTab())}
      >
        <Icon name="lists" size={OUTPUT_ICON_SIZE} />
      </PanelPopover>
      <NavigationSeam>
        {(navigate) => <AppMenuButton navigate={navigate} />}
      </NavigationSeam>
    </div>
  );
}

/**
 * Shared right-cluster popover host (w26.gfix-C Lead ruling: popovers over
 * the full-width strip must open and be reachable) — trigger + the frame's
 * settings panel (SettingsPanel.tsx), the AppMenuButton mount pattern:
 * outside-pointer dismiss, Escape restoring trigger focus (e12 F8), and the
 * tab state reset per open. The open trigger carries .lumen-icon-host--open
 * → the frame's #D37918 active-trigger ink (Controls.svg variant sheets).
 */
function PanelPopover({
  dataControl,
  label,
  idPrefix,
  initialTab,
  tabs,
  renderTab,
  renderFooter,
  children,
}: {
  dataControl: string;
  label: string;
  idPrefix: string;
  initialTab: string;
  tabs: SettingsPanelTab[];
  renderTab: (tab: string) => ReactNode;
  renderFooter?: (tab: string, close: () => void) => ReactNode;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [activeTab, setActiveTab] = useState(initialTab);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Outside pointer dismiss (the MenuPopup pattern's caller half — the
  // AppMenuButton listener, verbatim). Focus is not stolen: the click target
  // keeps it, so this path deliberately does NOT restore trigger focus.
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

  // Escape and footer activation restore focus to the trigger (e12 F8 — the
  // AppMenuButton close pattern). The footer's close() deliberately does
  // NOT restore focus: the tools dialog takes it.
  const close = (restoreFocus: boolean): void => {
    setOpen(false);
    if (restoreFocus) triggerRef.current?.focus();
  };

  return (
    <div className="lumen-controls__popover" ref={rootRef}>
      <IconButton
        ref={triggerRef}
        data-control={dataControl}
        label={label}
        aria-haspopup="dialog"
        aria-expanded={open}
        className={open ? "lumen-icon-host--open" : undefined}
        onClick={() =>
          setOpen((prev) => {
            if (prev) return false;
            setActiveTab(initialTab);
            return true;
          })
        }
      >
        {children}
      </IconButton>
      {open ? (
        <SettingsPanel
          ariaLabel={label}
          idPrefix={idPrefix}
          tabs={tabs}
          activeTab={activeTab}
          onTabSelect={setActiveTab}
          onRequestClose={() => close(true)}
          footer={renderFooter !== undefined ? renderFooter(activeTab, () => close(false)) : null}
        >
          {renderTab(activeTab)}
        </SettingsPanel>
      ) : null}
    </div>
  );
}

/**
 * The app menu (layout.v1 [[menu]] id="app") — rebuilt w26.f13 to the
 * menu.png grammar (e12 F5): ONE comfortable panel, the frame's 12 rows in
 * pinned order (8 + 48×12 + 1 separator = 585), right-aligned accelerators
 * produced from keymap DATA — the generated keymap already carries every
 * frame combo verbatim (media.open-file Ctrl+O … window.about Shift+F1).
 * Honesty per row: real actions only (the shell open-media / open-customize
 * CustomEvent seams, route navigations, the demo transport's remembered
 * openings through the ONE open funnel, client/open-path.ts); no-seam rows
 * (folder — no folder source exists in any transport; disc; capture device;
 * the profile system) render inert with their frame accelerators — never
 * theater (doc 08 §4.4).
 *
 * w26.f13 Lead ruling: the app menu lives in the ControlBar FOR NOW —
 * f08/AppShell may relocate the trigger into the shell title bar (MenuBar);
 * this content moves with it unchanged (logged in the lane log).
 *
 * w26.k07 (j07 P2-5): memo'd — the entry array + flyout rebuild was the
 * per-tick re-render hotspot while the menu is closed; the only prop
 * (navigate) is identity-stable (NavigationSeam), so a position tick now
 * re-renders neither this button nor its 13 MenuEntry objects.
 */
const AppMenuButton = memo(function AppMenuButton({
  navigate,
}: {
  readonly navigate: (to: string) => void;
}) {
  const m = useMsg();
  const client = useClient();
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const nextRequestId = useRef(0);
  const [open, setOpen] = useState(false);

  // w26.i16 — the profile store feed for the flyout rows (see profile entry).
  const profiles = useMediaProfiles();

  // Outside pointer dismiss (the MenuPopup pattern's caller half — MenuBar
  // carries the same listener). Focus is not stolen: the click target keeps
  // it, so this path deliberately does NOT restore trigger focus.
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

  // Open Recent — real data through the ONE funnel when the transport
  // remembers openings (the demo shell; the open-media dialog's own Recent
  // section reads the same getter); the honest empty reason row otherwise
  // (v2 §4.3's honest-unknown pattern, restated as a menu: discoverable,
  // labeled, inert — never a fabricated history).
  const recents = isDemoTransport(client) ? client.recentOpenings : [];
  const recentSubmenu: MenuEntry<string>[] =
    recents.length === 0
      ? [{ id: "recent-empty", labelKey: m("open.recent.empty"), disabled: true }]
      : recents.map((entry, index): MenuEntry<string> => {
          const source = entry.source;
          return {
            id: `recent-${index}`,
            labelKey: entry.title ?? (source.kind === "file" ? source.path : source.url),
            onSelect: () => {
              nextRequestId.current += 1;
              void client
                .send(openPathRequest(source, nextRequestId.current))
                .catch(() => {
                  /* Transport unreachable — the player surfaces its own
                   * typed errors; the menu still closes and routes (the
                   * dialog's openSource contract). */
                });
              navigate(openPathRoute(source));
            },
          };
        });

  // The frame's 12 rows (menu.png / analysis §A1). Labels are pre-resolved
  // through useMsg (w26.k07: every key is registered in the catalog — the
  // f13/i16 lane seam is gone; dynamic recent titles arrive as strings — the
  // Dropdown identity pattern), so the popup resolves with the identity
  // seam.
  const entries: MenuEntry<string>[] = [
    {
      id: "open-files",
      labelKey: m("player.appmenu.open-files"),
      icon: "open-file",
      accelerator: acceleratorOf("media.open-file"),
      onSelect: () => window.dispatchEvent(new CustomEvent("lumen:open-media")),
    },
    {
      // No folder source exists in any transport (OpenMediaDialog's honest
      // file/stream subset) — inert with its frame accelerator.
      id: "open-folder",
      labelKey: m("hotkeys.open-folder"),
      icon: "open-folder",
      accelerator: acceleratorOf("media.open-folder"),
      disabled: true,
    },
    {
      id: "open-network-stream",
      labelKey: m("hotkeys.open-network-stream"),
      icon: "network-stream",
      accelerator: acceleratorOf("media.open-network-stream"),
      onSelect: () => navigate("/open-network-stream"),
    },
    {
      // No disc seam (the keymap's open-disc uiAction has no surface) — inert.
      id: "open-disc",
      labelKey: m("hotkeys.open-disc"),
      icon: "open-disc",
      accelerator: acceleratorOf("media.open-disc"),
      disabled: true,
    },
    {
      // No capture-device surface — inert.
      id: "open-capture-device",
      labelKey: m("hotkeys.open-capture-device"),
      icon: "capture-device",
      accelerator: acceleratorOf("media.open-capture-device"),
      disabled: true,
    },
    {
      id: "convert",
      labelKey: m("hotkeys.convert"),
      icon: "convert",
      accelerator: acceleratorOf("media.convert"),
      onSelect: () => navigate("/convert"),
    },
    {
      id: "open-recent",
      labelKey: m("player.appmenu.open-recent"),
      icon: "recent",
      submenu: recentSubmenu,
    },
    { id: "sep-open", kind: "separator" },
    {
      // w26.i16 (h18 6.4 NO closed): the Profile flyout's real rows
      // (Profiles.png): the local lumen.profiles.v1 store (frame-seeded
      // Movies/Shows/Music/Work; select + "+ New" quick-create persist).
      // No engine profile command exists — local device data, the
      // playlistsStore precedent (i16's ruling, Lead-landed).
      id: "profile",
      labelKey: m("player.appmenu.profile"),
      icon: "profile",
      iconAccent: true, // menu.svg: person glyph, two-tone orange head dot
      submenu: buildProfileFlyout({
        profiles: profiles.profiles,
        selectedId: profiles.selectedId,
        onSelect: selectProfile,
        onCreate: () => createProfile(m("player.appmenu.profile.auto-name")),
        newLabel: m("player.appmenu.profile.new"),
      }),
    },
    {
      id: "keyboard-shortcuts",
      labelKey: m("player.appmenu.keyboard-shortcuts"),
      icon: "keyboard-shortcuts",
      onSelect: () => navigate("/keyboard-shortcuts"),
    },
    {
      // The customize surface's real trigger — f09's lumen:open-customize
      // seam (see OPEN_CUSTOMIZE_EVENT above).
      id: "customize-player",
      labelKey: m("player.appmenu.customize"),
      icon: "customize-player",
      iconAccent: true, // menu.svg: wand with the orange tip
      onSelect: () => window.dispatchEvent(new CustomEvent(OPEN_CUSTOMIZE_EVENT)),
    },
    {
      id: "settings",
      labelKey: m("menu.view.settings"),
      icon: "settings",
      accelerator: acceleratorOf("window.preferences"),
      onSelect: () => navigate("/settings"),
    },
    {
      // The About panel's real home: settings/global (the settings
      // registry's about→global fold) — no standalone about dialog exists.
      id: "about",
      labelKey: m("player.appmenu.about"),
      icon: "about",
      accelerator: acceleratorOf("window.about"),
      onSelect: () => navigate("/settings/global"),
    },
  ];

  // w26.f13 (e12 F8): Escape and item activation restore focus to the
  // trigger — MenuBar's closeMenu pattern. The outside-pointer path above
  // stays focus-neutral on purpose.
  const close = (restoreFocus: boolean): void => {
    setOpen(false);
    if (restoreFocus) triggerRef.current?.focus();
  };

  return (
    <div className="lumen-controls__menu" ref={rootRef}>
      <IconButton
        ref={triggerRef}
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
          className="lumen-controls__menu-popup"
          ariaLabel={m("player.menu.app")}
          onRequestClose={() => close(true)}
          onDismissAll={() => close(true)}
        />
      ) : null}
    </div>
  );
});

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
