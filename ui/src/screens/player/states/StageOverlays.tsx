// Card 4.C3 — the five playback UX-state overlays (doc 08 §5.3/§5.4):
// Loading, Buffering, Unsupported, Error, Empty (first-run). Each is a
// stage-tag component: token-only styling (states.css), reduced-motion
// respect (spinner/progress animation lives inside a no-preference media
// query), i18n keys only (no raw JSX literals), and engine-truth data only —
// no fabricated progress, success, or format info (doc 05 §1 C1–C8).

import { Button, Icon } from "../../../components";
import { useMsg, type MessageKey } from "../../../i18n/messages";
import type { CommandError } from "../../../generated/commands";
import {
  ERROR_ACTION_MESSAGE_KEYS,
  STAGE_MESSAGE_KEYS,
  unsupportedFormatInfo,
  type UnsupportedDetailKey,
} from "./error-display";

/** The shell-wide open-media contract (Solstice redesign): the app-shell
 * file dialog (another workstream) listens for exactly this event name on
 * `window`. The `onOpenMedia` prop remains as the composition seam for
 * hosts/tests — both fire, the event first. */
const OPEN_MEDIA_EVENT = "lumen:open-media";

function dispatchOpenMedia(onOpenMedia?: () => void): void {
  window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT));
  onOpenMedia?.();
}

/** Shared overlay chrome: sits on the canvas region, one visible at a time.
 * Transient states (loading/buffering) are polite live regions; failure
 * surfaces (error/unsupported) are assertive alerts (doc 08 §5.4 “errors are
 * events”); first-run empty is static content. Solstice restyle: failure
 * surfaces float on a washi-taped sticky note (`note`), transient states stay
 * bare (spinner + label over the warm scrim). */
function OverlayShell({
  kind,
  busy = false,
  role = "status",
  note = false,
  children,
}: {
  kind: string;
  busy?: boolean;
  role?: "status" | "alert" | "presentation";
  /** Solstice: render the card as a taped sticky note (failure surfaces). */
  note?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div
      className="lumen-state-overlay"
      data-overlay={kind}
      aria-busy={busy || undefined}
      role={role}
    >
      <div
        className={note ? "lumen-state-overlay__card solstice-note" : "lumen-state-overlay__card"}
        data-tape={note ? "" : undefined}
      >
        {children}
      </div>
    </div>
  );
}

/** Determinate/indeterminate progress indicator. `progress` is the engine's
 * 0..=1 value from `session.opening-progress`/`session.buffering`; null =
 * no engine percentage exists → indeterminate, NEVER a fake number. */
function ProgressIndicator({
  progress,
  labelKey,
}: {
  progress: number | null;
  labelKey: MessageKey;
}) {
  const m = useMsg();
  const determinate = progress !== null;
  const percent = determinate ? Math.round(progress * 100) : null;
  return (
    <div
      className="lumen-state-overlay__progress"
      data-progress={determinate ? "determinate" : "indeterminate"}
      role="progressbar"
      aria-label={m(labelKey)}
      aria-valuemin={determinate ? 0 : undefined}
      aria-valuemax={determinate ? 100 : undefined}
      aria-valuenow={percent ?? undefined}
    >
      <span className="lumen-state-overlay__spinner solstice-sun" aria-hidden="true" />
      {determinate ? (
        <span className="lumen-state-overlay__bar" aria-hidden="true">
          <span className="lumen-state-overlay__fill" style={{ width: `${percent}%` }} />
        </span>
      ) : null}
    </div>
  );
}

/** Loading — `opening` state: initial open, no first frame yet (doc 08 §5.3). */
export function LoadingOverlay({ progress }: { progress: number | null }) {
  const m = useMsg();
  return (
    <OverlayShell kind="loading" busy>
      <ProgressIndicator progress={progress} labelKey="player.state.loading.label" />
      <p className="lumen-state-overlay__label">{m("player.state.loading.label")}</p>
    </OverlayShell>
  );
}

/** Buffering — `buffering` state: playback stalled while media is open. The
 * controls bar stays interactive (doc 08 §5.3) — this overlay never blocks it. */
export function BufferingOverlay({ progress }: { progress: number | null }) {
  const m = useMsg();
  return (
    <OverlayShell kind="buffering" busy>
      <ProgressIndicator progress={progress} labelKey="player.state.buffering.label" />
      <p className="lumen-state-overlay__label">{m("player.state.buffering.label")}</p>
    </OverlayShell>
  );
}

/** Unsupported — open failed with a typed `unsupported` error. Shows the
 * format info the error payload actually carries (doc 05 §2 rule 3 details
 * hints, e.g. `"format": "matroska"`), and the “no playback attempted”
 * wording (doc 08 §5.3) — never a black frame, never an invented codec. */
