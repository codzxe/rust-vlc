// Card 4.D2 — pure formatters for the media-info screen. Every function is
// total and honest: absent input returns `null` (rendered as the i18n
// "Unknown" by the panel — never a fabricated value).

import type { CodecDescriptor, Duration, SourceSpec } from "../../client/transport";
import { usToSeconds } from "../../client/wire-time";

/**
 * wire µs → `h:mm:ss` (or `m:ss` under an hour). Truncates, never rounds.
 * `Duration.time` is a µs `Timestamp` on the 1.D4 wire (w26.k17 family
 * fix — the old ms formatter was one of the three unit splits j17 flagged).
 */
export function formatDurationUs(us: number): string {
  const total = Math.max(0, Math.floor(usToSeconds(us)));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const mm = String(minutes).padStart(2, "0");
  const ss = String(seconds).padStart(2, "0");
  return hours > 0 ? `${hours}:${mm}:${ss}` : `${minutes}:${ss}`;
}

/**
 * Duration wire type → display string. Returns `null` for `unknown` — the
 * panel renders the honest "Unknown". `infinite` renders the labeled ∞ form
 * (live streams), which is engine-reported truth, not invention.
 */
export function formatDuration(duration: Duration, infiniteLabel: string): string | null {
  switch (duration.kind) {
    case "known":
      return formatDurationUs(duration.time);
    case "infinite":
      return infiniteLabel;
    case "unknown":
      return null;
  }
}

/**
 * Codec label from the descriptor: `description (fourcc)` when both are
 * reported, whichever exists otherwise, `null` when the engine reported
 * neither — the panel then shows "Unknown".
 */
export function codecLabel(codec: CodecDescriptor): string | null {
  const parts = [codec.description, codec.fourcc].filter(
    (p): p is string => typeof p === "string" && p.length > 0,
  );
  if (parts.length === 0) return null;
  if (parts.length === 1) return parts[0]!;
  return `${parts[0]} (${parts[1]})`;
}

/** Source location: the file path or URL verbatim; `null` never happens for
 * a valid SourceSpec but the seam stays total. */
export function sourceLocation(source: SourceSpec): string {
  return source.kind === "file" ? source.path : source.url;
}
