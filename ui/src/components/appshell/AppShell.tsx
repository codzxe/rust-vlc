// AppShell (W26 Figma redesign, lane w26.a10) — the persistent chrome
// around every route except /onboarding (full-bleed hero) and /dev/tokens
// (dev fixture): TITLE BAR · ICON RAIL · PLAYLISTS SIDEBAR · main region.
// Reference frames: design/figma/Player.svg (+ Player-1..6), sidebar.svg,
// Miniplayer.svg — geometry measured from the SVGs (see docs/waves/w26/
// logs/w26.a10.md for the measurement table).
//
// Landmark contract (kept aligned with e2e/a11y.spec.ts, which runs axe
// wcag2a/2aa + a DOM-order tab audit on the wrapped routes):
//   * <header class="app-shell__banner"> — the `banner` landmark (title
//     bar: orange app dot · sidebar-toggle · breadcrumb left · centered
//     wordmark · window controls right; Player.svg topbar anatomy).
//   * <nav aria-label> (rail) — Player / Playlist / Media info / Discovery
//     / Shortcuts / Settings + the pinned add-media foot stop; every
//     target is a ≥44px hit (IconWrapper `lg`; W26 contract).
//   * <nav aria-label> (sidebar) — the playlists rail (sidebar.svg: header
//     with back/…/search, "N Items" meta row + sort/list/grid, create row,
//     rows composed through the LIST PRIMITIVES' API — VirtualList list
//     view, ScrollArea grid view; the list components are imported, never
//     edited).
//   * <main> — the routed screen, ONLY when the screen does not render its
//     own <main> (playlist/settings/media-info do — `provideMain` false
//     keeps exactly one main landmark per page and valid HTML).
//
// W26 shell facts (Player.svg at 1920×1080 → our 1440×900 viewport):
//   title bar 40 → 30px (W25 spec §2 allows 24–32) · rail 50 → 52px wide
//   (spec: 48–56; 44px+ hits) · sidebar 377 → 208px (spec pins 200–220) ·
//   playlist rows 42px pitch → 36px · active nav icon = THE orange accent.
//
// The Solstice menubar is GONE (no W25 frame carries one): navigation
// lives in the rail + breadcrumb; open-media stays shell-owned (the ONE
// OPEN_MEDIA_EVENT listener for every screen); the window controls are
// IN-APP actions (miniplayer / document fullscreen / stop + home) — the
// web shell has no OS chrome to drive, and none of the buttons pretend
// otherwise (labels say what they do).
//
// ONE-SIDEBAR COMPOSITION (Player.png, Lead decision w26): the frame shows
// rail + ONE sidebar — the shell's playlists library sidebar (sidebar.svg
// anatomy). The player's docked QUEUE panel (PlayerScreen, lane w26.a13) is
// the player's own surface; the shell never renders a second docked panel.
// The title-bar sidebar toggle keeps the composition at one column: while
// the player's dock is open it also dispatches PLAYER_TOGGLE_SIDEBAR_EVENT
// (the player's tested window-event seam) so the dock collapses together
// with — or instead of — the shell sidebar. The player publishes its dock
// state as `data-sidebar` on `.lumen-player`; the shell READS it, it never
// guesses.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Link, NavLink, useLocation, useNavigate } from "react-router-dom";
import { IconButton } from "../button/IconButton";
import { Icon } from "../icon/Icon";
import { IconWrapper } from "../icon/IconWrapper";
import type { IconName } from "../icon/icons";
import { ScrollArea, VirtualList } from "../list";
import { Panel } from "../surface";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.a10";
import { useClient } from "../../client/ClientProvider";
import type { CommandRequest } from "../../client/transport";
import { OpenMediaDialog, OPEN_MEDIA_EVENT } from "../../screens/open-media";
import { PLAYER_TOGGLE_SIDEBAR_EVENT } from "../../screens/player";
import {
  GlyphChevronLeft,
  GlyphClose,
  GlyphGrid,
  GlyphList,
  GlyphMaximize,
  GlyphMinimize,
  GlyphMore,
  GlyphPlaylist,
  GlyphPlus,
  GlyphSearch,
  GlyphSort,
} from "./shellGlyphs";
import "./appshell.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead). */
type LaneKey = keyof typeof laneKeys;
const laneCatalog: Readonly<Record<string, string>> = laneKeys;

