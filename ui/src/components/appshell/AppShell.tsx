// AppShell (Solstice app-shell workstream, 2026-11-08) — the persistent
// chrome around every route except /onboarding (full-bleed hero) and
// /dev/tokens (dev fixture).
//
// Landmark contract (kept aligned with e2e/a11y.spec.ts, which runs axe
// wcag2a/2aa + a DOM-order tab audit on the wrapped routes):
//   * <header class="app-shell__banner"> — top-level `banner` landmark;
//     contains the MenuBar (role="menubar", WAI-ARIA menubar model) and the
//     Demo chip. The menubar contributes exactly ONE tab stop (its roving
//     tabindex keeps a single tabIndex=0 trigger).
//   * <nav aria-label> — the icon rail: Player / Playlist / Media info /
//     Shortcuts / Settings, NavLink-driven (aria-current="page" comes from
//     react-router). Icon-only links carry aria-labels from the nav.* keys.
//   * <main> — the routed screen, ONLY when the screen does not render its
//     own <main> (playlist/settings/media-info do — passing `provideMain`
//     false for those keeps exactly one main landmark per page and valid
//     HTML: <main> may never nest).
//
// Menu accelerators come from REAL keymap data (hotkeys generated bindings +
// the variant keymap seed), never hardcoded strings. Playback items dispatch
// real commands through the ClientProvider transport; navigation items use
// react-router. Items with no backing surface render disabled (honest).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { NavLink, useLocation, useNavigate } from "react-router-dom";
import { MenuBar } from "../menu/MenuBar";
import type { MenuBarMenu } from "../menu/MenuBar";
import { Icon } from "../icon/Icon";
import type { IconName } from "../icon/icons";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { useClient } from "../../client/ClientProvider";
import type { CommandRequest, CommandTransport, RepeatMode } from "../../client/transport";
import { OpenMediaDialog, OPEN_MEDIA_EVENT } from "../../screens/open-media";
import { HOTKEY_BINDINGS } from "../../screens/settings/areas/hotkeys/generated/bindings";
import { VARIANT_KEYMAP } from "../../screens/player/variants";
import "./appshell.css";

// ---------------------------------------------------------------------------
// Accelerators from keymap DATA (doc 05 §8.1 — binding-id indirection)
// ---------------------------------------------------------------------------

/** Accelerator display string for a hotkeys binding id (undefined = none). */
const acceleratorOf = (bindingId: string): string | undefined =>
  HOTKEY_BINDINGS.find((binding) => binding.id === bindingId && !binding.disabled)?.key;

/** Accelerator for a shell-owned variant recipe (variantKeymap seed). */
const variantAcceleratorOf = (recipe: string): string | undefined =>
  VARIANT_KEYMAP.get(recipe)?.key;

// ---------------------------------------------------------------------------
// The icon rail
// ---------------------------------------------------------------------------

const RAIL_ITEMS: ReadonlyArray<{
  to: string;
  labelKey: MessageKey;
  icon: IconName;
}> = [
  { to: "/player", labelKey: "nav.player", icon: "play" },
  { to: "/playlist", labelKey: "nav.playlist", icon: "right-sidebar" },
  { to: "/media-info", labelKey: "nav.media-info", icon: "about" },
  { to: "/keyboard-shortcuts", labelKey: "nav.shortcuts", icon: "keyboard-shortcuts" },
  { to: "/settings", labelKey: "nav.settings", icon: "settings" },
];

// ---------------------------------------------------------------------------
// Playlist summary (shuffle/repeat checked states for the Playback menu)
// ---------------------------------------------------------------------------

/** Command request minus the transport-assigned requestId (distributive). */
type DistributiveOmit<T, K extends keyof never> = T extends unknown ? Omit<T, K> : never;
type CommandDraft = DistributiveOmit<CommandRequest, "requestId">;

interface PlaylistSummaryState {
  readonly shuffle: boolean | null;
  readonly repeat: RepeatMode | null;
}

/**
 * Engine-reported shuffle/repeat for the menubar checkboxes. Truth sources:
 * the state snapshot once, then `playlist.changed` events — the same seam
 * PlaylistProvider keeps authoritative.
 */
