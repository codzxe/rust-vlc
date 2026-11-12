// PlayerScreen: the player screen core — the W26 Player.png composition
// (doc 08 §1.6 anatomy, §4.3 unobtrusive controls, §4.5 a11y; doc 07 §7
// layout-config structure; figma-fidelity-spec §2 "Player" family).
//
// COMPOSITION (Player.png, Player-6, Player - Light — all 7 frames agree):
//   * window title bar + icon rail + PLAYLISTS SIDEBAR — the APP SHELL's
//     regions (appshell lane owns them, W26); this screen is hosted inside
//     AppShell's main and never duplicates them. Lead decision (w26.f09,
//     e02 F3): the ONE default-open left sidebar on /player is the player's
//     DOCKED QUEUE (the real queue semantics below); the SHELL's playlists
//     sidebar is default-CLOSED on the player route — the per-route default
//     lives in AppShell's persisted sidebar state (f08 seam, see the
//     PLAYER_TOGGLE_SIDEBAR_EVENT note + the lane log).
//   * DOCKED QUEUE PANEL, left of the canvas, open by default and
//     collapsible — the one sidebar of the default composition. It also
//     opens via `openPanels: ["playlist"]` and the
//     `lumen:toggle-sidebar` window event (exported below).
//   * MAIN CANVAS — near-black in BOTH themes (the picture-frame rule); the
//     honest engine-unwired placeholder (no <video>/<canvas> until 4.H1)
//     centers the playing media's identity; the first-run EMPTY state is the
//     onboarding-style hero from ./states (never a blank void)
//   * BOTTOM CONTROL BAR — a FULL-WIDTH band below the dock + canvas (w26.f09,
//     e02 F2: Player.svg's strip is translate(0,1026) width=1920 — the rail,
//     sidebar and canvas all END ABOVE it; the old canvas-column footer is
//     gone). The sibling ControlBar (./controls) renders into it.
//
// HONEST PLACEHOLDERS still true here:
//   * video canvas — a labelled region div, NOT a <video>/<canvas> element:
//     nothing is wired to the render engine yet; the engine wiring handoff
//     for card 4.H1 is the sr-only note + this comment + ui/NEXT-STEPS.md.
//   * "Open media" affordances dispatch the shell-wide `lumen:open-media`
//     event (the app-shell file dialog, another workstream, listens for it).
//
// Owned here: chrome show/auto-hide/lock, fullscreen + keymap-driven ESC/F/I
// (doc 05 §8.1 semantics from keymap DATA, never hardcoded), layout-config
// consumption, ARIA landmarks. Playback UX overlays live in ./states and
// render over this canvas region via the `engineState` prop / honest fallback.
//
// w26.c6 — CANVAS CONTEXT MENU (right-click menu.svg — 14 rows + 3
// separators, REAL frame labels: Play/Stop/Previous Item/Next Item/Record ·
// Hide UI · Playback/Video/Audio/Subtitle submenus · More Settings/Playlist/
// Open Media submenus · Quit VLC). Right-click on the canvas region opens a
// MenuPopup at the pointer (ControlBar's AppMenuButton mount pattern — the
// ContextMenu wrapper component would add an extra flex child between
// __main and the flex:1 canvas and collapse it, so the popup mounts INSIDE
// the region, position:fixed, zero layout change). Accelerators are display
// strings produced from keymap DATA (HOTKEY_BINDINGS — doc 05 §8.1): the
// frame's "Ctrl + \" spelling. Honest wiring: real commands/routes only;
// Record (no catalog command — keymap D-1 note), Hide UI (no hide surface —
// Player - hide UI.svg state unimplemented) and Quit VLC (window close is
// host-owned) render disabled — never theater.
//
// w26.f09 — CUSTOMIZE TRIGGER (e04 F1: the customize surface was unreachable).
// "Customize interface…" (VLC 4 grammar — the toolbar editor) joins the canvas
// menu after "More Settings", and Ctrl+A opens the same surface. Both fire
// the `lumen:open-customize` window event — the same decoupled CustomEvent
// contract as `lumen:open-media` / `lumen:open-tools`.
//
// w26.gfixA (issue #62) — THE SEAM IS HOSTED: this screen owns the listener
// + the per-open mount of the customize MODAL (customize/CustomizePanels —
// the frame's centered 1048×655 editor, Lead ruling: modal, not a rail).
// The three triggers (canvas-menu row, app-menu row, Ctrl+A) all reach it;
// the open state joins the chrome lock so editing never fights auto-hide.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as c6Keys } from "../../i18n/keys/w26.c6";
import { keys as f09Keys } from "../../i18n/keys/w26.f09";
import { useClient } from "../../client/ClientProvider";
import { Icon } from "../../components";
import { MenuPopup } from "../../components/menu/MenuPopup";
import { identityResolveLabel } from "../../components/menu/types";
import type { MenuEntry } from "../../components/menu";
import { HOTKEY_BINDINGS } from "../settings/areas/hotkeys/generated/bindings";
import { MediaInfoPanel, MediaInfoProvider } from "../media-info";
import { PlaylistPanel, PlaylistProvider, usePlaylist } from "../playlist";
import type { PlaylistContextValue } from "../playlist";
import { PlayerControlsProvider, usePlayerControls } from "./context";
import type { PlayerControls } from "./context";
import { ControlBar, controlCommandOf, mapTransportControls } from "./controls";
import { selectedTrackId, trackDisplayName, tracksOfKind } from "./context";
import { CustomizePanels } from "./customize";
import { GotoTimeDialog } from "./dialogs/GotoTimeDialog";
import { BookmarksDialog } from "./dialogs/BookmarksDialog";
import { ToolsDialogHost, openToolsDialog } from "./dialogs";
import { usePlayerEngineView } from "./states/context-bridge";
import {
  DIALOG_KEYMAP,
  matchBindings,
  matchModifierBindings,
  shellActionFor,
  type KeyBinding,
} from "./keymap";
import {
  evaluateCondition,
  type LayoutConditionContext,
  type LayoutRegion,
} from "./layout";
import { StateRouter, resolveOverlay, type PlayerEngineView } from "./states";
import {
  MiniPlayer,
  MusicMode,
  PLAYER_SHELL_KEYMAP,
  loadPersistedVariant,
  persistVariant,
  resolveVariantLayout,
  type PlayerVariant,
} from "./variants";
import "./player.css";
import { useChromeAutoHide } from "./useChromeAutoHide";
import { createPortal } from "react-dom";
import { useAppShellStrip } from "../../components/appshell/stripContext";
import { useFullscreen } from "./useFullscreen";

