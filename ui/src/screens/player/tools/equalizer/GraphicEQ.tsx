// GraphicEQ (w26.f05) — the frame-true EQUALIZER surface: the A2
// vertical-fader panel composed to `EQ.svg` (and its `Settings - audio-1.svg`
// embed at x240 y192.5 — identical geometry, +220/+116.5 offset). The Lead's
// w26.f05 ruling pins EQ.png as THE equalizer anatomy; the A1 horizontal
// rows card it replaces is retired.
//
// Frame anatomy (EQ.svg, SVG-measured):
//   * COLLAPSED — 784×36 rx2 #090909 bar, centered "Open Equalizer" +
//     chevron-∨ (Vector 27, 10×5 stroke 1.5 #8F8F8F);
//   * EXPANDED — 784×489 rx2 #191919 panel: a 36px header row with centered
//     "Close Equalizer" + chevron-∨ (Vector 27_2), 18px gap, then 11 fader
//     columns at 68px pitch (first track center 52px from the panel edge):
//       - track 16×320 rx6 #090909 (EQSlider orientation="vertical"),
//         FLAT #371F06 fill 12px rx4 growing UP from the bottom,
//         24×8 rx4 #AB6314 thumb centered on the value;
//       - the 10 BAND faders carry a 0 dB tick (Vector 60: 6px mark,
//         stroke 2 #1A1611, round caps) at the track midpoint — painted by
//         CSS over the fill, under the thumb (the frame z-order); Preamp
//         carries none;
//       - label ≈14px #6B6B6B centered under the track, then a 60×32 rx6
//         #090909 value chip with ≈12px #8F8F8F "12.0 dB" text;
//   * NO band accent dots — the frame renders the columns monochrome
//     (w26.e07 F8; the eq.1–7 tokens stay in the token layer for the
//     visualizer's spectrum bars).
//
// State contract (the w26.c1 seam, unchanged): all state is LOCAL by
// default (preamp, gains, open) with an `onChange` wire seam; hosts that
// own the state (the settings area's separate "Equalizer preset" row, the
// tools dialog's preset selector) pass `state` and drive it through the
// same seam. The preset SELECTOR itself lives OUTSIDE this panel (the
// frames place it in a settings row / above the panel — w26.e07 F6).

import { useId, useState } from "react";
import type { ReactNode } from "react";
import { ValueChip } from "../../../../components/form";
import { Icon } from "../../../../components/icon";
import { EQSlider } from "../../../../components/slider";
import { EQ_BANDS } from "./bands";
import {
  DEFAULT_EQUALIZER_STATE,
  EQ_GAIN_MAX_DB,
  EQ_GAIN_MIN_DB,
  EQ_GAIN_STEP_DB,
} from "./presets";
import type { EqualizerState } from "./EqualizerTool";
import { useEqMsg } from "./useEqMsg";
import "./equalizer.css";

/** "12.0 dB" — one decimal + the i18n unit (frame chip values). */
function formatDb(value: number, unit: string): string {
  return `${value.toFixed(1)} ${unit}`;
}

/**
 * The 0.1 dB lattice as CLEAN one-decimal doubles. w26.k10 (j10 P2-1): the
 * interaction engine's `quantize()` lands on the lattice mathematically, but
 * binary 0.1 arithmetic leaves float dust (5.6 + 0.1 → 5.700000000000003)
 * that `aria-valuenow` and the `onChange` payload must not carry — the chip
 * hides it behind `toFixed(1)`, the slider attributes and the wire seam
 * would not. Snapping at this boundary covers both keyboard and drag
 * commits; it never moves a value off the lattice (quantize already did).
 */
const snapDb = (value: number): number => Math.round(value * 10) / 10;

export interface GraphicEQProps {
  /**
   * Initial expanded state (uncontrolled). The settings frame ships the
   * panel EXPANDED ("Close Equalizer" header) → the settings mount passes
   * `defaultOpen`.
   * @default false
   */
  defaultOpen?: boolean;
  /**
   * Expanded state override (controlled) — the tools-dialog tool passes its
   * own open state so the preset selector above the panel hides with it.
   */
  open?: boolean;
  /** Fires with the next expanded state when the header toggle is used. */
  onOpenChange?: (open: boolean) => void;
  /** Initial state override (preamp / gains / preset). */
  defaultState?: Partial<EqualizerState>;
  /**
   * Controlled state override — the host-owned mount (the settings area's
   * "Equalizer preset" row applies preset shapes through it). Edits still
   * report via {@link onChange}; the host echoes them back.
   */
  state?: EqualizerState;
  /**
   * Fires on every committed change (fader commit) with the panel's full
   * state — the wire seam for the effects settings area / tools dialog.
   */
  onChange?: (state: EqualizerState) => void;
  /** Extra classes on the panel root. */
  className?: string;
}

/**
 * GraphicEQ — the A2 vertical-fader EQUALIZER panel (EQ.svg). Mount directly
 * (settings Effects embed) or through the EqualizerTool (tools dialog); the
 * panel is width-agnostic with a frame-exact 748px fader rail that pans
 * horizontally on mounts narrower than the 784px frame (the tools dialog's
 * 398px column) — every column keeps the frame geometry.
 */
