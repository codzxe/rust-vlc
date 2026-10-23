// Card 4.D1 — playlist panel content (doc 08 §4.5: playlist → listbox/
// option; §1.6: playlist panel 377px translucent; §5.3 queue controls).
// Rendered both inside the player shell's `playlist` region (PlayerScreen
// wiring) and as the /playlist route body (PlaylistScreen).
//
// HONESTY: current marker, shuffle and repeat reflect ENGINE-reported state
// only (playlist.changed / snapshot). `null` (not yet reported) renders as
// the unpressed control — commented at each site; never a faked "known off".

import { useState, type DragEvent } from "react";
import { useMsg } from "../../i18n/messages";
import { IconButton, Button } from "../../components/button";
import { Icon } from "../../components/icon";
import { VirtualList } from "../../components/list";
import { usePlaylist, type PlaylistEntry } from "./PlaylistProvider";
import { cycleRepeat, dragToMoveIndex, formatDuration, sourceLocation } from "./queue";
import "./playlist.css";

/** doc 08 §1.5 (option.svg): 48px rows. */
const ROW_HEIGHT = 48;

export function PlaylistPanel({ className = "" }: { className?: string }) {
  const m = useMsg();
  const playlist = usePlaylist();
  const [dragFrom, setDragFrom] = useState<number | null>(null);

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
    <section className={`lumen-playlist ${className}`.trim()} data-chrome-part="">
      <header className="lumen-playlist__header">
        <h2 className="lumen-playlist__title">{m("playlist.panel.title")}</h2>
        <span className="lumen-playlist__count">
          {String(playlist.entries.length)} {m("playlist.panel.count-suffix")}
        </span>
        <div className="lumen-playlist__controls">
          <IconButton label={m("playlist.controls.prev")} size="sm" onClick={() => playlist.prev()}>
            <Icon name="skip-back-15" size={14} />
          </IconButton>
          <IconButton label={m("playlist.controls.next")} size="sm" onClick={() => playlist.next()}>
            <Icon name="skip-forward-15" size={14} />
          </IconButton>
          {/* aria-pressed only claims "on" when the engine reported it;
              unknown (null) renders unpressed — never a faked known-off. */}
          <IconButton
            label={m("playlist.controls.shuffle")}
            size="sm"
            aria-pressed={playlist.shuffle === true}
            onClick={() => playlist.setShuffle(!(playlist.shuffle === true))}
          >
            <Icon name="shuffle" size={14} />
          </IconButton>
          <IconButton
            label={repeatLabel}
            size="sm"
            data-mode={repeatMode}
            onClick={() => playlist.setRepeat(cycleRepeat(repeatMode))}
          >
            <Icon name="repeat" size={14} />
          </IconButton>
          <IconButton label={m("playlist.controls.clear")} size="sm" onClick={() => playlist.clear()}>
            <Icon name="stop" size={14} />
          </IconButton>
        </div>
      </header>

      {playlist.notExposed ? <p className="lumen-playlist__note">{m("playlist.panel.not-exposed")}</p> : null}

      {playlist.entries.length === 0 ? (
        <div className="lumen-playlist__empty">
          <p className="lumen-playlist__empty-title">{m("playlist.panel.empty-title")}</p>
          <p className="lumen-playlist__empty-body">{m("playlist.panel.empty-body")}</p>
        </div>
      ) : (
        <VirtualList
          data={playlist.entries}
          itemHeight={ROW_HEIGHT}
          label={m("playlist.panel.title")}
          scrollToIndex={currentIndex >= 0 ? { index: currentIndex, align: "start" } : null}
          className="lumen-playlist__list"
          renderItem={(entry, index) => (
            <PlaylistRow
              key={entry.id}
              entry={entry}
              index={index}
              isCurrent={entry.id === playlist.currentId}
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
    </section>
  );
}

function PlaylistRow({
  entry,
  index,
  isCurrent,
  onDragStart,
  onDrop,
}: {
  entry: PlaylistEntry;
  index: number;
  isCurrent: boolean;
  onDragStart: (event: DragEvent<HTMLDivElement>) => void;
  onDrop: (event: DragEvent<HTMLDivElement>) => void;
}) {
  const m = useMsg();
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
    <div
      className={`lumen-plrow${isCurrent ? " lumen-plrow--current" : ""}`}
      data-current={isCurrent || undefined}
      draggable
      onDragStart={onDragStart}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onDrop}
    >
      <span className="lumen-plrow__marker" aria-hidden="true">
        {isCurrent ? <Icon name="play" size={10} /> : null}
      </span>
      {isCurrent ? <span className="lumen-playlist__sr">{m("playlist.entry.current")}</span> : null}
      <span className="lumen-plrow__main">
        <span className="lumen-plrow__title">{entry.title ?? m("playlist.entry.unknown-title")}</span>
        {location !== null ? <span className="lumen-plrow__location">{location}</span> : null}
      </span>
      <span className="lumen-plrow__duration">{duration ?? m("playlist.entry.unknown-duration")}</span>
      <span className="lumen-plrow__actions">
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
      </span>
    </div>
  );
}
