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
//           cluster, each trigger opening its 400px popover UP from the
//           strip (w26.gfix-C Lead ruling: popovers over the full-width
//           strip must open and be reachable; MenuPopup primitive +
//           scrollport cap, submenus flip left). The app menu (w26.f13
//           ruling: it lives in the bar until f08 relocates the trigger to
//           the shell title bar) stays as the cluster's documented extra.
//
// Behavior contract UNCHANGED (state-mapping.ts is the doc 08 §5 mapping —
// play/pause/seek/volume/mute wiring below is a restyle of the same
// dispatch, never a semantic change). New controls are additive seams on
// the same honest pattern: disabled-with-reason, never fabricated state.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { Button, IconButton, Icon, Seekbar, VolumeSlider } from "../../../components";
import { MenuPopup } from "../../../components/menu/MenuPopup";
import { identityResolveLabel } from "../../../components/menu/types";
import type { MenuEntry } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as f13Keys } from "../../../i18n/keys/w26.f13";
import { keys as gfixCKeys } from "../../../i18n/keys/w26.gfixC";
import { useClient } from "../../../client/ClientProvider";
import { isDemoTransport } from "../../../client/demo-transport";
import { openPathRequest, openPathRoute } from "../../../client/open-path";
import { HOTKEY_BINDINGS } from "../../settings/areas/hotkeys/generated/bindings";
import { usePlayerControls } from "../context";
import type { PlayerControls } from "../context";
import { selectedTrackId, trackDisplayName, tracksOfKind } from "../context";
import { usePlaylist } from "../../playlist";
import { GotoTimeDialog, openToolsDialog } from "../dialogs";
import { mapTransportControls, seekDisableReason } from "./state-mapping";
import "./controls.css";

/**
 * w26.f13 lane keys (the app menu's frame-exact labels) — contract §4.5:
 * catalog first, lane copy while the Lead's post-wave merge is pending
 * (PlayerScreen's c6/f09 seam-first pattern; identical before/after).
 */
const f13Catalog: Readonly<Record<string, string>> = f13Keys;

/** w26.f13 lane key ids (typed for the seam resolver below). */
type F13LaneKey = keyof typeof f13Keys;

/** w26.gfixC lane keys (this lane's labels) — same seam-first pattern. */
const gfixCCatalog: Readonly<Record<string, string>> = gfixCKeys;

/** w26.gfixC lane key ids (typed for the seam resolver below). */
type GfixCLaneKey = keyof typeof gfixCKeys;

/** A resolver key: a registered MessageKey or one of this lane's ids. */
type LaneMessageKey = MessageKey | GfixCLaneKey;

/** The lane-key resolver factory (contract §4.5): catalog first, lane copy
 * while unregistered — identical before/after the Lead's merge. */
const laneResolver =
  (m: (key: MessageKey) => string, catalog: Readonly<Record<string, string>>) =>
  (key: LaneMessageKey): string =>
    m(key as MessageKey) ?? catalog[key as string];

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

/**
 * The engine-truth track list for one kind — PlayerScreen's
 * canvasTrackSubmenu logic restated (that helper is not exported; the bar's
 * audio/subtitles popovers and the gear's submenus carry the same honest
 * rows as the canvas menu): no honest track data → the single disabled
 * no-media reason row; else Automatic / Disabled / real engine ids, the
 * engine-reported selection checked (never optimistic).
 */
function trackMenuEntries(
  kind: "video" | "audio" | "subtitle",
  controls: PlayerControls,
  m: (key: MessageKey) => string,
): MenuEntry<string>[] {
  const { state, actions } = controls;
  const tracks = tracksOfKind(state.tracks, kind);
  const selected = selectedTrackId(state.tracks, state.selections, kind);
  const unreported = !state.engineReported && tracks.length === 0;
  const noMedia = state.playbackState === "idle" || state.playbackState === "opening" || unreported;
  if (noMedia) {
    return [{ id: `track-${kind}-no-media`, labelKey: m("player.controls.reason.no-media"), disabled: true }];
  }
  return [
    {
      id: `track-${kind}-auto`,
      labelKey: m("player.track.auto"),
      kind: "radio",
      /* "Automatic" is an action, never the reported selection. */
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
  ];
}

/** previous · play pill · next (Player.svg Frame 8 → play → Frame 10). */
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
        onClick={actions.togglePlayPause}
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
}

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
  const tm = laneResolver(m, gfixCCatalog);
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
            ? `${tm("player.controls.time-current")} (${m(reason)})`
            : tm("player.controls.time-current")
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
  const tm = laneResolver(m, gfixCCatalog);
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
      aria-label={tm(remaining ? "player.controls.time-remaining" : "player.controls.time-duration")}
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
 * the keymap's F binding PlayerScreen wires; cast never had a surface). */