export function UnsupportedOverlay({
  error,
  onOpenMedia,
}: {
  error: CommandError;
  /** Re-issue the open flow (open a different file). Composition seam — the
   * button always opens the shell-wide file dialog via `lumen:open-media`
   * (see dispatchOpenMedia); a provided prop additionally fires. */
  onOpenMedia?: () => void;
}) {
  const m = useMsg();
  const info = unsupportedFormatInfo(error);
  const detailLabels: Readonly<Record<UnsupportedDetailKey, MessageKey>> = {
    format: "player.state.unsupported.container-label",
    codec: "player.state.unsupported.codec-label",
  };
  const hasInfo = Object.keys(info).length > 0;
  return (
    <OverlayShell kind="unsupported" role="alert" note>
      <Icon name="about" size={24} className="lumen-state-overlay__icon" />
      <h2 className="lumen-state-overlay__title">{m("player.state.unsupported.title")}</h2>
      <p className="lumen-state-overlay__body">{m("player.state.unsupported.body")}</p>
      {hasInfo ? (
        <dl className="lumen-state-overlay__facts">
          {(["format", "codec"] as const).map((key) =>
            info[key] === undefined ? null : (
              <div className="lumen-state-overlay__fact" key={key}>
                <dt className="lumen-state-overlay__fact-label">{m(detailLabels[key])}</dt>
                <dd className="lumen-state-overlay__fact-value">{info[key]}</dd>
              </div>
            ),
          )}
        </dl>
      ) : (
        <p className="lumen-state-overlay__body">{m("player.state.unsupported.no-format")}</p>
      )}
      <Button
        // Real affordance: opens the file dialog via the shell-wide
        // lumen:open-media event (see dispatchOpenMedia); the prop stays as
        // the composition seam. Never a fabricated open.
        variant="primary"
        onClick={() => dispatchOpenMedia(onOpenMedia)}
      >
        {m("player.empty.open")}
      </Button>
    </OverlayShell>
  );
}

/** Error — stage-tagged + actionable + honest (doc 08 §5.4, doc 05 C5).
 * Renders the typed error's Display strings only (redaction-safe, V8);
 * never internal paths, never “recovered” unless the engine said so. */
export function ErrorOverlay({
  error,
  onRetry,
}: {
  /** Typed error payload; absent = engine entered `error` without a payload
   * (protocol violation) → honest unknown presentation, nothing invented. */
  error: CommandError | null;
  /** Retry affordance (doc 08 §5.4: only where a meaningful action exists).
   * Absent → disabled honest stub. */
  onRetry?: () => void;
}) {
  const m = useMsg();
  const stageKey = error === null ? null : STAGE_MESSAGE_KEYS[error.stage];
  const actionKey = error === null ? null : ERROR_ACTION_MESSAGE_KEYS[error.code];
  return (
    <OverlayShell kind="error" role="alert" note>
      <Icon name="about" size={24} className="lumen-state-overlay__icon" />
      <h2 className="lumen-state-overlay__title">{m("player.state.error.title")}</h2>
      <p className="lumen-state-overlay__stagetag">
        <span className="lumen-state-overlay__stagetag-label">
          {m("player.state.error.stage-label")}
        </span>
        <span className="lumen-state-overlay__stagetag-value">
          {stageKey === null ? m("player.state.error.stage.unknown") : m(stageKey)}
        </span>
      </p>
      {error === null ? (
        <p className="lumen-state-overlay__body">{m("player.state.error.unknown")}</p>
      ) : (
        <>
          <p className="lumen-state-overlay__body">{error.message}</p>
          {actionKey === null ? null : (
            <p className="lumen-state-overlay__action">{m(actionKey)}</p>
          )}
        </>
      )}
      <Button
        // Honest stub: retry only when a real action is wired — the UI never
        // offers a control that would fake recovery (doc 05 §1 C1/C3).
        disabled={onRetry === undefined}
        data-wiring-stub={onRetry === undefined ? "retry command dispatch — card 4.C2" : undefined}
        onClick={onRetry}
      >
        {m("player.state.error.retry")}
      </Button>
    </OverlayShell>
  );
}

/** Empty / first-run — no media open (`idle`). Migrated from the 4.C1 shell
 * markup (same i18n keys). Solstice restyle: a glowing sun disc above a
 * washi-taped sticky note; the gold primary button opens media through the
 * shell-wide `lumen:open-media` event (the app-shell dialog listens for
 * exactly that name) — a real affordance, enabled from the start. */
export function EmptyOverlay({ onOpenMedia }: { onOpenMedia?: () => void }) {
  const m = useMsg();
  return (
    <div className="lumen-player__empty" data-empty-state="first-run" data-overlay="empty">
      <span className="solstice-sun lumen-player__empty-sun" aria-hidden="true" />
      <div className="solstice-note lumen-player__empty-card solstice-rise" data-tape>
        <h2 className="lumen-player__empty-title">{m("player.empty.title")}</h2>
        <p className="lumen-player__empty-body">{m("player.empty.body")}</p>
        <Button variant="primary" onClick={() => dispatchOpenMedia(onOpenMedia)}>
          {m("player.empty.open")}
        </Button>
      </div>
    </div>
  );
}