/**
 * Shell seam (W26): dispatching this window event toggles the player's
 * docked queue panel (the on-demand left dock). The shell's title-bar
 * sidebar-toggle owns the SHELL playlists sidebar; if it also dispatches
 * this event the two collapses compose (Player.png ↔ Player-6.png).
 * Same CustomEvent contract shape as `lumen:open-media`.
 */
export const PLAYER_TOGGLE_SIDEBAR_EVENT = "lumen:toggle-sidebar";

/**
 * Customize-interface trigger seam (w26.f09, e04 F1) — the decoupled
 * `lumen:open-media` / `lumen:open-tools` CustomEvent contract: the canvas
 * context-menu row "Customize interface…" and the Ctrl+A keymap combo
 * dispatch it; f12's CustomizeDialog (customize/) owns the listener and
 * mounts per-open. Never rename.
 */
export const PLAYER_OPEN_CUSTOMIZE_EVENT = "lumen:open-customize";

// ---------------------------------------------------------------------------
// w26.c6 — canvas context menu (right-click menu.svg)
// ---------------------------------------------------------------------------

/** c6 lane keys for the canvas menu (contract §4.5 — catalog first, lane
 * copy while unregistered; identical before/after the Lead's merge). */
const c6Catalog: Readonly<Record<string, string>> = c6Keys;

/** f09 lane keys (the customize trigger row) — same seam-first contract. */
const f09Catalog: Readonly<Record<string, string>> = f09Keys;

/** Lane keys the canvas menu resolves (c6 frame rows + the f09 trigger). */
type CanvasLaneKey = keyof typeof c6Keys | keyof typeof f09Keys;

/**
 * w26.f09 — the customize trigger's keymap row, straight from keymap DATA:
 * keymaps/default.toml `playlist.advanced-controls` = Ctrl+A (VLC's Advanced
 * Controls combo — that row's own note: "design replaces the VLC Advanced
 * Controls toolbar with the Customize Player panel"). Matched through the
 * shared modifier matcher so the combo is DATA, never a hardcoded string;
 * graduates into keymap.ts's DIALOG_KEYMAP when the keymap lane adopts the
 * row (lane-log handoff).
 */
const CUSTOMIZE_DIALOG_KEYMAP: ReadonlyMap<string, KeyBinding> = new Map(
  HOTKEY_BINDINGS.filter((entry) => entry.id === "playlist.advanced-controls").map(
    (entry): [string, KeyBinding] => [
      entry.id,
      {
        id: entry.id,
        key: entry.key,
        command: entry.command,
        recipe: entry.recipe,
        uiAction: entry.uiAction,
        mode: entry.mode,
        when: entry.when,
      },
    ],
  ),
);

/** True when a keyboard event originates inside a text-editing field — the
 * player's Ctrl+A claim (select all) never fires there. */
function fromEditableField(event: KeyboardEvent): boolean {
  const target = event.target as Element | null;
  if (target === null || typeof target.closest !== "function") return false;
  return target.closest("input, textarea, select, [contenteditable='true']") !== null;
}

/** Binding-id → accelerator display string from keymap DATA (HOTKEY_BINDINGS
 * is the generated keymaps/default.toml excerpt — doc 05 §8.1: menus render
 * the binding's key, never a hardcoded string). Frame spelling "Ctrl + Q"
 * (right-click menu.svg): modifiers re-joined with spaces, plain keys as-is. */
const acceleratorOf = (bindingId: string): string | undefined => {
  const binding = HOTKEY_BINDINGS.find((entry) => entry.id === bindingId);
  if (binding === undefined) return undefined;
  return binding.key
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean)
    .join(" + ");
};

/** The three engine-truth track submenus (Video/Audio/Subtitle) — ControlBar's
 * AppMenuButton pattern verbatim: no honest track data → the submenu opens
 * onto its single disabled no-media reason row (discoverable, labelled,
 * inert — never a fabricated list). */
