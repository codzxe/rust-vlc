// * (not-found) — the W26 404 (w26.b13): a CENTERED QUIET HERO, not a
// stage (the Solstice sunset scene died with the v2 aesthetic, W25 §0).
// The composition follows the onboarding hero grammar (Onboarding.svg) at
// a smaller scale: the cone mark in accent ink (frame-exact port), a
// display-scale "Page not found" heading, one quiet body line, and exactly
// ONE way back to the player. No backdrop tier, no scenery — the near-black
// app surface IS the stage.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b13";
import "./not-found.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead). */
type LaneKey = keyof typeof laneKeys;
const laneCatalog: Readonly<Record<string, string>> = laneKeys;

/**
 * useNotFoundMsg — `useMsg` extended with this lane's pre-registration keys
 * (w26.b12 discovery precedent). REGISTERED (w26.b19): the catalog now owns
 * these keys, so the CATALOG wins (pseudo locale + l10n tooling flow through
 * it); the lane table remains only as a never-hit fallback.
 */
function useNotFoundMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

/**
 * ConeMark — the traffic-cone logo art (frame: Onboarding.svg Icon/Frame 4,
 * the same 107×126 three-segment mark the onboarding hero and the player
 * empty-state use; geometry is frame-measured, duplicated here because the
 * onboarding module does not export it — cross-screen imports stay one-way
 * per the lane rules). `fill: currentColor` so the accent token drives ink.
 */
function ConeMark({ className }: { className?: string }): React.ReactElement {
  return (
    // frame: Onboarding.svg (Icon/Frame 4) — three-segment cone, 107×126 art box
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

export function NotFoundScreen() {
  const m = useNotFoundMsg();
  const navigate = useNavigate();

  return (
    <div className="not-found" data-screen="not-found">
      <section className="not-found__hero" aria-labelledby="not-found-title">
        {/* The cone mark — the onboarding hero's logo art, accent ink. */}
        <ConeMark className="not-found__cone" />
        <h1 id="not-found-title" className="not-found__title">
          {m("not-found.heading")}
        </h1>
        <p className="not-found__body">{m("not-found.body-quiet")}</p>
        {/* One clear way home — the orange primary pill (Onboarding "Next"
         * grammar). */}
        <Button
          variant="primary"
          className="not-found__cta"
          onClick={() => navigate("/player")}
        >
          {m("not-found.action")}
        </Button>
      </section>
    </div>
  );
}
