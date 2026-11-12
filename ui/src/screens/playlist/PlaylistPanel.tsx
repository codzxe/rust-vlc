// Card 4.D1 → w26.b09 — playlist QUEUE panel content, rebuilt on the list
// primitives (ListRow/ListEmpty/ROW_HEIGHTS — logs/w26.a09).
//
// Rendered inside the player shell's `playlist` dock (PlayerScreen wiring —
// a13's 216px deep-surface sidebar). W26 grammar (Miniplayer.svg queue +
// media.svg rows): drag-handle rows · title + source location · tabular
// duration · hover-revealed actions overlaying the row's trailing edge
// (never squeezing the title in the narrow dock). The 2026-11-09 v2 row
// markup (grip dots, index column, sun disc, chip) is retired.
//
// HONESTY: current marker, shuffle and repeat reflect ENGINE-reported state
// only (playlist.changed / snapshot). `null` (not yet reported) renders as
// the unpressed control — commented at each site; never a faked "known off".
//
// w26.c6 — QUEUE-ROW CONTEXT MENU (playlist context menu.svg, 8 icon rows:
// play · heart · shuffle arrows · save · add-to-queue · subtitle card ·
// waveform · info, every row two-tone #8F8F8F + #AB6314 → `iconAccent`).
// The frame's labels are "Option" placeholders, so row semantics come from
// the pinned icons. Honest wiring per contract §5: real seams only —
// Play (playlist.play-item), Shuffle (playlist.set-shuffle, engine state),
// Add media… (the shell-wide open-media event), Add subtitle tracks (the
// AddSubtitlesDialog, w26.f06) and Media info (the /media-info route).
// Favorite / Save playlist / Add audio tracks have NO seam in this build
// (no favorites store, no queue-export command, no per-item track-add)
// and render disabled — never theater (PlaylistScreen's honest-Play
// precedent). Right-click coverage spans the row's content box — the
// .lumen-vrow > wrapper fill is scoped in playlist.css (w26.f06) until the
// menu lane ships the shared list.css rule.

import { useCallback, useState, type DragEvent, type ReactNode } from "react";
import { useInRouterContext, useNavigate } from "react-router-dom";
import { useMsg, type MessageKey } from "../../i18n/messages";
import { keys as c6Keys } from "../../i18n/keys/w26.c6";
import { IconButton, Button } from "../../components/button";
import { Icon } from "../../components/icon";
import { ContextMenu } from "../../components/menu";
import type { MenuEntry } from "../../components/menu";
import { ListEmpty, ListRow, ROW_HEIGHTS, VirtualList } from "../../components/list";
import { OPEN_MEDIA_EVENT } from "../open-media";
import { usePlaylist, type PlaylistContextValue, type PlaylistEntry } from "./PlaylistProvider";
import { AddSubtitlesDialog } from "./subtitles/AddSubtitlesDialog";
import { entryDisplayTitle } from "./options/sorting";
import { cycleRepeat, dragToMoveIndex, formatDuration, sourceLocation } from "./queue";
import "./playlist.css";

// ---------------------------------------------------------------------------
// w26.c6 lane i18n seam (contract §4.5) — same shape as usePlaylistMsg
// ---------------------------------------------------------------------------

/** Label keys the queue-row menu can carry (c6 lane keys + reused catalog). */
type QueueMenuKey = keyof typeof c6Keys | "playlist.controls.shuffle" | "menu.view.media-info";

const c6Catalog: Readonly<Record<string, string>> = c6Keys;

/** useMsg extended with the c6 lane's pre-registration keys (catalog first,
 * lane copy as the fallback — identical before/after registration). */
function useQueueMenuMsg() {
  const m = useMsg();
  return useCallback(
    (key: QueueMenuKey): string => m(key as MessageKey) ?? c6Catalog[key],
    [m],
  );
}

