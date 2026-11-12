// w26.b09 — /playlist route body: the playlists LIBRARY screen
// (playlist.svg grammar), plus the create-playlist flow
// (new_playlist.svg → NewPlaylistDialog).
//
// W26 composition (Figma fidelity spec §2 "Playlist"): the near-black
// #000-class panel (r14 Card on --color-surface) holds ~66px r10 rows —
// playlist glyph + title, quiet #9a9a9a-class text lifting to bright on
// hover, the `•••` kebab revealed on hover — rendered through the list
// primitives (VirtualList + ListRow comfortable). Rows natural-sort by
// title (naturalSort — "Playlist 2" before "Playlist 10", never lexical).
//
// w26.f06 — the page is now TWO levels:
//  · LIBRARY (playlist.png) — the saved playlists (playlistsStore). Opening
//    a row (click or the row menu's Open) enters…
//  · BROWSE (thumbnail.png / media.png + Sorting Options.png /
//    View Options.png) — the opened playlist's media: a toolbar row with the
//    REAL sort + view triggers (PlaylistOptionsToolbar) and the body view
//    switch (PlaylistOptionsViews: Grid View → ThumbnailGrid, List View →
//    MediaList) over a LOCAL data scope (PlaylistOptionsProvider entries =
//    the saved item names — an engine PlaylistProvider is deliberately NOT
//    mounted here: saved items are names-only client data, engine
//    playlist.changed events must not contaminate the view). The toolbar
//    also carries the Add-subtitles action (F2) opening AddSubtitlesDialog.
//
// w26.gfix-E (g18 P2-1 + g20 P2-4) — the browse media rows now carry the
// FILE-ROW context menu (file context menu.svg, 12 rows): the media.svg
// hover "•••" and right-click open it (MediaList's entryMenu seam). Real
// seams only: Mark as Watched (the provider's client-local override),
// Convert + Properties & Metadata (routes), Add to Playlist (the local
// store), Add Subtitle Tracks (the dialog scoped to the one file).
//
// Data: playlistsStore (local saved playlists, persisted). Every row action
// is honest — Play in the context menu is disabled until a catalog command
// exists; Remove is the real local delete. Saved media items carry no play
// seam (names only), so the browse views render them as presentation, never
// dead play buttons.

import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode, type RefObject } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { useMsg } from "../../i18n/messages";
import { Button } from "../../components/button";
import { Icon } from "../../components/icon";
import { Card } from "../../components/surface";
import { ContextMenu } from "../../components/menu";
import type { MenuEntry } from "../../components/menu";
import { ListRow, ListEmpty, ROW_HEIGHTS, VirtualList, naturalSort } from "../../components/list";
import { addItemToPlaylist, removePlaylist, usePlaylists, type SavedPlaylist } from "./playlistsStore";
import type { PlaylistEntry } from "./PlaylistProvider";
import { NewPlaylistDialog } from "./NewPlaylistDialog";
import { AddSubtitlesDialog } from "./subtitles/AddSubtitlesDialog";
import { PlaylistOptionsProvider } from "./options/PlaylistOptionsProvider";
import { usePlaylistOptions } from "./options/PlaylistOptionsProvider";
import { PlaylistOptionsToolbar, PlaylistOptionsViews } from "./options/PlaylistOptionsToolbar";
import { MEDIA_ROW_HEIGHT } from "./options/MediaList";
import { fileMenuResolveLabel, fileRowMenu, playlistTargetKey } from "./options/fileRowMenu";
import { useOptionsMsg } from "./options/useOptionsMsg";
import { usePlaylistMsg, type PlaylistLaneKey } from "./usePlaylistMsg";
import "./playlist.css";

/** Lane keys as seen by the row's context menu (same seam, shared hook). */
type LaneKey = PlaylistLaneKey;

// ---------------------------------------------------------------------------
// w26.i10 (h16 a11y shame #2) — ARROW-KEY GRID MODEL, both /playlist browse
// grids (library playlist.svg rows + media.svg rows). APG "grid" pattern in
// the aria-activedescendant flavor: keyboard focus stays on the VirtualList
// grid container (tabIndex=0 is pinned by the shared primitive), the active
// row is pointed at by aria-activedescendant + data-active (playlist.css
// paints it only while the grid holds focus), ArrowUp/ArrowDown/Home/End
// move it — scrolling the window when the target row sits outside the
// rendered range — and Enter/Space run the active row's real seam (library:
// open the playlist; media: open the row's file context menu).
//
// The roving-TABINDEX flavor is deliberately not used: rows embed interactive
// kebab/••• buttons, so a focusable row div would nest interactive
// descendants inside a widget role and add a second tab stop beside the
// container's — the container already owns the tab stop. aria-selected stays
// false (the shared VirtualList pins it — list-lane seam, logged); arrow
// movement is focus, not selection.
// ---------------------------------------------------------------------------

