// Card 4.C1 — PlayerScreen: the player screen shell (doc 08 §1.6 anatomy,
// §4.3 unobtrusive controls, §4.5 a11y; doc 07 §7 layout-config structure).
//
// HONEST PLACEHOLDERS in this shell (each labeled in code + i18n copy):
//   * video canvas — a labelled region div, NOT a <video>/<canvas> element:
//     nothing is wired to the render engine yet; the engine wiring handoff
//     for card 4.H1 is the sr-only note + this comment + ui/NEXT-STEPS.md.
//   * side panels + controls-bar region — empty containers from layout
//     config; CONTENT is 4.C2 (transport controls) / 4.F3 (library,
//     playlist) / later cards' work.
//   * "Open media" affordance — disabled until 4.C2 wires session.open +
//     the file-dialog ui-action (doc 07 §7.2 ui-actions registry).
//
// Owned here: chrome show/auto-hide/lock, fullscreen + keymap-driven ESC/F/I
// (doc 05 §8.1 semantics from keymap DATA, never hardcoded), layout-config
// consumption, ARIA landmarks. The first-run empty state + playback UX
// overlays moved to ./states (card 4.C3) — the shell renders them on the
// canvas region via the `engineState` prop / honest fallback.

import { useEffect, useMemo, useRef } from "react";
import { useMsg } from "../../i18n/messages";
import { PlayerControlsProvider, usePlayerControls } from "./context";
import { ControlBar, controlCommandOf } from "./controls";
import {
  DEFAULT_KEYMAP,
  matchBindings,
  shellActionFor,
  type KeyBinding,
} from "./keymap";
import {
  evaluateCondition,
  resolveLayout,
  type LayoutConditionContext,
  type LayoutRegion,
} from "./layout";
import { EmptyOverlay, StateRouter, type PlayerEngineView } from "./states";
import "./player.css";
import { useChromeAutoHide } from "./useChromeAutoHide";
import { useFullscreen } from "./useFullscreen";

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
}

export function PlayerScreen(props: PlayerScreenProps) {
  // Shared player state seam (card 4.C2 context.tsx) — the transport comes
  // from ClientProvider (hosts/tests may not inject one here; tests wrap
  // with ClientProvider, matching main.tsx composition).
  return (
    <PlayerControlsProvider>
      <PlayerScreenInner {...props} />
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
}: PlayerScreenProps) {
  const m = useMsg();
  const layout = useMemo(() => resolveLayout(layoutSource), [layoutSource]);

  const rootRef = useRef<HTMLDivElement>(null);
  const fullscreen = useFullscreen(rootRef);
  const { isFullscreen, toggle: toggleFullscreen, exit: exitFullscreen } = fullscreen;
  const hasMedia = media !== null;

  // Doc 08 §4.3 [PROPOSAL]: auto-hide on idle in fullscreen; empty state and
  // any open panel/menu lock the chrome visible (§4.5, §5.3).
  const chrome = useChromeAutoHide({
    enabled: isFullscreen,
    locked: !hasMedia || openPanels.length > 0,
  });

  // Control-bar commands (4.C2) via the shared state seam — keymap DATA
  // drives which bindings reach here (see controls/state-mapping.ts).
  const { actions } = usePlayerControls();

  // Keymap-data-driven shortcuts (doc 05 §8.1): the shell listens for
  // bindings it owns (fullscreen-toggle, exit-fullscreen,
  // show-controls-fullscreen per keymaps/default.toml) and forwards the rest
  // to the 4.C2 seam. Native ESC exit is handled by the browser.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      for (const binding of matchBindings(DEFAULT_KEYMAP, event)) {
        const action = shellActionFor(binding);
        switch (action.kind) {
          case "toggle-fullscreen":
            void toggleFullscreen();
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
  }, [toggleFullscreen, exitFullscreen, chrome, onBinding, actions]);

  const conditionCtx: LayoutConditionContext = {
    hasMedia,
    hasVideo: media?.hasVideo ?? false,
    fullscreen: isFullscreen,
    hasChapters: media?.hasChapters ?? false,
  };
  const visible = (region: LayoutRegion): boolean =>
    evaluateCondition(region.visibleWhen, conditionCtx);
  const regionsOf = (anchor: LayoutRegion["anchor"]): readonly LayoutRegion[] =>
    layout.regions.filter((region) => region.anchor === anchor && visible(region));
  const canvasRegion = regionsOf("center")[0];

  const sidePanel = (region: LayoutRegion): boolean => openPanels.includes(region.id);
  const leftRegions = regionsOf("left").filter(sidePanel);
  const rightRegions = regionsOf("right").filter(sidePanel);

  return (
    <div
      ref={rootRef}
      className="lumen-player"
      data-screen="player"
      data-chrome={chrome.visibility}
      data-fullscreen={String(isFullscreen)}
      data-has-media={hasMedia}
    >
      {leftRegions.map((region) => (
        <PanelRegion key={region.id} region={region} side="left" />
      ))}

      <div className="lumen-player__main">
        {regionsOf("top").map((region) => (
          <header
            key={region.id}
            data-region-id={region.id}
            data-chrome-part=""
            className="lumen-player__topbar"
          >
            {/* Screen title (window title template is product-config's job;
                the top-bar composition itself is later work). */}
            <h1 className="lumen-player__title">{m("screen.player")}</h1>
          </header>
        ))}

        {canvasRegion === undefined ? null : (
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

            {/* Card 4.C3 playback UX states: exactly zero or one overlay on
                the canvas region, driven by the engine view. No engine view
                provided (scaffold/tests) → honest fallback: first-run Empty
                when no media, nothing otherwise — never a fabricated state. */}
            {engineState !== undefined ? (
              <StateRouter view={engineState} onRetry={onRetry} onOpenMedia={onOpenMedia} />
            ) : hasMedia ? null : (
              <EmptyOverlay onOpenMedia={onOpenMedia} />
            )}
          </div>
        )}

        {regionsOf("bottom").map((region) => (
          <footer
            key={region.id}
            data-region-id={region.id}
            data-chrome-part=""
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
        <PanelRegion key={region.id} region={region} side="right" />
      ))}
    </div>
  );
}

/** An empty, honestly-labelled side-panel container from layout config. */
function PanelRegion({ region, side }: { region: LayoutRegion; side: "left" | "right" }) {
  const m = useMsg();
  return (
    <aside
      data-region-id={region.id}
      data-chrome-part=""
      role="complementary"
      aria-label={m(region.labelKey)}
      className={`lumen-player__panel lumen-player__panel--${side}`}
    >
      {/* Placeholder container — panel content belongs to later cards
          (library/playlist: 4.F3; media-info: later). One shared
          placeholder MessageKey; per-panel copy lands with those cards. */}
      <p className="lumen-player__sr-note">{m("player.panel.placeholder")}</p>
    </aside>
  );
}
