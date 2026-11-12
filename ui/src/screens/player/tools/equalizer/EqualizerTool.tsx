// EqualizerTool (w26.c1; w26.f05 A2 ruling) — the Audio-tab Equalizer tool:
// the preset selector (`EQ presets.svg`) above the GraphicEQ vertical-fader
// panel (`EQ.svg`) — the Lead's frame-true EQUALIZER surface. The w26.f05
// ruling retires the A1 horizontal rows card (`Audio - Equalizer.svg`): the
// tools-dialog tool and the settings Effects embed BOTH mount the A2
// vertical-fader anatomy (the settings frame embeds it at x240 y192.5).
//
//   * collapsed — EQ.svg's 784×36 rx2 #090909 bar, centered "Open
//     Equalizer" + chevron-∨ (the GraphicEQ's own morphing surface);
//   * expanded — the preset selector (EQ presets.svg grammar: two-layer
//     48px trigger, 44px option rows @ 46 pitch — w26.e10 F2, scoped in
//     equalizer.css) above the 784×489 rx2 #191919 panel ("Close
//     Equalizer" header, 11 vertical faders, labels + value chips below);
//   * all state LOCAL to the tool (preamp, band gains, preset, open) with
//     the unchanged `onChange` seam — the frozen w26.c1 contract.
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
import { GraphicEQ } from "./GraphicEQ";
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
   * Fires on every committed change (fader commit, preset pick) with the
   * tool's full state — the wire seam for the effects settings area.
   */
  onChange?: (state: EqualizerState) => void;
  /** Extra classes on the tool root. */
  className?: string;
}

/**
 * EqualizerTool — the tools-dialog Equalizer surface: preset selector +
 * the A2 vertical-fader GraphicEQ panel (the w26.f05 Lead ruling). The
 * tool root carries the `eq-tool`/`eq-tool--open` classes the tools
 * dialog's staged-note rules key on (dialogs.css); the GraphicEQ inside
 * is the named "Equalizer" region.
 */
export function EqualizerTool({
  defaultOpen = false,
  defaultState,
  onChange,
  className = "",
}: EqualizerToolProps): ReactNode {
  const eq = useEqMsg();
  const [open, setOpen] = useState(defaultOpen);
  const [state, setState] = useState<EqualizerState>({
    ...DEFAULT_EQUALIZER_STATE,
    ...defaultState,
  });

  const applyPreset = (id: string): void => {
    const spec = EQ_PRESETS.find((candidate) => candidate.id === id);
    const next: EqualizerState = {
      ...state,
      gains: spec === undefined ? NEUTRAL_GAINS : spec.gains,
      preset: id,
    };
    setState(next);
    onChange?.(next);
  };

  const commitFader = (next: EqualizerState): void => {
    setState(next);
    onChange?.(next);
  };

  const presetOptions: DropdownOption<EqMessageKey>[] = EQ_PRESETS.map(
    (spec) => ({ value: spec.id, labelKey: spec.labelKey }),
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
        onChange={commitFader}
      />
    </div>
  );
}
