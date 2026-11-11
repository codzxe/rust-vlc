// Settings area: Recording — "Recording, Casting & Streaming Out"
// (Settings - recording.svg, analysis tools-settings.md §C12).
//
// w26.b02 SIBLING CONTRACT: this module owns the WHOLE area screen and
// embeds the schema-static sections generated from the w26.b02 seed:
//   Recording (SCHEMA — <GENERATED_SCREENS.recording />: the
//   recording.directory path row + recording.format enum row; the registry
//   REPLACED the generated record for this id, so the embed below is the
//   seeded rows' only on-route render path) →
//   Casting + Streaming out (hand — see the honest-surface note below).
//
// HONEST SURFACE RULES (config-never-invents, same discipline as the sibling
// areas): the seeded rows are feature-gated (`recording` = false,
// product.toml Phase 7, rule F2) → hidden + inert until the feature ships —
// the generated screen renders them the moment exposure flips. The frame's
// Casting/Streaming rows have NO schemas/settings.toml entries → NOTHING
// renders for them, never dead controls that look alive.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTINGS_ENTRIES } from "../generated/schema";

/** Recording — the w26.b02-seeded rows (directory "~/Videos" + format). */
const GeneratedRecordingScreen = GENERATED_SCREENS.recording;

/** True when any recording-screen schema row is exposed (rule F2 gate). */
const RECORDING_EXPOSED = SETTINGS_ENTRIES.some(
  (entry) => entry.screen === "recording" && entry.featureExposed,
);

/**
 * Recording settings area (route /settings/recording; System group). Today
 * the recording feature is gated off, so the seeded rows render hidden +
 * inert and the area shows the honest empty state; when the gate flips the
 * generated Recording section (directory + format rows) renders in place.
 */
export function RecordingArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="recording" data-testid="recording-area">
      <GeneratedRecordingScreen />
      {/* Casting (frame §C12 y232: "Discover cast devices", "Cast quality")
       * and Streaming out (y379: "Play locally while streaming", "Include
       * audio" / "Include video" / "Include subtitles", "Encoding preset",
       * "Tuning"): NO schemas/settings.toml entries and no cast/stream-out
       * commands in the catalog → rendered as NOTHING, never dead controls
       * that look alive. Handoff note for the schema owner:
       * docs/waves/w26/logs/w26.b07.md. */}
      {!RECORDING_EXPOSED ? (
        <p className="settings-screen__empty" data-testid="settings-area-empty" role="note">
          {m("settings.area.empty")}
        </p>
      ) : null}
    </div>
  );
}
