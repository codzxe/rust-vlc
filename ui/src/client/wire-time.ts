// w26.k17 — the duration-units FAMILY fix (issue #64, j17 P1-1).
//
// THE WIRE CONTRACT (authoritative, crates/commands + schemas): every
// `Timestamp` on the 1.D4 wire is MICROSECONDS — `Duration.time`,
// `timing.duration-known.duration`, `timing.time-tick.time`,
// `timing.seek-completed.position`, `Bookmark.time`, `Chapter.timeStart`,
// `OpenOptions.startTime`, `StateSnapshot.time`, `TimeRange` buffered
// ranges, `SeekTarget.absolute.time` and `media.add-bookmark`'s `time`
// (types.rs:30 "Microseconds since media start"). The `position`/
// `fraction` scalars (`timing.time-tick`, `media.get-current-time`,
// `playback.set-position`) are 0..1 FRACTIONS of duration; relative seek
// deltas are the only true milliseconds (`SeekTarget.relative.deltaMs`).
//
// THE UI SEAMS stay SECONDS (player state `position`/`durationSeconds`,
// `PlayerBufferRange`, the playlist formatters) — every conversion crosses
// the wire edge HERE, through named constants. The three pre-k17 unit
// splits (media-info ms / player+demo seconds / wire µs) are gone; there
// is exactly one unit contract per surface now.

import type { Duration } from "../generated/commands";

/** Microseconds per second — the single wire↔seam conversion factor. */
export const US_PER_SECOND = 1_000_000;

/** Seam seconds → wire µs (send side: seeks, bookmarks). */
export function secondsToUs(seconds: number): number {
  return seconds * US_PER_SECOND;
}

/** Wire µs → seam seconds (read side: ticks, durations, snapshots). */
export function usToSeconds(us: number): number {
  return us / US_PER_SECOND;
}

/** Wire `Duration` → seam seconds; `null` for unknown/infinite. */
export function durationToSeconds(duration: Duration): number | null {
  return duration.kind === "known" ? usToSeconds(duration.time) : null;
}
