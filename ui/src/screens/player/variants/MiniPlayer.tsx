// Card 4.F2 — MiniPlayer: the compact always-on-top-style card, composed to
// the W26 frame `design/figma/Miniplayer.svg` (all five artboards):
//   * floating card (r16, surface-panel, 1px hairline, the frame's card
//     shadow) ~340px wide — compact desktop density, not the old 391px
//     Solstice note,
//   * video area r12 + 1px border, 16:9, dark in both themes,
//   * slim 4–6px progress with orange fill + circular thumb (the shared
//     Seekbar grammar, mini-weighted) riding the compact transport row,
//   * title row + meta row (folder chip from the REAL file path + the queue
//     toggle; the frame's person/artist chip stays hidden until the engine
//     reports artist data — honesty rule doc 05 §1 C4),
//   * collapsible Queue panel ABOVE the video (frame artboards 2/4):
//     centered "Queue" header, "N items · H:MM:SS" summary, drag-handle
//     rows with tabular durations, collapse + add affordances, the quiet
//     empty state with the shell Open-media CTA.
//
// Transport is NOT duplicated: the compact cluster reuses card 4.C2's
// control logic (controls/state-mapping.ts mapTransportControls /
// seekDisableReason), the shared IconButton/Seekbar/ListRow primitives and
// the player + playlist contexts (context.tsx / PlaylistProvider) — only
// the COMPACT COMPOSITION is variant-owned (variantLayout.ts). Geometry
// lives in variants.css with semantic tokens for every color; raw px only
// for frame-measured structural dimensions (marked `frame:`).

import { useState, type DragEvent } from "react";
import { Icon, IconButton, ListEmpty, ListRow, Seekbar } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.a16";
import { OPEN_MEDIA_EVENT } from "../../open-media";
import { formatDuration, dragToMoveIndex, usePlaylist } from "../../playlist";
import type { PlaylistEntry } from "../../playlist";
import { usePlayerControls } from "../context";
import { mapTransportControls, seekDisableReason } from "../controls";
import type { LayoutRegion } from "../layout";
import "./variants.css";

export interface MiniPlayerProps {
  /** The compact canvas region (mini-thumbnail from the miniplayer layout). */
  readonly thumbnailRegion: LayoutRegion;
  /** The compact transport region (mini-controls from the miniplayer layout). */
  readonly controlsRegion: LayoutRegion;
  /** Expand back to the full variant (variant-switch seam). */
  readonly onExpand: () => void;
  /**
   * Close affordance — host/shell-owned. Renders only when provided (an
   * unwired affordance would be a fake button; doc 05 §1 C4).
   */
  readonly onClose?: () => void;
}

/** Lane keys not yet registered in the catalog (contract §4.5 — Lead merges). */
type LaneKey = keyof typeof laneKeys;

/**
 * `useMsg` extended with this lane's pre-registration keys: lane keys win
 * while unregistered; once the Lead merges them into the catalog both paths
 * return the same string. No user-visible literals (same seam as w26.a10).
 */
function useLaneMsg() {
  const m = useMsg();
  const laneCatalog = laneKeys as Record<string, string>;
  return (key: MessageKey | LaneKey): string => laneCatalog[key] ?? m(key as MessageKey);
}

/** DOM id of the collapsible queue panel (the toggle's aria-controls). */
const QUEUE_ID = "lumen-mini-queue";

/** Open the app-shell media dialog (cross-screen contract, never renamed). */
const openMediaDialog = (): void => {
  window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT));
};

/** Containing folder of a file source ("/a/b/clip.mp4" → "b"); null when
 * the source is not a file or has no parent segment (honest hide). */
function folderOf(entry: PlaylistEntry): string | null {
  if (entry.source === null || entry.source.kind !== "file") return null;
  const segments = entry.source.path.split(/[\\/]/).filter((s) => s.length > 0);
  return segments.length >= 2 ? (segments[segments.length - 2] as string) : null;
}

