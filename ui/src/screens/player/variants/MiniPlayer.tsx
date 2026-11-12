// Card 4.F2 — MiniPlayer: the compact always-on-top-style card, composed to
// the W26 frame `design/figma/Miniplayer.svg` (all five artboards — card
// geometry re-pinned to the frame by w26.f11, e04 F3):
//   * floating card 391px wide, rx14 (#1B1B1B in the frame — the semantic
//     surface-panel), 1px hairline, the frame's card shadow — identical on
//     every artboard (default 391×339, queue-open 391×689.7),
//   * video area (frame: 373×209 rx5 at a 9px inset) + 1px border, 16:9,
//     dark in both themes,
//   * slim seek row DIRECTLY under the video (frame: 375×16 rx6 #090909
//     well bleeding 1px past the video on both sides, #371F06 played fill
//     inset 2, 8×12 rx4 accent thumb),
//   * title row + meta row (folder chip from the REAL file path + the queue
//     toggle; the frame's person/artist chip stays hidden until the engine
//     reports artist data — honesty rule doc 05 §1 C4),
//   * the compact transport cluster rides as an OVERLAY on the video area
//     (Miniplayer.svg playing artboards: close top-left / expand top-right
//     corners + a centered prev · play · next row at video mid-height —
//     skip glyph centers ±75px from the play center, w26.i04) — dimmed at
//     rest, lit on hover/focus, never fake: the buttons keep C2's honest
//     engine-state mapping,
//   * collapsible Queue panel ABOVE the video (frame artboards 2/4):
//     centered "Queue" header, "N items · H:MM:SS" summary, drag-handle
//     rows with tabular durations, collapse + add affordances, the quiet
//     empty state. The "+" is the frame's ADD affordance wired to the
//     product's first real enqueue caller (w26.i04, g20-P2-3): a compact
//     popup listing the same pickable sources the open-media dialog offers
//     (demo clips + the transport's remembered openings — real transport
//     data, never fabricated), each enqueueing through the playlist
//     context's playLater (`playlist.enqueue`), plus the shell's ONE
//     open-media funnel as the footer row.
//
// Transport is NOT duplicated: the compact cluster reuses card 4.C2's
// control logic (controls/state-mapping.ts mapTransportControls /
// seekDisableReason), the shared IconButton/Seekbar/ListRow primitives and
// the player + playlist contexts (context.tsx / PlaylistProvider) — only
// the COMPACT COMPOSITION is variant-owned (variantLayout.ts). Geometry
// lives in variants.css with semantic tokens for every color; raw px only
// for frame-measured structural dimensions (marked `frame:`).

import { useEffect, useRef, useState } from "react";
import type { DragEvent } from "react";
import { Icon, IconButton, ListEmpty, ListRow, Seekbar } from "../../../components";
import { MenuPopup } from "../../../components/menu/MenuPopup";
import { identityResolveLabel } from "../../../components/menu/types";
import type { MenuEntry } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.a16";
import { keys as i04Keys } from "../../../i18n/keys/w26.i04";
import { useClient } from "../../../client/ClientProvider";
import { DEMO_CLIPS, isDemoTransport } from "../../../client/demo-transport";
import { isDesktopShell } from "../../../client/app-transport";
import type { SourceSpec } from "../../../client/transport";
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
type LaneKey = keyof typeof laneKeys | keyof typeof i04Keys;

/**
 * `useMsg` extended with this lane's pre-registration keys (w26.a16 + the
 * w26.i04 enqueue-seam copy): lane keys win while unregistered; once the
 * Lead merges them into the catalog both paths return the same string. No
 * user-visible literals (same seam as w26.a10).
 */
