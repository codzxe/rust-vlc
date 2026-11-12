// w26.i05 (h07 defect 1, state 7 — "Save edits nothing real"): the
// consumer half of the `lumen.player.toolbar.v1` seam. The customize
// modal's Save persists the arrangement; THIS component renders the saved
// band items in the REAL control bar, so the customization lands in the
// product (frame -7: no modal, the dropped Favorite living in the real
// transport band).
//
// The Lead lands the wiring in ControlBar.tsx (the controls lane's file —
// the exact 3-line diff is in docs/waves/w26/logs/w26.i05.md):
//   import { CustomizeToolbarItems } from "../customize/toolbarItems";
//   …  <TransportCluster />  <CustomizeToolbarItems />  <SeekCluster /> …
// Everything behavioral lives here (this lane's file), tested in
// CustomizePanels.test.tsx — the controls lane keeps its sheet untouched.
//
// Behavior contract (doc 08 §5 — never fabricated, never dead):
//   * engine-bound tiles dispatch their command (the same grammar as the
//     box's click; feature-not-exposed replies are honest no-change);
//   * shuffle/repeat prefer the REAL playlist context state (the bar's
//     own provider — the confirmed mode/bool, never optimistic);
//   * UI-owned tiles with a real surface wire it: bookmark drops a
//     media.add-bookmark at the engine-reported position (the 1.D4 wire
//     union carries it), playlists/right-sidebar fire the shell's
//     `lumen:toggle-sidebar` seam, vlc-settings/go-home navigate;
//   * UI-owned tiles with NO surface in this shell (favorite, bookmarks,
//     quit, the text-block family, replay/chapters) render
//     disabled-with-reason (PlayerScreen's quit-row precedent — never
//     theater);
//   * spacer/divider are layout primitives: inert, aria-hidden separators.
import { useCallback } from "react";
import type { ReactNode } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { useClient } from "../../../client/ClientProvider";
import type { CommandRequest, CommandTransport } from "../../../client/transport";
import { Icon } from "../../../components/icon/Icon";
import { IconButton } from "../../../components/button/IconButton";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as a17Keys } from "../../../i18n/keys/w26.a17";
import { keys as gfixAKeys } from "../../../i18n/keys/w26.gfixA";
import { keys as i05Keys } from "../../../i18n/keys/w26.i05";
import { usePlayerControls } from "../context";
// Direct module import (ThumbnailGrid's pattern): the playlist barrel does
// not re-export usePlaylistOptional yet (playlist lane's file — the barrel
// re-export is logged as a cross-lane nicety, not a need).
import { usePlaylistOptional } from "../../playlist/PlaylistProvider";
import { cycleArgs, tileById } from "./panels";
import { useToolbarLayout } from "./toolbarLayout";
import type { CommandArgs, CustomizeTile } from "./types";
import "./customize.css";

/** Seam-first label resolver (the CustomizePanels pattern + this lane's keys). */
function useToolbarMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | CustomizeTile["labelKey"] | keyof typeof i05Keys): string =>
      m(key as MessageKey) ??
      a17Keys[key as keyof typeof a17Keys] ??
      gfixAKeys[key as keyof typeof gfixAKeys] ??
      i05Keys[key as keyof typeof i05Keys] ??
      key,
    [m],
  );
}

/** The shell's docked-sidebar toggle seam (PlayerScreen's frozen contract string). */
const TOGGLE_SIDEBAR_EVENT = "lumen:toggle-sidebar";

/** Repeat's cycle order (the box's cycle grammar: off → all → one → off). */
const REPEAT_ORDER: readonly ("off" | "all" | "one")[] = ["off", "all", "one"];

/**
 * The real bar's customized items — the persisted band arrangement
 * (`lumen.player.toolbar.v1`), live across Save through the
 * `lumen:toolbar-changed` seam. Renders nothing while the band is empty
 * (frame -1's clean baseline); never renders unknown ids.
 */
