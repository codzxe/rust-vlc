// Settings area: Effects — the Settings → Audio screen's Effects section,
// composed to `Settings - audio-1.svg` (the Effects section's expanded
// state; `Settings - audio.svg` is the same section at the fold).
//
// COMPOSITION (frame truth, top → bottom of the section — w26.f05 rewrite):
//   * "Equalizer" — toggle row, staged OFF (the EQ enable awaits the
//     `audio.set-eq` catalog command; engine `Eq3Band` exists, doc 05 §2),
//     with the frame's ⓘ info affordance;
//   * "Equalizer preset" — a SEPARATE settings row OUTSIDE the EQ panel
//     (w26.e07 F6: frame row 2, 320×36 two-layer dropdown right-aligned)
//     with the ⓘ affordance — interactive LOCAL state (the pick shapes the
//     faders below; never a dead control), staged-marked;
//   * the EQ PANEL — the A2 vertical-fader GraphicEQ (w26.f05 F5/F1: the
//     frame embeds `EQ.svg`'s 784×489 rx2 "Close Equalizer" panel at
//     x240 y192.5, NOT the A1 horizontal card), full-width in the content
//     column, EXPANDED per the frame (`defaultOpen`), staged-marked;
//   * "Two-pass" / "Spatial audio" / "Spatial audio mode" / "Stereo
//     widening" — the frame's remaining rows, staged per the honest pattern.
//
// HONEST SURFACE RULES (the area's staged-command pattern, unchanged):
//   • STAGED rows are the effects the engine cannot take commands for yet —
//     equalizer enable + two-pass (`audio.set-eq`), spatial audio + its
//     mode (`audio.set-spatializer`, the w26.c2 forward declaration) and
//     stereo widening (`audio.set-stereo-widener`). They render DISABLED
//     with the honest reason and a data-pending-command marker naming the
//     command they await — visible, inert, never a dead control that looks
//     alive. The embedded GraphicEQ + the preset row keep their interactive
//     local state (the w26.c1 contract) wrapped in the same staged markers.
//   • RETIRED this integration (Settings - audio-1 carries none of them in
//     the Effects section): the v0 "Master volume"/"Mute" live controls and
//     the balance pending slider (live playback control stays in the player
//     ControlBar; the schema-owned Volume/Loudness sections of `Settings -
//     audio.svg` own settings volume when seeded — handoff noted in the
//     w26.c5 lane log), and the video-adjust pending sliders (the wave-c
//     ColorsTool carries that staging in the player Tools dialog; the
//     `Settings - video-1.svg` "Picture" section is the video area owner's
//     mount). The engine stages behind them (Balance, Eq3Band,
//     AdjustParams) are documented in doc 05 §2 and ui/NEXT-STEPS.md.
//   • No schemas/settings.toml entries exist for effects yet — when the
//     schema owner seeds them, this area's schema-backed fields move to the
//     generator (same migration note as audio.tsx).

import { useState } from "react";
import type { ReactElement } from "react";
import { Switch } from "../../../components/form";
import { Dropdown } from "../../../components/menu";
import type { DropdownOption } from "../../../components/menu";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as eqLaneKeys } from "../../../i18n/keys/w26.c1";
import { keys as laneKeys } from "../../../i18n/keys/w26.c5";
import {
  DEFAULT_EQUALIZER_STATE,
  EQ_PRESETS,
  NEUTRAL_GAINS,
  GraphicEQ,
  useEqMsg,
} from "../../player/tools/equalizer";
import type { EqualizerState } from "../../player/tools/equalizer";
import type { EqMessageKey } from "../../player/tools/equalizer";
import "./effects.css";

type LaneKey = keyof typeof laneKeys;
type EqLaneKey = keyof typeof eqLaneKeys;

/** The lane catalog: this lane's keys over the reused w26.c1 equalizer keys
 * (screen.eq.title = the frame's "Equalizer" row — reuse, not re-declare). */
const laneCatalog: Readonly<Record<string, string>> = {
  ...eqLaneKeys,
  ...laneKeys,
};

/** Keys resolvable by `useEffectsMsg`. */
export type EffectsLabelKey = MessageKey | LaneKey | EqLaneKey;

/** useEffectsMsg — useMsg widened with the w26.c5 + w26.c1 lane keys (the
 * w26.c1 useEqMsg seam-first pattern: central catalog wins once the Lead
 * registers the keys, the lane copy is the fallback reference). */
export function useEffectsMsg() {
  const m = useMsg();
  return (key: EffectsLabelKey): string => m(key as MessageKey) ?? laneCatalog[key];
}

/**
 * The ⓘ info affordance (frame: Settings - audio-1.svg Ellipse 9_5/9_6 —
 * r7.5 circle, stroke #404040 1.125, "i" glyph; the Equalizer and Equalizer
 * preset rows carry it). Graphical, aria-hidden: the frames show no
 * interaction beyond the affordance.
 */
function InfoDot(): ReactElement {
  return <span className="effects-info" aria-hidden="true" />;
}

/** The staged note badge (the honest-unbound line — one per staged field). */
function StagedNote({ msg }: { msg: (key: EffectsLabelKey) => string }): ReactElement {
  return (
    <p className="settings-field__badges" role="note">
      <span className="settings-badge">{msg("settings.effects.pending-command")}</span>
    </p>
  );
}

/**
 * A staged toggle row: label left, OFF switch right, awaiting its catalog
 * command (the effects-area staged pattern — disabled WITH reason, never a
 * dead control that looks alive).
 */