/**
 * Router-presence seam for the menu's Media info entry (ControlBar's
 * NavigationSeam pattern): in-app navigation inside the Router, document
 * navigation in bare mounts (embedded hosts / jsdom trees without a Router
 * would crash useNavigate). Split keeps hook calls unconditional.
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

/** Queue toolbar glyphs — 16px line glyphs, 28px `sm` hit (W26 frames). */
const TOOLBAR_ICON_SIZE = 16;

/** Open the app-shell media dialog (cross-screen contract, never renamed). */
const openMediaDialog = (): void => {
  window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT));
};

export function PlaylistPanel({
  className = "",
  /** Hide the panel's own h2 (a host page that renders its own heading).
   * Kept in the public API (wave-1 freeze); the player dock shows it. */
  headingless = false,
}: {
  className?: string;
  headingless?: boolean;
}) {
  const m = useMsg();
  const playlist = usePlaylist();
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  // w26.f06 (e09 F2): the queue-row "Add subtitle tracks" entry opens the
  // Add-Subtitles flow scoped to the queue's media names.
  const [subsOpen, setSubsOpen] = useState(false);

  const ids = playlist.entries.map((e) => e.id);
  const currentIndex = playlist.currentId === null ? -1 : ids.indexOf(playlist.currentId);
  // Presentation mode for the repeat cycle control; state itself stays
  // `null`-honest in the provider.
  const repeatMode = playlist.repeat ?? "off";
  const repeatLabel =
    repeatMode === "one" ? m("playlist.controls.repeat.one") : repeatMode === "all" ? m("playlist.controls.repeat.all") : m("playlist.controls.repeat.off");

  const onRowDrop = (event: DragEvent<HTMLDivElement>, beforeIndex: number) => {
    event.preventDefault();
    const from = dragFrom ?? Number(event.dataTransfer.getData("text/plain"));
    setDragFrom(null);
    if (!Number.isInteger(from) || from === beforeIndex) return;
    const entry = playlist.entries[from];
    if (!entry) return;
    playlist.move(entry.id, dragToMoveIndex(beforeIndex, from, playlist.entries.length));
  };

  return (
    <section
      className={`lumen-playlist ${className}`.trim()}
      data-chrome-part=""
      aria-label={headingless ? m("playlist.panel.title") : undefined}
    >
      <header className="lumen-playlist__header">
        {headingless ? null : <h2 className="lumen-playlist__title">{m("playlist.panel.title")}</h2>}
        <span className="lumen-playlist__count">
          {String(playlist.entries.length)} {m("playlist.panel.count-suffix")}
        </span>
        <div className="lumen-playlist__controls">
          <IconButton label={m("playlist.controls.prev")} size="sm" onClick={() => playlist.prev()}>
            <Icon name="skip-back-15" size={TOOLBAR_ICON_SIZE} />
          </IconButton>
          <IconButton label={m("playlist.controls.next")} size="sm" onClick={() => playlist.next()}>
            <Icon name="skip-forward-15" size={TOOLBAR_ICON_SIZE} />
          </IconButton>
          {/* aria-pressed only claims "on" when the engine reported it;
              unknown (null) renders unpressed — never a faked known-off. */}
          <IconButton
            label={m("playlist.controls.shuffle")}
            size="sm"
            aria-pressed={playlist.shuffle === true}
            onClick={() => playlist.setShuffle(!(playlist.shuffle === true))}
          >
            <Icon name="shuffle" size={TOOLBAR_ICON_SIZE} />
          </IconButton>
          <IconButton
            label={repeatLabel}
            size="sm"
            data-mode={repeatMode}
            onClick={() => playlist.setRepeat(cycleRepeat(repeatMode))}
          >
            <Icon name="repeat" size={TOOLBAR_ICON_SIZE} />
          </IconButton>
          <IconButton label={m("playlist.controls.clear")} size="sm" onClick={() => playlist.clear()}>
            <Icon name="stop" size={TOOLBAR_ICON_SIZE} />
          </IconButton>
        </div>
      </header>

      {/* Honesty line — plain quiet caption (no decorative chip);
          never a fake-known state. */}
      {playlist.notExposed ? <p className="lumen-playlist__note">{m("playlist.panel.not-exposed")}</p> : null}

      {playlist.entries.length === 0 ? (
        <ListEmpty
          className="lumen-playlist__empty"
          title={m("playlist.panel.empty-title")}
          body={m("playlist.panel.empty-body")}
          action={
            /* The empty state teaches: same dialog the player's empty state
             * opens (AppShell owns the single listener for this event). */
            <Button
              variant="primary"
              onClick={openMediaDialog}
              data-testid="playlist-empty-open-media"
            >
              {m("player.empty.open")}
            </Button>
          }
        />
      ) : (
        <VirtualList
          data={playlist.entries}
          itemHeight={ROW_HEIGHTS.compact} /* Miniplayer.svg queue pitch — the dock's density */
          label={m("playlist.panel.title")}
          scrollToIndex={currentIndex >= 0 ? { index: currentIndex, align: "start" } : null}
          className="lumen-playlist__list"
          renderItem={(entry, index) => (
            <PlaylistRow
              key={entry.id}
              entry={entry}
              index={index}
              isCurrent={entry.id === playlist.currentId}
              onAddSubtitles={() => setSubsOpen(true)}
              onDragStart={(e) => {
                e.dataTransfer.setData("text/plain", String(index));
                setDragFrom(index);
              }}
              onDrop={(e) => onRowDrop(e, index)}
            />
          )}
        />
      )}

      <footer className="lumen-playlist__footer">{m("playlist.panel.load-handoff")}</footer>

      {/* Add-Subtitles flow (w26.f06, e09 F2): scoped to the queue's media
       * names. No folder-scan / OpenSubtitles seam exists yet, so candidates
       * render the honest empty state — never fabricated. */}
      <AddSubtitlesDialog
        open={subsOpen}
        onClose={() => setSubsOpen(false)}
        mediaNames={playlist.entries.map((entry) => entryDisplayTitle(entry, m("playlist.entry.unknown-title")))}
      />
    </section>
  );
}