function canvasTrackSubmenu(
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

/**
 * The canvas context-menu tree (right-click menu.svg — 14 rows, 3
 * separators, pinned order). Labels are pre-resolved through useMsg (the
 * ControlBar identity pattern — dynamic track names arrive as strings), so
 * the popup resolves with the identity seam. Frame icon accents: every row
 * of this frame carries an orange detail (#AB6314/#D37918) → iconAccent.
 */
function canvasMenuEntries({
  m,
  tm,
  controls,
  playlist,
  navigate,
  sidebarOpen,
  toggleSidebar,
}: {
  m: (key: MessageKey) => string;
  tm: (key: CanvasLaneKey) => string;
  controls: PlayerControls;
  playlist: PlaylistContextValue;
  navigate: (to: string) => void;
  sidebarOpen: boolean;
  toggleSidebar: () => void;
}): MenuEntry<string>[] {
  const { state, actions } = controls;
  const mapped = mapTransportControls(state);
  // Prev/next honesty (ControlBar's Next rule): the engine exposes no
  // playlist surface / the queue is empty → disabled, never a dead click.
  const queueEmpty = playlist.notExposed || playlist.entries.length === 0;
  // Repeat radio face is engine truth only — null (not reported) checks
  // nothing (the panel header's honesty rule).
  const repeat = playlist.repeat;
  return [
    {
      id: "play",
      labelKey: m("player.controls.play"),
      icon: "play",
      iconAccent: true,
      accelerator: acceleratorOf("navigation.toggle-play-pause"),
      disabled: mapped.play.disabled,
      onSelect: actions.togglePlayPause,
    },
    {
      id: "stop",
      labelKey: m("player.controls.stop"),
      icon: "stop",
      iconAccent: true,
      accelerator: acceleratorOf("navigation.stop"),
      disabled: mapped.stop.disabled,
      onSelect: actions.stop,
    },
    {
      id: "previous",
      labelKey: tm("player.canvas.menu.previous"),
      icon: "previous",
      iconAccent: true,
      accelerator: acceleratorOf("navigation.prev-track"),
      disabled: queueEmpty,
      onSelect: () => playlist.prev(),
    },
    {
      id: "next",
      labelKey: tm("player.canvas.menu.next"),
      icon: "next",
      iconAccent: true,
      accelerator: acceleratorOf("navigation.next-track"),
      disabled: queueEmpty,
      onSelect: () => playlist.next(),
    },
    {
      // No record command in the v1 catalog (keymaps/default.toml record
      // note: feature-gated) — disabled. Accelerator from DATA (Shift+R —
      // the keymap's D-1 resolution of the frame's "R").
      id: "record",
      labelKey: m("hotkeys.record"),
      icon: "record",
      iconAccent: true,
      accelerator: acceleratorOf("video.record"),
      disabled: true,
    },
    { id: "sep-transport", kind: "separator" },
    {
      // No hide-UI surface exists yet (Player - hide UI.svg is the target
      // state; chrome auto-hide is idle-driven only) — disabled.
      id: "hide-ui",
      labelKey: tm("player.canvas.menu.hide-ui"),
      icon: "hide-ui",
      iconAccent: true,
      accelerator: acceleratorOf("window.hide-ui"),
      disabled: true,
    },
    { id: "sep-hide-ui", kind: "separator" },
    {
      // Frame: sliders glyph (orange) — the registry's equalizer icon.
      id: "playback",
      labelKey: m("menu.playback"),
      icon: "equalizer",
      iconAccent: true,
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
            checked: repeat === mode,
            onSelect: () => playlist.setRepeat(mode),
          }),
        ),
      ],
    },
    {
      id: "video",
      labelKey: tm("player.canvas.menu.video"),
      icon: "pip",
      iconAccent: true,
      submenu: canvasTrackSubmenu("video", controls, m),
    },
    {
      id: "audio",
      labelKey: tm("player.canvas.menu.audio"),
      icon: "audio",
      iconAccent: true,
      submenu: canvasTrackSubmenu("audio", controls, m),
    },
    {
      id: "subtitle",
      labelKey: tm("player.canvas.menu.subtitle"),
      icon: "subtitles",
      iconAccent: true,
      submenu: canvasTrackSubmenu("subtitle", controls, m),
    },
    { id: "sep-tools", kind: "separator" },
    {
      id: "more-settings",
      labelKey: tm("player.canvas.menu.more-settings"),
      icon: "settings",
      iconAccent: true,
      submenu: [
        {
          id: "more-settings-open",
          labelKey: m("menu.view.settings"),
          onSelect: () => navigate("/settings"),
        },
      ],
    },
    {
      // w26.f09 — the customize surface's real trigger (e04 F1: unreachable
      // before). VLC 4's "Customize Interface…" grammar; fires the
      // lumen:open-customize seam f12's CustomizeDialog listens for. The
      // accelerator is keymap DATA (playlist.advanced-controls = Ctrl+A,
      // that row's adapted-to-customize note). Frame: right-click menu.svg
      // pins 14 rows — this 15th row is the Lead-approved trigger addition.
      id: "customize-interface",
      labelKey: tm("player.canvas.menu.customize-interface"),
      icon: "customize-player",
      iconAccent: true,
      accelerator: acceleratorOf("playlist.advanced-controls"),
      onSelect: () => window.dispatchEvent(new CustomEvent(PLAYER_OPEN_CUSTOMIZE_EVENT)),
    },
    {
      id: "playlist",
      labelKey: m("menu.view.playlist"),
      icon: "playlist",
      iconAccent: true,
      submenu: [
        // The docked queue panel's real toggle seam (this screen's own
        // PLAYER_TOGGLE_SIDEBAR_EVENT state) + the library route.
        {
          id: "playlist-toggle-dock",
          labelKey: sidebarOpen ? tm("player.canvas.menu.hide-queue") : tm("player.canvas.menu.show-queue"),
          onSelect: toggleSidebar,
        },
        {
          id: "playlist-open",
          labelKey: tm("player.canvas.menu.open-playlist"),
          onSelect: () => navigate("/playlist"),
        },
      ],
    },
    {
      id: "open-media",
      labelKey: tm("player.canvas.menu.open-media"),
      icon: "open-folder",
      iconAccent: true,
      submenu: [
        {
          id: "open-media-file",
          labelKey: m("menu.file.open-media"),
          onSelect: () => window.dispatchEvent(new CustomEvent("lumen:open-media")),
        },
        {
          id: "open-media-network",
          labelKey: m("menu.file.open-network-stream"),
          onSelect: () => navigate("/open-network-stream"),
        },
      ],
    },
    {
      // Window-level close is host-owned (the MiniPlayer close note) — no
      // quit seam exists in the web app — disabled.
      id: "quit",
      labelKey: tm("player.canvas.menu.quit"),
      icon: "quit",
      iconAccent: true,
      accelerator: acceleratorOf("media.quit"),
      disabled: true,
    },
  ];
}

