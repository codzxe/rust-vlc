// Settings area: Effects — the w26.c5 tools integration of the Settings →
// Audio screen, composed to `Settings - audio-1.svg` (the Effects section's
// expanded state; `Settings - audio.svg` is the same section collapsed).
//
// COMPOSITION (frame truth, top → bottom of the section):
//   * "Equalizer" — toggle row, staged OFF (the EQ enable awaits the
//     `audio.set-eq` catalog command; engine `Eq3Band` exists, doc 05 §2)
//   * the EQ PANEL — the wave-c EqualizerTool (w26.c1) embedded full-width
//     in the content column: 10 frame bands + Preamp + the presets panel
//     (`EQ presets.svg` grammar, covering the frame's "Equalizer preset"
//     row — one preset control, not two; documented deviation: the frame
//     carries that row OUTSIDE the panel, the frozen tool API carries it
//     inside). The frame ships the panel EXPANDED ("Close Equalizer"
//     header) → `defaultOpen`.
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
//     alive. The embedded EqualizerTool keeps its own interactive local
//     state (the w26.c1 contract) wrapped in the same staged markers.
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

import type { ReactElement } from "react";
import { Switch } from "../../../components/form";
import { Dropdown } from "../../../components/menu";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as eqLaneKeys } from "../../../i18n/keys/w26.c1";
import { keys as laneKeys } from "../../../i18n/keys/w26.c5";
import { EqualizerTool } from "../../player/tools/equalizer";

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
}: {
  labelKey: EffectsLabelKey;
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
 * Effects section with the wave-c EqualizerTool embedded.
 */
export function EffectsArea(): ReactElement {
  const m = useEffectsMsg();
  return (
    <div className="settings-sections" data-settings-area="effects">
      <section className="settings-section" data-section="effects">
        <h2 className="settings-section__title">{m("settings.section.effects")}</h2>
        {/* Frame row 1 — the EQ enable, staged OFF (audio.set-eq). */}
        <StagedToggleRow labelKey="screen.eq.title" pendingCommand="audio.set-eq" />
        {/* The EQ PANEL (Settings - audio-1 "EQ" block): the w26.c1 tool
         * embedded full-width, expanded per the frame ("Close Equalizer"),
         * presets included (EQ presets.svg). Staged wrapper: the tool keeps
         * interactive LOCAL state (its frozen contract) while no
         * `audio.set-eq` command exists — the marker names the command the
         * whole panel awaits. */}
        <div
          className="settings-field settings-field--tool"
          data-setting-key="pending.audio.set-eq"
          data-staged-tool="equalizer"
          data-pending-command="audio.set-eq"
        >
          <EqualizerTool defaultOpen />
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