function StagedToggleRow({
  labelKey,
  pendingCommand,
  info = false,
}: {
  labelKey: EffectsLabelKey;
  pendingCommand: string;
  /** The frame's ⓘ affordance (Equalizer row only in the Effects section). */
  info?: boolean;
}): ReactElement {
  const m = useEffectsMsg();
  const label = m(labelKey);
  return (
    <div
      className="settings-field"
      data-setting-key={`pending.${pendingCommand}`}
      data-pending-command={pendingCommand}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
          {info ? <InfoDot /> : null}
        </div>
        <div className="settings-field__control">
          <Switch
            checked={false}
            onChange={() => {
              /* staged — disabled means never fired */
            }}
            disabled
            aria-label={label}
          />
        </div>
      </div>
      <StagedNote msg={m} />
    </div>
  );
}

/**
 * A staged dropdown row: label left, the frame's selected value in a compact
 * dropdown right. One honest frame-verbatim option — no invented engine
 * modes (the frame exports no open list for this control).
 */
function StagedDropdownRow({
  labelKey,
  optionKey,
  pendingCommand,
}: {
  labelKey: EffectsLabelKey;
  optionKey: EffectsLabelKey;
  pendingCommand: string;
}): ReactElement {
  const m = useEffectsMsg();
  const label = m(labelKey);
  return (
    <div
      className="settings-field"
      data-setting-key={`pending.${pendingCommand}`}
      data-pending-command={pendingCommand}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
        </div>
        <div className="settings-field__control">
          <Dropdown
            options={[{ value: optionKey, labelKey: optionKey }]}
            value={optionKey}
            onChange={() => {
              /* staged — disabled means never fired */
            }}
            label={label}
            resolveLabel={m}
            density="compact"
            disabled
          />
        </div>
      </div>
      <StagedNote msg={m} />
    </div>
  );
}

/**
 * Effects settings area (hand-authored; see module header) — the frame's
 * Effects section: staged rows around the A2 GraphicEQ panel + the separate
 * preset row that shapes it (w26.f05).
 */
export function EffectsArea(): ReactElement {
  const m = useEffectsMsg();
  const eq = useEqMsg();
  // The EQ state is AREA-LOCAL: the preset row and the fader panel share it
  // (the preset pick re-shapes the faders; a fader commit keeps the last
  // applied preset label — VLC behavior). Never a dead control, never an
  // engine write (the staged markers name the awaited `audio.set-eq`).
  const [eqState, setEqState] = useState<EqualizerState>(DEFAULT_EQUALIZER_STATE);

  const applyPreset = (id: string): void => {
    const spec = EQ_PRESETS.find((candidate) => candidate.id === id);
    setEqState({
      ...eqState,
      gains: spec === undefined ? NEUTRAL_GAINS : spec.gains,
      preset: id,
    });
  };

  const presetOptions: DropdownOption<EqMessageKey>[] = EQ_PRESETS.map((spec) => ({
    value: spec.id,
    labelKey: spec.labelKey,
  }));

  return (
    <div className="settings-sections" data-settings-area="effects">
      <section className="settings-section" data-section="effects">
        <h2 className="settings-section__title">{m("settings.section.effects")}</h2>
        {/* Frame row 1 — the EQ enable, staged OFF (audio.set-eq), with ⓘ. */}
        <StagedToggleRow labelKey="screen.eq.title" pendingCommand="audio.set-eq" info />
        {/* Frame row 2 — "Equalizer preset" OUTSIDE the panel (w26.e07 F6):
         * a settings row whose compact two-layer dropdown (frame EQ_2:
         * 320×36 rx6 #090909 + 316×32 rx4 #191919 field, effects.css) picks
         * the preset that shapes the faders below. Interactive local state,
         * staged-marked like the panel it drives. */}
        <div
          className="settings-field"
          data-setting-key="pending.audio.set-eq"
          data-pending-command="audio.set-eq"
        >
          <div className="settings-field__row">
            <div className="settings-field__lead">
              <span className="settings-field__label">{eq("screen.eq.preset")}</span>
              <InfoDot />
            </div>
            <div className="settings-field__control">
              <Dropdown
                className="effects-eq-preset"
                options={presetOptions}
                value={eqState.preset}
                onChange={applyPreset}
                label={eq("screen.eq.preset")}
                resolveLabel={eq}
                density="compact"
              />
            </div>
          </div>
          <StagedNote msg={m} />
        </div>
        {/* Frame row 3 — the EQ PANEL (Settings - audio-1 x240 y192.5): the
         * A2 vertical-fader GraphicEQ (w26.f05 F5), full-width, expanded per
         * the frame ("Close Equalizer" header). Staged wrapper: the panel
         * keeps interactive LOCAL state (its frozen contract) while no
         * `audio.set-eq` command exists — the marker names the command the
         * whole panel awaits. */}
        <div
          className="settings-field settings-field--tool"
          data-setting-key="pending.audio.set-eq"
          data-staged-tool="equalizer"
          data-pending-command="audio.set-eq"
        >
          <GraphicEQ defaultOpen state={eqState} onChange={setEqState} />
          <StagedNote msg={m} />
        </div>
        {/* Frame rows below the EQ panel, in frame order. Two-pass is EQ
         * configuration (VLC parity) — same awaited command. */}
        <StagedToggleRow labelKey="settings.effects.two-pass" pendingCommand="audio.set-eq" />
        <StagedToggleRow
          labelKey="settings.effects.spatial-audio"
          pendingCommand="audio.set-spatializer"
        />
        <StagedDropdownRow
          labelKey="settings.effects.spatial-audio-mode"
          optionKey="settings.effects.spatial-mode.headphone-virtualization"
          pendingCommand="audio.set-spatializer"
        />
        <StagedToggleRow
          labelKey="settings.effects.stereo-widening"
          pendingCommand="audio.set-stereo-widener"
        />
      </section>
    </div>
  );
}