export function MiniPlayer({
  thumbnailRegion,
  controlsRegion,
  onExpand,
  onClose,
}: MiniPlayerProps) {
  const m = useMsg();
  const t = useLaneMsg();
  const { state, actions } = usePlayerControls();
  const playlist = usePlaylist();
  const [queueOpen, setQueueOpen] = useState(false);
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

  const playLabel =
    mapped.play.glyph === "pause" ? m("player.controls.pause") : m("player.controls.play");

  // Real data only: engine-reported title first, the current playlist
  // entry's title second; null renders the honest quiet empty.
  const current = playlist.entries.find((entry) => entry.id === playlist.currentId) ?? null;
  const title = state.mediaTitle ?? current?.title ?? null;
  const folder = current === null ? null : folderOf(current);

  return (
    <section
      className="lumen-mini"
      data-variant="miniplayer"
      role="region"
      aria-label={m("player.variant.miniplayer.label")}
    >
      {/* Collapsible queue panel ABOVE the video (Miniplayer.svg artboards
          2/4). Renders only while opened — the meta-row toggle owns it. */}
      {queueOpen ? (
        <MiniQueue
          id={QUEUE_ID}
          onCollapse={() => setQueueOpen(false)}
          onAdd={openMediaDialog}
        />
      ) : null}

      <div
        data-region-id={thumbnailRegion.id}
        role="region"
        aria-label={m(thumbnailRegion.labelKey)}
        className="lumen-mini__video"
        data-placeholder="engine-wiring-4.H1"
      >
        {/* Honest placeholder: the thumbnail is the video-canvas seam; no
            media element until card 4.H1 (same contract as the full shell). */}
        <p className="lumen-player__sr-note">{m("player.canvas.placeholder")}</p>
      </div>

      {/* The controls region wraps the whole lower card in the frame's
          vertical order — progress row · title row · meta row · compact
          transport (Miniplayer.svg Default: video → progress → title →
          meta; the transport row is this shell's honest always-available
          cluster — the frame's expanded state reveals the same controls as
          video overlays). */}
      <footer
        data-region-id={controlsRegion.id}
        role="region"
        aria-label={m(controlsRegion.labelKey)}
        className="lumen-mini__controls"
      >
        <Seekbar
          data-control="seekbar"
          data-variant="mini"
          className="lumen-mini__seek"
          /* same calm-unplayed-track rule as the full bar — no phantom
           * zero thumb while there is no duration to map onto. */
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

        {/* Title row (frame: filename, 13px, primary tone, one ellipsised
            line). Null title renders the honest quiet empty — never a fake. */}
        <h2 className="lumen-mini__title" data-empty={title === null ? "" : undefined}>
          {title ?? t("player.mini.title-empty")}
        </h2>

        {/* Meta row (frame: folder chip + person chip left, queue toggle
            right). The folder chip is REAL data (the current entry's parent
            folder); the frame's person/artist chip renders only once the
            engine reports artist metadata — no fabricated show/artist. */}
        <div className="lumen-mini__meta">
          {folder === null ? null : (
            <span className="lumen-mini__meta-chip">
              <Icon name="folder" size={14} />
              <span className="lumen-mini__meta-text">{folder}</span>
            </span>
          )}
          <IconButton
            size="sm"
            data-control="queue-toggle"
            data-active={queueOpen ? "" : undefined}
            className="lumen-mini__queue-toggle"
            label={t("player.mini.queue.toggle")}
            title={t("player.mini.queue.toggle")}
            aria-expanded={queueOpen}
            aria-controls={QUEUE_ID}
            onClick={() => setQueueOpen((open) => !open)}
          >
            <Icon name="list" size={16} />
          </IconButton>
        </div>

        <div className="lumen-mini__transport">
          <IconButton
            data-control="play-pause"
            data-glyph={mapped.play.glyph}
            label={playLabel}
            title={playLabel}
            loading={mapped.play.pending}
            disabled={mapped.play.disabled}
            onClick={actions.togglePlayPause}
            className="lumen-mini__play"
          >
            <Icon name={mapped.play.glyph} size={16} />
          </IconButton>
          <IconButton
            data-control="expand"
            label={m("player.variant.miniplayer.expand")}
            title={m("player.variant.miniplayer.expand")}
            onClick={onExpand}
          >
            <Icon name="fullscreen" size={16} />
          </IconButton>
          {onClose === undefined ? null : (
            <IconButton
              data-control="close"
              label={m("player.variant.miniplayer.close")}
              title={m("player.variant.miniplayer.close")}
              onClick={onClose}
            >
              <Icon name="close" size={16} />
            </IconButton>
          )}
        </div>
      </footer>
    </section>
  );
}

