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
// The queue panel (PlaylistPanel) stays the PLAYER's docked sidebar; this
// route is the library surface.
//
// Data: playlistsStore (local saved playlists, persisted). Every row action
// is honest — Play in the context menu is disabled until a catalog command
// exists; Remove is the real local delete. The sort/view dropdowns
// (Sorting Options/View Options frames) and the media-rows browse view
// (media.svg) are the wave-b sorting/view + thumbnail/media lanes' — the
// header leaves room under `lumen-plpage__actions`.

import { useMemo, useRef, useState } from "react";
import { useMsg } from "../../i18n/messages";
import { Button } from "../../components/button";
import { Icon } from "../../components/icon";
import { Card } from "../../components/surface";
import { ContextMenu } from "../../components/menu";
import type { MenuEntry } from "../../components/menu";
import { ListRow, ListEmpty, ROW_HEIGHTS, VirtualList, naturalSort } from "../../components/list";
import { removePlaylist, usePlaylists, type SavedPlaylist } from "./playlistsStore";
import { NewPlaylistDialog } from "./NewPlaylistDialog";
import { usePlaylistMsg, type PlaylistLaneKey } from "./usePlaylistMsg";
import "./playlist.css";

/** Lane keys as seen by the row's context menu (same seam, shared hook). */
type LaneKey = PlaylistLaneKey;

export function PlaylistScreen() {
  const m = useMsg();
  const t = usePlaylistMsg();
  const { playlists } = usePlaylists();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);

  // Natural order (the 2026-11-09 v2 flow brief fix that must survive the
  // reskin): numbered playlists read in human order.
  const sorted = useMemo(() => naturalSort(playlists, (p) => p.title), [playlists]);

  const openCreate = () => setCreateOpen(true);

  return (
    <main className="lumen-plpage" data-screen="playlist">
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
          <VirtualList
            data={sorted}
            itemHeight={ROW_HEIGHTS.comfortable} /* frame: playlist.svg ~66px rows */
            label={t("playlist.library.list-label")}
            className="lumen-plpage__list"
            renderItem={(playlist) => (
              <LibraryRow
                key={playlist.id}
                playlist={playlist}
                selected={playlist.id === selectedId}
                onSelect={() => setSelectedId(playlist.id)}
              />
            )}
          />
        )}
      </Card>

      <NewPlaylistDialog open={createOpen} onClose={() => setCreateOpen(false)} />
    </main>
  );
}

/** One library row (playlist.svg): playlist glyph + title + hover kebab. */
function LibraryRow({
  playlist,
  selected,
  onSelect,
}: {
  playlist: SavedPlaylist;
  selected: boolean;
  onSelect: () => void;
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

  // Honest entries only: Play has no catalog command (disabled), Remove is
  // the real local delete. The full 8-entry playlist context menu.svg port
  // is the wave-c context-menu lane's.
  const entries: MenuEntry<LaneKey>[] = [
    { id: "play", labelKey: "playlist.library.menu.play", icon: "play", disabled: true },
    { id: "sep", kind: "separator" },
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
