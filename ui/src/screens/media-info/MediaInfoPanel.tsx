// Card 4.D2 — media information panel (doc 08 §4.3 "Media information"
// row: MediaInfoPanel via `media.get-info`). W26 Figma language (w26.b13
// rebuild; no direct frame — CONTRACT §3): composed from the design
// language with the nearest grammars —
//   * Settings - *.svg — section headers (15px-class semibold, the
//     SectionHeader primitive) and the label-left / value-right row
//     grammar with 1px hairlines between rows;
//   * playlist.svg — the r14 #191919 bordered card per section;
//   * tabular numerals on every value (W25 §1: timecodes, dimensions,
//     rates hold their columns);
//   * ListEmpty — the quiet empty state (no artwork; lists stay calm).
//
// Rendered both inside the player shell's `media-info` region (PlayerScreen
// wiring) and as the /media-info route body (MediaInfoScreen).
//
// HONESTY (the card's AC): every absent field renders the i18n "Unknown" —
// `null`/`undefined`/empty never becomes a value that looks real. Streams
// that lack a field still SHOW the field (info-density per the design), and
// CATALOG GAPS are explicit: the command result carries no `size`/`bitrate`
// fields (see generated/commands.ts MediaInfo), so those General rows
// honestly render "Unknown" until the catalog grows them (handoff recorded
// in ui/NEXT-STEPS.md) — they are never invented.

import { useMsg } from "../../i18n/messages";
import type { MediaInfo, TrackInfo } from "../../client/transport";
import { Card, SectionHeader } from "../../components/surface/Surface";
import { ListEmpty } from "../../components/list/Empty";
import { Icon } from "../../components/icon/Icon";
import type { IconName } from "../../components/icon/icons";
import { useMediaInfo } from "./MediaInfoProvider";
import { codecLabel, formatDuration, sourceLocation } from "./format";
import "./media-info.css";

/** A dt/dd row; the value is pre-resolved (already "Unknown" when absent). */
function Row({
  field,
  label,
  value,
  unknown = null,
}: {
  field: string;
  label: string;
  value: string;
  /** The honest-unknown sentinel — when `value` equals it the row flags
   * itself `data-unknown` so CSS renders it muted, never invented. */
  unknown?: string | null;
}) {
  return (
    <div
      className="lumen-mi__row"
      data-field={field}
      data-unknown={(unknown !== null && value === unknown) || undefined}
    >
      <dt className="lumen-mi__term">{label}</dt>
      <dd className="lumen-mi__value">{value}</dd>
    </div>
  );
}

export function MediaInfoPanel({
  className = "",
  showTitle = true,
}: {
  className?: string;
  /** Panel header. The /media-info route supplies its own page heading and
   * passes false (one heading per landmark); the player's side region keeps
   * the default — the panel title is its only header. */
  showTitle?: boolean;
}) {
  const m = useMsg();
  const { info, notExposed } = useMediaInfo();

  // The honest unknown — single i18n key, reused for EVERY absent field.
  const unknown = m("media-info.unknown");

  if (notExposed) {
    return (
      <section
        className={`lumen-mi ${className}`.trim()}
        data-screen-part="media-info"
        data-state="not-exposed"
      >
        {showTitle ? <h2 className="lumen-mi__title">{m("media-info.panel.title")}</h2> : null}
        <p className="lumen-mi__note">{m("media-info.panel.not-exposed")}</p>
      </section>
    );
  }

  if (info === null) {
    // Honest empty state: nothing open / nothing reported — quiet, no art.
    return (
      <section
        className={`lumen-mi ${className}`.trim()}
        data-screen-part="media-info"
        data-state="empty"
      >
        {showTitle ? <h2 className="lumen-mi__title">{m("media-info.panel.title")}</h2> : null}
        <ListEmpty
          className="lumen-mi__empty"
          data-testid="media-info-empty"
          title={m("media-info.empty.title")}
          body={m("media-info.empty.body")}
        />
      </section>
    );
  }

  return (
    <MediaInfoSections info={info} unknown={unknown} className={className} showTitle={showTitle} />
  );
}

