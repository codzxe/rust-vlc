// w26.i02 (Lead ruling, wave I relaunch #4) — the canvas's PLAYLIST BROWSE
// face: the Player-4 / Player-3 / Player-1/-2 composition split, resolved.
//
// PLAYER-4 (grid + bar): the frame shows a saved playlist's media as a grid
// of 316×178 r12 thumbnail cards with filename captions over the #0E0E0E
// main surface, with the transport bar spanning the window bottom — in this
// app the bar is the shell STRIP PORTAL (PlayerScreen's full-width control
// band, player.css), and the playlist grid sits above it in the canvas.
// PlayerScreen mounts this face when a dock row opens a saved playlist
// (the dock is the playlists browser — h03 #1); the exit affordance in the
// header returns to the player canvas.
//
// HONESTY (doc 05 §1 C4, the gfix-C home-card precedent): the saved
// playlists carry file NAMES only (playlistsStore — no sources, no play
// seam, no resume positions), so the cards render as honest presentation
// (the ThumbnailGrid inert-card precedent) and the frame's 4px #D37918
// watched-progress bars render NOWHERE — no engine seam exists for resume
// positions in any transport, and a fabricated bar would be theater.
//
// PLAYER-3 / PLAYER-1 (the floating window): while a session is up over the
// browse face, the frame's 391×339 r14 preview card floats bottom-right
// ABOVE the band — the now-playing window. The card is the frame's own
// anatomy (Player-3.svg preview card ≡ Miniplayer.svg Default family):
// 373×209 r5 art box (honest placeholder — the decode half stays 4.H1), the
// 375×16 r6 #090909 seek well with #371F06 fill (the REAL engine position —
// the shared Seekbar, mini-weighted), the title line (engine truth first,
// playlist truth second), the folder chip (real file data), and the ✕ that
// collapses the card (the header's PiP toggle re-opens it — both states of
// the ruling stay reachable). PlayerScreen keeps the <video> media surface
// mounted UNDER the opaque grid, so url sources keep their real bytes and
// audio flowing while the user browses — never a silently stalled engine.

import { useMemo } from "react";
import { Icon, IconButton, Seekbar } from "../../../components";
import { naturalSort } from "../../../components/list";
import { useMsg } from "../../../i18n/messages";
import { keys as i02Keys } from "../../../i18n/keys/w26.i02";
import { usePlaylist, type PlaylistEntry } from "../../playlist";
import { usePlayerControls } from "../context";
import { useLaneMsg } from "../laneMsg";
import { mapTransportControls, seekDisableReason } from "../controls";

/** The saved-playlist record the browse face renders (playlistsStore data). */
export interface BrowsePlaylistData {
  readonly id: string;
  readonly title: string;
  readonly itemNames: readonly string[];
}

// w26.k06 (J06-9): the lane resolver is the shared player helper
// (../laneMsg) — this file's local copy is gone.

/** Containing folder of a file source ("/a/b/clip.mp4" → "b"); null when the
 * source is not a file or has no parent segment (honest hide). MiniPlayer's
 * helper, restated locally (it is not exported from variants/). */
function folderOf(entry: PlaylistEntry): string | null {
  if (entry.source === null || entry.source.kind !== "file") return null;
  const segments = entry.source.path.split(/[\\/]/).filter((s) => s.length > 0);
  return segments.length >= 2 ? (segments[segments.length - 2] as string) : null;
}

export interface PlaylistBrowseProps {
  /** The saved playlist being browsed (real store data — never fabricated). */
  readonly playlist: BrowsePlaylistData;
  /** PlayerScreen's media truth (host-injected or engine-derived). */
  readonly hasMedia: boolean;
  /** Whether the floating now-playing card is open (Player-3). */
  readonly previewOpen: boolean;
  /** Toggle the floating card (the header affordance + the card's ✕). */
  readonly onPreviewToggle: (open: boolean) => void;
  /** Leave the browse face — back to the player canvas. */
  readonly onExit: () => void;
}