/**
 * Router-presence seam for the canvas menu's route entries (ControlBar's
 * NavigationSeam pattern): in-app navigation inside the Router; bare mounts
 * (jsdom trees without a Router, where useNavigate throws) fall back to
 * document navigation. Split keeps hook calls unconditional.
 */
function NavigationSeam({ children }: { children: (navigate: (to: string) => void) => ReactNode }) {
  return useInRouterContext() ? (
    <RoutedNavigation>{children}</RoutedNavigation>
  ) : (
    <>{children((to) => window.location.assign(to))}</>
  );
}

function RoutedNavigation({ children }: { children: (navigate: (to: string) => void) => ReactNode }) {
  const navigate = useNavigate();
  return <>{children((to) => navigate(to))}</>;
}

/** Playback context for layout condition tokens; null = first-run empty state. */
export interface PlayerMediaContext {
  readonly hasVideo: boolean;
  readonly hasChapters: boolean;
}

export interface PlayerScreenProps {
  /**
   * Layout config source (layout.v1 TOML, doc 07 §7.2). Omitted/empty → the
   * [PROPOSAL] default layout. A present-but-invalid source throws
   * (LayoutParseError) — never a silent fallback.
   */
  readonly layoutSource?: string;
  /** Media presence + traits; absent → empty state (has-media = false). */
  readonly media?: PlayerMediaContext | null;
  /** Open panel ids (layout region ids). Any open panel locks chrome visible. */
  readonly openPanels?: readonly string[];
  /**
   * Wiring seam for bindings the shell does not own — card 4.C2's command
   * dispatch attaches here (e.g. exit-fullscreen's dialog-close half).
   */
  readonly onBinding?: (binding: KeyBinding) => void;
  /**
   * Engine-truth playback view for the 4.C3 state overlays. Absent → no
   * engine connected yet: first-run Empty when no media, nothing otherwise
   * (never a fabricated state — doc 05 §1 C4).
   */
  readonly engineState?: PlayerEngineView;
  /** Retry / open-media affordances for the 4.C3 overlays (C2 wires real
   * commands; unwired affordances render disabled, never fake success). */
  readonly onRetry?: () => void;
  readonly onOpenMedia?: () => void;
  /**
   * Player variant override (card 4.F2 — controlled mode). Absent → the
   * internal persisted variant state ("full" first run). Keymap-driven
   * toggles update it and call onVariantChange.
   */
  readonly variant?: PlayerVariant;
  /** Variant-change seam (card 4.F2): fires on keymap-driven switches and
   * the miniplayer expand/close affordances. */
  readonly onVariantChange?: (variant: PlayerVariant) => void;
}

export function PlayerScreen(props: PlayerScreenProps) {
  // Shared seams: the player state context (card 4.C2 context.tsx) and the
  // playlist context (card 4.D1) are mounted ONCE around the whole player
  // composition — the docked sidebar, the music-mode panel and the canvas
  // now-playing block all consume the same provider instances. Transports
  // come from ClientProvider (hosts/tests wrap with it, matching main.tsx).
  return (
    <PlayerControlsProvider>
      <PlaylistProvider>
        <PlayerScreenInner {...props} />
      </PlaylistProvider>
    </PlayerControlsProvider>
  );
}