/**
 * useShellMsg — `useMsg` extended with this lane's pre-registration keys.
 * Lane keys win while they are unregistered; once the Lead merges them into
 * the catalog both paths return the same string (the lane file copy is the
 * reference). Every user-visible shell string flows through here — no
 * literals (doc 07 §6.5).
 */
function useShellMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

// ---------------------------------------------------------------------------
// Structural data (frames + real routes — no invented surfaces)
// ---------------------------------------------------------------------------

/** Rail stops: real routes only; icons from the pinned registry. */
const RAIL_ITEMS: ReadonlyArray<{
  to: string;
  labelKey: MessageKey;
  icon: IconName;
}> = [
  // Player.svg rail stop 1 is a house ("home") — the player is home.
  { to: "/player", labelKey: "nav.player", icon: "go-home" },
  // stop 2/3 are folder glyphs (Videos/Music) — /playlist is the library.
  { to: "/playlist", labelKey: "nav.playlist", icon: "open-folder" },
  { to: "/media-info", labelKey: "nav.media-info", icon: "about" },
  { to: "/discovery", labelKey: "nav.discovery", icon: "network-stream" },
  { to: "/keyboard-shortcuts", labelKey: "nav.shortcuts", icon: "keyboard-shortcuts" },
  { to: "/settings", labelKey: "nav.settings", icon: "settings" },
];

/** Breadcrumb (title-bar left cluster): route → section label + glyph. */
const CRUMB_ITEMS: ReadonlyArray<{
  when: (pathname: string) => boolean;
  labelKey: MessageKey | LaneKey;
  icon: IconName;
}> = [
  { when: (p) => p.startsWith("/player/music"), labelKey: "menu.view.music-mode", icon: "play" },
  { when: (p) => p.startsWith("/miniplayer"), labelKey: "menu.view.miniplayer", icon: "pip" },
  { when: (p) => p.startsWith("/player"), labelKey: "screen.player", icon: "go-home" },
  { when: (p) => p.startsWith("/playlist"), labelKey: "nav.playlist", icon: "open-folder" },
  { when: (p) => p.startsWith("/media-info"), labelKey: "nav.media-info", icon: "about" },
  { when: (p) => p.startsWith("/discovery"), labelKey: "nav.discovery", icon: "network-stream" },
  { when: (p) => p.startsWith("/settings"), labelKey: "screen.settings", icon: "settings" },
  { when: (p) => p.startsWith("/keyboard-shortcuts"), labelKey: "nav.shortcuts", icon: "keyboard-shortcuts" },
  { when: (p) => p.startsWith("/open-network-stream"), labelKey: "menu.file.open-network-stream", icon: "network-stream" },
  { when: (p) => p.startsWith("/convert"), labelKey: "convert.title", icon: "convert" },
];

/**
 * Sidebar rows — the STRUCTURAL reference fixture (Player.png sidebar:
 * "Cool Songs", "Anime to Watch"). The playlists-LIBRARY data seam is not
 * wired in this build (the engine v1 catalog has queue commands only); the
 * rows navigate to the real /playlist surface and the create row renders
 * disabled-honest, mirroring the shell's old menubar discipline (unbacked
 * actions never pretend). The playlist-browse lane owns the real store.
 */
const SHELL_PLAYLISTS: ReadonlyArray<{ id: string; nameKey: LaneKey }> = [
  { id: "cool-songs", nameKey: "shell.playlist.cool-songs" },
  { id: "anime-to-watch", nameKey: "shell.playlist.anime-to-watch" },
];

/** Playlist row height (Player.svg 42px pitch at 1920 → 36px at our scale). */
const PLAYLIST_ROW_HEIGHT = 36;

/** Sidebar DOM id (the title-bar toggle's aria-controls target). */
const SIDEBAR_ID = "lumen-app-sidebar";

// ---------------------------------------------------------------------------
// Command dispatch (window-close stop) — same shape as the old menubar seam
// ---------------------------------------------------------------------------

/** Command request minus the transport-assigned requestId (distributive). */
type DistributiveOmit<T, K extends keyof never> = T extends unknown ? Omit<T, K> : never;
type CommandDraft = DistributiveOmit<CommandRequest, "requestId">;

// ---------------------------------------------------------------------------
// Sidebar open/close — persisted UI state + narrow-viewport auto-collapse
// ---------------------------------------------------------------------------

const SIDEBAR_STORAGE_KEY = "lumen.shell.sidebar.v1";
/** Below this width the sidebar auto-collapses (W26: at 1024×768 it may
 * collapse to the icon rail; the user can still re-open it — flex keeps
 * every region unclipped). */