/** Exported for fixture-driven tests: renders from a bare MediaInfo value. */
function MediaInfoSections({
  info,
  unknown,
  className = "",
  showTitle = true,
}: {
  info: MediaInfo;
  unknown: string;
  className?: string;
  showTitle?: boolean;
}) {
  const m = useMsg();
  const text = (value: string | null | undefined): string =>
    value === null || value === undefined || value.length === 0 ? unknown : value;
  const bool = (value: boolean): string => (value ? m("media-info.value.yes") : m("media-info.value.no"));

  const video = info.tracks.filter((t) => t.kind === "video");
  const audio = info.tracks.filter((t) => t.kind === "audio");
  const subtitles = info.tracks.filter((t) => t.kind === "subtitle");
  const metadataEntries = Object.entries(info.metadata);

  return (
    <section
      className={`lumen-mi ${className}`.trim()}
      data-screen-part="media-info"
      data-state="ready"
    >
      {showTitle ? <h2 className="lumen-mi__title">{m("media-info.panel.title")}</h2> : null}

      {/* --- General --- */}
      <Card
        as="section"
        className="lumen-mi__section"
        data-mi-section="general"
        aria-labelledby="lumen-mi-general"
      >
        <SectionHeader as="h3" id="lumen-mi-general">
          {m("media-info.section.general")}
        </SectionHeader>
        <dl className="lumen-mi__rows">
          <Row field="title" label={m("media-info.field.title")} value={text(info.title)} unknown={unknown} />
          <Row field="source" label={m("media-info.field.source")} value={sourceLocation(info.source)} unknown={unknown} />
          <Row field="container" label={m("media-info.field.container")} value={text(info.container)} unknown={unknown} />
          <Row
            field="duration"
            label={m("media-info.field.duration")}
            value={formatDuration(info.duration, m("media-info.duration.infinite")) ?? unknown}
            unknown={unknown}
          />
          {/* CATALOG GAP: size/bitrate are not in the MediaInfo wire type yet —
              honest "Unknown", never invented (ui/NEXT-STEPS.md handoff). */}
          <Row field="size" label={m("media-info.field.size")} value={unknown} unknown={unknown} />
          <Row field="bitrate" label={m("media-info.field.bitrate")} value={unknown} unknown={unknown} />
          <Row field="live" label={m("media-info.field.live")} value={bool(info.isLive)} unknown={unknown} />
          <Row field="seekable" label={m("media-info.field.seekable")} value={bool(info.seekable)} unknown={unknown} />
          <Row field="chapters" label={m("media-info.field.chapters")} value={String(info.chapters.length)} unknown={unknown} />
        </dl>
      </Card>

      {/* --- Video streams --- */}
      <Card
        as="section"
        className="lumen-mi__section"
        data-mi-section="video"
        aria-labelledby="lumen-mi-video"
      >
        <SectionHeader as="h3" id="lumen-mi-video">
          {m("media-info.section.video")}
        </SectionHeader>
        {video.map((track, index) => (
          <VideoStream key={track.id} track={track} index={index} unknown={unknown} />
        ))}
      </Card>

      {/* --- Audio streams --- */}
      <Card
        as="section"
        className="lumen-mi__section"
        data-mi-section="audio"
        aria-labelledby="lumen-mi-audio"
      >
        <SectionHeader as="h3" id="lumen-mi-audio">
          {m("media-info.section.audio")}
        </SectionHeader>
        {audio.map((track, index) => (
          <AudioStream key={track.id} track={track} index={index} unknown={unknown} />
        ))}
      </Card>

      {/* --- Subtitles --- */}
      <Card
        as="section"
        className="lumen-mi__section"
        data-mi-section="subtitle"
        aria-labelledby="lumen-mi-subtitle"
      >
        <SectionHeader as="h3" id="lumen-mi-subtitle">
          {m("media-info.section.subtitle")}
        </SectionHeader>
        {subtitles.map((track, index) => (
          <SubtitleStream key={track.id} track={track} index={index} unknown={unknown} />
        ))}
      </Card>

      {/* --- Metadata tags --- */}
      <Card
        as="section"
        className="lumen-mi__section"
        data-mi-section="metadata"
        aria-labelledby="lumen-mi-metadata"
      >
        <SectionHeader as="h3" id="lumen-mi-metadata">
          {m("media-info.section.metadata")}
        </SectionHeader>
        {metadataEntries.length === 0 ? (
          <p className="lumen-mi__note">{m("media-info.metadata.none")}</p>
        ) : (
          <dl className="lumen-mi__rows">
            {metadataEntries.map(([key, value]) => (
              <Row key={key} field={`metadata.${key}`} label={key} value={text(value)} unknown={unknown} />
            ))}
          </dl>
        )}
      </Card>
    </section>
  );
}

function streamHeading(kind: string, index: number): string {
  return `${kind} ${String(index + 1)}`;
}

/** Registry glyph per stream kind (pinned Icon.svg vocabulary only). */
function streamIcon(kind: "video" | "audio" | "subtitle"): IconName {
  switch (kind) {
    case "video":
      return "tv";
    case "audio":
      return "audio";
    case "subtitle":
      return "subtitles";
  }
}

/** The stream sub-heading row: leading line icon + the numbered label. */
function StreamHeading({
  kind,
  label,
  index,
}: {
  kind: "video" | "audio" | "subtitle";
  label: string;
  index: number;
}) {
  return (
    <h4 className="lumen-mi__stream-title">
      <Icon name={streamIcon(kind)} size={16} className="lumen-mi__stream-icon" />
      <span className="lumen-mi__stream-label">{streamHeading(label, index)}</span>
    </h4>
  );
}

