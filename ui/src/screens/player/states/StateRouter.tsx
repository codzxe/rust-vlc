// Card 4.C3 — StateRouter: maps the engine/session state to the overlay that
// renders over the (4.C1) canvas region (doc 08 §5.3/§5.4). Pure mapping +
// one component; the engine view comes from context-bridge (or C2's canonical
// context once it lands) — never from optimistic UI state (doc 05 §1 C4).

import type { PlayerEngineView } from "./context-bridge";
import { isUnsupportedError } from "./error-display";
import {
  BufferingOverlay,
  ErrorOverlay,
  IdleSurface,
  LoadingOverlay,
  UnsupportedOverlay,
} from "./StageOverlays";

/** Which overlay is up. `none` = canvas alone (playing/paused/stopped/ended). */
export type OverlayKind = "loading" | "buffering" | "unsupported" | "error" | "empty" | "none";

/** Pure state → overlay mapping (the state matrix, doc 08 §5.1–§5.4):
 * - `idle` — no session → the idle surface: first-run Empty hero, or the
 *   Player-5 home face once the transport remembers openings (w26.gfix-C,
 *   g04 P1-7 — StageOverlays' IdleSurface picks the face).
 * - `opening` — Loading (auto-dismissed by first playing, replaced by error).
 * - `buffering` — Buffering indicator; controls stay interactive (§5.3).
 * - `error` — Unsupported surface for typed `unsupported` codes, else the
 *   stage-tagged Error surface. No payload → honest unknown, never invented.
 * - `playing`/`paused`/`stopped`/`ended` — no overlay; the canvas is truth. */
export function resolveOverlay(view: PlayerEngineView): OverlayKind {
  const { state } = view;
  switch (state) {
    case "idle":
      return "empty";
    case "opening":
      return "loading";
    case "buffering":
      return "buffering";
    case "error":
      return view.error !== null && isUnsupportedError(view.error) ? "unsupported" : "error";
    case "playing":
    case "paused":
    case "stopped":
    case "ended":
      return "none";
  }
}

export interface StateRouterProps {
  /** Engine-truth view (from usePlayerEngineView / C2's context). */
  readonly view: PlayerEngineView;
  /** Retry an open of the same source (Error surface affordance — doc 08
   * §5.4: only where the command layer offers a meaningful action). */
  readonly onRetry?: () => void;
  /** Open (different) media — Empty + Unsupported affordances. */
  readonly onOpenMedia?: () => void;
}

/** Renders the resolved overlay over the canvas region. Exactly zero or one
 * overlay is ever mounted; `data-state-overlay` carries the kind for tests
 * and layout tooling. */
export function StateRouter({ view, onRetry, onOpenMedia }: StateRouterProps) {
  const kind = resolveOverlay(view);
  switch (kind) {
    case "empty":
      return <IdleSurface onOpenMedia={onOpenMedia} />;
    case "loading":
      return <LoadingOverlay progress={view.openingProgress} />;
    case "buffering":
      return <BufferingOverlay progress={view.bufferingProgress} />;
    case "unsupported":
      return <UnsupportedOverlay error={view.error as NonNullable<typeof view.error>} onOpenMedia={onOpenMedia} />;
    case "error":
      return <ErrorOverlay error={view.error} onRetry={onRetry} />;
    case "none":
      return null;
  }
}