function OutputCluster() {
  const m = useMsg();
  const tm = laneResolver(m, gfixCCatalog);
  const controls = usePlayerControls();
  const playlist = usePlaylist();

  // The gear popover: the frame's tabbed settings popover (Playback / Video
  // / Audio / Subtitles + the Advanced footer) as menu rows — the four
  // families as submenus over the same engine-truth builders the canvas
  // menu carries (shuffle/repeat under Playback; the track lists), plus the
  // frame's 360×32 "Show Advanced Settings" footer wired to the real A/V
  // tools surface (w26.c5).
  const settingsEntries: MenuEntry<string>[] = [
    {
      id: "settings-playback",
      labelKey: m("menu.playback"),
      icon: "equalizer",
      submenu: [
        {
          id: "playback-shuffle",
          labelKey: m("menu.playback.shuffle"),
          kind: "checkbox",
          checked: playlist.shuffle === true,
          onSelect: () => playlist.setShuffle(!(playlist.shuffle === true)),
        },
        ...(["off", "all", "one"] as const).map(
          (mode): MenuEntry<string> => ({
            id: `playback-repeat-${mode}`,
            labelKey: m(`playlist.controls.repeat.${mode}`),
            kind: "radio",
            checked: playlist.repeat === mode,
            onSelect: () => playlist.setRepeat(mode),
          }),
        ),
      ],
    },
    {
      id: "settings-video",
      labelKey: m("player.canvas.menu.video"),
      icon: "pip",
      submenu: trackMenuEntries("video", controls, m),
    },
    {
      id: "settings-audio",
      labelKey: m("player.canvas.menu.audio"),
      icon: "audio",
      submenu: trackMenuEntries("audio", controls, m),
    },
    {
      id: "settings-subtitle",
      labelKey: m("player.canvas.menu.subtitle"),
      icon: "subtitles",
      submenu: trackMenuEntries("subtitle", controls, m),
    },
    { id: "settings-sep-advanced", kind: "separator" },
    {
      id: "settings-advanced",
      labelKey: tm("player.controls.advanced-settings"),
      onSelect: () => openToolsDialog("audio"),
    },
  ];

  // The queue popover: the frame's Queue tab as menu rows — the playlist
  // context's real entries (click plays; the engine-reported current row
  // checked), or the honest empty reason; the docked queue panel itself is
  // the canvas menu's "Playlist" seam (PLAYER_TOGGLE_SIDEBAR_EVENT).
  const queueEmpty = playlist.notExposed || playlist.entries.length === 0;
  const queueEntries: MenuEntry<string>[] = queueEmpty
    ? [
        {
          id: "queue-empty",
          labelKey: tm(
            playlist.notExposed ? "player.controls.reason.unknown" : "player.controls.queue-empty",
          ),
          disabled: true,
        },
      ]
    : playlist.entries.map((entry): MenuEntry<string> => ({
        id: `queue-${entry.id}`,
        labelKey: entry.title ?? m("playlist.entry.unknown-title"),
        kind: "radio",
        checked: playlist.currentId === entry.id,
        onSelect: () => playlist.playItem(entry.id),
      }));

  return (
    <div
      className="lumen-controls__output"
      role="group"
      aria-label={m("player.controls.group-output")}
    >
      <ClusterPopover
        dataControl="audio"
        label={tm("player.controls.audio-tracks")}
        idPrefix="lumen-controls-audio"
        entries={[...trackMenuEntries("audio", controls, m), { id: "audio-sep-advanced", kind: "separator" }, { id: "audio-advanced", labelKey: tm("player.controls.advanced-settings"), onSelect: () => openToolsDialog("audio") }]}
      >
        <Icon name="audio" size={OUTPUT_ICON_SIZE} />
      </ClusterPopover>
      <ClusterPopover
        dataControl="subtitles"
        label={tm("player.controls.subtitle-tracks")}
        idPrefix="lumen-controls-subtitles"
        entries={[...trackMenuEntries("subtitle", controls, m), { id: "subtitles-sep-advanced", kind: "separator" }, { id: "subtitles-advanced", labelKey: tm("player.controls.advanced-settings"), onSelect: () => openToolsDialog("audio") }]}
      >
        <Icon name="subtitles" size={OUTPUT_ICON_SIZE} />
      </ClusterPopover>
      <ClusterPopover
        dataControl="settings"
        label={m("menu.view.settings")}
        idPrefix="lumen-controls-settings"
        entries={settingsEntries}
      >
        <Icon name="settings" size={OUTPUT_ICON_SIZE} />
      </ClusterPopover>
      <ClusterPopover
        dataControl="queue"
        label={tm("player.controls.queue")}
        idPrefix="lumen-controls-queue"
        entries={queueEntries}
      >
        <Icon name="lists" size={OUTPUT_ICON_SIZE} />
      </ClusterPopover>
      <NavigationSeam>
        {(navigate) => <AppMenuButton navigate={navigate} />}
      </NavigationSeam>
    </div>
  );
}