const NARROW_QUERY = "(max-width: 1100px)";

function loadSidebarOpen(): boolean {
  try {
    return (globalThis.localStorage?.getItem(SIDEBAR_STORAGE_KEY) ?? "open") !== "closed";
  } catch {
    return true; // storage unavailable — session default (open)
  }
}

function persistSidebarOpen(open: boolean): void {
  try {
    globalThis.localStorage?.setItem(SIDEBAR_STORAGE_KEY, open ? "open" : "closed");
  } catch {
    // Unavailable host — session-only sidebar state, honest.
  }
}

// ---------------------------------------------------------------------------
// Document fullscreen (the in-app "maximize" — no OS window API in the web
// shell; the whole app fills the screen)
// ---------------------------------------------------------------------------

function useShellFullscreen(): readonly [boolean, () => void] {
  const [active, setActive] = useState(
    () => typeof document !== "undefined" && document.fullscreenElement !== null,
  );

  useEffect(() => {
    const onChange = (): void => setActive(document.fullscreenElement !== null);
    document.addEventListener("fullscreenchange", onChange);
    return () => document.removeEventListener("fullscreenchange", onChange);
  }, []);

  const toggle = useCallback(() => {
    // Hosts without the fullscreen API (jsdom, embedded webviews) get an
    // honest no-op instead of a crash; the pressed state never lies.
    if (document.fullscreenElement === null) {
      void document.documentElement.requestFullscreen?.().catch(() => {});
    } else {
      void document.exitFullscreen?.().catch(() => {});
    }
  }, []);

  return [active, toggle] as const;
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
  const t = useShellMsg();
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

  // ---- sidebar open/close (persisted + narrow auto-collapse) -------------
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(loadSidebarOpen);
  const toggleSidebar = useCallback((): void => {
    setSidebarOpen((open) => {
      persistSidebarOpen(!open);
      return !open;
    });
    // One-sidebar composition (see the file header): if the player's docked
    // queue sidebar is open, collapse it in the same toggle — the dock never
    // stacks a second sidebar column next to the shell's (or the canvas).
    // Plain `Event`, the exact shape the player's tested seam listens for.
    if (document.querySelector(".lumen-player[data-sidebar='open']") !== null) {
      window.dispatchEvent(new Event(PLAYER_TOGGLE_SIDEBAR_EVENT));
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const query = window.matchMedia(NARROW_QUERY);
    // Entering the narrow band collapses the sidebar ONCE (the icon rail
    // keeps every destination reachable); the toggle stays live so the
    // user can re-open it — nothing clips either way (flex + min-width:0).
    if (query.matches) setSidebarOpen(false);
    const onChange = (event: MediaQueryListEvent): void => {
      if (event.matches) setSidebarOpen(false);
    };
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  // ---- window controls ----------------------------------------------------
  const [fullscreenActive, toggleFullscreen] = useShellFullscreen();
  const onMiniplayerRoute = location.pathname.startsWith("/miniplayer");
  const onWindowMinimize = useCallback((): void => {
    navigate("/miniplayer");
  }, [navigate]);
  const onWindowClose = useCallback((): void => {
    // The in-app "close": stop playback (a real command) and return home —
    // the web shell has no window to close, and the label says exactly this.
    dispatch({ cmd: "playback.stop" });
    navigate("/player");
  }, [dispatch, navigate]);

  // ---- breadcrumb ----------------------------------------------------------
  const crumb = useMemo(() => {
    const hit = CRUMB_ITEMS.find((item) => item.when(location.pathname));
    return hit ?? { labelKey: "app.name" as LaneKey, icon: "go-home" as IconName };
  }, [location.pathname]);

  // ---- sidebar rows: filter (search) + sort (alpha) over the fixture ------
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [alphaSort, setAlphaSort] = useState(false);
  const [view, setView] = useState<"list" | "grid">("list");
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (searchOpen) searchInputRef.current?.focus();
  }, [searchOpen]);

  const toggleSearch = useCallback((): void => {
    setSearchOpen((open) => {
      if (open) setQuery(""); // closing drops the filter (no stale hidden rows)
      return !open;
    });
  }, []);

  const rows = useMemo(() => {
    const named = SHELL_PLAYLISTS.map((playlist) => ({ id: playlist.id, label: t(playlist.nameKey) }));
    const needle = query.trim().toLowerCase();
    const filtered =
      needle === "" ? named : named.filter((row) => row.label.toLowerCase().includes(needle));
    return alphaSort ? [...filtered].sort((a, b) => a.label.localeCompare(b.label)) : filtered;
  }, [alphaSort, query, t]);

  return (
    <div className="app-shell" data-demo={demo || undefined}>
      {/* ---- title bar (banner landmark; Player.svg topbar) ---------------- */}
      <header className="app-shell__banner">
        <div className="app-shell__banner-cluster">
          <span className="app-shell__appdot" aria-hidden="true" />
          <span className="app-shell__tick" aria-hidden="true" />
          <IconButton
            label={t("shell.title.toggle-sidebar")}
            size="sm"
            className="app-shell__sidebar-toggle"
            aria-expanded={sidebarOpen}
            aria-controls={SIDEBAR_ID}
            onClick={toggleSidebar}
          >
            <Icon name="right-sidebar" size={16} />
          </IconButton>
          <span className="app-shell__tick" aria-hidden="true" />
          <span className="app-shell__crumb">
            <Icon name={crumb.icon} size={14} />
            <span className="app-shell__crumb-label">{t(crumb.labelKey)}</span>
          </span>
        </div>

        <span className="app-shell__wordmark">{t("app.name")}</span>

        <div className="app-shell__banner-cluster app-shell__banner-cluster--end">
          {demo ? (
            <span className="app-shell__demo-badge" title={m("demo.notice.title")}>
              {m("demo.badge")}
            </span>
          ) : null}
          <div className="app-shell__window" role="group" aria-label={t("shell.window.group")}>
            <IconButton
              label={t("shell.window.minimize")}
              size="sm"
              className="app-shell__winctl"
              onClick={onWindowMinimize}
              disabled={onMiniplayerRoute}
            >
              <GlyphMinimize size={14} />
            </IconButton>
            <IconButton
              label={t("shell.window.maximize")}
              size="sm"
              className="app-shell__winctl"
              aria-pressed={fullscreenActive}
              onClick={toggleFullscreen}
            >
              <GlyphMaximize size={14} />
            </IconButton>
            <IconButton
              label={t("shell.window.close")}
              size="sm"
              className="app-shell__winctl"
              onClick={onWindowClose}
            >
              <GlyphClose size={14} />
            </IconButton>
          </div>
        </div>
      </header>

      <div className="app-shell__frame">
        {/* ---- icon rail (Player.svg rail: nav glyphs + pinned "+" foot) --
         * <Panel edges="end"> = the surface lane's square .lumen-pane fill
         * (#191919) + the 1px inline-end hairline (w26.a08 adoption). */}
        <Panel as="nav" className="app-shell__rail" edges="end" aria-label={m("nav.title")}>
          <ul className="app-shell__rail-list">
            {RAIL_ITEMS.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  aria-label={m(item.labelKey)}
                  /* hover affordance mirrors the accessible name (§4 audit) */
                  title={m(item.labelKey)}
                  className={({ isActive }) =>
                    ["app-shell__rail-link", isActive ? "app-shell__rail-link--active" : ""]
                      .filter(Boolean)
                      .join(" ")
                  }
                >
                  {({ isActive }) => (
                    /* 44px hit frame (Icon Wrapper.svg); the ACTIVE stop is
                     * the ONE orange accent (W25 spec §2) at stroke 2. */
                    <IconWrapper size="lg" active={isActive}>
                      <Icon name={item.icon} size={20} variant={isActive ? "strong" : "default"} />
                    </IconWrapper>
                  )}
                </NavLink>
              </li>
            ))}
          </ul>
          <div className="app-shell__rail-foot">
            <IconButton
              label={t("shell.rail.add")}
              className="app-shell__rail-add"
              onClick={() => setOpenMediaOpen(true)}
            >
              <IconWrapper size="lg">
                <GlyphPlus size={20} />
              </IconWrapper>
            </IconButton>
          </div>
        </Panel>

        {/* ---- playlists sidebar (sidebar.svg; collapsible) ----------------
         * <Panel tone="deep" edges="end"> = the #0E0E0E sidebar fill
         * (sidebar.svg measures the deep surface) + the 1px hairline
         * against the main region (w26.a08 adoption). */}
        <Panel
          as="nav"
          tone="deep"
          edges="end"
          className="app-shell__sidebar"
          id={SIDEBAR_ID}
          aria-label={t("shell.sidebar.title")}
          hidden={!sidebarOpen}
        >
          <header className="app-shell__sidebar-head">
            <IconButton
              label={t("shell.sidebar.back")}
              size="sm"
              className="app-shell__sidebar-back"
              onClick={() => navigate(-1)}
            >
              <GlyphChevronLeft size={16} />
            </IconButton>
            <h2 className="app-shell__sidebar-title">{t("shell.sidebar.title")}</h2>
            <IconButton
              label={t("shell.sidebar.more")}
              size="sm"
              disabled
              title={t("shell.sidebar.more.hint")}
            >
              <GlyphMore size={16} />
            </IconButton>
            <IconButton
              label={t("shell.sidebar.search")}
              size="sm"
              aria-expanded={searchOpen}
              aria-controls={searchOpen ? "lumen-app-sidebar-search" : undefined}
              onClick={toggleSearch}
            >
              <GlyphSearch size={16} />
            </IconButton>
          </header>

          {searchOpen ? (
            <div className="app-shell__sidebar-search" id="lumen-app-sidebar-search">
              <input
                ref={searchInputRef}
                type="search"
                className="app-shell__sidebar-search-input"
                aria-label={t("shell.sidebar.search")}
                placeholder={t("shell.sidebar.search.placeholder")}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </div>
          ) : null}

          <div className="app-shell__sidebar-meta">
            <span className="app-shell__sidebar-count">
              {String(rows.length)} {t("shell.sidebar.count-suffix")}
            </span>
            <div className="app-shell__sidebar-modes">
              <IconButton
                label={t("shell.sidebar.sort")}
                size="sm"
                aria-pressed={alphaSort}
                onClick={() => setAlphaSort((value) => !value)}
              >
                <GlyphSort size={16} />
              </IconButton>
              <IconButton
                label={t("shell.sidebar.view-list")}
                size="sm"
                aria-pressed={view === "list"}
                onClick={() => setView("list")}
              >
                <GlyphList size={16} />
              </IconButton>
              <IconButton
                label={t("shell.sidebar.view-grid")}
                size="sm"
                aria-pressed={view === "grid"}
                onClick={() => setView("grid")}
              >
                <GlyphGrid size={16} />
              </IconButton>
            </div>
          </div>

          {/* Unbacked surface → disabled-honest (the old menubar's rule):
           * the engine has no create-playlist command; the row never
           * pretends it can. */}
          <IconButton
            label={t("shell.sidebar.create")}
            size="sm"
            className="app-shell__create"
            disabled
            title={t("shell.sidebar.create.hint")}
          >
            <GlyphPlus size={14} />
            <span className="app-shell__create-label">{t("shell.sidebar.create")}</span>
          </IconButton>

          <div className="app-shell__sidebar-body">
            {rows.length === 0 ? (
              <p className="app-shell__sidebar-empty">
                {query.trim() === "" ? t("shell.sidebar.empty") : t("shell.sidebar.search.empty")}
              </p>
            ) : view === "list" ? (
              /* List view — the list primitive's own API (windowed rows). */
              <VirtualList
                data={rows}
                itemHeight={PLAYLIST_ROW_HEIGHT}
                label={t("shell.sidebar.list-label")}
                className="app-shell__sidebar-list"
                renderItem={(row) =>
                  /* Guard: VirtualList's windowed range can lag one render
                   * behind a shrinking dataset (filter applied) — the list
                   * lane owns the clamp; the shell never crashes on it. */
                  row ? (
                    <Link to="/playlist" className="app-shell__plrow" draggable={false}>
                      <GlyphPlaylist size={14} />
                      <span className="app-shell__plrow-label">{row.label}</span>
                    </Link>
                  ) : null
                }
              />
            ) : (
              /* Grid view — the scroll primitive over a 2-up card grid. */
              <ScrollArea label={t("shell.sidebar.list-label")} className="app-shell__sidebar-scroll">
                <div className="app-shell__plgrid">
                  {rows.map((row) => (
                    <Link key={row.id} to="/playlist" className="app-shell__plcard" draggable={false}>
                      <span className="app-shell__plcard-thumb" aria-hidden="true">
                        <Icon name="open-folder" size={20} />
                      </span>
                      <span className="app-shell__plcard-label">{row.label}</span>
                    </Link>
                  ))}
                </div>
              </ScrollArea>
            )}
          </div>
        </Panel>

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