function useLaneMsg() {
  const m = useMsg();
  const laneCatalog: Record<string, string> = { ...laneKeys, ...i04Keys };
  return (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key];
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

  // Previous/next — the playlist context's real prev/next commands, honestly
  // disabled with a reason while the queue is empty or the engine exposes no
  // playlist surface (the same truth TransportCluster reads).
  const queueEmpty = playlist.notExposed || playlist.entries.length === 0;
  const prevLabel = queueEmpty
    ? `${m("menu.playback.previous")} — ${m(
        playlist.notExposed ? "player.controls.reason.unknown" : "player.controls.reason.no-media",
      )}`
    : m("menu.playback.previous");
  const nextLabel = queueEmpty
    ? `${m("player.controls.next")} — ${m(
        playlist.notExposed ? "player.controls.reason.unknown" : "player.controls.reason.no-media",
      )}`
    : m("player.controls.next");

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
      {/* Level-1 heading, screen-reader-only (w26.f11, e17 F5 — the wave-a
          sr-only h1 contract: every route carries exactly one h1; the
          miniplayer branch of PlayerScreen renders no visible title bar, so
          the card owns the heading. Same pattern as PlayerScreen's
          .lumen-player__sr-title). */}
      <h1 className="lumen-player__sr-title">{m("player.variant.miniplayer.label")}</h1>

      {/* Collapsible queue panel ABOVE the video (Miniplayer.svg artboards
          2/4). Renders only while opened — the meta-row toggle owns it. */}
      {queueOpen ? (
        <MiniQueue
          id={QUEUE_ID}
          onCollapse={() => setQueueOpen(false)}
          onAdd={openMediaDialog}
        />
      ) : null}

      {/* The controls region wraps the card content below the queue panel
          in the frame's vertical order — video (with the overlay transport)
          → progress row → title row → meta row (Miniplayer.svg Default).
          The transport cluster overlays the VIDEO (playing artboards: close
          left · play centered · expand right) instead of occupying a row the
          frame's default card does not carry. */}
      <footer
        data-region-id={controlsRegion.id}
        role="region"
        aria-label={m(controlsRegion.labelKey)}
        className="lumen-mini__controls"
      >
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

          {/* Overlay transport (frame playing artboards). Real commands
              only — play is honestly disabled while the engine is idle, and
              prev/next ride the same playlist truth as the full bar.
              w26.i04: the centered row is prev · play · next at video
              mid-height (Miniplayer.svg Variant3: skip glyph centers ±75px
              from the play center), close top-left / expand top-right. */}
          <div className="lumen-mini__transport">
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
            <IconButton
              data-control="previous"
              className="lumen-mini__skip lumen-mini__skip--prev"
              label={prevLabel}
              title={prevLabel}
              disabled={queueEmpty}
              onClick={() => playlist.prev()}
            >
              <Icon name="previous" size={16} />
            </IconButton>
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
              data-control="next"
              className="lumen-mini__skip lumen-mini__skip--next"
              label={nextLabel}
              title={nextLabel}
              disabled={queueEmpty}
              onClick={() => playlist.next()}
            >
              <Icon name="next" size={16} />
            </IconButton>
            <IconButton
              data-control="expand"
              label={m("player.variant.miniplayer.expand")}
              title={m("player.variant.miniplayer.expand")}
              onClick={onExpand}
            >
              <Icon name="fullscreen" size={16} />
            </IconButton>
          </div>
        </div>

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
      </footer>
    </section>
  );
}

/** One pickable enqueue source (real transport data — the same clips and
 * remembered openings the open-media dialog lists; never fabricated). */
interface AddSource {
  readonly id: string;
  readonly label: string;
  readonly source: SourceSpec;
  readonly title: string | null;
}

/** Human location string for a source spec (data, not UI copy — the
 * open-media dialog's sourceLocation grammar). */
const sourceLocationOf = (source: SourceSpec): string =>
  source.kind === "file" ? source.path : source.url;

/** Source-kind affordance glyph (the open-media dialog's sourceIcon mapping). */
const sourceIconOf = (source: SourceSpec): "open-file" | "network-stream" =>
  source.kind === "file" ? "open-file" : "network-stream";

