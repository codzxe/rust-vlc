// EqualizerTool (w26.c1) — the 10-band equalizer tool card, composed to the
// frames `Audio - Equalizer.svg` (both stacked states) + `EQ presets.svg`
// (the preset selector). Wave-c mission item 1/2/4:
//
//   * the collapsible tool card — collapsed 360×48 rx6 #090909 with
//     "Expand Settings" + chevron-›; expanded rx6 #1B1B1B card whose 59px
//     header row carries "Collapse Settings" + chevron-∨ (the card MORPHS —
//     the frames show one card per state, not a bar plus a panel);
//   * 11 rows (Preamp + 10 bands), label left / value chip right, with a
//     full-width EQSlider (components/slider, w26.a04) under each row head;
//     band rows carry a per-band accent dot from the eq token group;
//   * the preset panel — the shared Dropdown primitive (components/menu,
//     the `Dropdown.svg`/`Setting Preset.svg` grammar the EQ presets frame
//     reuses verbatim per w26.a20 §A3), full-width above the rows;
//   * all state LOCAL to the tool (preamp, band gains, preset, open), with
//     an `onChange` seam so the effects settings area (settings.audio-1
//     embeds the EQ in Effects) and the tools dialog shell can wire the
//     engine command card when it lands. <EqualizerTool/> mounts as-is.
//
// The value chip is the form lane's <ValueChip/> COMPOSED (mission item 3:
// "compose, don't duplicate") — equalizer.css scopes the frame-measured
// anatomy onto it (138×34 rx5 #090909 + 2px #191919 stroke, #8F8F8F ≈12px
// value, w26.a20 §A1) the same way dialogs.css scopes the shared TextField
// into the dialog row grammar.

import { useId, useState } from "react";
import type { ReactNode } from "react";
import { ValueChip } from "../../../../components/form";
import { Icon } from "../../../../components/icon";
import { Dropdown } from "../../../../components/menu";
import type { DropdownOption } from "../../../../components/menu";
import { EQSlider } from "../../../../components/slider";
import { EQ_BANDS } from "./bands";
import {
  DEFAULT_EQUALIZER_STATE,
  EQ_GAIN_MAX_DB,
  EQ_GAIN_MIN_DB,
  EQ_GAIN_STEP_DB,
  EQ_PRESETS,
  NEUTRAL_GAINS,
} from "./presets";
import { useEqMsg } from "./useEqMsg";
import type { EqMessageKey } from "./useEqMsg";
import "./equalizer.css";

/** The tool's full local state (also the `onChange` payload). */
export interface EqualizerState {
  /** Preamp in dB (frame default 12.0). */
  readonly preamp: number;
  /** 10 band gains in dB, band order = EQ_BANDS. */
  readonly gains: readonly number[];
  /** Last applied preset id (manual band edits keep it — VLC behavior). */
  readonly preset: string;
}

export interface EqualizerToolProps {
  /**
   * Initial expanded state (uncontrolled). Frames ship tools COLLAPSED.
   * @default false
   */
  defaultOpen?: boolean;
  /** Initial state override (preamp / gains / preset). */
  defaultState?: Partial<EqualizerState>;
  /**
   * Fires on every committed change (slider commit, preset pick) with the
   * tool's full state — the wire seam for the effects settings area.
   */
  onChange?: (state: EqualizerState) => void;
  /** Extra classes on the card root. */
  className?: string;
}

/** "12.0 dB" — one decimal + the i18n unit (frame chip values). */
function formatDb(value: number, unit: string): string {
  return `${value.toFixed(1)} ${unit}`;
}

/**
 * EqualizerTool — the collapsible EQ card. Mount directly into the effects
 * settings area or a tools dialog body; the card is width-agnostic (the
 * frames' 360px tool family at the settings area's 784px column alike).
 */