/** Frame budget for the activedescendant settle retries: a moved-to row can
 * sit outside the rendered window for a few frames until the scrollport's
 * scroll event re-renders it (30 frames ≈ 500 ms at 60 fps). Structural. */
const GRID_SYNC_RETRIES = 30;

/** rAF with a jsdom-safe fallback (vitest's jsdom may run without
 * pretendToBeVisual — the fallback keeps the settle loop testable). */
const requestFrame = (callback: () => void): number =>
  typeof requestAnimationFrame === "function"
    ? requestAnimationFrame(callback)
    : window.setTimeout(callback, 0);
const cancelFrame = (handle: number): void => {
  if (typeof cancelAnimationFrame === "function") cancelAnimationFrame(handle);
  else window.clearTimeout(handle);
};

/**
 * The arrow-key grid controller (see the block comment above). `itemHeight`
 * resolves a rendered row's absolute index from its pinned `style.top` — the
 * window renders consecutive indices, and the shared VirtualList keys row
 * nodes BY INDEX, so the imperative ids stay consistent across window
 * shifts (a reused node always is its key's index).
 */
function useGridRoving({
  count,
  itemHeight,
  onActivate,
}: {
  count: number;
  itemHeight: number;
  /** The active row's real seam (Enter/Space). */
  onActivate: (index: number, grid: HTMLElement) => void;
}): {
  wrapRef: RefObject<HTMLDivElement | null>;
  onKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void;
} {
  const wrapRef = useRef<HTMLDivElement>(null);
  const gridId = useId();
  const [activeIndex, setActiveIndex] = useState<number | null>(null);

  // Keep the active index inside the dataset (Remove / set changes shrink
  // it; identical values bail out of the re-render).
  useEffect(() => {
    setActiveIndex((index) => (index === null || index < count ? index : Math.max(0, count - 1)));
  }, [count]);

  // Sync pass: id + data-active over the rendered rows, aria-activedescendant
  // on the grid container. Runs after EVERY commit (no dep array — the grid
  // can remount without the index changing: library ↔ browse switch, list ↔
  // grid view switch), retrying a few frames while the active row sits
  // outside the rendered window (the scroll nudge below re-renders it
  // asynchronously). The writes are idempotent DOM attributes React never
  // manages (the shared VirtualList pins no ids), so re-running is free.
  useEffect(() => {
    const wrap = wrapRef.current;
    if (wrap === null) return;
    let frame = 0;
    let attempts = 0;
    const sync = (): void => {
      const grid = wrap.querySelector<HTMLElement>('[role="grid"]');
      if (grid !== null) {
        let found = false;
        for (const row of Array.from(grid.querySelectorAll<HTMLElement>('[role="row"]'))) {
          const top = Number.parseFloat(row.style.top);
          if (!Number.isFinite(top)) continue;
          const index = Math.round(top / itemHeight);
          if (index >= count) continue;
          row.id = `${gridId}-row-${index}`;
          if (index === activeIndex) {
            found = true;
            row.setAttribute("data-active", "true");
          } else {
            row.removeAttribute("data-active");
          }
        }
        if (activeIndex === null) {
          grid.removeAttribute("aria-activedescendant");
        } else if (found) {
          grid.setAttribute("aria-activedescendant", `${gridId}-row-${activeIndex}`);
        }
        if (activeIndex !== null && !found && attempts < GRID_SYNC_RETRIES) {
          attempts += 1;
          frame = requestFrame(sync);
        }
      }
    };
    sync();
    return () => cancelFrame(frame);
  });

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const wrap = wrapRef.current;
    const grid = wrap?.querySelector<HTMLElement>('[role="grid"]') ?? null;
    // Only while the GRID CONTAINER itself holds focus (the
    // activedescendant model): keys from a row's kebab/••• buttons keep
    // their own behavior (Enter opens the menu, not the row).
    if (grid === null || event.target !== grid || count === 0) return;
    const move = (next: number): void => {
      event.preventDefault();
      const clamped = Math.max(0, Math.min(count - 1, next));
      setActiveIndex(clamped);
      // Bring the target into the rendered window when it sits outside the
      // scrollport (the window re-renders on the container's scroll event;
      // the sync effect retries until the row exists).
      const top = clamped * itemHeight;
      if (top < grid.scrollTop) {
        grid.scrollTop = top;
      } else if (top + itemHeight > grid.scrollTop + grid.clientHeight) {
        grid.scrollTop = top + itemHeight - grid.clientHeight;
      }
    };
    const { key } = event;
    if (key === "ArrowDown") {
      move((activeIndex ?? -1) + 1);
      return;
    }
    if (key === "ArrowUp") {
      move(activeIndex === null ? count - 1 : activeIndex - 1);
      return;
    }
    if (key === "Home") {
      move(0);
      return;
    }
    if (key === "End") {
      move(count - 1);
      return;
    }
    if (key === "Enter" || key === " ") {
      if (activeIndex === null) return;
      event.preventDefault();
      onActivate(activeIndex, grid);
    }
  };

  return { wrapRef, onKeyDown };
}