function PlayerScreenInner({
  layoutSource,
  media = null,
  openPanels = [],
  onBinding,
  engineState,
  onRetry,
  onOpenMedia,
  variant: variantProp,
  onVariantChange,
}: PlayerScreenProps) {
  const m = useMsg();

  // Card 4.F2 — player variant (UI-owned state per doc 08 §4, persisted via
  // the same localStorage pattern as the settings seam). Controlled prop
  // wins; otherwise the internal state starts from the persisted value.
  const [variantState, setVariantState] = useState<PlayerVariant>(
    () => loadPersistedVariant() ?? "full",
  );
  const variant = variantProp ?? variantState;
  const setVariant = useCallback(
    (next: PlayerVariant) => {
      setVariantState(next);
      persistVariant(next);
      onVariantChange?.(next);
    },
    [onVariantChange],
  );

  // w26.f09 (e02 F3, Lead decision): the docked queue panel is THE one
  // default-open left sidebar on /player — OPEN first-run, collapsible
  // (variants.test.tsx asserts it present in the full variant; this screen
  // owns the toggle seam). Player.png shows exactly one sidebar; the SHELL's
  // playlists sidebar composes default-CLOSED on this route — that per-route
  // default lives in AppShell's persisted sidebar state (f08 seam: route-
  // aware loadSidebarOpen default, noted in the w26.f09 lane log). The
  // shell's title-bar toggle still drives this dock through the exported
  // window event (PLAYER_TOGGLE_SIDEBAR_EVENT), and `openPanels` forcing it
  // open keeps the host seam honest (a host-requested panel never hides).
  const [sidebarOpen, setSidebarOpen] = useState(true);
  useEffect(() => {
    const onToggleSidebar = (): void => setSidebarOpen((open) => !open);
    window.addEventListener(PLAYER_TOGGLE_SIDEBAR_EVENT, onToggleSidebar);
    return () => window.removeEventListener(PLAYER_TOGGLE_SIDEBAR_EVENT, onToggleSidebar);
  }, []);

  // w26.gfixA (issue #62, g07/g08/g09-P0) — the customize seam's LISTENER
  // half: every `lumen:open-customize` dispatch (canvas-menu row, app-menu
  // "Customize Player" row, Ctrl+A — the keymap handler above) opens the
  // customize modal for a per-open mount (fresh arrangement state from
  // persistence, the ToolsDialogHost pattern). The open state also feeds
  // the chrome lock below — an open editor never fights auto-hide.
  const [customizeOpen, setCustomizeOpen] = useState(false);
  useEffect(() => {
    const onOpenCustomize = (): void => setCustomizeOpen(true);
    window.addEventListener(PLAYER_OPEN_CUSTOMIZE_EVENT, onOpenCustomize);
    return () => window.removeEventListener(PLAYER_OPEN_CUSTOMIZE_EVENT, onOpenCustomize);
  }, []);

  const layout = useMemo(
    () => resolveVariantLayout(layoutSource, variant),
    [layoutSource, variant],
  );

  const rootRef = useRef<HTMLDivElement>(null);
  const fullscreen = useFullscreen(rootRef);
  const { isFullscreen, toggle: toggleFullscreen, exit: exitFullscreen } = fullscreen;

  // w26.c6 — canvas context menu state (right-click menu.svg). Open state
  // + pointer coordinates + the previously-focused element (Escape restores
  // focus — the ContextMenu component's contract, reproduced for the direct
  // MenuPopup mount; see the header note for why the wrapper component is
  // not used on the flex:1 canvas).
  const [canvasMenu, setCanvasMenu] = useState<{ x: number; y: number } | null>(null);
  const canvasMenuFocus = useRef<HTMLElement | null>(null);
  const closeCanvasMenu = useCallback((): void => {
    setCanvasMenu(null);
    canvasMenuFocus.current?.focus();
  }, []);
  // Outside pointer dismiss (scrim): any pointerdown outside an open menu
  // panel closes it. Focus is not stolen (the click target keeps it).
  useEffect(() => {
    if (canvasMenu === null) return;
    const onPointerDown = (event: PointerEvent): void => {
      const target = event.target as Element | null;
      if (target !== null && typeof target.closest === "function" && target.closest('[role="menu"]') !== null) {
        return;
      }
      setCanvasMenu(null);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [canvasMenu]);

  // Issue #32 — player dialogs (Go-to-Time, Bookmarks). Each dialog is
  // mounted per-open so its form state starts fresh (the goto input prefills
  // with the position at open time; the bookmarks list re-fetches).
  const [gotoOpen, setGotoOpen] = useState(false);
  const [bookmarksOpen, setBookmarksOpen] = useState(false);

  // Hosts MAY inject explicit `media` / `engineState` props, but the default
  // composition (browser shell, /player route) owns no such objects — the
  // screen then derives BOTH from the live seams it already mounts: the 4.C3
  // bridge view (pure engine events) for the overlays, and the controls
  // state (tracks) for media presence. Nothing is fabricated: idle stays
  // idle until the engine's session.state-changed events say otherwise.
  const client = useClient();
  const bridgeView = usePlayerEngineView(client);
  const controls = usePlayerControls();
  // w26.c6 — the canvas context menu's queue rows (Previous/Next Item +
  // Playback shuffle/repeat) ride the same playlist context the docked
  // panel consumes (the provider is mounted around this whole composition).
  const playlist = usePlaylist();
  const effectiveEngineState: PlayerEngineView = engineState ?? bridgeView;
  const derivedMedia: PlayerMediaContext | null =
    media !== null
      ? media
      : effectiveEngineState.state === "idle"
        ? null
        : {
            // Track kinds are engine truth (tracks.list / tracks.list-ready).
            hasVideo: controls.state.tracks.some((track) => track.kind === "video"),
            // The seam carries no chapter truth — conservative false; hosts
            // that need it pass the full PlayerMediaContext.
            hasChapters: false,
          };
  const hasMedia = derivedMedia !== null;

  // w26.g03 P1-2 (Lead): the shell strip slot — when mounted inside the
  // AppShell, the footer region portals there (full-window control band).
  const stripHost = useAppShellStrip();

  // Doc 08 §4.3 [PROPOSAL]: auto-hide on idle in fullscreen; empty state and
  // any open panel/menu lock the chrome visible (§4.5, §5.3).
  // w26.f10 cleanup (the hook's compensation note): the lock is the
  // panels/menus lock ONLY (doc 08 §4.5) — the retired empty-state gate is
  // gone; the fade arms windowed and with no media (Player - hide UI.svg).
  const chrome = useChromeAutoHide({
    enabled: isFullscreen,
    // w26.gfixA: an open customize modal locks the chrome too — the edit
    // band + modal must stay visible while the user rearranges.
    locked: openPanels.length > 0 || customizeOpen,
  });

  // w26.g06 P1-1 Lead fix: mirror the hide-UI state onto <html> so the SHELL
  // chrome (title bar, rail, sidebar — outside .lumen-player) fades together
  // with the player's own parts. "Player - hide UI.svg" is a zero-chrome
  // windowed state; leaving shell chrome lit broke the frame contract.
  useEffect(() => {
    const root = document.documentElement;
    if (chrome.visibility === "hidden") root.setAttribute("data-lumen-chrome", "hidden");
    else root.removeAttribute("data-lumen-chrome");
    return () => root.removeAttribute("data-lumen-chrome");
  }, [chrome.visibility]);

  // Control-bar commands (4.C2) via the shared state seam — keymap DATA
  // drives which bindings reach here (see controls/state-mapping.ts).
  const { actions } = controls;

  // Keymap-data-driven shortcuts (doc 05 §8.1): the shell listens for
  // bindings it owns (fullscreen-toggle, exit-fullscreen,
  // show-controls-fullscreen per keymaps/default.toml) and forwards the rest
  // to the 4.C2 seam. Native ESC exit is handled by the browser.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // Dialog-opening modifier combos (issue #32): the DIALOG_KEYMAP seed
      // (verbatim keymaps/default.toml rows). `when` clauses are evaluated
      // here against the screen's honest media truth — jump-to-time is
      // has-media-gated, so an idle session ignores the combo entirely.
      for (const binding of matchModifierBindings(DIALOG_KEYMAP, event)) {
        if (binding.uiAction === "goto-time" && hasMedia) {
          event.preventDefault();
          setGotoOpen(true);
        } else if (binding.uiAction === "bookmarks") {
          // keymaps/default.toml carries no `when` for the bookmarks dialog
          // (VLC parity — the dialog opens on an idle session too and shows
          // the honest empty state).
          event.preventDefault();
          setBookmarksOpen(true);
        } else if (binding.uiAction === "av-effects") {
          // w26.c5 handoff: VLC's Ctrl+E "Extended settings" (Adjustments
          // and Effects) = our audio/video Tools dialog (audio tab first —
          // the VLC dialog's front page). No `when` in the keymap row: the
          // tools render honestly (staged notes) on an idle session too.
          event.preventDefault();
          openToolsDialog("audio");
        }
      }
      for (const binding of matchModifierBindings(CUSTOMIZE_DIALOG_KEYMAP, event)) {
        // w26.f09 — Ctrl+A → Customize interface (keymap DATA row above).
        // No `when` gate: the customize surface renders its own honest
        // empty/capability states. Ctrl+A is also "select all" — the player
        // never claims it inside a text field (dialogs stay text-editor-safe).
        if (binding.uiAction === "advanced-controls" && !fromEditableField(event)) {
          event.preventDefault();
          window.dispatchEvent(new CustomEvent(PLAYER_OPEN_CUSTOMIZE_EVENT));
        }
      }
      for (const binding of matchBindings(PLAYER_SHELL_KEYMAP, event)) {
        const action = shellActionFor(binding);
        switch (action.kind) {
          case "toggle-fullscreen":
            void toggleFullscreen();
            break;
          case "toggle-miniplayer":
            // Keymap-data-driven variant switch (4.F2 — recipe seed in
            // variants/variantKeymap.ts [PROPOSAL]).
            setVariant(variant === "miniplayer" ? "full" : "miniplayer");
            break;
          case "toggle-music-mode":
            setVariant(variant === "music" ? "full" : "music");
            break;
          case "exit-fullscreen":
            // "exit-fullscreen-close-dialog": exit when fullscreen; the
            // dialog-close half is 4.C2's overlay handling via the seam.
            if ((document.fullscreenElement ?? null) !== null) {
              void exitFullscreen();
            } else {
              onBinding?.(binding);
            }
            break;
          case "show-controls":
            chrome.show();
            break;
          case "forward": {
            const fwd = action.binding;
            const command = controlCommandOf(fwd);
            if (command === "playback.play") actions.play();
            else if (command === "playback.pause") actions.pause();
            else if (command === "playback.toggle-play-pause") actions.togglePlayPause();
            else if (command === "playback.stop") actions.stop();
            else onBinding?.(fwd);
            break;
          }
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [toggleFullscreen, exitFullscreen, chrome, onBinding, actions, setVariant, variant, hasMedia]);

  const conditionCtx: LayoutConditionContext = {
    hasMedia,
    hasVideo: media?.hasVideo ?? false,
    fullscreen: isFullscreen,
    hasChapters: media?.hasChapters ?? false,
    miniplayer: variant === "miniplayer",
    musicMode: variant === "music",
  };
  const visible = (region: LayoutRegion): boolean =>
    evaluateCondition(region.visibleWhen, conditionCtx);
  const regionsOf = (anchor: LayoutRegion["anchor"]): readonly LayoutRegion[] =>
    layout.regions.filter((region) => region.anchor === anchor && visible(region));
  const canvasRegion = regionsOf("center")[0];

  const sidePanel = (region: LayoutRegion): boolean => openPanels.includes(region.id);
  // Left regions: the playlist is the DOCKED SIDEBAR (default-open, W26);
  // any other left region (custom layouts) keeps the openPanels gate.
  const dockedSidebar = regionsOf("left").find((region) => region.id === "playlist");
  const leftRegions = regionsOf("left").filter(
    (region) => region.id !== "playlist" && sidePanel(region),
  );
  // Music mode (4.F2): the playlist panel is LAYOUT-DRIVEN prominent — every
  // visible right-anchor region renders without the openPanels gate
  // (design matrix in variants/variantLayout.ts).
  const rightRegions =
    variant === "music"
      ? regionsOf("right")
      : regionsOf("right").filter(sidePanel);
  const sidebarVisible = sidebarOpen || openPanels.includes("playlist");

  // Miniplayer (4.F2): the whole surface is the compact card — no top bar,
  // no side panels, no full control bar (design: Miniplayer.svg default).
  if (variant === "miniplayer") {
    const thumb = regionsOf("center")[0];
    const controlsRegion = regionsOf("bottom")[0];
    return (
      <div
        ref={rootRef}
        className="lumen-player"
        data-screen="player"
        data-variant="miniplayer"
        data-chrome={chrome.visibility}
        data-fullscreen={String(isFullscreen)}
        data-has-media={hasMedia}
      >
        {thumb === undefined || controlsRegion === undefined ? null : (
          <MiniPlayer
            thumbnailRegion={thumb}
            controlsRegion={controlsRegion}
            onExpand={() => setVariant("full")}
            onClose={() => {
              // [PROPOSAL]: window-level close is host-owned; the honest
              // in-app close stops playback (real command) and restores the
              // full variant.
              actions.stop();
              setVariant("full");
            }}
          />
        )}
        {gotoOpen ? (
          <GotoTimeDialog open onClose={() => setGotoOpen(false)} />
        ) : null}
        {bookmarksOpen ? (
          <BookmarksDialog open onClose={() => setBookmarksOpen(false)} />
        ) : null}
        {/* w26.gfixA — the customize modal (per-open mount; see the seam
            comment at the listener). */}
        {customizeOpen ? (
          <CustomizePanels onClose={() => setCustomizeOpen(false)} />
        ) : null}
      </div>
    );
  }

  return (
    <div
      ref={rootRef}
      className="lumen-player"
      data-screen="player"
      data-variant={variant}
      data-chrome={chrome.visibility}
      data-fullscreen={String(isFullscreen)}
      data-has-media={hasMedia}
      data-sidebar={sidebarVisible ? "open" : "closed"}
    >
      {/* w26.f09 (e02 F2) — BODY ROW: docked queue + canvas column + the
          floating tool panels. Player.svg: the rail, sidebar and canvas all
          END ABOVE the control strip, so the strip is a full-width sibling
          BELOW this row — never a column inside the canvas region. */}
      <div className="lumen-player__body">
        {/* Docked queue panel (the ONE default-open sidebar on /player — Lead
            decision w26.f09/e02 F3; the SHELL's playlists sidebar is
            default-closed on this route, f08 seam). Collapses via the shell
            toggle event (lumen:toggle-sidebar) or openPanels omission. */}
        {dockedSidebar !== undefined && sidebarVisible ? (
          <PlaylistSidebar region={dockedSidebar} />
        ) : null}

        {leftRegions.map((region) => (
          <PanelRegion key={region.id} region={region} side="left" />
        ))}

        <div className="lumen-player__main">
          {/* Level-1 heading is ALWAYS present for a11y (App gate tests +
              screen readers); visually hidden — the window title bar is the
              app shell's (W26). */}
          <h1 className="lumen-player__sr-title">{m("screen.player")}</h1>
          {/* Top regions render only when a layout declares one — the default
              W26 layout does not (the window title bar is the app shell's). */}
          {regionsOf("top").map((region) => (
            <header
              key={region.id}
              data-region-id={region.id}
              data-chrome-part=""
              className="lumen-player__topbar"
            >
              <span className="lumen-player__title">{m("screen.player")}</span>
            </header>
          ))}

          {canvasRegion === undefined ? null : variant === "music" ? (
            /* Music mode (4.F2): the canvas region IS the album-art placeholder
             * (honest — no fabricated artwork; doc 08 §1.4 album art 737²). */
            <MusicMode artRegion={canvasRegion} />
          ) : (
            <NavigationSeam>
              {(navigate) => (
                <div
                  data-region-id={canvasRegion.id}
                  role="region"
                  aria-label={m(canvasRegion.labelKey)}
                  className="lumen-player__canvas"
                  data-placeholder="engine-wiring-4.H1"
                  onContextMenu={(event) => {
                    // w26.c6 — the canvas context menu (right-click menu.svg)
                    // opens at the pointer; suppress the native browser menu.
                    event.preventDefault();
                    canvasMenuFocus.current = document.activeElement as HTMLElement | null;
                    setCanvasMenu({ x: event.clientX, y: event.clientY });
                  }}
                >
                  {/* Honest placeholder: no <video>/<canvas> element until the
                      render engine is wired (card 4.H1 handoff — recorded in
                      ui/NEXT-STEPS.md). Screen-reader note says so. */}
                  <p className="lumen-player__sr-note">{m("player.canvas.placeholder")}</p>

                  {/* Player.png "artwork centered": until the engine paints frames
                      (4.H1), the honest stand-in is the playing media's IDENTITY —
                      engine-reported title first (media.info-ready), the playlist
                      entry's title as fallback, a quiet kind mark when neither
                      reported. Real data only; renders only while an overlay-free
                      session state is up (loading/buffering/error take over). */}
                  {hasMedia && resolveOverlay(effectiveEngineState) === "none" ? (
                    <NowPlayingMark hasVideo={derivedMedia?.hasVideo ?? false} />
                  ) : null}

                  {/* Card 4.C3 playback UX states: exactly zero or one overlay on
                      the canvas region, driven by the engine view. The first-run
                      Empty surface only ever appears when NO media is present —
                      a host-injected `media` context (or a live session) suppresses
                      it, while engine-truth overlays (loading/buffering/error)
                      always render. Never a fabricated state. */}
                  {effectiveEngineState.state === "idle" && hasMedia ? null : (
                    <StateRouter
                      view={effectiveEngineState}
                      onRetry={onRetry}
                      onOpenMedia={onOpenMedia}
                    />
                  )}

                  {/* w26.c6 — the canvas context menu (right-click menu.svg),
                      fixed at the pointer; the menu API owns the keyboard model
                      (roving focus, arrows, Escape → closeAndRestore). */}
                  {canvasMenu !== null && (
                    <MenuPopup
                      entries={canvasMenuEntries({
                        m,
                        tm: (key) => m(key as MessageKey) ?? c6Catalog[key] ?? f09Catalog[key],
                        controls,
                        playlist,
                        navigate,
                        sidebarOpen,
                        toggleSidebar: () => setSidebarOpen((open) => !open),
                      })}
                      resolveLabel={identityResolveLabel}
                      idPrefix="lumen-player-canvas-menu"
                      className="lumen-context-menu__popup"
                      style={{ left: canvasMenu.x, top: canvasMenu.y }}
                      ariaLabel={m("player.canvas.menu.title" as MessageKey) ?? c6Catalog["player.canvas.menu.title"]}
                      onRequestClose={closeCanvasMenu}
                      onDismissAll={closeCanvasMenu}
                    />
                  )}
                </div>
              )}
            </NavigationSeam>
          )}
        </div>

        {/* Floating right tool panels (media-info, customize; custom
         * layouts) — anchored to the BODY row so they float over the
         * canvas and end above the control band. */}
        {rightRegions.map((region) => (
          <PanelRegion key={region.id} region={region} side="right" emphasized={variant === "music"} />
        ))}
      </div>

      {/* w26.f09 (e02 F2) — the FULL-WIDTH control band: Player.svg's strip
          (translate(0,1026), width=1920) spans the window BELOW rail,
          sidebar and canvas; inside the shell it spans the player root below
          the body row (dock + canvas + panels). The shell's rail hoist to a
          true window-wide band is the appshell seam (f08, lane log). */}
      {regionsOf("bottom").map((region) => {
        /* w26.g03 P1-2 (Lead): the band spans the FULL window under rail +
         * sidebar (Player.svg strip 0..1920) — portal into the shell strip
         * slot when mounted inside the shell; isolated hosts (tests) keep
         * the in-place render unchanged. */
        const node = (
          <footer
            key={region.id}
            data-region-id={region.id}
            data-chrome-part=""
            data-variant={variant === "music" ? "music" : undefined}
            role="region"
            aria-label={m(region.labelKey)}
            className="lumen-player__controls"
          >
            {/* Card 4.C2 — the real control bar: transport (doc 08 §5.1
                mapping), seekbar (§5.2 truthful pending), volume/mute, track
                menus — all wired to the command-layer client seam. */}
            <ControlBar />
          </footer>
        );
        return stripHost !== null ? createPortal(node, stripHost) : node;
      })}

      {gotoOpen ? (
        <GotoTimeDialog open onClose={() => setGotoOpen(false)} />
      ) : null}
      {bookmarksOpen ? (
        <BookmarksDialog open onClose={() => setBookmarksOpen(false)} />
      ) : null}
      {/* w26.c5: audio/video tools dialog — opens via openToolsDialog() /
          the `lumen:open-tools` event seam (gear menu, keymap). */}
      <ToolsDialogHost />
      {/* w26.gfixA (issue #62) — the customize modal: the centered
          1048×655 editor over a scrim + the control band's dashed edit
          wells. All three triggers reach it through the
          `lumen:open-customize` seam listener above. */}
      {customizeOpen ? (
        <CustomizePanels onClose={() => setCustomizeOpen(false)} />
      ) : null}
    </div>
  );
}

/** The docked queue panel: the player's one default-open sidebar (w26.f09) —
 * a deep surface with a hairline right edge, ending ABOVE the control band
 * (Player.svg: sidebar y 40..1026, strip 1026..1080). Content is the real
 * 4.D1 playlist panel (queue semantics); the shell's playlists-LIST rail
 * (Player.png "Cool Songs"/"Anime to Watch" rows) stays available through
 * the shell's own toggle — default-closed on this route (f08 seam). */
function PlaylistSidebar({ region }: { region: LayoutRegion }) {
  const m = useMsg();
  return (
    <aside
      data-region-id={region.id}
      data-chrome-part=""
      role="complementary"
      aria-label={m(region.labelKey)}
      className="lumen-player__sidebar"
    >
      <PlaylistPanel />
    </aside>
  );
}

/** The honest "artwork centered" stand-in for media-present sessions (see
 * the canvas region comment). Engine truth first, playlist truth second. */
function NowPlayingMark({ hasVideo }: { hasVideo: boolean }) {
  const { state } = usePlayerControls();
  const playlist = usePlaylist();
  const entryTitle =
    playlist.entries.find((entry) => entry.id === playlist.currentId)?.title ?? null;
  const title = state.mediaTitle ?? entryTitle;
  return (
    <div className="lumen-player__now" data-media-kind={hasVideo ? "video" : "audio"}>
      <span className="lumen-player__now-glyph" aria-hidden="true">
        <Icon name={hasVideo ? "play" : "volume-medium"} size={24} />
      </span>
      {title === null ? null : <h2 className="lumen-player__now-title">{title}</h2>}
    </div>
  );
}

/** An honestly-labelled side-panel container from layout config (custom
 * layouts + the right-side tool panels: media-info, customize). Music-mode
 * emphasis (4.F2) is a styling hook in player.css. The playlist context is
 * mounted by PlayerScreen around the whole composition; the panels consume
 * it (no per-panel provider nesting). */
function PanelRegion({
  region,
  side,
  emphasized = false,
}: {
  region: LayoutRegion;
  side: "left" | "right";
  /** Music-mode playlist emphasis (4.F2) — styling hook only. */
  emphasized?: boolean;
}) {
  const m = useMsg();
  return (
    <aside
      data-region-id={region.id}
      data-chrome-part=""
      data-emphasized={emphasized || undefined}
      role="complementary"
      aria-label={m(region.labelKey)}
      className={`lumen-player__panel lumen-player__panel--${side}`}
    >
      {/* Playlist region content landed with card 4.D1 (panel + queue
          semantics, empty-state Open-media CTA); media-info region content
          landed with card 4.D2. The customize region renders NO rail panel
          since w26.gfixA — the Lead ruling pins the surface as the CENTERED
          MODAL this screen hosts per-open (the `lumen:open-customize` seam
          above); a custom-layout host that opens the region id explicitly
          gets the honest placeholder, never a second surface. Library
          remains an honest placeholder. */}
      {region.id === "playlist" ? (
        <PlaylistPanel />
      ) : region.id === "media-info" ? (
        <MediaInfoProvider>
          <MediaInfoPanel />
        </MediaInfoProvider>
      ) : (
        <p className="lumen-player__sr-note">{m("player.panel.placeholder")}</p>
      )}
    </aside>
  );
}
