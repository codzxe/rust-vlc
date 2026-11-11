// w26.c3 — VisualizerDisplay: the levels-driven visualization area.
//
// HONEST RENDERING (mission pin): no FFT / audio-analysis source exists yet.
// The display renders ONLY what it is given —
//   * `levels` present  → one bar per entry (clamped 0..1), data-state=live;
//   * `levels` absent   → the mode's RESTING depiction (data-state=resting):
//     the frame's own icon geometry for that mode, static, never animated
//     and never simulated (the frame carries no idle shimmer — none added);
//   * mode "disabled"   → nothing at all: the frame's default state is the
//     "Disabled" dropdown with no visualization area.
//
// Bar grammar (frame, "Audio Visualizer.svg" icons): six 1.5-stroke
// round-capped bars at 4px pitch, baseline bars (Scope/Spectrum) or
// midline-centered (Waveform), plus Spectrum's accent peak dots floating
// above each bar top. The display scales that grammar up: 3px pills
// (2× the frame stroke), spread across the well, one per level — the bar
// COUNT is data-driven (levels.length), 6 at rest (frame).
//
// Colors (semantic tokens; mission pin "eq/accent token groups"):
//   * live Spectrum bars cycle the eq token group (--color-eq-1..7 — the
//     W25 "equalizer band/level accents") with accent-strong peak dots;
//   * Scope/Waveform keep the frame's two-tone alternation
//     (content-secondary / accent-strong);
//   * the Spectrometer dial is content-secondary rings + accent-strong arcs
//     (the frame's Ellipse 16/17 + 18 grammar).
//
// The well's 64px height is a house choice — the frame measures only the
// 20px icon grammar [VERIFY display scale with Lead].

import type { ReactElement } from "react";
import { restingLevels } from "./types";
import type { VisualizerLabelKey, VisualizerMode } from "./types";

/** Clamps one raw level to the 0..1 band (non-finite → 0). */
function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}

/** The label key naming a mode (aria-label composition). */
function modeLabelKey(mode: VisualizerMode): VisualizerLabelKey {
  return `player.tools.visualizer.mode.${mode}` as VisualizerLabelKey;
}

/** Public props (mountable standalone — e.g. a future music-mode host). */
export interface VisualizerDisplayProps {
  /** Rendered mode; "disabled" renders nothing (frame default). */
  mode: VisualizerMode;
  /** Live levels 0..1, one per bar; absent/empty → the resting state. */
  levels?: readonly number[];
  /** Resolves label keys (the useVisualizerMsg seam). */
  resolveLabel: (key: VisualizerLabelKey) => string;
  /** Extra classes on the well root. */
  className?: string;
}

/** Bar band metrics (px, structural — see module header). */
const BAND_PX = 48; // well inner band (64 well − 2×8 insets)
const SPECTRUM_BAR_PX = 41; // leaves the 3px peak dot + 2px gap headroom
const BAR_W_PX = 3; // 2× the frame's 1.5 stroke — the scaled pill

export function VisualizerDisplay({
  mode,
  levels,
  resolveLabel,
  className = "",
}: VisualizerDisplayProps): ReactElement | null {
  if (mode === "disabled") return null; // frame default: no display
  const live = levels !== undefined && levels.length > 0;
  const state = live ? "live" : "resting";
  const ariaLabel = `${resolveLabel(modeLabelKey(mode))} — ${
    live
      ? resolveLabel("player.tools.visualizer.display.live")
      : resolveLabel("player.tools.visualizer.display.resting")
  }`;

  return (
    <div
      className={["viz-display", className].filter(Boolean).join(" ")}
      data-mode={mode}
      data-state={state}
      role="img"
      aria-label={ariaLabel}
    >
      {mode === "spectrometer" ? (
        <SpectrometerDial levels={live ? levels : undefined} />
      ) : (
        <div className="viz-display__bars" data-align={mode === "waveform" ? "center" : "end"}>
          {(live ? levels.map(clamp01) : restingLevels(mode)).map((level, index) => (
            <BarSlot
              key={index}
              index={index}
              level={level}
              live={live}
              mode={mode}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/** One bar (+ Spectrum's peak dot) — geometry per the module header. */
function BarSlot({
  index,
  level,
  live,
  mode,
}: {
  index: number;
  level: number;
  live: boolean;
  mode: "scope" | "spectrum" | "waveform";
}): ReactElement {
  const barPx =
    mode === "spectrum"
      ? Math.max(BAR_W_PX, Math.round(level * SPECTRUM_BAR_PX))
      : Math.max(BAR_W_PX, Math.round(level * BAND_PX));
  // Live spectrum bars cycle the eq token group; every other bar keeps the
  // frame's two-tone alternation (scope: gray-first, waveform: accent-first).
  const isGray =
    mode === "spectrum" || (mode === "scope" ? index % 2 === 0 : index % 2 === 1);
  const tone =
    mode === "spectrum" && live
      ? { "data-accent": (index % 7) + 1 }
      : { "data-tone": isGray ? "gray" : "accent" };
  return (
    <div className="viz-display__slot">
      {mode === "spectrum" && (
        <span
          className="viz-display__peak"
          style={{ bottom: barPx + 2 }} /* frame: dot floats above the top */
        />
      )}
      <span
        className="viz-display__bar"
        data-level={level.toFixed(3)}
        {...tone}
        style={{ width: BAR_W_PX, height: barPx }}
      />
    </div>
  );
}

/**
 * SpectrometerDial — the frame's concentric grammar (Ellipse 16/17 circles +
 * Ellipse 18 arcs) scaled to the 48px band: resting = the icon's two rings
 * + four outer arcs; live = one ring per level, radius lerp(rMin, rMax,
 * (i + level) / n) — each band's level pushes its ring outward.
 */
function SpectrometerDial({ levels }: { levels?: readonly number[] }): ReactElement {
  const R_MIN = 6;
  const R_MAX = 20;
  const arcsR = 22.5;
  const arcsC = 2 * Math.PI * arcsR;
  const rings: number[] =
    levels === undefined
      ? [9, 16] /* frame: Ellipse 16/17 radii ×2.25 (the render scale) */
      : levels.map((level, index) =>
          R_MIN + ((index + clamp01(level)) / levels.length) * (R_MAX - R_MIN),
        );
  return (
    <svg
      className="viz-display__dial"
      viewBox="0 0 48 48"
      width={48}
      height={48}
      fill="none"
      strokeWidth={3} /* frame sw1.5 × ~2.2 dial scale [VERIFY] */
      strokeLinecap="round"
      aria-hidden="true"
      focusable="false"
    >
      {rings.map((r, index) => (
        <circle key={index} cx="24" cy="24" r={r} className="viz-tone-gray" />
      ))}
      {/* frame: Ellipse 18 — four arc segments on the outer ring */}
      <circle
        cx="24"
        cy="24"
        r={arcsR}
        className="viz-tone-accent"
        strokeDasharray={`${arcsC / 8} ${arcsC / 8}`}
        transform="rotate(22.5 24 24)"
      />
    </svg>
  );
}
