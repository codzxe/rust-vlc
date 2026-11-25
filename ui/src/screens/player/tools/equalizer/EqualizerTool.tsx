// EqualizerTool (w26.c1; w26.f05 A2 ruling; FX4-A08 wired) — the Audio-tab
// Equalizer tool: the preset selector (`EQ presets.svg`) above the
// GraphicEQ vertical-fader panel. The w26.f05 ruling retires the A1
// horizontal rows card; the tools-dialog tool and the settings Effects
// embed both mount the A2 vertical-fader anatomy.
//
// FX4-A08 (wave FX4, plan F1.1) — the tool is WIRED: the panel is cut to
// the ENGINE's 3-band truth (Eq3Band: low 250 Hz shelf · mid 1 kHz peak
// · high 4 kHz shelf) and every fader commit / preset pick dispatches the
// REAL `audio.set-eq` verb through the player context's single dispatch
// lane (`PlayerActions.setEq` — the same `wire.send` path the transport
// controls and volume/mute ride). The honest-staging markers and the
// in-body note the pre-wave tool carried (data-staged-tool /
// data-pending-command + "controls activate with the audio effects
// command card") are RETIRED — they named an awaited command that now
// exists and is called.
//
//   * collapsed — EQ.svg's 784×36 rx2 #090909 bar, centered "Open 3-band
//     equalizer" + chevron-∨ (the GraphicEQ's own morphing surface);
//   * expanded — the preset selector above the 3-band vertical-fader
//     panel ("Close 3-band equalizer" header);
//   * local state stays the EDITING surface (preamp, 3 band gains,
//     preset, open) with the unchanged `onChange` seam for hosts; the
//     dispatch is additional, and mounts WITHOUT a PlayerControlsProvider
//     above (the dev harness, bare test mounts) simply do not dispatch —
//     usePlayerControlsOrNull is the presence probe.
//
// The value chip is the form lane's <ValueChip/> COMPOSED (mission item 3:
// "compose, don't duplicate") — equalizer.css scopes the frame-measured
// anatomy onto it (60×32 rx6 #090909, #8F8F8F ≈12px value, EQ.svg chips)
// with the frame ink pinned in scope (w26.e10 F4 — the base chip's white
// ink is the form lane's call, noted for f15).

import { useState } from "react";
import type { ReactNode } from "react";
import { Dropdown } from "../../../../components/menu";
import type { DropdownOption } from "../../../../components/menu";
import {
  DEFAULT_EQUALIZER_STATE,
  EQ_PRESETS,
  NEUTRAL_GAINS,
} from "./presets";
import { EQ_BAND_INDEX } from "./bands";
import { GraphicEQ } from "./GraphicEQ";
import { useEqMsg } from "./useEqMsg";
import type { EqMessageKey } from "./useEqMsg";
import { usePlayerControlsOrNull } from "../../context";
import type { EqSetArgs } from "../../context";
import "./equalizer.css";

/** The tool's full local state (also the `onChange` payload). */
export interface EqualizerState {
  /** Preamp in dB (engine truth: `Volume::UNITY` = 0). */
  readonly preamp: number;
  /** 3 band gains in dB, band order = EQ_BANDS (low, mid, high). */
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
   * Fires on every committed change (fader commit, preset pick) with the
   * tool's full state — the host wiring seam (the settings Effects embed
   * drives its own local state through it).
   */
  onChange?: (state: EqualizerState) => void;
  /** Extra classes on the tool root. */
  className?: string;
}

/**
 * The wire payload for one editing state — the engine's three band gains
 * read through EQ_BAND_INDEX (never a positional guess) plus the preamp.
 */
const eqArgsOf = (state: EqualizerState): EqSetArgs => ({
  lowDb: state.gains[EQ_BAND_INDEX.low] ?? 0,
  midDb: state.gains[EQ_BAND_INDEX.mid] ?? 0,
  highDb: state.gains[EQ_BAND_INDEX.high] ?? 0,
  preampDb: state.preamp,
});

/**
 * EqualizerTool — the tools-dialog Equalizer surface: preset selector +
 * the A2 vertical-fader GraphicEQ panel (the w26.f05 Lead ruling), cut to
 * the engine's 3 bands (FX4-A08) and dispatching the real `audio.set-eq`
 * verb on every commit when mounted inside the player context.
 */
export function EqualizerTool({
  defaultOpen = false,
  defaultState,
  onChange,
  className = "",
}: EqualizerToolProps): ReactNode {
  const eq = useEqMsg();
  // The presence probe (w26.f10 F4 pattern): inside the player screen the
  // tools dialog sits under PlayerControlsProvider and dispatches for
  // real; bare mounts (dev harness, tests) get null and stay local-only.
  const controls = usePlayerControlsOrNull();
  const [open, setOpen] = useState(defaultOpen);
  const [state, setState] = useState<EqualizerState>({
    ...DEFAULT_EQUALIZER_STATE,
    ...defaultState,
  });

  // Every committed edit reaches the engine — the same dispatch lane the
  // transport controls use; typed engine refusals surface through the
  // provider's lastError, never silently.
  const commit = (next: EqualizerState): void => {
    setState(next);
    onChange?.(next);
    controls?.actions.setEq(eqArgsOf(next));
  };

  const applyPreset = (id: string): void => {
    const spec = EQ_PRESETS.find((candidate) => candidate.id === id);
    const next: EqualizerState = {
      ...state,
      gains: spec === undefined ? NEUTRAL_GAINS : spec.gains,
      preset: id,
    };
    commit(next);
  };

  const presetOptions: DropdownOption<EqMessageKey>[] = EQ_PRESETS.map(
    (spec) => ({
      value: spec.id,
      labelKey: spec.labelKey,
      // w26.k10 (j10 P3-6): structural per-option marker — equalizer.css
      // keys the preset curve glyphs on `data-preset`, never DOM position.
      data: { preset: spec.id },
    }),
  );

  return (
    <div
      className={["eq-tool", open ? "eq-tool--open" : "", className]
        .filter(Boolean)
        .join(" ")}
      data-tool="equalizer"
    >
      {/* The preset panel (EQ presets.svg) — visible with the expanded
       * surface; the frames place the preset control OUTSIDE the fader
       * panel (w26.e07 F6). */}
      {open ? (
        <Dropdown
          className="eq-tool__presets"
          options={presetOptions}
          value={state.preset}
          onChange={applyPreset}
          label={eq("screen.eq.preset")}
          resolveLabel={eq}
        />
      ) : null}
      <GraphicEQ
        open={open}
        onOpenChange={setOpen}
        state={state}
        onChange={commit}
      />
    </div>
  );
}
