// w26.c3 — the frame's two-tone mode icons, ported VERBATIM from
// design/figma/"Audio Visualizer.svg" (the open list's 20px leading icons,
// w26.a20 §A7: "two-tone #8F8F8F/#AB6314" — every path below is the frame's
// own geometry, cropped per icon into its measured viewBox).
//
// Colors are SEMANTIC TOKENS via the two tone classes in visualizer.css:
//   .viz-tone-gray   → var(--color-content-secondary)   /* frame #8F8F8F */
//   .viz-tone-accent → var(--color-accent-strong)       /* frame #AB6314 */
// so the icons theme-correctly (light mode / accent variants included).
//
// Strokes are the frame's 1.5 with round caps (the icon-system grammar,
// doc 08 §1.4); bar pitch 4px, baseline/centerline per the frame rows.

import type { ReactElement } from "react";

/** Shared svg shell for the four frame icons (decorative — the option
 * label carries the meaning; stroke-width + caps per the frame). */
function iconShell(
  viewBox: string,
  width: number,
  height: number,
  className: string | undefined,
  children: ReactElement,
): ReactElement {
  return (
    <svg
      viewBox={viewBox}
      width={width}
      height={height}
      fill="none"
      strokeWidth={1.5 /* frame: Audio Visualizer.svg icon strokes */}
      strokeLinecap="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {children}
    </svg>
  );
}

/**
 * Spectrometer — frame Icon_3/Frame 1_2: concentric circles (r4 + r7, gray)
 * + four outer arc segments with arrow heads (accent). 19×20 at (40,150).
 */
export function SpectrometerIcon({
  className,
}: {
  className?: string;
}): ReactElement {
  return iconShell(
    "39.25 149.25 20.5 21.5" /* frame: bbox + 0.75 stroke halo */,
    20.5,
    21.5,
    className,
    <>
      {/* frame: Ellipse 16 */}
      <circle cx="50" cy="160" r="4" className="viz-tone-gray" />
      {/* frame: Ellipse 17 */}
      <circle cx="50" cy="160" r="7" className="viz-tone-gray" />
      {/* frame: Ellipse 18 (arcs + heads) */}
      <path
        d="M50 150C52.0318 150 53.9221 150.606 55.5 151.647C56.996 152.634 58.2112 154.012 59 155.636M41 155.636C40.3593 156.955 40 158.435 40 160C40 161.619 40.3846 163.147 41.0673 164.5M58.9327 164.5C58.1429 166.065 56.9541 167.394 55.5 168.353C53.9221 169.394 52.0318 170 50 170"
        className="viz-tone-accent"
      />
    </>,
  );
}

/**
 * Scope — frame Icon_4/Frame 1_3: six bars from the baseline (y215),
 * heights 5/14/12/16/13/8, alternating gray/accent.
 */
export function ScopeIcon({
  className,
}: {
  className?: string;
}): ReactElement {
  return iconShell(
    "39.25 198.25 21.5 17.5" /* frame: bbox + 0.75 stroke halo */,
    21.5,
    17.5,
    className,
    <>
      {/* frame: Vector 84–89, bars at 4px pitch (x40…x60) */}
      <path d="M40 210V215" className="viz-tone-gray" />
      <path d="M44 201V215" className="viz-tone-accent" />
      <path d="M48 203V215" className="viz-tone-gray" />
      <path d="M52 199V215" className="viz-tone-accent" />
      <path d="M56 202V215" className="viz-tone-gray" />
      <path d="M60 207V215" className="viz-tone-accent" />
    </>,
  );
}

/**
 * Spectrum — frame Icon_5/Frame 1_4: six gray bars from the baseline (y261,
 * heights 5/14/12/19/13/8) with an accent peak dot floating above each top.
 */
export function SpectrumIcon({
  className,
}: {
  className?: string;
}): ReactElement {
  return iconShell(
    "39.25 241.25 21.5 20.5" /* frame: bbox + 0.75 stroke halo */,
    21.5,
    20.5,
    className,
    <>
      {/* frame: Vector 91–97 — zero-length round-cap strokes = peak dots */}
      <path d="M40 253V253" className="viz-tone-accent" />
      <path d="M44 244V244" className="viz-tone-accent" />
      <path d="M48 246V246" className="viz-tone-accent" />
      <path d="M52 242V242" className="viz-tone-accent" />
      <path d="M56 245V245" className="viz-tone-accent" />
      <path d="M60 250V250" className="viz-tone-accent" />
      {/* frame: Vector 84_2 + 85_2–89_2 — bars at 4px pitch */}
      <path d="M40 256V261" className="viz-tone-gray" />
      <path d="M44 247V261" className="viz-tone-gray" />
      <path d="M48 249V261" className="viz-tone-gray" />
      <path d="M52 245V261" className="viz-tone-gray" />
      <path d="M56 248V261" className="viz-tone-gray" />
      <path d="M60 253V261" className="viz-tone-gray" />
    </>,
  );
}

/**
 * Waveform — frame Icon_6/Frame 1_5: six bars centered on the midline
 * (y298, half-heights 2/7/5/9/6/4), alternating accent/gray.
 */
export function WaveformIcon({
  className,
}: {
  className?: string;
}): ReactElement {
  return iconShell(
    "39.25 288.25 21.5 19.5" /* frame: bbox + 0.75 stroke halo */,
    21.5,
    19.5,
    className,
    <>
      {/* frame: Vector 84_3–89_3, bars at 4px pitch */}
      <path d="M40 296V300" className="viz-tone-accent" />
      <path d="M44 291V305" className="viz-tone-gray" />
      <path d="M48 293V303" className="viz-tone-accent" />
      <path d="M52 289V307" className="viz-tone-gray" />
      <path d="M56 292V304" className="viz-tone-accent" />
      <path d="M60 294V302" className="viz-tone-gray" />
    </>,
  );
}