function VideoStream({
  track,
  index,
  unknown,
}: {
  track: TrackInfo;
  index: number;
  unknown: string;
}) {
  const m = useMsg();
  const v = track.video ?? null;
  return (
    <article className="lumen-mi__stream" data-mi-stream="video" data-track-id={track.id}>
      <StreamHeading kind="video" label={m("media-info.stream.video")} index={index} />
      <dl className="lumen-mi__rows">
        <Row field="codec" label={m("media-info.field.codec")} value={codecLabel(track.codec) ?? unknown} unknown={unknown} />
        <Row field="profile" label={m("media-info.field.profile")} value={textOrNull(track.codec?.profile, unknown)} unknown={unknown} />
        <Row field="level" label={m("media-info.field.level")} value={textOrNull(track.codec?.level, unknown)} unknown={unknown} />
        <Row
          field="dimensions"
          label={m("media-info.field.dimensions")}
          value={v === null ? unknown : `${String(v.width)} × ${String(v.height)}`}
          unknown={unknown}
        />
        <Row field="frame-rate" label={m("media-info.field.frame-rate")} value={numOrNull(v?.frameRate, (n) => `${String(n)} fps`, unknown)} unknown={unknown} />
        <Row field="bit-depth" label={m("media-info.field.bit-depth")} value={numOrNull(v?.bitDepth, (n) => `${String(n)}-bit`, unknown)} unknown={unknown} />
        <Row field="hdr" label={m("media-info.field.hdr")} value={v === null ? unknown : v.hdr ? m("media-info.value.yes") : m("media-info.value.no")} unknown={unknown} />
        <Row field="language" label={m("media-info.field.language")} value={textOrNull(track.language, unknown)} unknown={unknown} />
        <Row field="name" label={m("media-info.field.name")} value={textOrNull(track.name, unknown)} unknown={unknown} />
      </dl>
    </article>
  );
}

function AudioStream({
  track,
  index,
  unknown,
}: {
  track: TrackInfo;
  index: number;
  unknown: string;
}) {
  const m = useMsg();
  const a = track.audio ?? null;
  return (
    <article className="lumen-mi__stream" data-mi-stream="audio" data-track-id={track.id}>
      <StreamHeading kind="audio" label={m("media-info.stream.audio")} index={index} />
      <dl className="lumen-mi__rows">
        <Row field="codec" label={m("media-info.field.codec")} value={codecLabel(track.codec) ?? unknown} unknown={unknown} />
        <Row field="channels" label={m("media-info.field.channels")} value={a === null ? unknown : String(a.channels)} unknown={unknown} />
        <Row field="sample-rate" label={m("media-info.field.sample-rate")} value={numOrNull(a?.sampleRate, (n) => `${String(n)} Hz`, unknown)} unknown={unknown} />
        <Row field="bit-depth" label={m("media-info.field.bit-depth")} value={numOrNull(a?.bitDepth, (n) => `${String(n)}-bit`, unknown)} unknown={unknown} />
        <Row field="codec-params" label={m("media-info.field.codec-params")} value={textOrNull(a?.codecParams, unknown)} unknown={unknown} />
        <Row field="language" label={m("media-info.field.language")} value={textOrNull(track.language, unknown)} unknown={unknown} />
        <Row field="name" label={m("media-info.field.name")} value={textOrNull(track.name, unknown)} unknown={unknown} />
      </dl>
    </article>
  );
}

function SubtitleStream({
  track,
  index,
  unknown,
}: {
  track: TrackInfo;
  index: number;
  unknown: string;
}) {
  const m = useMsg();
  return (
    <article className="lumen-mi__stream" data-mi-stream="subtitle" data-track-id={track.id}>
      <StreamHeading kind="subtitle" label={m("media-info.stream.subtitle")} index={index} />
      <dl className="lumen-mi__rows">
        <Row field="codec" label={m("media-info.field.codec")} value={codecLabel(track.codec) ?? unknown} unknown={unknown} />
        <Row field="language" label={m("media-info.field.language")} value={textOrNull(track.language, unknown)} unknown={unknown} />
        <Row field="name" label={m("media-info.field.name")} value={textOrNull(track.name, unknown)} unknown={unknown} />
        <Row
          field="origin"
          label={m("media-info.field.origin")}
          value={track.origin === "external-subtitle" ? m("media-info.origin.external") : m("media-info.origin.stream")}
          unknown={unknown}
        />
      </dl>
    </article>
  );
}

// Honest-value helpers: `null`/`undefined` → the "Unknown" sentinel.
function textOrNull(value: string | null | undefined, unknown: string): string {
  return value === null || value === undefined || value.length === 0 ? unknown : value;
}

function numOrNull(
  value: number | null | undefined,
  format: (n: number) => string,
  unknown: string,
): string {
  return value === null || value === undefined ? unknown : format(value);
}
