// Card 4.D2 — media information panel (doc 08 §4.3 "Media information"
// row: MediaInfoPanel via `media.get-info`; design has no dedicated frame
// [VERIFY-DESIGN] → styling from the settings language per the doc).
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
import { useMediaInfo } from "./MediaInfoProvider";
import { codecLabel, formatDuration, sourceLocation } from "./format";
import "./media-info.css";

/** A dt/dd row; the value is pre-resolved (already "Unknown" when absent). */
function Row({
  field,
  label,
  value,
}: {
  field: string;
  label: string;
  value: string;
}) {
  return (
    <div className="lumen-mi__row" data-field={field}>
      <dt className="lumen-mi__term">{label}</dt>
      <dd className="lumen-mi__value">{value}</dd>
    </div>
  );
}

export function MediaInfoPanel({ className = "" }: { className?: string }) {
  const m = useMsg();
  const { info, notExposed } = useMediaInfo();

  // The honest unknown — single i18n key, reused for EVERY absent field.
  const unknown = m("media-info.unknown");

  if (notExposed) {
    return (
      <section className={`lumen-mi ${className}`.trim()} data-screen-part="media-info" data-state="not-exposed">
        <h2 className="lumen-mi__title">{m("media-info.panel.title")}</h2>
        <p className="lumen-mi__note">{m("media-info.panel.not-exposed")}</p>
      </section>
    );
  }

  if (info === null) {
    // Honest empty state: nothing open / nothing reported.
    return (
      <section className={`lumen-mi ${className}`.trim()} data-screen-part="media-info" data-state="empty">
        <h2 className="lumen-mi__title">{m("media-info.panel.title")}</h2>
        <p className="lumen-mi__empty-title">{m("media-info.empty.title")}</p>
        <p className="lumen-mi__empty-body">{m("media-info.empty.body")}</p>
      </section>
    );
  }

  return <MediaInfoSections info={info} unknown={unknown} className={className} />;
}

/** Exported for fixture-driven tests: renders from a bare MediaInfo value. */
function MediaInfoSections({
  info,
  unknown,
  className = "",
}: {
  info: MediaInfo;
  unknown: string;
  className?: string;
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
    <section className={`lumen-mi ${className}`.trim()} data-screen-part="media-info" data-state="ready">
      <h2 className="lumen-mi__title">{m("media-info.panel.title")}</h2>

      {/* --- General --- */}
      <section className="lumen-mi__section" data-mi-section="general" aria-labelledby="lumen-mi-general">
        <h3 className="lumen-mi__section-title" id="lumen-mi-general">
          {m("media-info.section.general")}
        </h3>
        <dl className="lumen-mi__rows">
          <Row field="title" label={m("media-info.field.title")} value={text(info.title)} />
          <Row field="source" label={m("media-info.field.source")} value={sourceLocation(info.source)} />
          <Row field="container" label={m("media-info.field.container")} value={text(info.container)} />
          <Row
            field="duration"
            label={m("media-info.field.duration")}
            value={formatDuration(info.duration, m("media-info.duration.infinite")) ?? unknown}
          />
          {/* CATALOG GAP: size/bitrate are not in the MediaInfo wire type yet —
              honest "Unknown", never invented (ui/NEXT-STEPS.md handoff). */}
          <Row field="size" label={m("media-info.field.size")} value={unknown} />
          <Row field="bitrate" label={m("media-info.field.bitrate")} value={unknown} />
          <Row field="live" label={m("media-info.field.live")} value={bool(info.isLive)} />
          <Row field="seekable" label={m("media-info.field.seekable")} value={bool(info.seekable)} />
          <Row field="chapters" label={m("media-info.field.chapters")} value={String(info.chapters.length)} />
        </dl>
      </section>

      {/* --- Video streams --- */}
      <section className="lumen-mi__section" data-mi-section="video" aria-labelledby="lumen-mi-video">
        <h3 className="lumen-mi__section-title" id="lumen-mi-video">
          {m("media-info.section.video")}
        </h3>
        {video.map((track, index) => (
          <VideoStream key={track.id} track={track} index={index} unknown={unknown} />
        ))}
      </section>

      {/* --- Audio streams --- */}
      <section className="lumen-mi__section" data-mi-section="audio" aria-labelledby="lumen-mi-audio">
        <h3 className="lumen-mi__section-title" id="lumen-mi-audio">
          {m("media-info.section.audio")}
        </h3>
        {audio.map((track, index) => (
          <AudioStream key={track.id} track={track} index={index} unknown={unknown} />
        ))}
      </section>

      {/* --- Subtitles --- */}
      <section className="lumen-mi__section" data-mi-section="subtitle" aria-labelledby="lumen-mi-subtitle">
        <h3 className="lumen-mi__section-title" id="lumen-mi-subtitle">
          {m("media-info.section.subtitle")}
        </h3>
        {subtitles.map((track, index) => (
          <SubtitleStream key={track.id} track={track} index={index} unknown={unknown} />
        ))}
      </section>

      {/* --- Metadata tags --- */}
      <section className="lumen-mi__section" data-mi-section="metadata" aria-labelledby="lumen-mi-metadata">
        <h3 className="lumen-mi__section-title" id="lumen-mi-metadata">
          {m("media-info.section.metadata")}
        </h3>
        {metadataEntries.length === 0 ? (
          <p className="lumen-mi__note">{m("media-info.metadata.none")}</p>
        ) : (
          <dl className="lumen-mi__rows">
            {metadataEntries.map(([key, value]) => (
              <Row key={key} field={`metadata.${key}`} label={key} value={text(value)} />
            ))}
          </dl>
        )}
      </section>
    </section>
  );
}