function PlaylistRow({
  entry,
  index,
  isCurrent,
  onAddSubtitles,
  onDragStart,
  onDrop,
}: {
  entry: PlaylistEntry;
  index: number;
  isCurrent: boolean;
  onAddSubtitles: () => void;
  onDragStart: (event: DragEvent<HTMLDivElement>) => void;
  onDrop: (event: DragEvent<HTMLDivElement>) => void;
}) {
  const m = useMsg();
  const t = useQueueMenuMsg();
  const playlist = usePlaylist();
  const duration = formatDuration(entry.durationSeconds);
  const location = sourceLocation(entry.source);
  // Queue semantics on an EXISTING id (stable ids, doc 03 §4): play-next /
  // play-later are `playlist.move`s — inserting right after the current item
  // / appending — never a re-enqueue (that would duplicate the entry).
  const playNextIndex = dragToMoveIndex(
    (playlist.currentId === null ? -1 : playlist.entries.findIndex((e) => e.id === playlist.currentId)) + 1,
    index,
    playlist.entries.length,
  );
  return (
    <NavigationSeam>
      {(navigate) => (
        <ContextMenu
          entries={queueRowMenu({ playlist, entry, navigate, onAddSubtitles })}
          resolveLabel={t}
          ariaLabel={t("playlist.queue.menu.title")}
        >
          <ListRow
            className={`lumen-plrow${isCurrent ? " lumen-plrow--current" : ""}`}
            density="compact"
            draggable
            selected={isCurrent}
            data-current={isCurrent || undefined}
            onDragStart={onDragStart}
            onDragOver={(e) => e.preventDefault()}
            onDrop={onDrop}
            icon={isCurrent ? <Icon name="play" size={14} /> : undefined}
            title={entry.title ?? m("playlist.entry.unknown-title")}
            meta={location ?? undefined}
            duration={duration ?? m("playlist.entry.unknown-duration")}
            actions={
              <>
                <IconButton label={m("playlist.entry.play-now")} size="sm" onClick={() => playlist.playItem(entry.id)}>
                  <Icon name="play" size={14} />
                </IconButton>
                <Button size="sm" onClick={() => playlist.move(entry.id, playNextIndex)}>
                  {m("playlist.entry.play-next")}
                </Button>
                <Button
                  size="sm"
                  onClick={() => playlist.move(entry.id, dragToMoveIndex(playlist.entries.length, index, playlist.entries.length))}
                >
                  {m("playlist.entry.play-later")}
                </Button>
                <IconButton label={m("playlist.entry.remove")} size="sm" onClick={() => playlist.remove(entry.id)}>
                  <Icon name="quit" size={14} />
                </IconButton>
              </>
            }
          >
            {isCurrent ? <span className="lumen-playlist__sr">{m("playlist.entry.current")}</span> : null}
          </ListRow>
        </ContextMenu>
      )}
    </NavigationSeam>
  );
}

