// Card 4.C3 — the five playback UX-state overlays (doc 08 §5.3/§5.4):
// Loading, Buffering, Unsupported, Error, Empty (first-run). Each is a
// stage-tag component: token-only styling (states.css), reduced-motion
// respect (spinner/progress animation lives inside a no-preference media
// query), i18n keys only (no raw JSX literals), and engine-truth data only —
// no fabricated progress, success, or format info (doc 05 §1 C1–C8).
//
// W26 FIGMA REDESIGN (2026-11-11, design/figma-fidelity-spec.md §2 "Player"):
// the Solstice stage (sun disc, washi-taped notes) is retired. New grammar —
// * Empty/first-run: the onboarding-style hero (spec §2: "empty state = the
//   onboarding-style hero, not a blank void") — orange cone mark ported
//   frame-exact from Onboarding.svg, display heading, secondary body, one
//   orange primary CTA (r6) opening media via the shell-wide event.
// * Loading/Buffering: quiet states on a dark scrim — thin 2px orange-arc
//   ring spinner (28px) + caption label; a determinate engine percentage
//   renders a slim 4px progress bar (orange fill, deep track, pill ends)
//   like the Player.svg seekbar grammar.
// * Unsupported/Error: honest cards in the Figma panel grammar — r12
//   surface-panel, hairline stroke-border border, overlay shadow, secondary
//   text, one primary action. Never invented detail (doc 05 §1).

import { Button, Icon } from "../../../components";
import { useMsg, type MessageKey } from "../../../i18n/messages";
import type { CommandError } from "../../../generated/commands";
import {
  ERROR_ACTION_MESSAGE_KEYS,
  STAGE_MESSAGE_KEYS,
  unsupportedFormatInfo,
  type UnsupportedDetailKey,
} from "./error-display";

/** The shell-wide open-media contract (app-shell redesign): the app-shell
 * file dialog (another workstream) listens for exactly this event name on
 * `window`. The `onOpenMedia` prop remains as the composition seam for
 * hosts/tests — both fire, the event first. */
const OPEN_MEDIA_EVENT = "lumen:open-media";

function dispatchOpenMedia(onOpenMedia?: () => void): void {
  window.dispatchEvent(new CustomEvent(OPEN_MEDIA_EVENT));
  onOpenMedia?.();
}

/** The Lumen cone mark — frame-exact port of the Onboarding.svg hero logo
 * (the traffic-cone glyph: base plinth, band, tip as three stacked paths,
 * `fill: currentColor` so the accent token drives ink). Geometry is
 * frame-measured; the viewBox crops the source art box. */
function ConeMark({ className }: { className?: string }) {
  return (
    // frame: Onboarding.svg (Icon/Frame 4) — three-segment cone, 106×125 art box
    <svg
      className={className}
      viewBox="492 122 107 126"
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      <path
        d="M573.648 200.666L575.932 207.52C569.082 212.546 558.58 217.116 545.795 217.116C532.553 217.116 522.508 212.546 515.659 207.52L517.942 200.666H507.44C505.156 200.666 503.787 202.036 502.873 203.864L492.828 242.247C491.915 244.989 494.198 248.188 497.394 248.188H593.739C596.936 248.188 598.762 245.446 598.306 242.247L587.803 203.864C587.347 202.036 585.52 200.666 583.237 200.666H573.648Z"
        fill="currentColor"
      />
      <path
        d="M521.138 187.413C526.618 191.526 534.837 195.181 545.339 195.181C555.841 195.181 564.06 191.069 569.539 187.413L563.147 165.48C559.037 168.222 553.101 170.049 545.339 170.049C537.576 170.049 531.64 167.765 527.531 165.48L521.138 187.413Z"
        fill="currentColor"
      />
      <path
        d="M533.466 144.461L537.575 130.296C539.858 122.985 550.361 122.985 552.644 130.296L556.753 144.461C553.557 145.375 549.904 146.289 545.338 146.289C540.772 146.289 537.119 145.375 533.466 144.461Z"
        fill="currentColor"
      />
    </svg>
  );
}

/** Shared overlay chrome: sits on the canvas region, one visible at a time.
 * Transient states (loading/buffering) are polite live regions; failure
 * surfaces (error/unsupported) are assertive alerts (doc 08 §5.4 “errors are
 * events”); first-run empty is static content. `card` upgrades the centered
 * cluster to the Figma panel grammar (r12 surface-panel + hairline border +
 * shadow) for the failure surfaces. */
function OverlayShell({
  kind,
  busy = false,
  role = "status",
  card = false,
  children,
}: {
  kind: string;
  busy?: boolean;
  role?: "status" | "alert" | "presentation";
  /** Render the cluster as a Figma panel card (failure surfaces). */
  card?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div
      className="lumen-state-overlay"
      data-overlay={kind}
      aria-busy={busy || undefined}
      role={role}
    >
      <div className="lumen-state-overlay__cluster" data-card={card ? "" : undefined}>
        {children}
      </div>
    </div>
  );
}

/** Determinate/indeterminate progress indicator. `progress` is the engine's
 * 0..=1 value from `session.opening-progress`/`session.buffering`; null =
 * no engine percentage exists → indeterminate, NEVER a fake number. The
 * quiet reading: a thin ring spinner always; the slim bar renders only when
 * the engine actually reported a percentage. */
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
      <span className="lumen-state-overlay__spinner" aria-hidden="true" />
      {determinate ? (
        <span className="lumen-state-overlay__bar" aria-hidden="true">
          <span className="lumen-state-overlay__fill" style={{ width: `${percent}%` }} />
        </span>
      ) : null}
    </div>
  );
}

/** Loading — `opening` state: initial open, no first frame yet (doc 08 §5.3).
 * Quiet grammar: ring spinner + caption over the dark canvas scrim. */
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
    <OverlayShell kind="unsupported" role="alert" card>
      <Icon name="about" size={20} className="lumen-state-overlay__icon" />
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
 * never internal paths, never “recovered” unless the engine said so.
 * Figma grammar: r12 panel card, secondary text, primary retry button. */
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
    <OverlayShell kind="error" role="alert" card>
      <Icon name="about" size={20} className="lumen-state-overlay__icon" />
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
        variant="primary"
        disabled={onRetry === undefined}
        data-wiring-stub={onRetry === undefined ? "retry command dispatch — card 4.C2" : undefined}
        onClick={onRetry}
      >
        {m("player.state.error.retry")}
      </Button>
    </OverlayShell>
  );
}

/** Empty / first-run — no media open (`idle`). W26 redesign: the
 * onboarding-style hero per the Figma fidelity spec (§2 Player: "empty
 * state = the onboarding-style hero, not a blank void") — the cone mark
 * (frame-exact Onboarding.svg port, accent ink), a display-scale heading,
 * secondary body copy, and the orange primary CTA opening media through the
 * shell-wide `lumen:open-media` event (the app-shell dialog listens for
 * exactly that name) — a real affordance, enabled from the start. */
export function EmptyOverlay({ onOpenMedia }: { onOpenMedia?: () => void }) {
  const m = useMsg();
  return (
    <div className="lumen-state-hero" data-empty-state="first-run" data-overlay="empty">
      <div className="lumen-state-hero__group">
        <ConeMark className="lumen-state-hero__mark" />
        <h2 className="lumen-state-hero__title">{m("player.empty.title")}</h2>
        <p className="lumen-state-hero__body">{m("player.empty.body")}</p>
        <Button variant="primary" onClick={() => dispatchOpenMedia(onOpenMedia)}>
          {m("player.empty.open")}
        </Button>
      </div>
    </div>
  );
}
