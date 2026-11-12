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
//           as an 8×24 r4 accent BAR (never a circular thumb).
//           w26.f13 (e02 F6): the THIRD timecode (remaining) is GONE —
//           Player.svg shows exactly two (00:02:10 | seekbar | 00:25:40);
//           Duration.svg's "−00:23:30" click-toggle face stays unported
//           (logged).
//   RIGHT   total duration · volume icon + the 89px level wedge
//           (volume.svg: green→yellow→orange→red gradient anchored to the
//           FULL track, revealed to the level — the spec §2 "small green
//           level pill") · fullscreen (Fullscreen API on the player host) ·
//           cast (honestly not-available — no surface yet) · app menu —
//           rebuilt w26.f13 to the menu.png grammar (see AppMenuButton) ·
//           gear (A/V tools dialog, w26.c5)
//
// Behavior contract UNCHANGED (state-mapping.ts is the doc 08 §5 mapping —
// play/pause/stop/seek/volume/mute wiring below is a restyle of the
// same dispatch, never a semantic change). New controls are additive seams
// on the same honest pattern: disabled-with-reason, never fabricated state.

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
import { useClient } from "../../../client/ClientProvider";
import { isDemoTransport } from "../../../client/demo-transport";
import { openPathRequest, openPathRoute } from "../../../client/open-path";
import { HOTKEY_BINDINGS } from "../../settings/areas/hotkeys/generated/bindings";
import { usePlayerControls } from "../context";
import { usePlaylist } from "../../playlist";
import { openToolsDialog } from "../dialogs";
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

/** Live document fullscreen truth (never cached — doc 08 §4 honest state). */
const fullscreenActive = (): boolean => (document.fullscreenElement ?? null) !== null;

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
    ? `${m("player.controls.next")} — ${m(
        playlist.notExposed
          ? "player.controls.reason.unknown"
          : "player.controls.reason.no-media",
      )}`
    : m("player.controls.next");

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

/** current timecode · seekbar (doc 08 §5.2/§5.3 honest).
 * w26.f13 (e02 F6): Player.svg shows exactly TWO timecodes — position left
 * of the bar, the total in the right cluster. The pre-f13 mid-bar
 * remaining readout (Duration.svg's "−00:23:30" click face) is dropped. */
function SeekCluster() {
  const { state, actions } = usePlayerControls();
  const m = useMsg();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

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

/** fullscreen · cast · app menu · gear (A/V tools dialog, w26.c5). */
function OutputCluster() {
  const m = useMsg();
  return (
    <div
      className="lumen-controls__output"
      role="group"
      aria-label={m("player.controls.group-output")}
    >
      <FullscreenButton />
      <IconButton
        data-control="cast"
        /* Honest not-available: no cast surface exists in the engine yet —
         * the control renders, disabled with its reason (doc 08 §4.4). */
        label={`${m("player.controls.cast")} — ${m("player.controls.reason.unknown")}`}
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
  const base = active
    ? m("player.controls.exit-fullscreen")
    : m("player.controls.fullscreen");
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
 * The Video/Audio/Subtitle track submenus that lived here since w26.a13
 * moved OUT with this rebuild — menu.png has no track rows; their frame
 * home is the canvas context menu (right-click menu.svg rows, kept by
 * PlayerScreen), which now solely owns them.
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