/**
 * The queue-row context menu (playlist context menu.svg — 8 rows, pinned
 * icon order; labels are frame placeholders, semantics from the icons).
 * Seams only: real commands are enabled, no-seam rows are disabled — never
 * theater (PlaylistScreen's honest-Play precedent).
 */
function queueRowMenu({
  playlist,
  entry,
  navigate,
  onAddSubtitles,
}: {
  playlist: PlaylistContextValue;
  entry: PlaylistEntry;
  navigate: (to: string) => void;
  onAddSubtitles: () => void;
}): MenuEntry<QueueMenuKey>[] {
  return [
    // 1 — Play (frame: orange play triangle) → the real play-item command.
    {
      id: "play",
      labelKey: "playlist.queue.menu.play",
      icon: "play",
      iconAccent: true,
      onSelect: () => playlist.playItem(entry.id),
    },
    // 2 — Favorite (frame: orange heart). No favorites store exists in this
    // build — disabled, never a fake toggle.
    { id: "favorite", labelKey: "playlist.queue.menu.favorite", icon: "favorite", iconAccent: true, disabled: true },
    // 3 — Shuffle (frame: two-tone crossing arrows) → the real set-shuffle
    // command. Checkbox face is engine truth only: null (not reported)
    // reads unchecked — the panel header's honesty rule.
    {
      id: "shuffle",
      labelKey: "playlist.controls.shuffle",
      icon: "shuffle",
      iconAccent: true,
      kind: "checkbox",
      checked: playlist.shuffle === true,
      onSelect: () => playlist.setShuffle(!(playlist.shuffle === true)),
    },
    // 4 — Save playlist (frame: floppy + orange label). No queue-export
    // command exists (saved playlists are the /playlist dialog's local
    // seam and need a user title) — disabled.
    { id: "save", labelKey: "playlist.queue.menu.save", icon: "save", iconAccent: true, disabled: true },
    // 5 — Add media… (frame: orange pill + lines = add-to-queue) → the
    // shell-wide open-media event (AppShell owns the dialog listener).
    {
      id: "add-media",
      labelKey: "playlist.queue.menu.add-media",
      icon: "queue",
      iconAccent: true,
      onSelect: () => window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT)),
    },
    // 6 — Add subtitle tracks (frame: subtitle card + orange dashes) → the
    // Add-Subtitles flow scoped to the queue's media names (w26.f06, e09 F2
    // — the dialog itself is the b11 lane's frame-exact build).
    {
      id: "add-subtitles",
      labelKey: "playlist.queue.menu.add-subtitles",
      icon: "subtitles",
      iconAccent: true,
      onSelect: onAddSubtitles,
    },
    // 7 — Add audio tracks (frame: two-tone waveform). No per-item
    // track-add seam in the catalog — disabled.
    { id: "add-audio", labelKey: "playlist.queue.menu.add-audio", icon: "audio", iconAccent: true, disabled: true },
    // 8 — Media info (frame: info circle + orange dot) → the real route.
    {
      id: "info",
      labelKey: "menu.view.media-info",
      icon: "info",
      iconAccent: true,
      onSelect: () => navigate("/media-info"),
    },
  ];
}