export function GraphicEQ({
  defaultOpen = false,
  open: openProp,
  onOpenChange,
  defaultState,
  state: stateProp,
  onChange,
  className = "",
}: GraphicEQProps): ReactNode {
  const eq = useEqMsg();
  const [openInternal, setOpenInternal] = useState(defaultOpen);
  const isOpen = openProp ?? openInternal;
  const [stateInternal, setStateInternal] = useState<EqualizerState>({
    ...DEFAULT_EQUALIZER_STATE,
    ...defaultState,
  });
  const state = stateProp ?? stateInternal;
  const toggleId = useId();
  const bodyId = `${toggleId}-body`;

  // w26.k10 (j10 P3-3): internal state is written ONLY when uncontrolled —
  // a controlled mount (the tools dialog passes both `open` and `state`)
  // used to shadow-write `openInternal`/`stateInternal` on every commit,
  // diverging behind the host's props and double-setStateing per fader
  // commit (commit → EqualizerTool.commitFader). The host is the truth.
  const setOpen = (next: boolean): void => {
    if (openProp === undefined) setOpenInternal(next);
    onOpenChange?.(next);
  };

  const commit = (next: EqualizerState): void => {
    if (stateProp === undefined) setStateInternal(next);
    onChange?.(next);
  };

  const commitPreamp = (value: number): void => {
    commit({ ...state, preamp: value });
  };

  const commitBand = (index: number, value: number): void => {
    const gains = state.gains.map((gain, i) => (i === index ? value : gain));
    commit({ ...state, gains });
  };

  const unitDb = eq("screen.eq.unit-db");

  return (
    <section
      className={["eq-panel", isOpen ? "eq-panel--open" : "", className]
        .filter(Boolean)
        .join(" ")}
      aria-label={eq("screen.eq.title")}
      data-tool="equalizer"
    >
      <button
        type="button"
        id={toggleId}
        className="eq-panel__toggle"
        aria-expanded={isOpen}
        aria-controls={bodyId}
        onClick={() => {
          setOpen(!isOpen);
        }}
      >
        <span className="eq-panel__toggle-label">
          {eq(isOpen ? "screen.eq.close-panel" : "screen.eq.open-panel")}
        </span>
        {/* Chevron-∨ in BOTH frame states (EQ.svg Vector 27 / Vector 27_2 —
         * the panel morphs bar↔panel; the glyph does not rotate). */}
        <Icon name="chevron-down" size={24} className="eq-panel__chevron" />
      </button>
      <div
        id={bodyId}
        className="eq-panel__body"
        role="group"
        aria-labelledby={toggleId}
        hidden={!isOpen}
      >
        <div className="eq-panel__rail">
          <EqFaderColumn
            key="preamp"
            label={eq("screen.eq.preamp")}
            value={state.preamp}
            unitDb={unitDb}
            onCommit={commitPreamp}
          />
          {EQ_BANDS.map((band, index) => (
            <EqFaderColumn
              key={band.id}
              label={eq(band.labelKey)}
              value={state.gains[index] ?? 0}
              unitDb={unitDb}
              zeroMark
              onCommit={(value) => {
                commitBand(index, value);
              }}
            />
          ))}
        </div>
      </div>
    </section>
  );
}

interface EqFaderColumnProps {
  /** Column label (already resolved through the i18n seam). */
  label: string;
  /** Committed value in dB. */
  value: number;
  /** Resolved unit copy ("dB"). */
  unitDb: string;
  /** The 0 dB tick — frame: the 10 band faders carry it, Preamp does not. */
  zeroMark?: boolean;
  /** Commit seam (drag release / keyboard set — doc 08 §5). */
  onCommit: (value: number) => void;
}

/**
 * EqFaderColumn — one frame column: vertical fader (320px track), label,
 * value chip. The chip reads the LIVE value during a drag (the EQSlider
 * previews internally; the chip follows via onChangeLive so both stay
 * truthful). Monochrome per the frame — no band accent dot (w26.e07 F8).
 */
function EqFaderColumn({
  label,
  value,
  unitDb,
  zeroMark = false,
  onCommit,
}: EqFaderColumnProps): ReactNode {
  const [live, setLive] = useState<number | null>(null);
  const shown = live ?? value;
  return (
    <div className="eq-panel__col">
      <EQSlider
        className={["eq-panel__fader", zeroMark ? "eq-panel__fader--zero" : ""]
          .filter(Boolean)
          .join(" ")}
        orientation="vertical"
        value={shown}
        min={EQ_GAIN_MIN_DB}
        max={EQ_GAIN_MAX_DB}
        step={EQ_GAIN_STEP_DB}
        aria-label={label}
        formatValue={(v) => formatDb(v, unitDb)}
        onChangeLive={(v) => {
          setLive(snapDb(v));
        }}
        onCommit={(next) => {
          setLive(null);
          onCommit(snapDb(next));
        }}
      />
      <span className="eq-panel__label">{label}</span>
      <ValueChip className="eq-panel__chip" value={formatDb(shown, unitDb)} />
    </div>
  );
}