/**
 * The collapsible queue panel (Miniplayer.svg "Queue" artboard): centered
 * header, "N items · total" summary row with collapse/add affordances, and
 * drag-handle rows with tabular durations. All state is playlist truth
 * (entries/currentId via usePlaylist); drag-to-reorder uses the same
 * `playlist.move` semantics as the full playlist panel (card 4.D1).
 */
function MiniQueue({
  id,
  onCollapse,
  onAdd,
}: {
  /** The toggle's aria-controls target. */
  readonly id: string;
  /** Collapse back into the card (the meta-row toggle's partner). */
  readonly onCollapse: () => void;
  /** Add media — the shell's open-media dialog (the frame's + affordance). */
  readonly onAdd: () => void;
}) {
  const m = useMsg();
  const t = useLaneMsg();
  const playlist = usePlaylist();
  const [dragFrom, setDragFrom] = useState<number | null>(null);

  const knownDurations = playlist.entries
    .map((entry) => entry.durationSeconds)
    .filter((seconds): seconds is number => seconds !== null && seconds > 0);
  const total = knownDurations.reduce((sum, seconds) => sum + seconds, 0);

  const onRowDrop = (event: DragEvent<HTMLDivElement>, beforeIndex: number): void => {
    event.preventDefault();
    const from = dragFrom ?? Number(event.dataTransfer.getData("text/plain"));
    setDragFrom(null);
    if (!Number.isInteger(from) || from === beforeIndex) return;
    const entry = playlist.entries[from];
    if (!entry) return;
    playlist.move(entry.id, dragToMoveIndex(beforeIndex, from, playlist.entries.length));
  };

  return (
    <div className="lumen-mini__queue" id={id} role="region" aria-label={t("player.mini.queue.label")}>
      <div className="lumen-mini__queue-head">
        <h3 className="lumen-mini__queue-title">{t("player.mini.queue.label")}</h3>
      </div>
      <div className="lumen-mini__queue-meta">
        <span className="lumen-mini__queue-summary">
          {`${playlist.entries.length} ${m("playlist.panel.count-suffix")}${
            total > 0 ? ` · ${formatDuration(total) ?? ""}` : ""
          }`}
        </span>
        <div className="lumen-mini__queue-tools">
          {/* The frame's sort affordance has no playlist command behind it
              yet (feature-not-exposed) — omitted rather than faked; the
              collapse (—) and add (+) affordances are real. */}
          <IconButton
            size="sm"
            label={t("player.mini.queue.toggle")}
            title={t("player.mini.queue.toggle")}
            onClick={onCollapse}
          >
            <Icon name="chevron-up" size={16} />
          </IconButton>
          <IconButton
            size="sm"
            label={m("player.empty.open")}
            title={m("player.empty.open")}
            onClick={onAdd}
          >
            <Icon name="add" size={16} />
          </IconButton>
        </div>
      </div>
      {playlist.entries.length === 0 ? (
        <ListEmpty
          className="lumen-mini__queue-empty"
          title={m("playlist.panel.empty-title")}
          body={m("playlist.panel.empty-body")}
        />
      ) : (
        <div
          className="lumen-mini__queue-list"
          role="listbox"
          aria-label={t("player.mini.queue.label")}
        >
          {playlist.entries.map((entry, index) => (
            <ListRow
              key={entry.id}
              role="option"
              aria-selected={entry.id === playlist.currentId}
              density="compact"
              selected={entry.id === playlist.currentId}
              title={entry.title ?? m("playlist.entry.unknown-title")}
              duration={formatDuration(entry.durationSeconds) ?? m("playlist.entry.unknown-duration")}
              draggable
              onDragStart={(event) => {
                event.dataTransfer.setData("text/plain", String(index));
                setDragFrom(index);
              }}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => onRowDrop(event, index)}
              onClick={() => playlist.playItem(entry.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