export function EqualizerTool({
  defaultOpen = false,
  defaultState,
  onChange,
  className = "",
}: EqualizerToolProps): ReactNode {
  const eq = useEqMsg();
  const [open, setOpen] = useState(defaultOpen);
  const [preamp, setPreamp] = useState(
    defaultState?.preamp ?? DEFAULT_EQUALIZER_STATE.preamp,
  );
  const [gains, setGains] = useState<readonly number[]>(
    defaultState?.gains ?? DEFAULT_EQUALIZER_STATE.gains,
  );
  const [preset, setPreset] = useState(
    defaultState?.preset ?? DEFAULT_EQUALIZER_STATE.preset,
  );
  const toggleId = useId();
  const bodyId = `${toggleId}-body`;

  const commitPreamp = (value: number): void => {
    setPreamp(value);
    onChange?.({ preamp: value, gains, preset });
  };

  const commitBand = (index: number, value: number): void => {
    const next = gains.map((gain, i) => (i === index ? value : gain));
    setGains(next);
    // Manual band edits keep the last applied preset label (VLC behavior —
    // the selection names the last APPLIED preset, not live values).
    onChange?.({ preamp, gains: next, preset });
  };

  const applyPreset = (id: string): void => {
    const spec = EQ_PRESETS.find((candidate) => candidate.id === id);
    const next = spec === undefined ? NEUTRAL_GAINS : spec.gains;
    setGains(next);
    setPreset(id);
    onChange?.({ preamp, gains: next, preset: id });
  };

  const unitDb = eq("screen.eq.unit-db");
  const presetOptions: DropdownOption<EqMessageKey>[] = EQ_PRESETS.map(
    (spec) => ({ value: spec.id, labelKey: spec.labelKey }),
  );

  return (
    <section
      className={["eq-tool", open ? "eq-tool--open" : "", className]
        .filter(Boolean)
        .join(" ")}
      aria-label={eq("screen.eq.title")}
    >
      <button
        type="button"
        id={toggleId}
        className="eq-tool__toggle"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => {
          setOpen((prev) => !prev);
        }}
      >
        <span className="eq-tool__toggle-label">
          {eq(open ? "player.tools.collapse-settings" : "player.tools.expand-settings")}
        </span>
        {/* Collapsed: chevron-› (frame Vector 27); expanded: rotated to ∨
         * (frame Vector 27_2) — same shared glyph, CSS rotates it. */}
        <Icon name="chevron-right" size={24} className="eq-tool__chevron" />
      </button>
      <div
        id={bodyId}
        className="eq-tool__body"
        role="group"
        aria-labelledby={toggleId}
        hidden={!open}
      >
        {/* The preset panel (EQ presets.svg) — shared Dropdown grammar. */}
        <Dropdown
          className="eq-tool__presets"
          options={presetOptions}
          value={preset}
          onChange={applyPreset}
          label={eq("screen.eq.preset")}
          resolveLabel={eq}
        />
        <div className="eq-tool__rows">
          <EqBandRow
            key="preamp"
            label={eq("screen.eq.preamp")}
            value={preamp}
            unitDb={unitDb}
            onCommit={commitPreamp}
          />
          {EQ_BANDS.map((band, index) => (
            <EqBandRow
              key={band.id}
              label={eq(band.labelKey)}
              accent={band.accent}
              value={gains[index] ?? 0}
              unitDb={unitDb}
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

interface EqBandRowProps {
  /** Row label (already resolved through the i18n seam). */
  label: string;
  /** `--color-eq-N` index; omit on Preamp (spacer keeps labels aligned). */
  accent?: 1 | 2 | 3 | 4 | 5 | 6 | 7;
  /** Committed value in dB. */
  value: number;
  /** Resolved unit copy ("dB"). */
  unitDb: string;
  /** Commit seam (drag release / keyboard set — doc 08 §5). */
  onCommit: (value: number) => void;
}

/**
 * EqBandRow — one frame row: label + value chip head (frame pitch 90:
 * head 34 → 15px → slider 16 → 25px), the chip reading the LIVE value
 * during a drag (the EQSlider previews internally; the chip follows via
 * onChangeLive so both stay truthful).
 */
function EqBandRow({ label, accent, value, unitDb, onCommit }: EqBandRowProps): ReactNode {
  const [live, setLive] = useState<number | null>(null);
  const shown = live ?? value;
  return (
    <div className="eq-tool__row">
      <div className="eq-tool__row-head">
        <span className="eq-tool__row-label">
          <span
            className="eq-tool__accent"
            data-accent={accent}
            aria-hidden="true"
          />
          {label}
        </span>
        <ValueChip
          className="eq-tool__chip"
          value={formatDb(shown, unitDb)}
        />
      </div>
      <EQSlider
        className="eq-tool__slider"
        value={shown}
        min={EQ_GAIN_MIN_DB}
        max={EQ_GAIN_MAX_DB}
        step={EQ_GAIN_STEP_DB}
        aria-label={label}
        formatValue={(v) => formatDb(v, unitDb)}
        onChangeLive={setLive}
        onCommit={(next) => {
          setLive(null);
          onCommit(next);
        }}
      />
    </div>
  );
}
