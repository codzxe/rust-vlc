// /convert (card 7.E3, issue #44) — the conversion screen: a source picker,
// a target picker limited to the ENGINE's real capability (WAV record /
// MP4-remux — see ./convertTargets.ts), and a job queue whose lifecycle
// mirrors the transcode engine's job outcomes.
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
//     recording/transcode features off per rule F2, so every enqueued job
//     transitions queued → blocked with the typed not-exposed reason and
//     the panel shows the honest unavailable banner. The UI NEVER
//     simulates progress or completion — a blocked row is the truth;
//   * cancel drives the same lifecycle the engine's JobHandle.cancel
//     would (queued/running jobs only; a cancelled running job keeps its
//     last progress — the honest partial-output shape).
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { TextField } from "../../components/form/TextField";
import { RadioGroup } from "../../components/form/RadioGroup";
import { useClient } from "../../client/ClientProvider";
import { useMsg } from "../../i18n/messages";
import type { CommandListing } from "../../generated/commands";
import { CONVERT_TARGETS, convertTargetDef } from "./convertTargets";
import type { ConvertTargetId } from "./convertTargets";
import {
  LIST_COMMANDS_REQUEST_ID,
  canEnqueue,
  cancellable,
  convertQueueReduce,
  formatProgressLabel,
  laneCommandsOf,
  nextJobId,
  sourceBasename,
} from "./convertQueue";
import type { ConvertJob, ConvertJobState } from "./convertQueue";
import "./convert.css";

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
  const m = useMsg();
  const client = useClient();
  const navigate = useNavigate();
  const [sourcePath, setSourcePath] = useState("");
  const [target, setTarget] = useState<ConvertTargetId>("wav");
  const [queue, setQueue] = useState<readonly ConvertJob[]>([]);
  const [lane, setLane] = useState<LaneState>(LANE_ABSENT);
  const [formError, setFormError] = useState<string | null>(null);
  const nextRequestId = useRef(LIST_COMMANDS_REQUEST_ID);

  // Solstice v2 (brief §1): APP-tier screen — calm backdrop while mounted,
  // cleaned up on unmount.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

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

  const startJob = (): void => {
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
      const id = nextJobId(current);
      const enqueued = convertQueueReduce(current, {
        kind: "enqueue",
        sourcePath: path,
        target,
      });
      // THE ENGINE-BINDING SEAM (module docs). The catalog exposes no
      // conversion lane today, so the job is blocked with the typed
      // not-exposed reason — a local refusal with the same meaning as the
      // wire's `feature-not-exposed` error code, never a fabricated
      // request (no convert.* command id exists to send). When the Lead
      // lands catalog commands, this is the single branch that changes:
      // dispatch the start request and drive progress/complete/fail/cancel
      // from the engine's lifecycle events.
      if (!laneAvailable) {
        return convertQueueReduce(enqueued, {
          kind: "block",
          id,
          reason: m("convert.job.blocked.reason"),
        });
      }
      return enqueued;
    });
  };

  const cancelJob = (id: number): void => {
    setQueue((current) => convertQueueReduce(current, { kind: "cancel", id }));
  };

  return (
    <div className="convert">
      <div className="convert__group">
        <section
          className="convert__card solstice-rise"
          aria-labelledby="convert-title"
          data-testid="convert-card"
        >
          <h1 id="convert-title" className="convert__title">
            {m("convert.title")}
          </h1>
          <p className="convert__subtitle">{m("convert.subtitle")}</p>

          <form
            className="convert__form"
            onSubmit={(event) => {
              event.preventDefault();
              startJob();
            }}
          >
            <TextField
              label={m("convert.source.label")}
              placeholder={m("convert.source.placeholder")}
              value={sourcePath}
              error={formError ?? undefined}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => {
                setSourcePath(event.target.value);
                if (formError !== null) setFormError(null);
              }}
            />
            <div className="convert__target">
              <RadioGroup
                label={m("convert.target.label")}
                value={target}
                onChange={(value) => {
                  if (value === "wav" || value === "mp4") setTarget(value);
                }}
                options={CONVERT_TARGETS.map((def) => ({
                  value: def.id,
                  label: m(def.labelKey),
                }))}
              />
              <p className="convert__target-note" data-testid="convert-target-note">
                {m(targetDef.noteKey)}
              </p>
            </div>
            <div className="convert__actions">
              <Button type="submit" variant="primary" data-testid="convert-start">
                {m("convert.start")}
              </Button>
              <Button
                variant="default"
                onClick={() => navigate("/player")}
                data-testid="convert-back"
              >
                {m("convert.back")}
              </Button>
            </div>
          </form>
        </section>

        <section
          className="convert__queue"
          aria-labelledby="convert-queue-title"
          data-testid="convert-queue"
        >
          <h2 id="convert-queue-title" className="convert__queue-title">
            {m("convert.queue.title")}
          </h2>
          {lane.resolved && !laneAvailable ? (
            <p className="convert__lane-note" data-testid="convert-lane-note">
              {m("convert.lane.unavailable")}
            </p>
          ) : null}
          {queue.length === 0 ? (
            <p className="convert__queue-empty">{m("convert.queue.empty")}</p>
          ) : (
            <ul className="convert__queue-list">
              {queue.map((job) => (
                <li key={job.id} className="convert__job" data-state={job.state}>
                  <span className="convert__job-name" title={job.sourcePath}>
                    {sourceBasename(job.sourcePath)}
                  </span>
                  <span className="convert__job-target">
                    {m(convertTargetDef(job.target).labelKey)}
                  </span>
                  <span className="convert__job-status" data-testid={`convert-job-status-${job.id}`}>
                    {m(STATUS_KEY[job.state])}
                  </span>
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
                  {job.state === "cancelled" && job.progress !== null ? (
                    <span className="convert__job-partial">
                      {formatProgressLabel(job.progress)}
                    </span>
                  ) : null}
                  {cancellable(job.state) ? (
                    <Button
                      variant="default"
                      className="convert__job-cancel"
                      onClick={() => cancelJob(job.id)}
                      data-testid={`convert-job-cancel-${job.id}`}
                    >
                      {m("convert.job.cancel")}
                    </Button>
                  ) : null}
                  {job.detail !== null ? (
                    <p className="convert__job-detail" data-testid={`convert-job-detail-${job.id}`}>
                      {job.detail}
                    </p>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