/**
 * Saved-playlist items → browse entries. Ids are client-local and NEGATIVE:
 * engine PlaylistItemIds are sequential positives, so these can never be
 * mistaken for engine items (there is no play seam for saved names — no
 * dispatch ever carries these ids).
 */
function savedPlaylistEntries(playlist: SavedPlaylist): PlaylistEntry[] {
  return playlist.itemNames.map((name, index) => ({
    id: -(index + 1),
    title: name,
    source: null,
    durationSeconds: null,
  }));
}

/**
 * Router-presence seam for the file menu's Convert / Properties & Metadata
 * entries (PlaylistPanel's NavigationSeam pattern): in-app navigation inside
 * the Router, document navigation in bare mounts (jsdom trees without a
 * Router would crash useNavigate).
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

export function PlaylistScreen() {
  const m = useMsg();
  const t = usePlaylistMsg();
  const { playlists } = usePlaylists();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [subsId, setSubsId] = useState<string | null>(null);

  // Natural order (the 2026-11-09 v2 flow brief fix that must survive the
  // reskin): numbered playlists read in human order.
  const sorted = useMemo(() => naturalSort(playlists, (p) => p.title), [playlists]);

  const opened = openId === null ? null : (playlists.find((p) => p.id === openId) ?? null);
  const subsPlaylist = subsId === null ? null : (playlists.find((p) => p.id === subsId) ?? null);
  const openedEntries = useMemo(() => (opened === null ? [] : savedPlaylistEntries(opened)), [opened]);

  const openCreate = () => setCreateOpen(true);
  const openBrowse = (id: string) => {
    setSelectedId(id);
    setOpenId(id);
  };
  const openSubtitles = (id: string) => setSubsId(id);

  // w26.i10 — the library grid's arrow-key model (h16 a11y): Enter/Space
  // opens the active playlist — the same real seam as the row click and the
  // row menu's Open entry.
  const libraryRoving = useGridRoving({
    count: sorted.length,
    itemHeight: ROW_HEIGHTS.comfortable,
    onActivate: (index) => {
      const target = sorted[index];
      if (target !== undefined) openBrowse(target.id);
    },
  });

  return (
    <main className="lumen-plpage" data-screen="playlist" data-browse={opened !== null || undefined}>
      {opened === null ? (
        <>
          <header className="lumen-plpage__head">
            <h1 className="lumen-plpage__title">{m("nav.playlist")}</h1>
            <p className="lumen-plpage__count">
              {String(sorted.length)} {t("playlist.library.count-suffix")}
            </p>
            <div className="lumen-plpage__actions">
              <Button size="sm" onClick={openCreate}>
                <Icon name="add" size={16} />
                {t("playlist.library.create")}
              </Button>
            </div>
          </header>

          <Card as="section" className="lumen-plpage__panel" aria-label={m("nav.playlist")}>
            {sorted.length === 0 ? (
              <ListEmpty
                className="lumen-plpage__empty"
                title={t("playlist.library.empty-title")}
                body={t("playlist.library.empty-body")}
                action={
                  <Button size="sm" onClick={openCreate}>
                    {t("playlist.library.create")}
                  </Button>
                }
              />
            ) : (
              <div
                ref={libraryRoving.wrapRef}
                onKeyDown={libraryRoving.onKeyDown}
                className="lumen-plpage__listwrap"
              >
                <VirtualList
                  data={sorted}
                  itemHeight={ROW_HEIGHTS.comfortable} /* frame: playlist.svg ~66px rows */
                  label={t("playlist.library.list-label")}
                  className="lumen-plpage__list"
                  semantics="grid"
                  renderItem={(playlist) => (
                    <LibraryRow
                      key={playlist.id}
                      playlist={playlist}
                      selected={playlist.id === selectedId}
                      onSelect={() => openBrowse(playlist.id)}
                      onOpenSubtitles={() => openSubtitles(playlist.id)}
                    />
                  )}
                />
              </div>
            )}
          </Card>
        </>
      ) : (
        <PlaylistOptionsProvider entries={openedEntries}>
          <PlaylistBrowse
            opened={opened}
            otherPlaylists={playlists.filter((p) => p.id !== opened.id)}
            onBack={() => setOpenId(null)}
            onOpenSubtitles={() => openSubtitles(opened.id)}
          />
        </PlaylistOptionsProvider>
      )}

      <NewPlaylistDialog open={createOpen} onClose={() => setCreateOpen(false)} />
      {/* Add-Subtitles flow (w26.f06 F2): scoped to the playlist whose media
       * is in scope. No folder-scan / OpenSubtitles seam exists yet, so the
       * candidate list renders the honest empty state — never fabricated. */}
      <AddSubtitlesDialog
        open={subsPlaylist !== null}
        onClose={() => setSubsId(null)}
        mediaNames={subsPlaylist?.itemNames ?? []}
        initialDestination="playlist"
      />
    </main>
  );
}