function streamHeading(kind: string, index: number): string {
  return `${kind} ${String(index + 1)}`;
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
      <h4 className="lumen-mi__stream-title">{streamHeading(m("media-info.stream.video"), index)}</h4>
      <dl className="lumen-mi__rows">
        <Row field="codec" label={m("media-info.field.codec")} value={codecLabel(track.codec) ?? unknown} />
        <Row field="profile" label={m("media-info.field.profile")} value={textOrNull(track.codec?.profile, unknown)} />
        <Row field="level" label={m("media-info.field.level")} value={textOrNull(track.codec?.level, unknown)} />
        <Row
          field="dimensions"
          label={m("media-info.field.dimensions")}
          value={v === null ? unknown : `${String(v.width)} × ${String(v.height)}`}
        />
        <Row field="frame-rate" label={m("media-info.field.frame-rate")} value={numOrNull(v?.frameRate, (n) => `${String(n)} fps`, unknown)} />
        <Row field="bit-depth" label={m("media-info.field.bit-depth")} value={numOrNull(v?.bitDepth, (n) => `${String(n)}-bit`, unknown)} />
        <Row field="hdr" label={m("media-info.field.hdr")} value={v === null ? unknown : v.hdr ? m("media-info.value.yes") : m("media-info.value.no")} />
        <Row field="language" label={m("media-info.field.language")} value={textOrNull(track.language, unknown)} />
        <Row field="name" label={m("media-info.field.name")} value={textOrNull(track.name, unknown)} />
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
      <h4 className="lumen-mi__stream-title">{streamHeading(m("media-info.stream.audio"), index)}</h4>
      <dl className="lumen-mi__rows">
        <Row field="codec" label={m("media-info.field.codec")} value={codecLabel(track.codec) ?? unknown} />
        <Row field="channels" label={m("media-info.field.channels")} value={a === null ? unknown : String(a.channels)} />
        <Row field="sample-rate" label={m("media-info.field.sample-rate")} value={numOrNull(a?.sampleRate, (n) => `${String(n)} Hz`, unknown)} />
        <Row field="bit-depth" label={m("media-info.field.bit-depth")} value={numOrNull(a?.bitDepth, (n) => `${String(n)}-bit`, unknown)} />
        <Row field="codec-params" label={m("media-info.field.codec-params")} value={textOrNull(a?.codecParams, unknown)} />
        <Row field="language" label={m("media-info.field.language")} value={textOrNull(track.language, unknown)} />
        <Row field="name" label={m("media-info.field.name")} value={textOrNull(track.name, unknown)} />
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
      <h4 className="lumen-mi__stream-title">{streamHeading(m("media-info.stream.subtitle"), index)}</h4>
      <dl className="lumen-mi__rows">
        <Row field="codec" label={m("media-info.field.codec")} value={codecLabel(track.codec) ?? unknown} />
        <Row field="language" label={m("media-info.field.language")} value={textOrNull(track.language, unknown)} />
        <Row field="name" label={m("media-info.field.name")} value={textOrNull(track.name, unknown)} />
        <Row
          field="origin"
          label={m("media-info.field.origin")}
          value={track.origin === "external-subtitle" ? m("media-info.origin.external") : m("media-info.origin.stream")}
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
