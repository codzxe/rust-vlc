// Card 7.E3 (issue #44) — the conversion job-queue model, PURE.
//
// The lifecycle mirrors the engine's transcode job (`crates/transcode`
// `JobOutcome`: Completed / Cancelled(stats) / Failed(err)) plus the one
// state the UI can hit before any engine wiring exists:
//
//   queued ──start──► running ──complete──► completed
//     │                  │        fail────► failed (reason)
//     │                  └ cancel ─► cancelled (progress kept — the honest
//                                  partial-output shape: what had been
//                                  processed when the stop landed)
//     └ block ──► blocked (reason)  the engine lane is not exposed
//
// `blocked` is the honesty seam: the engine-side conversion operations
// exist (crates/transcode, wave 23), but the COMMAND CATALOG carries no
// convert/record commands yet (Lead-owned interface — crates/commands has
// 64 rows, none in the convert/record namespaces; product.toml keeps
// `recording = false`, `transcode = false` per rule F2). A job that
// cannot reach the engine becomes `blocked` with the typed reason shown
// verbatim in its row — never a simulated progress bar, never a fake
// "completed" file.
//
// The reducer is total (dirty-world rule D-3): unknown ids, illegal state
// transitions and out-of-range fractions are no-ops or clamps, never
// throws. Every rule is pinned by convertQueue.test.ts.

import type { CommandListing } from "../../generated/commands";
import type { ConvertTargetId } from "./convertTargets";

/** One queue row's lifecycle state (module diagram). */
export type ConvertJobState =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "blocked";

/** One queued conversion job (immutable rows — events produce new rows). */
export interface ConvertJob {
  readonly id: number;
  readonly sourcePath: string;
  readonly target: ConvertTargetId;
  readonly state: ConvertJobState;
  /** Progress fraction 0..1 while `running`; `null` otherwise. A cancelled
   * running job keeps its last fraction — the honest partial amount. */
  readonly progress: number | null;
  /** Terminal detail: the blocked/failed reason; `null` otherwise. */
  readonly detail: string | null;
}

/** Queue bound [PROPOSAL: 8 rows — the queue is UI state; the engine's own
 * jobs are bounded by `JobLimits`, and a long unbounded row list would be
 * an unbounded scroll surface]. */
export const MAX_CONVERT_QUEUE_JOBS = 8;

/** The event vocabulary that drives the lifecycle. */
export type ConvertQueueEvent =
  | { readonly kind: "enqueue"; readonly sourcePath: string; readonly target: ConvertTargetId }
  | { readonly kind: "start"; readonly id: number }
  | { readonly kind: "progress"; readonly id: number; readonly fraction: number }
  | { readonly kind: "complete"; readonly id: number }
  | { readonly kind: "fail"; readonly id: number; readonly reason: string }
  | { readonly kind: "cancel"; readonly id: number }
  | { readonly kind: "block"; readonly id: number; readonly reason: string };

/** True while `state` admits a cancel (the only honest cancel targets). */
export function cancellable(state: ConvertJobState): boolean {
  return state === "queued" || state === "running";
}

/** True when the queue has room for one more row (the bound above). */
export function canEnqueue(queue: readonly ConvertJob[]): boolean {
  return queue.length < MAX_CONVERT_QUEUE_JOBS;
}

/** Clamp a progress fraction into 0..1 (dirty values never propagate). */
function clampFraction(fraction: number): number {
  if (Number.isNaN(fraction)) return 0;
  return Math.min(1, Math.max(0, fraction));
}

/** The next monotonic job id for `queue` (1-based; max + 1). */
export function nextJobId(queue: readonly ConvertJob[]): number {
  return queue.reduce((max, job) => Math.max(max, job.id), 0) + 1;
}

/**
 * The pure reducer. Total: illegal events (unknown id, wrong source state,
 * a full queue) return the queue unchanged — the SCREEN checks
 * `canEnqueue` first and surfaces the honest bound error itself.
 */
export function convertQueueReduce(
  queue: readonly ConvertJob[],
  event: ConvertQueueEvent,
): readonly ConvertJob[] {
  const mapJob = (id: number, map: (job: ConvertJob) => ConvertJob): readonly ConvertJob[] => {
    const index = queue.findIndex((job) => job.id === id);
    if (index === -1) return queue;
    const mapped = map(queue[index]!);
    // A refused transition is a TRUE no-op: the same array reference
    // returns (React state updates bail; tests pin the identity).
    if (mapped === queue[index]) return queue;
    return [...queue.slice(0, index), mapped, ...queue.slice(index + 1)];
  };

  switch (event.kind) {
    case "enqueue": {
      if (!canEnqueue(queue)) return queue;
      const job: ConvertJob = {
        id: nextJobId(queue),
        sourcePath: event.sourcePath,
        target: event.target,
        state: "queued",
        progress: null,
        detail: null,
      };
      return [...queue, job];
    }
    case "start":
      return mapJob(event.id, (job) =>
        job.state === "queued" ? { ...job, state: "running", progress: 0 } : job,
      );
    case "progress":
      return mapJob(event.id, (job) =>
        job.state === "running"
          ? { ...job, progress: clampFraction(event.fraction) }
          : job,
      );
    case "complete":
      return mapJob(event.id, (job) =>
        job.state === "running" ? { ...job, state: "completed", progress: 1 } : job,
      );
    case "fail":
      return mapJob(event.id, (job) =>
        job.state === "running" ? { ...job, state: "failed", detail: event.reason } : job,
      );
    case "cancel":
      return mapJob(event.id, (job) =>
        cancellable(job.state) ? { ...job, state: "cancelled" } : job,
      );
    case "block":
      return mapJob(event.id, (job) =>
        job.state === "queued" ? { ...job, state: "blocked", detail: event.reason } : job,
      );
  }
}

/**
 * The engine-lane probe: which of the catalog's listed commands belong to
 * the conversion namespaces? The command catalog (`system.list-commands`,
 * doc 05 §3) is the engine's own capability surface — the customize
 * panels' `useCommandCapabilities` precedent. The namespace prefixes
 * (`convert.*`, `record.*`) follow the catalog's existing dotted-namespace
 * convention (`session.*`, `playback.*`, `media.*`, …).
 *
 * Today this returns an empty list for both the demo transport and the
 * full 64-command engine catalog (verified against crates/commands) — the
 * honest truth: the conversion engine lane is not wired into the command
 * layer yet. When the Lead lands catalog commands, this probe flips and
 * the queue's engine binding (ConvertScreen's dispatch seam) starts
 * forwarding — no other UI change needed.
 */
export function laneCommandsOf(listings: readonly CommandListing[]): readonly CommandListing[] {
  return listings.filter(
    (listing) => listing.id.startsWith("convert.") || listing.id.startsWith("record."),
  );
}

/** `system.list-commands` request ids this screen uses (module-local). */
export const LIST_COMMANDS_REQUEST_ID = 0x4400;

/**
 * Format a progress fraction as a display percent ("42%") — computed, not
 * a JSX literal (i18n rule: copy from keys; numbers from code).
 */
export function formatProgressLabel(fraction: number): string {
  const percent = Math.round(clampFraction(fraction) * 100);
  return `${percent}%`;
}

/**
 * The display name of a queued source: the final path segment (both `/`
 * and `\` separators — Windows paths pasted into any host), the whole
 * string when it has none. Pure display sugar; the full path stays in the
 * job row's `title` attribute for a11y.
 */
export function sourceBasename(path: string): string {
  const cut = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return cut === -1 ? path : path.slice(cut + 1);
}
