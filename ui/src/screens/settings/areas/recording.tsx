// Settings area: Recording — "Recording, Casting & Streaming Out"
// (Settings - recording.svg, analysis tools-settings.md §C12).
//
// w26.i15 (closing h12): the §C12 frame rows render STAGED INERT — the
// wave-I fix directive (the honest-empty pattern in the frame's own visual
// grammar, the Remotes precedent). The seeded rows are feature-gated
// (`recording` = false, product.toml Phase 7, rule F2) and the Casting/
// Streaming rows have no [[setting]] keys at all, so every frame row
// renders full-strength, disabled, marked data-staged with the honest
// tooltip — the frame's 10 rows visible, never dead controls that look
// alive.
//
// Yield rule: the Recording section (the two schema-backed rows —
// recording.directory + recording.format) renders staged ONLY while the
// gate is off; the moment exposure flips, GENERATED_SCREENS.recording
// renders them live (the embed below is their only on-route render path)
// and the staged copy yields to it — no double render. Casting/Streaming
// stay staged until the Lead seeds their keys (exact list in
// docs/waves/w26/logs/w26.i15.md).
//
// w26.i15 (g14-P3-4): the wrapper no longer carries data-settings-area —
// exactly ONE [data-settings-area="recording"] renders per route (the
// embedded generated screen's root).

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTINGS_ENTRIES } from "../generated/schema";
import { StagedAdvancedBar } from "./staged";
import { StagedRow } from "./staged";
import { useStagedMsg } from "./staged";
import "./staged.css";

/** Recording — the w26.b02-seeded rows (directory "~/Videos" + format). */
const GeneratedRecordingScreen = GENERATED_SCREENS.recording;

/** True when any recording-screen schema row is exposed (rule F2 gate). */
const RECORDING_EXPOSED = SETTINGS_ENTRIES.some(
  (entry) => entry.screen === "recording" && entry.featureExposed,
);

/**
 * Recording settings area (route /settings/recording; System group) — the
 * §C12 frame: Recording + Casting + Streaming + the Advanced bar, staged
 * behind the `recording` feature gate.
 */
export function RecordingArea(): ReactElement {
  const t = useStagedMsg();
  const m = useMsg();
  const staged = t("settings.recording.staged");
  const seam = "recording";
  const advancedLabel = m("settings.advanced-options");
  const browse = m("settings.field.browse");
  const directory = m("settings.recording.directory");
  const directoryValue = t("settings.recording.value.directory.default");

  return (
    <div
      className="settings-sections"
      data-testid="recording-area"
      data-advanced-staged="true"
    >
      {/* The generated screen — the seeded rows' only on-route render path
          (hidden + inert while the gate is off; live the moment it flips). */}
      <GeneratedRecordingScreen />
      {/* §C12 Recording (y83): staged while gated; yields to the generated
          screen above when exposure flips (no double render). */}
      {!RECORDING_EXPOSED ? (
        <section className="settings-section" data-section="recording">
          <h2 className="settings-section__title">{m("settings.section.recording.recording")}</h2>
          <StagedRow
            rowKey="recording.directory"
            label={directory}
            control={{ kind: "path", value: directoryValue, browse }}
            seam={seam}
            reason={staged}
            info
          />
          <StagedRow
            rowKey="recording.without-reencoding"
            label={t("settings.recording.without-reencoding")}
            control={{ kind: "switch", on: true }}
            seam={seam}
            reason={staged}
          />
        </section>
      ) : null}
      {/* §C12 Casting (y232): no schema keys → staged until seeded. */}
      <section className="settings-section" data-section="casting">
        <h2 className="settings-section__title">{t("settings.section.recording.casting")}</h2>
        <StagedRow
          rowKey="recording.discover-cast"
          label={t("settings.recording.discover-cast")}
          control={{ kind: "switch", on: true }}
          seam="schema-seed"
          reason={staged}
        />
        <StagedRow
          rowKey="recording.cast-quality"
          label={t("settings.recording.cast-quality")}
          control={{ kind: "enum", value: t("settings.recording.value.cast.automatic") }}
          seam="schema-seed"
          reason={staged}
          info
        />
      </section>
      {/* §C12 Streaming (y379): no schema keys → staged until seeded. */}
      <section className="settings-section" data-section="streaming">
        <h2 className="settings-section__title">{t("settings.section.recording.streaming")}</h2>
        <StagedRow
          rowKey="recording.play-locally"
          label={t("settings.recording.play-locally")}
          control={{ kind: "switch", on: true }}
          seam="schema-seed"
          reason={staged}
        />
        <StagedRow
          rowKey="recording.include-audio"
          label={t("settings.recording.include-audio")}
          control={{ kind: "switch", on: true }}
          seam="schema-seed"
          reason={staged}
        />
        <StagedRow
          rowKey="recording.include-video"
          label={t("settings.recording.include-video")}
          control={{ kind: "switch", on: true }}
          seam="schema-seed"
          reason={staged}
        />
        <StagedRow
          rowKey="recording.include-subtitles"
          label={t("settings.recording.include-subtitles")}
          control={{ kind: "switch", on: true }}
          seam="schema-seed"
          reason={staged}
        />
        <StagedRow
          rowKey="recording.encoding-preset"
          label={t("settings.recording.encoding-preset")}
          control={{ kind: "enum", value: t("settings.recording.value.preset.medium") }}
          seam="schema-seed"
          reason={staged}
          info
        />
        <StagedRow
          rowKey="recording.tuning"
          label={t("settings.recording.tuning")}
          control={{ kind: "enum", value: t("settings.recording.value.tuning.none") }}
          seam="schema-seed"
          reason={staged}
          info
        />
      </section>
      {/* §C12 y685: the collapsed Advanced bar, staged (nothing behind it). */}
      <StagedAdvancedBar reason={t("settings.advanced.staged")} label={advancedLabel} />
    </div>
  );
}