/**
 * Shared right-cluster popover host (w26.gfix-C Lead ruling: popovers over
 * the full-width strip must open and be reachable) — the MenuPopup
 * primitive in the AppMenuButton mount pattern: trigger + 400px comfortable
 * popup (menu.css floor) opening UP from the strip, right-anchored, its
 * height capped with a scrollport (controls.css — the g17 Dropdown fix
 * grammar) so long lists scroll instead of overflowing the window top.
 * The open trigger carries .lumen-icon-host--open → the frame's #D37918
 * active-trigger ink (Controls.svg variant sheets).
 */
function ClusterPopover({
  dataControl,
  label,
  idPrefix,
  entries,
  children,
}: {
  dataControl: string;
  label: string;
  idPrefix: string;
  entries: MenuEntry<string>[];
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
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

  // Escape and item activation restore focus to the trigger (e12 F8 — the
  // AppMenuButton close pattern).
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
        aria-haspopup="menu"
        aria-expanded={open}
        className={open ? "lumen-icon-host--open" : undefined}
        onClick={() => setOpen((prev) => !prev)}
      >
        {children}
      </IconButton>
      {open ? (
        <MenuPopup
          entries={entries}
          resolveLabel={identityResolveLabel}
          idPrefix={idPrefix}
          className="lumen-controls__popover-popup"
          ariaLabel={label}
          onRequestClose={() => close(true)}
          onDismissAll={() => close(true)}
        />
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
 */
function AppMenuButton({ navigate }: { readonly navigate: (to: string) => void }) {
  const m = useMsg();
  const client = useClient();
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const nextRequestId = useRef(0);
  const [open, setOpen] = useState(false);

  // Lane resolver (contract §4.5): catalog first, w26.f13 lane copy while
  // unregistered — identical before/after the Lead's merge.
  const tm = (key: F13LaneKey): string => m(key as MessageKey) ?? f13Catalog[key];

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
  // through useMsg/lane seam (dynamic recent titles arrive as strings — the
  // Dropdown identity pattern), so the popup resolves with the identity
  // seam.
  const entries: MenuEntry<string>[] = [
    {
      id: "open-files",
      labelKey: tm("player.appmenu.open-files"),
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
      labelKey: tm("player.appmenu.open-recent"),
      icon: "recent",
      submenu: recentSubmenu,
    },
    { id: "sep-open", kind: "separator" },
    {
      // No profile engine seam (Profiles.png's Movies/Shows/… would be a
      // fabricated list) — the flyout opens onto the honest unavailable row.
      id: "profile",
      labelKey: tm("player.appmenu.profile"),
      icon: "profile",
      iconAccent: true, // menu.svg: person glyph with the orange head dot
      submenu: [
        {
          id: "profile-none",
          labelKey: m("player.controls.reason.unknown"),
          disabled: true,
        },
      ],
    },
    {
      id: "keyboard-shortcuts",
      labelKey: tm("player.appmenu.keyboard-shortcuts"),
      icon: "keyboard-shortcuts",
      onSelect: () => navigate("/keyboard-shortcuts"),
    },
    {
      // The customize surface's real trigger — f09's lumen:open-customize
      // seam (see OPEN_CUSTOMIZE_EVENT above).
      id: "customize-player",
      labelKey: tm("player.appmenu.customize"),
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
      labelKey: tm("player.appmenu.about"),
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