/**
 * The collapsible queue panel (Miniplayer.svg "Queue" artboard): centered
 * header, "N items · total" summary row with collapse/add affordances, and
 * drag-handle rows with tabular durations. All state is playlist truth
 * (entries/currentId via usePlaylist); drag-to-reorder uses the same
 * `playlist.move` semantics as the full playlist panel (card 4.D1).
 *
 * w26.i04 (h06 defect 4, the g20-P2-3 enqueue seam): the "+" is the frame's
 * ADD affordance, now wired to the product's first real enqueue UI caller —
 * a compact popup of pickable sources (demo clips + the transport's
 * remembered openings, deduplicated), each appended through the playlist
 * context's `playLater` (`playlist.enqueue`). The shell's ONE open-media
 * funnel rides as the popup's footer row, so the panel is self-sufficient.
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
  /** Open media — the shell's open-media dialog (the popup's footer row). */
  readonly onAdd: () => void;
}) {
  const m = useMsg();
  const t = useLaneMsg();
  const client = useClient();
  const playlist = usePlaylist();
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [addAnchor, setAddAnchor] = useState<{ left: number; top: number } | null>(null);
  const addTriggerRef = useRef<HTMLButtonElement>(null);
  const queueRootRef = useRef<HTMLDivElement>(null);

  // Outside pointer dismiss (the ClusterPopup/ContextMenu caller half — the
  // click target keeps focus, so this path deliberately does not steal it).
  useEffect(() => {
    if (!addOpen) return;
    const onPointerDown = (event: PointerEvent): void => {
      if (queueRootRef.current !== null && !queueRootRef.current.contains(event.target as Node)) {
        setAddOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [addOpen]);

  // Escape and item activation restore focus to the trigger (e12 F8 close
  // pattern — the MenuPopup caller contract).
  const closeAdd = (): void => {
    setAddOpen(false);
    addTriggerRef.current?.focus();
  };

  // Pickable sources: the demo clips (hidden in the desktop shell, the same
  // rule as the open-media dialog) + the transport's remembered openings,
  // deduplicated by location (an opening of a listed clip is one row).
  const sources: AddSource[] = [];
  const seenLocations = new Set<string>();
  for (const row of [
    ...(isDesktopShell()
      ? []
      : DEMO_CLIPS.map((clip) => ({
          id: `clip-${clip.id}`,
          label: m(clip.titleKey),
          source: clip.source,
          title: m(clip.titleKey) as string | null,
        }))),
    ...(isDemoTransport(client)
      ? client.recentOpenings.map((entry, index) => ({
          id: `recent-${index}`,
          label: entry.title ?? sourceLocationOf(entry.source),
          source: entry.source,
          title: entry.title,
        }))
      : []),
  ]) {
    const location = sourceLocationOf(row.source);
    if (seenLocations.has(location)) continue;
    seenLocations.add(location);
    sources.push(row);
  }

  const addEntries: MenuEntry<string>[] =
    sources.length === 0
      ? // Honest inert row — never a fabricated source (doc 05 §1 C4).
        [{ id: "add-empty", labelKey: t("player.mini.queue.add-empty"), disabled: true }]
      : [
          ...sources.map((row): MenuEntry<string> => ({
            id: row.id,
            labelKey: row.label,
            icon: sourceIconOf(row.source),
            onSelect: () => playlist.playLater(row.source, row.title),
          })),
          { id: "add-sep", kind: "separator" },
          {
            id: "add-open-media",
            labelKey: m("player.empty.open"),
            icon: "open-file",
            onSelect: onAdd,
          },
        ];

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
    <div
      ref={queueRootRef}
      className="lumen-mini__queue"
      id={id}
      role="region"
      aria-label={t("player.mini.queue.label")}
    >
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
              collapse (—) and add (+) affordances are real. The + opens the
              enqueue popup (w26.i04). */}
          <IconButton
            size="sm"
            label={t("player.mini.queue.toggle")}
            title={t("player.mini.queue.toggle")}
            onClick={onCollapse}
          >
            <Icon name="chevron-up" size={16} />
          </IconButton>
          <IconButton
            ref={addTriggerRef}
            size="sm"
            data-control="queue-add"
            label={t("player.mini.queue.add")}
            title={t("player.mini.queue.add")}
            aria-haspopup="menu"
            aria-expanded={addOpen}
            onClick={() => {
              if (addOpen) {
                setAddOpen(false);
                return;
              }
              const rect = addTriggerRef.current?.getBoundingClientRect();
              setAddAnchor(rect ? { left: rect.left, top: rect.bottom + 4 } : { left: 0, top: 0 });
              setAddOpen(true);
            }}
          >
            <Icon name="add" size={16} />
          </IconButton>
        </div>
      </div>
      {/* The enqueue popup — fixed at the trigger (the ContextMenu pointer
          contract; the queue panel's overflow:hidden cannot clip a
          fixed-position layer). MenuPopup viewport-clamps the anchor. */}
      {addOpen && addAnchor !== null ? (
        <MenuPopup
          entries={addEntries}
          resolveLabel={identityResolveLabel}
          idPrefix="lumen-mini-add"
          density="compact"
          className="lumen-mini__add-popup"
          ariaLabel={t("player.mini.queue.add")}
          style={{ left: addAnchor.left, top: addAnchor.top }}
          onRequestClose={closeAdd}
          onDismissAll={closeAdd}
        />
      ) : null}
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
              /* w26.gfix-F (g07-P3-4): the frame's queue rows carry handle +
               * filename only — the duration slot renders when the entry has
               * one, never a printed "Unknown" placeholder (ListRow drops the
               * slot entirely on null). */
              duration={formatDuration(entry.durationSeconds) ?? undefined}
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