function usePlaylistSummary(client: CommandTransport): PlaylistSummaryState {
  const [summary, setSummary] = useState<PlaylistSummaryState>({ shuffle: null, repeat: null });

  useEffect(() => {
    let cancelled = false;
    client
      .send({ cmd: "system.get-state-snapshot", requestId: 1 })
      .then((response) => {
        if (cancelled) return;
        const result = response.result;
        if ("completed" in result && result.completed.kind === "state-snapshot") {
          const playlist = result.completed.snapshot.playlist;
          setSummary({ shuffle: playlist.shuffle ?? null, repeat: playlist.repeat ?? null });
        }
      })
      .catch(() => {
        // Host unreachable — honest nulls (checkboxes render unchecked).
      });
    return () => {
      cancelled = true;
    };
  }, [client]);

  useEffect(
    () =>
      client.subscribe((event) => {
        if (event.kind !== "playlist.changed") return;
        const change = event.change;
        if (change.kind === "shuffle-set") {
          setSummary((prev) => ({ ...prev, shuffle: change.enabled }));
        } else if (change.kind === "repeat-set") {
          setSummary((prev) => ({ ...prev, repeat: change.mode }));
        }
      }),
    [client],
  );

  return summary;
}

// ---------------------------------------------------------------------------
// AppShell
// ---------------------------------------------------------------------------