/**
 * The opened-playlist BROWSE level (w26.f06 + w26.gfix-E). A child of
 * PlaylistOptionsProvider so the file-row menu can read the options context
 * (metaOf/setWatched) — the provider mounts INSIDE this screen, so the
 * screen component itself cannot call usePlaylistOptions.
 */
function PlaylistBrowse({
  opened,
  otherPlaylists,
  onBack,
  onOpenSubtitles,
}: {
  opened: SavedPlaylist;
  otherPlaylists: readonly SavedPlaylist[];
  onBack: () => void;
  onOpenSubtitles: () => void;
}) {
  const m = useMsg();
  const t = usePlaylistMsg();
  const optionsT = useOptionsMsg();
  const options = usePlaylistOptions();
  const entries = useMemo(() => savedPlaylistEntries(opened), [opened]);
  // The file-scoped Add-Subtitles flow (file context menu row 10): the
  // dialog opens on the ONE file, Folder-2's "To This File" destination.
  const [subsEntryName, setSubsEntryName] = useState<string | null>(null);

  // The Add-to-Playlist submenu targets (dynamic titles — the resolver
  // catalog fallback, c6 lane precedent).
  const targets = useMemo(
    () => new Map(otherPlaylists.map((playlist) => [playlistTargetKey(playlist.id), playlist.title])),
    [otherPlaylists],
  );
  const resolveMenuLabel = useMemo(
    () => fileMenuResolveLabel(optionsT as (key: string) => string, targets),
    [optionsT, targets],
  );

  // w26.i10 — the media grid's arrow-key model (h16 a11y): Enter/Space opens
  // the active row's file context menu — the row's one real seam in the
  // saved-playlist scope (rows are presentation; the ••• is the action). In
  // the thumbnail grid view there is no role=grid — the controller no-ops.
  const viewRoving = useGridRoving({
    count: options.sortedEntries.length,
    itemHeight: MEDIA_ROW_HEIGHT,
    onActivate: (index, grid) => {
      const row = Array.from(grid.querySelectorAll<HTMLElement>('[role="row"]')).find(
        (element) => Math.round(Number.parseFloat(element.style.top) / MEDIA_ROW_HEIGHT) === index,
      );
      row?.querySelector<HTMLElement>(".lumen-plo-mrow__menu")?.click();
    },
  });

  return (
    <>
      <nav className="lumen-plpage__backrow" aria-label={m("nav.playlist")}>
        <Button variant="ghost" size="sm" className="lumen-plpage__back" onClick={onBack}>
          <Icon name="chevron-left" size={16} />
          {t("playlist.browse.back")}
        </Button>
      </nav>

      <header className="lumen-plpage__head">
        <h1 className="lumen-plpage__title">{opened.title}</h1>
        <p className="lumen-plpage__count">
          {String(entries.length)} {t("playlist.library.items-suffix")}
        </p>
        {/* The playlist page toolbar (w26.f06 F1/F2): the real sort + view
            triggers and the Add-subtitles action, one row. */}
        <div className="lumen-plpage__actions">
          <Button size="sm" onClick={onOpenSubtitles}>
            <Icon name="subtitles" size={16} />
            {t("playlist.subtitles.open")}
          </Button>
          <PlaylistOptionsToolbar align="end" />
        </div>
      </header>

      <Card as="section" className="lumen-plpage__panel" aria-label={opened.title}>
        {entries.length === 0 ? (
          <ListEmpty
            className="lumen-plpage__empty"
            title={t("playlist.browse.empty-title")}
            body={t("playlist.browse.empty-body")}
          />
        ) : (
          <NavigationSeam>
            {(navigate) => (
              <div
                ref={viewRoving.wrapRef}
                onKeyDown={viewRoving.onKeyDown}
                className="lumen-plpage__view"
              >
                <PlaylistOptionsViews
                  entryMenu={(entry) =>
                    fileRowMenu({
                      entry,
                      watched: options.metaOf(entry)?.watched === true,
                      otherPlaylists,
                      onSetWatched: (target, watched) => options.setWatched(target, watched),
                      onAddToPlaylist: (playlistId, name) => addItemToPlaylist(playlistId, name),
                      onAddSubtitles: (target) => setSubsEntryName(target.title ?? ""),
                      navigate,
                    })
                  }
                  menuResolveLabel={resolveMenuLabel}
                  menuAriaLabel={optionsT("playlist.file.menu.title")}
                />
              </div>
            )}
          </NavigationSeam>
        )}
      </Card>

      {/* File-scoped Add-Subtitles (file context menu row 10): the same
       * honest empty-candidate state, scoped to the one file. */}
      <AddSubtitlesDialog
        open={subsEntryName !== null}
        onClose={() => setSubsEntryName(null)}
        mediaNames={subsEntryName === null ? [] : [subsEntryName]}
        initialDestination="file"
      />
    </>
  );
}

