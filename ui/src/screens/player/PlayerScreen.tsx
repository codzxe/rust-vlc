// PlayerScreen: the player screen core — the W26 Player.png composition
// (doc 08 §1.6 anatomy, §4.3 unobtrusive controls, §4.5 a11y; doc 07 §7
// layout-config structure; figma-fidelity-spec §2 "Player" family).
//
// COMPOSITION (Player.png, Player-6, Player - Light — all 7 frames agree):
//   * window title bar + icon rail — the APP SHELL's regions (appshell lane;
//     this screen is hosted inside AppShell's main and never duplicates them)
//   * DOCKED PLAYLIST SIDEBAR, left of the canvas, open by default — the
//     `lumen:toggle-sidebar` window event (exported below) is the seam the
//     shell's title-bar toggle drives; `openPanels: ["playlist"]` also shows it
//   * MAIN CANVAS — near-black in BOTH themes (the picture-frame rule); the
//     honest engine-unwired placeholder (no <video>/<canvas> until 4.H1)
//     centers the playing media's identity; the first-run EMPTY state is the
//     onboarding-style hero from ./states (never a blank void)
//   * BOTTOM CONTROL BAR SLOT — the sibling ControlBar (./controls) renders
//     into the footer region this shell provides.
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

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMsg } from "../../i18n/messages";
import { useClient } from "../../client/ClientProvider";
import { Icon } from "../../components";
import { MediaInfoPanel, MediaInfoProvider } from "../media-info";
import { PlaylistPanel, PlaylistProvider, usePlaylist } from "../playlist";
import { PlayerControlsProvider, usePlayerControls } from "./context";
import { ControlBar, controlCommandOf } from "./controls";
import { CustomizePanels } from "./customize";
import { GotoTimeDialog } from "./dialogs/GotoTimeDialog";
import { BookmarksDialog } from "./dialogs/BookmarksDialog";
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
import { useFullscreen } from "./useFullscreen";

/**
 * Shell seam (W26): the window title bar's sidebar-toggle dispatches this
 * window event; the player answers by collapsing/expanding the docked
 * playlist sidebar (Player.png ↔ Player-6.png). Same CustomEvent contract
 * shape as `lumen:open-media`.
 */
export const PLAYER_TOGGLE_SIDEBAR_EVENT = "lumen:toggle-sidebar";

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

  // W26 Player.png: the docked playlist sidebar is part of the player's
  // default composition — OPEN first-run, collapsible. The shell's
  // sidebar-toggle drives it through the exported window event (see
  // PLAYER_TOGGLE_SIDEBAR_EVENT); `openPanels` forcing it open keeps the
  // old host seam honest (a panel a host asked for never hides).
  const [sidebarOpen, setSidebarOpen] = useState(true);
  useEffect(() => {
    const onToggleSidebar = (): void => setSidebarOpen((open) => !open);
    window.addEventListener(PLAYER_TOGGLE_SIDEBAR_EVENT, onToggleSidebar);
    return () => window.removeEventListener(PLAYER_TOGGLE_SIDEBAR_EVENT, onToggleSidebar);
  }, []);

  const layout = useMemo(
    () => resolveVariantLayout(layoutSource, variant),
    [layoutSource, variant],
  );

  const rootRef = useRef<HTMLDivElement>(null);
  const fullscreen = useFullscreen(rootRef);
  const { isFullscreen, toggle: toggleFullscreen, exit: exitFullscreen } = fullscreen;

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

  // Doc 08 §4.3 [PROPOSAL]: auto-hide on idle in fullscreen; empty state and
  // any open panel/menu lock the chrome visible (§4.5, §5.3).
  const chrome = useChromeAutoHide({
    enabled: isFullscreen,
    locked: !hasMedia || openPanels.length > 0,
  });

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
      {/* Docked playlist sidebar (Player.png) — collapses via the shell's
          sidebar-toggle (lumen:toggle-sidebar) or openPanels omission. */}
      {dockedSidebar !== undefined && sidebarVisible ? (
        <PlaylistSidebar region={dockedSidebar} />
      ) : null}

      {leftRegions.map((region) => (
        <PanelRegion key={region.id} region={region} side="left" />
      ))}

      <div className="lumen-player__main">
        {/* Top regions render only when a layout declares one — the default
            W26 layout does not (the window title bar is the app shell's). */}
        {regionsOf("top").map((region) => (
          <header
            key={region.id}
            data-region-id={region.id}
            data-chrome-part=""
            className="lumen-player__topbar"
          >
            <h1 className="lumen-player__title">{m("screen.player")}</h1>
          </header>
        ))}

        {canvasRegion === undefined ? null : variant === "music" ? (
          /* Music mode (4.F2): the canvas region IS the album-art placeholder
           * (honest — no fabricated artwork; doc 08 §1.4 album art 737²). */
          <MusicMode artRegion={canvasRegion} />
        ) : (
          <div
            data-region-id={canvasRegion.id}
            role="region"
            aria-label={m(canvasRegion.labelKey)}
            className="lumen-player__canvas"
            data-placeholder="engine-wiring-4.H1"
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
          </div>
        )}

        {regionsOf("bottom").map((region) => (
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
        ))}
      </div>

      {rightRegions.map((region) => (
        <PanelRegion key={region.id} region={region} side="right" emphasized={variant === "music"} />
      ))}

      {gotoOpen ? (
        <GotoTimeDialog open onClose={() => setGotoOpen(false)} />
      ) : null}
      {bookmarksOpen ? (
        <BookmarksDialog open onClose={() => setBookmarksOpen(false)} />
      ) : null}
    </div>
  );
}

/** The docked playlist sidebar (Player.png left rail of the canvas):
 * a full-height deep surface with a hairline right edge. Content is the
 * real 4.D1 playlist panel (queue semantics) — the playlists-LIST model
 * (multiple saved playlists, Player.png "Cool Songs"/"Anime to Watch" rows)
 * has no engine surface yet and is NOT fabricated here (see lane log). */
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
          landed with card 4.D2; customize panels landed with 4.F3
          (capability-gated fields); library remains an honest placeholder. */}
      {region.id === "playlist" ? (
        <PlaylistPanel />
      ) : region.id === "media-info" ? (
        <MediaInfoProvider>
          <MediaInfoPanel />
        </MediaInfoProvider>
      ) : region.id === "customize" ? (
        <CustomizePanels />
      ) : (
        <p className="lumen-player__sr-note">{m("player.panel.placeholder")}</p>
      )}
    </aside>
  );
}
