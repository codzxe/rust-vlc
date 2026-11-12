// /convert (card 7.E3, issue #44) — the conversion screen: a source picker,
// a target picker limited to the ENGINE's real capability (WAV record /
// MP4-remux — see ./convertTargets.ts), and a job queue whose lifecycle
// mirrors the transcode engine's job outcomes.
//
// W26 Figma language (w26.b13 rebuild; no direct frame — CONTRACT §3): the
// composition follows the design language (§2) with the nearest grammars —
//   * Settings - *.svg — the working-screen page head (15px-class semibold
//     heading + one quiet subtitle) and the section-header grammar
//     (SectionHeader primitive);
//   * Onboarding.svg / Settings - global.svg — the "input + Browse" source
//     row (the .lumen-browse combo; the action is the honest
//     disabled-from-data stub — no catalog command exposes a native picker);
//   * Dropdown.svg / Setting Preset.svg — the target picker rides the menu
//     API (Dropdown primitive, compact settings density);
//   * playlist.svg — the queue rows (name + meta + right-aligned status,
//     r6 hover fill) inside the r14 bordered Card primitive;
//   * Open network stream.svg — the footer action row (primary action
//     right-aligned, 100px r6 buttons).
//
// CAPABILITY HONESTY (the card's acceptance criterion — "no capability
// shown that is not implemented"):
//   * the target picker offers exactly the two operations crates/transcode
//     implements, each with its honest note (WAV: decode → uncompressed
//     PCM, lossless widening only; MP4: H.264/AAC stream copy, other
//     codecs refused by the engine);
//   * the queue is driven by the ENGINE-BINDING SEAM below: the screen
//     probes the command catalog (`system.list-commands`, the customize
//     panels' capability-gate precedent) for a conversion lane. Today the
//     catalog carries NO convert.* / record.* commands (verified against
//     crates/commands — Lead-owned interface) and product.toml keeps
//     recording/transcode features off per rule F2. w26.i18 (h19 §4): the
//     queue lane is now DISABLED until that seam exists — the primary
//     action renders disabled-from-data with the gate line said ONCE in
//     the queue card (aria-describedby wires the reason to the action),
//     instead of letting users perform the enqueue→blocked ritual. The UI
//     NEVER simulates progress or completion;
//   * cancel drives the same lifecycle the engine's JobHandle.cancel
//     would (queued/running jobs only; a cancelled running job keeps its
//     last progress — the honest partial-output shape).
//
// The screen renders NO <main> of its own — the AppShell provides it. (The
// Solstice data-backdrop veil is retired with the v2 aesthetic, W25 §0.)

import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { Dropdown } from "../../components/menu/Dropdown";
import { Card, SectionHeader } from "../../components/surface/Surface";
import { useClient } from "../../client/ClientProvider";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b13";
import { keys as i18Keys } from "../../i18n/keys/w26.i18";
import type { CommandListing } from "../../generated/commands";
import { CONVERT_TARGETS, convertTargetDef, isConvertTargetId } from "./convertTargets";
import type { ConvertTargetId } from "./convertTargets";
import {
  LIST_COMMANDS_REQUEST_ID,
  canEnqueue,
  cancellable,
  convertQueueReduce,
  formatProgressLabel,
  laneCommandsOf,
  sourceBasename,
} from "./convertQueue";
import type { ConvertJob, ConvertJobState } from "./convertQueue";
import "./convert.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead). */
type LaneKey = keyof typeof laneKeys | keyof typeof i18Keys;
const laneCatalog: Readonly<Record<string, string>> = { ...laneKeys, ...i18Keys };

/**
 * useConvertMsg — `useMsg` extended with this lane's pre-registration keys.
 * Lane keys win while unregistered; once the Lead merges them into the
 * catalog both paths return the same string (w26.b12 discovery precedent).
 */
function useConvertMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

/** The lane probe's settled shape: what the catalog exposes + resolved. */
interface LaneState {
  readonly commands: readonly CommandListing[];
  readonly resolved: boolean;
}

const LANE_ABSENT: LaneState = { commands: [], resolved: false };

const STATUS_KEY: Readonly<Record<ConvertJobState, "convert.job.status.queued" | "convert.job.status.running" | "convert.job.status.completed" | "convert.job.status.failed" | "convert.job.status.cancelled" | "convert.job.status.blocked">> = {
  queued: "convert.job.status.queued",
  running: "convert.job.status.running",
  completed: "convert.job.status.completed",
  failed: "convert.job.status.failed",
  cancelled: "convert.job.status.cancelled",
  blocked: "convert.job.status.blocked",
};