/** One library row (playlist.svg): playlist glyph + title + hover kebab. */
function LibraryRow({
  playlist,
  selected,
  onSelect,
  onOpenSubtitles,
}: {
  playlist: SavedPlaylist;
  selected: boolean;
  onSelect: () => void;
  onOpenSubtitles: () => void;
}) {
  const t = usePlaylistMsg();
  const wrapRef = useRef<HTMLDivElement>(null);

  // The `•••` opens the SAME context menu as right-click: ListRow's kebab
  // callback carries no pointer event, so the kebab synthesizes a
  // contextmenu at its own screen position for the shared ContextMenu
  // (fixed popup, full keyboard model, focus restore).
  const openMenuAtKebab = () => {
    const el = wrapRef.current;
    if (el === null) return;
    const rect = el.getBoundingClientRect();
    el.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: rect.right - 24,
        clientY: rect.bottom + 4,
      }),
    );
  };

  // Honest entries only: Open (the browse view) and Add subtitles (the
  // AddSubtitlesDialog) are real local seams; Play has no catalog command
  // (disabled); Remove is the real local delete. The full 8-entry playlist
  // context menu.svg port is the wave-c context-menu lane's.
  const entries: MenuEntry<LaneKey>[] = [
    { id: "open", labelKey: "playlist.library.menu.open", icon: "open-folder", onSelect: onSelect },
    { id: "sep", kind: "separator" },
    { id: "play", labelKey: "playlist.library.menu.play", icon: "play", disabled: true },
    {
      id: "add-subtitles",
      labelKey: "playlist.subtitles.open",
      icon: "subtitles",
      onSelect: onOpenSubtitles,
    },
    { id: "sep2", kind: "separator" },
    {
      id: "remove",
      labelKey: "playlist.library.menu.remove",
      icon: "remove",
      onSelect: () => removePlaylist(playlist.id),
    },
  ];

  return (
    <ContextMenu
      ref={wrapRef}
      entries={entries}
      resolveLabel={t}
      ariaLabel={t("playlist.library.menu")}
      density="compact"
      className="lumen-plpage__row-wrap"
    >
      <ListRow
        density="comfortable"
        selected={selected}
        aria-current={selected ? "true" : undefined}
        icon={<Icon name="playlist" size={24} />}
        title={playlist.title}
        kebab={{ label: t("playlist.library.menu"), onClick: openMenuAtKebab }}
        onClick={onSelect}
      />
    </ContextMenu>
  );
}