/** Player-4's grid face + Player-3's floating now-playing card. */
export function PlaylistBrowse({
  playlist,
  hasMedia,
  previewOpen,
  onPreviewToggle,
  onExit,
}: PlaylistBrowseProps) {
  const tm = useLaneMsg(i02Keys);

  // Natural order — the app-wide library order ("S01E2" before "S01E10"),
  // the same sort the dock rows and /playlist read.
  const rows = useMemo(
    () => naturalSort(playlist.itemNames, (name) => name),
    [playlist.itemNames],
  );

  return (
    <div className="lumen-state-browse" data-browse={playlist.id}>
      {/* Header — the frame's browse chrome adapted to the canvas (Player-4
          carries the title/count in the library sidebar; this dock is the
          playlists browser, so the canvas header owns the context). Exit
          chevron + title + "N items" left; the PiP toggle right — disabled
          with its reason while no session is up (never a dead button). */}
      <header className="lumen-state-browse__head">
        <IconButton
          className="lumen-state-browse__exit"
          label={tm("player.browse.exit")}
          size="sm"
          onClick={onExit}
        >
          <Icon name="chevron-left" size={16} aria-hidden="true" />
        </IconButton>
        <h2 className="lumen-state-browse__title">{playlist.title}</h2>
        <span className="lumen-state-browse__count">
          {String(rows.length)} {tm("player.dock.count-suffix")}
        </span>
        <IconButton
          className="lumen-state-browse__preview-toggle"
          label={tm("player.preview.show")}
          title={hasMedia ? tm("player.preview.show") : tm("player.preview.show.hint")}
          size="sm"
          aria-pressed={previewOpen && hasMedia}
          disabled={!hasMedia}
          onClick={() => onPreviewToggle(!previewOpen)}
        >
          <Icon name="pip" size={16} aria-hidden="true" />
        </IconButton>
      </header>

      {rows.length === 0 ? (
        <p className="lumen-state-browse__empty">{tm("player.browse.empty")}</p>
      ) : (
        <ul className="lumen-state-browse__grid" aria-label={tm("player.browse.label")}>
          {rows.map((name, index) => (
            // w26.k06 (J06-7, P2): saved playlists carry NAMES only (store
            // data, no uniqueness guarantee) — a name-only key collides on
            // duplicate filenames; the index keeps siblings distinct.
            <li key={`${index}:${name}`} className="lumen-state-browse__cell">
              {/* Honest presentation, never a dead button: names-only local
                  data has no play seam (playlistsStore; the ThumbnailGrid
                  inert-card precedent). */}
              <div className="lumen-state-browse__card" title={name}>
                <span className="lumen-state-browse__card-art" aria-hidden="true">
                  <Icon name="thumbnail" />
                </span>
              </div>
              <span className="lumen-state-browse__card-title">{name}</span>
            </li>
          ))}
        </ul>
      )}

      {/* Player-3's floating window — mounted only while a session is up
          and the user has not collapsed it. */}
      {hasMedia && previewOpen ? (
        <NowPlayingCard onHide={() => onPreviewToggle(false)} />
      ) : null}
    </div>
  );
}

/** The floating now-playing card (Player-3.svg's preview card). All data is
 * engine/playlist truth (the same seams MiniPlayer reads); the art box is
 * the honest 4.H1 placeholder — the canvas's <video> surface (still mounted
 * under the grid) is the one real media element. */
function NowPlayingCard({ onHide }: { onHide: () => void }) {
  const m = useMsg();
  const tm = useLaneMsg(i02Keys);
  const { state, actions } = usePlayerControls();
  const playlist = usePlaylist();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

  // Real data only: engine-reported title first, the current playlist
  // entry's title second (MiniPlayer's order); null renders the honest
  // quiet empty — never a fake.
  const current = playlist.entries.find((entry) => entry.id === playlist.currentId) ?? null;
  const title = state.mediaTitle ?? current?.title ?? null;
  const folder = current === null ? null : folderOf(current);
  const hasVideo = state.tracks.some((track) => track.kind === "video");

  return (
    <section
      className="lumen-state-preview"
      data-chrome-part=""
      role="region"
      aria-label={tm("player.preview.label")}
    >
      <div className="lumen-state-preview__video" data-placeholder="engine-wiring-4.H1">
        {/* Honest placeholder note: the decode/frames half stays
            engine-wired nowhere (card 4.H1 handoff — same contract as the
            full canvas and the miniplayer card). */}
        <p className="lumen-player__sr-note">{m("player.canvas.placeholder")}</p>
        <span className="lumen-state-preview__glyph" aria-hidden="true">
          <Icon name={hasVideo ? "play" : "music"} size={24} />
        </span>
      </div>

      <Seekbar
        data-control="seekbar"
        data-variant="mini"
        className="lumen-state-preview__seek"
        /* calm unplayed track — no phantom zero thumb/fill while there is
         * no duration to map onto (the mini bar's honesty rule). */
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

      <h2 className="lumen-state-preview__title" data-empty={title === null ? "" : undefined}>
        {title ?? tm("player.preview.title-empty")}
      </h2>

      <div className="lumen-state-preview__meta">
        {folder === null ? null : (
          <span className="lumen-state-preview__chip">
            <Icon name="folder" size={14} aria-hidden="true" />
            <span className="lumen-state-preview__chip-text">{folder}</span>
          </span>
        )}
        <IconButton
          className="lumen-state-preview__hide"
          size="sm"
          label={tm("player.preview.hide")}
          onClick={onHide}
        >
          <Icon name="close" size={16} aria-hidden="true" />
        </IconButton>
      </div>
    </section>
  );
}