export function ConvertScreen() {
  const m = useConvertMsg();
  const client = useClient();
  const navigate = useNavigate();
  const [sourcePath, setSourcePath] = useState("");
  const [target, setTarget] = useState<ConvertTargetId>("wav");
  const [queue, setQueue] = useState<readonly ConvertJob[]>([]);
  const [lane, setLane] = useState<LaneState>(LANE_ABSENT);
  const [formError, setFormError] = useState<string | null>(null);
  const nextRequestId = useRef(LIST_COMMANDS_REQUEST_ID);

  // The lane probe (module docs): one `system.list-commands` per transport.
  // A failed or non-listing answer resolves to the ABSENT lane — absence of
  // proof is never proof of support (the capabilities.ts precedent).
  useEffect(() => {
    let cancelled = false;
    nextRequestId.current += 1;
    client
      .send({ cmd: "system.list-commands", requestId: nextRequestId.current })
      .then((response) => {
        if (cancelled) return;
        const result = response.result;
        const listings =
          "completed" in result && result.completed.kind === "command-listings"
            ? result.completed.commands
            : [];
        setLane({ commands: laneCommandsOf(listings), resolved: true });
      })
      .catch(() => {
        if (!cancelled) setLane({ commands: [], resolved: true });
      });
    return () => {
      cancelled = true;
    };
  }, [client]);

  const laneAvailable = lane.commands.length > 0;
  const targetDef = convertTargetDef(target);
  // w26.i18 (h19 §4): the queue lane is disabled-from-data once the probe
  // settles on an absent lane — the enqueue→blocked ritual is gone; the
  // gate line in the queue card (id convert-lane-note) says it once and
  // aria-describedby wires it to the action. While the probe is in flight
  // the action stays as-is (no verdict yet — never presumptuous).
  const laneGate = lane.resolved && !laneAvailable;

  const startJob = (): void => {
    // The capability gate: no conversion lane → no job, no ritual. The
    // engine-binding seam below is the single branch that changes when
    // the Lead lands catalog commands.
    if (laneGate) return;
    const path = sourcePath.trim();
    if (path.length === 0) {
      setFormError(m("convert.source.required"));
      return;
    }
    if (!canEnqueue(queue)) {
      setFormError(m("convert.queue.full"));
      return;
    }
    setFormError(null);
    setSourcePath("");
    setQueue((current) => {
      // THE ENGINE-BINDING SEAM (module docs). When the Lead lands catalog
      // commands, this is the single branch that changes: dispatch the
      // start request and drive progress/complete/fail/cancel from the
      // engine's lifecycle events. Until then the lane gate above keeps
      // this branch unreachable — no fabricated request is ever sent (no
      // convert.* command id exists).
      return convertQueueReduce(current, {
        kind: "enqueue",
        sourcePath: path,
        target,
      });
    });
  };

  const cancelJob = (id: number): void => {
    setQueue((current) => convertQueueReduce(current, { kind: "cancel", id }));
  };

  return (
    <div className="convert" data-screen="convert">
      {/* Page head — the compact working-screen scale (Settings top-bar
       * grammar: 15px-class semibold heading + one quiet subtitle line). */}
      <header className="convert__head">
        <h1 id="convert-title" className="convert__title">
          {m("convert.title")}
        </h1>
        <p className="convert__subtitle">{m("convert.subtitle")}</p>
      </header>

      {/* The conversion panel — one r14 bordered card (Settings - *.svg
       * content-card grammar) carrying the settings rows + footer. */}
      <form
        className="lumen-card convert__card"
        aria-labelledby="convert-title"
        data-testid="convert-card"
        onSubmit={(event) => {
          event.preventDefault();
          startJob();
        }}
      >
        {/* Source row — Settings - global.svg "Log file" grammar: label
         * left, the input + Browse combo right (the .lumen-browse row). The
         * Browse action is the honest disabled-from-data stub: no catalog
         * command (nor desktop event) exposes a native file picker to the
         * UI yet, so it renders with the recorded reason instead of a dead
         * enabled button (extensions.tsx precedent). */}
        <div className="convert__field convert__field--row convert__field--top">
          <label className="convert__field-label" htmlFor="convert-source">
            {m("convert.source.label")}
          </label>
          <div className="lumen-browse convert__source-combo">
            <input
              id="convert-source"
              className="lumen-field__control lumen-browse__control"
              placeholder={m("convert.source.placeholder")}
              value={sourcePath}
              spellCheck={false}
              autoComplete="off"
              aria-invalid={formError !== null || undefined}
              aria-describedby={formError !== null ? "convert-source-error" : undefined}
              onChange={(event) => {
                setSourcePath(event.target.value);
                if (formError !== null) setFormError(null);
              }}
            />
            <button
              type="button"
              className="lumen-browse__action"
              disabled
              aria-describedby="convert-browse-gate"
              data-wiring-stub="native file picker — no catalog command exposed (card 7.E3)"
            >
              {m("convert.source.browse")}
            </button>
          </div>
        </div>
        <p className="convert__field-gate" id="convert-browse-gate">
          {m("convert.source.browse-gate")}
        </p>
        {formError !== null ? (
          <p className="lumen-field__error" id="convert-source-error">
            {formError}
          </p>
        ) : null}

        {/* Target row — the settings label-left / control-right grammar
         * over the menu API (Dropdown primitive; exactly the engine's two
         * real operations, honest note under the control). */}
        <div className="convert__field convert__field--row">
          <span className="convert__field-label" id="convert-target-label">
            {m("convert.target.label")}
          </span>
          <div className="convert__target-control">
            <Dropdown
              options={CONVERT_TARGETS.map((def) => ({
                value: def.id,
                labelKey: def.labelKey,
              }))}
              value={target}
              onChange={(value) => {
                if (isConvertTargetId(value)) setTarget(value);
              }}
              label={m("convert.target.label")}
              resolveLabel={m}
              density="compact"
            />
          </div>
        </div>
        <p className="convert__target-note" data-testid="convert-target-note">
          {m(targetDef.noteKey)}
        </p>

        {/* Footer — Open network stream.svg grammar: actions right-aligned,
         * the primary submit first, 100px r6 (Button primitive). w26.i18:
         * disabled-from-data while no conversion lane exists (the gate
         * line below is its aria-describedby reason — said once). */}
        <div className="convert__actions">
          <Button
            variant="default"
            onClick={() => navigate("/player")}
            data-testid="convert-back"
          >
            {m("convert.back")}
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={laneGate}
            aria-describedby={laneGate ? "convert-lane-note" : undefined}
            data-wiring-stub={
              laneGate ? "conversion lane — no catalog command exposed (card 7.E3)" : undefined
            }
            data-testid="convert-start"
          >
            {m("convert.start")}
          </Button>
        </div>
      </form>

      {/* Queue lane: section header at the column edge (Settings grammar,
       * quiet count in the trailing slot) + the r14 card holding rows. */}
      <section className="convert__queue-section" aria-labelledby="convert-queue-title">
        <SectionHeader
          as="h2"
          id="convert-queue-title"
          trailing={
            <span className="convert__queue-count">
              <span className="convert__queue-count-num">{String(queue.length)}</span>{" "}
              {m("convert.queue.count-suffix")}
            </span>
          }
        >
          {m("convert.queue.title")}
        </SectionHeader>
        <Card className="convert__queue" data-testid="convert-queue">
          {lane.resolved && !laneAvailable ? (
            <p className="convert__lane-note" id="convert-lane-note" data-testid="convert-lane-note">
              {m("convert.lane.gate")}
            </p>
          ) : null}
          {queue.length === 0 ? (
            <p className="convert__queue-empty">{m("convert.queue.empty")}</p>
          ) : (
            <ul className="convert__queue-list">
              {queue.map((job) => (
                <li key={job.id} className="convert__job" data-state={job.state}>
                  <div className="convert__job-line">
                    <span className="convert__job-name" title={job.sourcePath}>
                      {sourceBasename(job.sourcePath)}
                    </span>
                    <span
                      className="convert__job-status"
                      data-testid={`convert-job-status-${job.id}`}
                    >
                      {m(STATUS_KEY[job.state])}
                    </span>
                  </div>
                  <div className="convert__job-line convert__job-line--meta">
                    <span className="convert__job-target">
                      {m(convertTargetDef(job.target).labelKey)}
                    </span>
                    <span className="convert__job-side">
                      {job.state === "running" && job.progress !== null ? (
                        <span className="convert__job-partial">
                          {formatProgressLabel(job.progress)}
                        </span>
                      ) : null}
                      {job.state === "cancelled" && job.progress !== null ? (
                        <span className="convert__job-partial">
                          {formatProgressLabel(job.progress)}
                        </span>
                      ) : null}
                      {cancellable(job.state) ? (
                        <Button
                          variant="default"
                          size="sm"
                          className="convert__job-cancel"
                          onClick={() => cancelJob(job.id)}
                          data-testid={`convert-job-cancel-${job.id}`}
                        >
                          {m("convert.job.cancel")}
                        </Button>
                      ) : null}
                    </span>
                  </div>
                  {job.state === "running" && job.progress !== null ? (
                    <progress
                      className="convert__job-progress"
                      value={job.progress}
                      max={1}
                      aria-label={m("convert.job.progress")}
                    >
                      {formatProgressLabel(job.progress)}
                    </progress>
                  ) : null}
                  {job.detail !== null ? (
                    <p
                      className="convert__job-detail"
                      data-testid={`convert-job-detail-${job.id}`}
                    >
                      {job.detail}
                    </p>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </Card>
      </section>
    </div>
  );
}
