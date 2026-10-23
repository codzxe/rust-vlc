// Card 4.F2 — MiniPlayer: the compact always-on-top-style card (doc 08 §1.4
// "Miniplayer | card 391×339 rx14 #1B1B1B + shadow; thumbnail 373×209 rx5 +
// 2px #090909 stroke; seek 375×16 rx6, knob 8×12 rx4").
//
// Transport is NOT duplicated: the compact cluster reuses card 4.C2's
// control logic (controls/state-mapping.ts mapTransportControls /
// seekDisableReason) and the shared components + player state seam
// (context.tsx) — only the COMPACT COMPOSITION is variant-owned (the design
// matrix above in variantLayout.ts). Geometry lives in variants.css with
// semantic tokens where they exist; raw px only for the design's structural
// card dimensions with no token (C1's player.css precedent).

import { IconButton, Icon, Seekbar } from "../../../components";
import { useMsg } from "../../../i18n/messages";
import { usePlayerControls } from "../context";
import { mapTransportControls, seekDisableReason } from "../controls";
import type { LayoutRegion } from "../layout";
import "./variants.css";

export interface MiniPlayerProps {
  /** The compact canvas region (mini-thumbnail from the miniplayer layout). */
  readonly thumbnailRegion: LayoutRegion;
  /** The compact transport region (mini-controls from the miniplayer layout). */
  readonly controlsRegion: LayoutRegion;
  /** Expand back to the full variant (variant-switch seam). */
  readonly onExpand: () => void;
  /**
   * Close affordance — host/shell-owned. Renders only when provided (an
   * unwired affordance would be a fake button; doc 05 §1 C4).
   */
  readonly onClose?: () => void;
}

export function MiniPlayer({
  thumbnailRegion,
  controlsRegion,
  onExpand,
  onClose,
}: MiniPlayerProps) {
  const m = useMsg();
  const { state, actions } = usePlayerControls();
  const mapped = mapTransportControls(state);
  const reason = seekDisableReason(state);

  const playLabel =
    mapped.play.glyph === "pause" ? m("player.controls.pause") : m("player.controls.play");

  return (
    <section
      className="lumen-mini"
      data-variant="miniplayer"
      role="region"
      aria-label={m("player.variant.miniplayer.label")}
    >
      <div
        data-region-id={thumbnailRegion.id}
        role="region"
        aria-label={m(thumbnailRegion.labelKey)}
        className="lumen-mini__thumb"
        data-placeholder="engine-wiring-4.H1"
      >
        {/* Honest placeholder: the thumbnail is the video-canvas seam; no
            media element until card 4.H1 (same contract as the full shell). */}
        <p className="lumen-player__sr-note">{m("player.canvas.placeholder")}</p>
      </div>

      <footer
        data-region-id={controlsRegion.id}
        role="region"
        aria-label={m(controlsRegion.labelKey)}
        className="lumen-mini__bar"
      >
        <IconButton
          data-control="play-pause"
          data-glyph={mapped.play.glyph}
          label={playLabel}
          loading={mapped.play.pending}
          disabled={mapped.play.disabled}
          onClick={actions.togglePlayPause}
        >
          <Icon name={mapped.play.glyph} />
        </IconButton>
        <Seekbar
          data-control="seekbar"
          data-variant="mini"
          aria-busy={mapped.seek.busy || undefined}
          aria-label={
            reason !== null
              ? `${m("player.controls.seekbar")} (${m(reason)})`
              : m("player.controls.seekbar")
          }
          value={state.position}
          duration={state.durationSeconds ?? 0}
          buffered={state.buffered}
          pending={state.pendingSeek ?? undefined}
          disabled={mapped.seek.disabled}
          onCommit={actions.seek}
        />
        <IconButton
          data-control="expand"
          label={m("player.variant.miniplayer.expand")}
          onClick={onExpand}
        >
          <Icon name="zoom" />
        </IconButton>
        {onClose === undefined ? null : (
          <IconButton
            data-control="close"
            label={m("player.variant.miniplayer.close")}
            onClick={onClose}
          >
            <Icon name="quit" />
          </IconButton>
        )}
      </footer>
    </section>
  );
}