export function CustomizeToolbarItems({ transport }: { readonly transport?: CommandTransport }) {
  const client = useClient();
  const wire = transport ?? client;
  const t = useToolbarMsg();
  const { state } = usePlayerControls();
  const playlist = usePlaylistOptional();

  const layout = useToolbarLayout();
  const items = layout.band
    .map((id) => tileById(id))
    .filter((tile): tile is CustomizeTile => tile !== null);
  if (items.length === 0) return null;

  const dispatch = async (tile: CustomizeTile, args: CommandArgs): Promise<void> => {
    if (tile.command === null) return;
    // Forward-declared commands (record/PiP/HDR) are not yet in the 1.D4
    // wire union — the cast bridge is CustomizePanels' pattern (recorded
    // in ui/NEXT-STEPS.md); errors are honest no-change.
    const request = { cmd: tile.command, args, requestId: 1 } as unknown as CommandRequest;
    try {
      await wire.send(request);
    } catch {
      // Transport failure — same honest no-change policy as the box.
    }
  };

  // The NavigationSeam pattern (PlayerScreen/ControlBar): in-app routing
  // inside the Router; bare mounts (jsdom trees without a Router, where
  // useNavigate throws) fall back to document navigation. Split keeps
  // every hook call unconditional (rules-of-hooks).
  return (
    <ToolbarNavigationSeam>
      {(navigate) => (
        <div
          className="lumen-toolbar-items"
          role="group"
          aria-label={t("customize.toolbar.label")}
        >
          {items.map((tile) => {
        const label = t(tile.labelKey);
        if (tile.id === "spacer") {
          return <span key="spacer" className="lumen-toolbar-items__spacer" aria-hidden="true" />;
        }
        if (tile.id === "divider") {
          return <span key="divider" className="lumen-toolbar-items__divider" aria-hidden="true" />;
        }
        if (tile.id === "bookmark") {
          // Real engine command (the 1.D4 union carries media.add-bookmark):
          // drop a bookmark at the engine-reported position.
          return (
            <IconButton
              key={tile.id}
              data-field-id={tile.id}
              label={label}
              onClick={() => {
                void wire.send({
                  cmd: "media.add-bookmark",
                  args: { time: Math.max(0, Math.floor(state.position)) },
                  requestId: 2,
                });
              }}
            >
              {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
            </IconButton>
          );
        }
        if (tile.id === "playlists" || tile.id === "right-sidebar") {
          return (
            <IconButton
              key={tile.id}
              data-field-id={tile.id}
              label={label}
              onClick={() => window.dispatchEvent(new CustomEvent(TOGGLE_SIDEBAR_EVENT))}
            >
              {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
            </IconButton>
          );
        }
        if (tile.id === "vlc-settings") {
          return (
            <IconButton
              key={tile.id}
              data-field-id={tile.id}
              label={label}
              onClick={() => navigate("/settings")}
            >
              {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
            </IconButton>
          );
        }
        if (tile.id === "go-home") {
          return (
            <IconButton
              key={tile.id}
              data-field-id={tile.id}
              label={label}
              onClick={() => navigate("/")}
            >
              {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
            </IconButton>
          );
        }
        if (tile.control.kind !== "ui") {
          // Engine-bound: the box's dispatch grammar. shuffle/repeat prefer
          // the playlist context's REAL confirmed state; other cycles ride
          // the default-based args until the engine reports (honest).
          if (tile.id === "shuffle" && playlist !== null) {
            return (
              <IconButton
                key={tile.id}
                data-field-id={tile.id}
                label={label}
                aria-pressed={playlist.shuffle === true ? true : undefined}
                onClick={() => playlist.setShuffle(!(playlist.shuffle === true))}
              >
                {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
              </IconButton>
            );
          }
          if (tile.id === "repeat" && playlist !== null) {
            const current = playlist.repeat ?? "off";
            const next = REPEAT_ORDER[(REPEAT_ORDER.indexOf(current) + 1) % REPEAT_ORDER.length]!;
            return (
              <IconButton
                key={tile.id}
                data-field-id={tile.id}
                label={label}
                onClick={() => playlist.setRepeat(next)}
              >
                {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
              </IconButton>
            );
          }
          if (tile.control.kind === "action" || tile.control.kind === "toggle-action") {
            const args =
              tile.control.kind === "action"
                ? tile.control.args
                : tile.control.argsFor(playlist?.shuffle ?? null);
            return (
              <IconButton
                key={tile.id}
                data-field-id={tile.id}
                label={label}
                onClick={() => void dispatch(tile, args)}
              >
                {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
              </IconButton>
            );
          }
          // cycle without a playlist surface: default-based next (the box's
          // "before any report" path — never optimistic). The const captures
          // the flow-narrowed cycle control for the closure below.
          const cycleControl = tile.control;
          const nextArgs = cycleArgs(cycleControl, null);
          return (
            <IconButton
              key={tile.id}
              data-field-id={tile.id}
              label={label}
              onClick={() => void dispatch(tile, nextArgs)}
            >
              {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
            </IconButton>
          );
        }
        // UI-owned with no surface in this shell — disabled with a reason
        // (favorite, bookmarks, quit, the text-block family, replay,
        // chapters; the PlayerScreen quit-row precedent — never theater).
        return (
          <IconButton
            key={tile.id}
            data-field-id={tile.id}
            label={`${label} — ${t("customize.toolbar.unavailable")}`}
            disabled
          >
            {tile.icon ? <Icon name={tile.icon} size={18} /> : null}
          </IconButton>
        );
          })}
        </div>
      )}
    </ToolbarNavigationSeam>
  );
}

/**
 * Router-presence seam (PlayerScreen/ControlBar's NavigationSeam pattern):
 * in-app navigation inside the Router; bare mounts (jsdom trees without a
 * Router, where useNavigate throws) fall back to document navigation.
 * Split keeps every hook call unconditional (rules-of-hooks).
 */
function ToolbarNavigationSeam({
  children,
}: {
  readonly children: (navigate: (to: string) => void) => ReactNode;
}): ReactNode {
  return useInRouterContext() ? (
    <RoutedToolbarNavigation>{children}</RoutedToolbarNavigation>
  ) : (
    <>{children((to) => window.location.assign(to))}</>
  );
}

function RoutedToolbarNavigation({
  children,
}: {
  readonly children: (navigate: (to: string) => void) => ReactNode;
}): ReactNode {
  const navigate = useNavigate();
  return <>{children((to) => navigate(to))}</>;
}
