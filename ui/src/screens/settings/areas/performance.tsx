// Settings area: Performance — "Performance Settings" (Settings -
// performance.svg, analysis tools-settings.md §C11).
//
// w26.k13 (j13 P1-1, the wave's last open P1): the frame carries 10 r7.5
// #404040 ⓘ circles (Ellipse 9..9_10 — circle extraction: the four Decoding
// rows at cy 132/178/224/271, Process priority at cy 462, and the five
// Developer rows at cy 668/714/760/807/855). The pure-generated screen wired
// ZERO of them (the i13/i14 hint wave covered five hand areas and skipped
// the sixth schema screen — no hint keys existed, no test existed to catch
// it). This module hand-renders the area (the GlobalAreaScreen precedent:
// normal sections + the Advanced Options disclosure) to pass per-row `hint`
// props through SettingField, with this lane's hint copy
// (ui/src/i18n/keys/w26.k13.ts, resolved through the settingsMsg seam).
// The screen stays schema-driven: sections/entries/order come from the
// generated seed, feature gates + special widgets + dirty/restart contracts
// stay framework-driven.
//
// Frame truth vs schema (composition unchanged from the generated screen —
// the w26.f01 seed comment documents the C11 grammar):
//   • Decoding (y83) — 4 live rows, every one carries the frame's ⓘ.
//   • Smooth Playback (y359) — Drop frames / real-time priority carry NO
//     circle in the frame (reproduced honestly); Process priority does.
//   • Advanced Options bar (y548) — the frame ships it EXPANDED with the
//     Developer header; the app's documented grammar (SettingsSections,
//     the global.svg Frame 265 pattern) renders the advanced rows flat
//     behind the collapsed bar: the 3 resources rows + the 5 Developer
//     rows, all 5 Developer rows ⓘ-carrying (global-1 flat grammar).
//   • The frame's "Rescan plugins" row (§C11 y491) is an ACTION with no
//     command in the catalog and no [[setting]] key — not rendered (the
//     honest-empty rule; flagged to the Lead in the w26.k13 lane log).
//
// `Settings - performance`'s resources rows sit below the frame's exported
// fold — the frame carries no ⓘ claim for them, so they render bare.

import { useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as k13Keys } from "../../../i18n/keys/w26.k13";
import type { SettingEntry } from "../generated/schema";
import { AdvancedDisclosure } from "../SettingsSections";
import { SettingField } from "../SettingField";
import { partitionAdvanced } from "./shared";
import { schemaSectionViews } from "./shared";

type K13Key = keyof typeof k13Keys;

/** The ⓘ hint msgid cast through the registry seam (the hintMsg precedent —
 * the settingsMsg catalog carries the lane file until registration). */
const hintMsg = (key: K13Key): MessageKey => key as string as MessageKey;

/** ⓘ hint msgids per schema row (w26.k13, j13 P1-1 — the frame's 10 circles;
 * the two Smooth-Playback toggles and the 3 resources rows carry none,
 * reproduced honestly). */
const PERFORMANCE_HINTS: Readonly<Record<string, MessageKey>> = {
  "performance.hw-decode": hintMsg("settings.hint.performance.hw-decode"),
  "performance.video-post-processing": hintMsg("settings.hint.performance.video-post-processing"),
  "performance.h264-loop-filter": hintMsg("settings.hint.performance.h264-loop-filter"),
  "performance.decoding-threads": hintMsg("settings.hint.performance.decoding-threads"),
  "performance.process-priority": hintMsg("settings.hint.performance.process-priority"),
  "performance.encoder": hintMsg("settings.hint.performance.encoder"),
  "performance.decoder": hintMsg("settings.hint.performance.decoder"),
  "performance.clock-synchronisation": hintMsg("settings.hint.performance.clock-synchronisation"),
  "performance.clock-jitter": hintMsg("settings.hint.performance.clock-jitter"),
  "performance.clock-reference-averaging": hintMsg(
    "settings.hint.performance.clock-reference-averaging",
  ),
};

/**
 * Performance settings area (route /settings/performance; System group) —
 * the frame's Decoding + Smooth Playback sections from the schema seed, the
 * advanced rows (resources + Developer) behind the in-flow Advanced Options
 * disclosure (the shell's ONE AdvancedDisclosure, w26.k12), hand-rendered
 * with per-row ⓘ hints (j13 P1-1).
 */
export function PerformanceArea(): ReactElement {
  const m = useMsg();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const { sections, advanced } = partitionAdvanced(schemaSectionViews("performance"));

  return (
    <div className="settings-sections" data-settings-area="performance">
      {sections.map(({ section, entries }) => (
        <section key={section.id} className="settings-section" data-section={section.id}>
          <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
          {entries.map((entry: SettingEntry) => (
            <SettingField key={entry.key} entry={entry} hint={PERFORMANCE_HINTS[entry.key]} />
          ))}
        </section>
      ))}
      {advanced.length > 0 ? (
        <AdvancedDisclosure open={advancedOpen} onToggle={() => setAdvancedOpen((open) => !open)}>
          {advanced.map((entry: SettingEntry) => (
            <SettingField key={entry.key} entry={entry} hint={PERFORMANCE_HINTS[entry.key]} />
          ))}
        </AdvancedDisclosure>
      ) : null}
    </div>
  );
}