export function AppShell({
  children,
  /** True while the transport is the demo simulation (badges the chrome). */
  demo = false,
  /**
   * Render the `<main>` landmark here. Pass false for screens that render
   * their own <main> (playlist/settings/media-info) — a second, nested main
   * would be invalid HTML and a duplicate landmark.
   * @default true
   */
  provideMain = true,
}: {
  children: ReactNode;
  demo?: boolean;
  provideMain?: boolean;
}) {
  const m = useMsg();
  const client = useClient();
  const navigate = useNavigate();
  const location = useLocation();
  const [openMediaOpen, setOpenMediaOpen] = useState(false);
  const nextRequestId = useRef(0x9000); // high base — no collisions with the player seam's counters

  const dispatch = useCallback(
    (request: CommandDraft): void => {
      nextRequestId.current += 1;
      void client.send({ ...request, requestId: nextRequestId.current } as CommandRequest).catch(() => {
        // Transport unreachable — screens surface their own honest errors.
      });
    },
    [client],
  );

  // The open-media trigger contract: ONE listener for the whole shell.
  useEffect(() => {
    const onOpenMedia = (): void => setOpenMediaOpen(true);
    window.addEventListener(OPEN_MEDIA_EVENT, onOpenMedia);
    return () => window.removeEventListener(OPEN_MEDIA_EVENT, onOpenMedia);
  }, []);

  const { shuffle, repeat } = usePlaylistSummary(client);
  const repeatMode = repeat ?? "off";

  const menus = useMemo<MenuBarMenu<MessageKey>[]>(() => {
    const isActive = (path: string): boolean => location.pathname === path;
    return [
      {
        id: "file",
        labelKey: "menu.file",
        entries: [
          {
            id: "open-media",
            labelKey: "menu.file.open-media",
            accelerator: acceleratorOf("media.open-file"),
            onSelect: () => setOpenMediaOpen(true),
          },
          {
            id: "open-network-stream",
            labelKey: "menu.file.open-network-stream",
            accelerator: acceleratorOf("media.open-network-stream"),
            onSelect: () => navigate("/open-network-stream"),
          },
          { id: "file-sep", kind: "separator" },
          {
            id: "exit",
            labelKey: "menu.file.exit",
            // Honest disable: app-exit is host-owned; the v1 command catalog
            // has no quit command, so the shell cannot act (no accelerator
            // shown either — a disabled shortcut would be a false promise).
            disabled: true,
          },
        ],
      },
      {
        id: "playback",
        labelKey: "menu.playback",
        entries: [
          {
            id: "play-pause",
            labelKey: "menu.playback.play-pause",
            accelerator: acceleratorOf("navigation.toggle-play-pause"),
            onSelect: () => dispatch({ cmd: "playback.toggle-play-pause" }),
          },
          {
            id: "stop",
            labelKey: "menu.playback.stop",
            accelerator: acceleratorOf("navigation.stop"),
            onSelect: () => dispatch({ cmd: "playback.stop" }),
          },
          { id: "playback-sep", kind: "separator" },
          {
            id: "previous",
            labelKey: "menu.playback.previous",
            accelerator: acceleratorOf("navigation.prev-track"),
            onSelect: () => dispatch({ cmd: "playlist.prev" }),
          },
          {
            id: "next",
            labelKey: "menu.playback.next",
            accelerator: acceleratorOf("navigation.next-track"),
            onSelect: () => dispatch({ cmd: "playlist.next" }),
          },
          { id: "playback-sep-2", kind: "separator" },
          {
            id: "shuffle",
            labelKey: "menu.playback.shuffle",
            accelerator: acceleratorOf("playlist.shuffle"),
            checked: shuffle === true,
            onSelect: () =>
              dispatch({ cmd: "playlist.set-shuffle", args: { enabled: !(shuffle === true) } }),
          },
          {
            id: "repeat",
            labelKey: "menu.playback.repeat",
            // No keymap row exists for repeat — no accelerator (honest).
            submenu: (["off", "all", "one"] as const).map((mode) => ({
              id: `repeat-${mode}`,
              labelKey: (`customize.repeat.${mode}` as MessageKey),
              kind: "radio" as const,
              checked: repeatMode === mode,
              onSelect: () => dispatch({ cmd: "playlist.set-repeat", args: { mode } }),
            })),
          },
        ],
      },
      {
        id: "view",
        labelKey: "menu.view",
        entries: [
          {
            id: "player",
            labelKey: "menu.view.player",
            checked: isActive("/player"),
            onSelect: () => navigate("/player"),
          },
          {
            id: "music-mode",
            labelKey: "menu.view.music-mode",
            accelerator: variantAcceleratorOf("music-mode-toggle"),
            checked: isActive("/player/music"),
            onSelect: () => navigate("/player/music"),
          },
          {
            id: "miniplayer",
            labelKey: "menu.view.miniplayer",
            accelerator: variantAcceleratorOf("miniplayer-toggle"),
            checked: isActive("/miniplayer"),
            onSelect: () => navigate("/miniplayer"),
          },
          { id: "view-sep", kind: "separator" },
          {
            id: "playlist",
            labelKey: "menu.view.playlist",
            checked: isActive("/playlist"),
            onSelect: () => navigate("/playlist"),
          },
          {
            id: "media-info",
            labelKey: "menu.view.media-info",
            checked: isActive("/media-info"),
            onSelect: () => navigate("/media-info"),
          },
          { id: "view-sep-2", kind: "separator" },
          {
            id: "shortcuts",
            labelKey: "menu.view.shortcuts",
            checked: isActive("/keyboard-shortcuts"),
            onSelect: () => navigate("/keyboard-shortcuts"),
          },
          {
            id: "settings",
            labelKey: "menu.view.settings",
            checked: location.pathname.startsWith("/settings"),
            onSelect: () => navigate("/settings/global"),
          },
        ],
      },
      {
        id: "tools",
        labelKey: "menu.tools",
        entries: [
          {
            id: "customize",
            labelKey: "menu.tools.customize",
            // Honest disable: the customize panels live inside PlayerScreen's
            // openPanels seam (screens/player ownership); no route or command
            // opens them from the shell yet — handoff to the player scope.
            disabled: true,
          },
          {
            id: "tools-media-info",
            labelKey: "menu.tools.media-info",
            accelerator: acceleratorOf("window.media-info"),
            onSelect: () => navigate("/media-info"),
          },
          {
            id: "preferences",
            labelKey: "menu.tools.preferences",
            accelerator: acceleratorOf("window.preferences"),
            onSelect: () => navigate("/settings/global"),
          },
        ],
      },
      {
        id: "help",
        labelKey: "menu.help",
        entries: [
          {
            id: "about",
            labelKey: "menu.help.about",
            // The About surface is the settings "about" area (updates-about).
            onSelect: () => navigate("/settings/about"),
          },
          {
            id: "help-shortcuts",
            labelKey: "menu.help.keyboard-shortcuts",
            onSelect: () => navigate("/keyboard-shortcuts"),
          },
        ],
      },
    ];
  }, [dispatch, location.pathname, navigate, repeatMode, shuffle]);

  return (
    <div className="app-shell" data-demo={demo || undefined}>
      <header className="app-shell__banner">
        <MenuBar menus={menus} resolveLabel={m} id="lumen-app-menubar" />
        {demo ? (
          <span
            className="solstice-chip solstice-chip--accent app-shell__demo-badge"
            title={m("demo.notice.title")}
          >
            {m("demo.badge")}
          </span>
        ) : null}
      </header>

      <div className="app-shell__frame">
        <nav className="app-shell__rail solstice-glass" aria-label={m("nav.title")}>
          <span className="app-shell__logo solstice-sun" aria-hidden="true" />
          <ul className="app-shell__rail-list">
            {RAIL_ITEMS.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  aria-label={m(item.labelKey)}
                  className={({ isActive }) =>
                    ["app-shell__rail-link", isActive ? "app-shell__rail-link--active" : ""]
                      .filter(Boolean)
                      .join(" ")
                  }
                >
                  <Icon name={item.icon} size={18} />
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>

        {provideMain ? (
          <main className="app-shell__main">{children}</main>
        ) : (
          <div className="app-shell__main">{children}</div>
        )}
      </div>

      <OpenMediaDialog open={openMediaOpen} onClose={() => setOpenMediaOpen(false)} />
    </div>
  );
}
