// Settings area: Input & Codecs — card 4.E4, doc 07 §5.4 (the codecs→video
// registry fold; renders below the generated Video sections).
//
// HONEST SURFACE RULES (same as 4.E3):
//   • hw-decode policy (playback.hw-decode-policy — a REAL schema key): ADR-001
//     is ACCEPTED (hw-first H.264/HEVC via OS decoders), but the implementation
//     is Phase 7 (cards 7.B) and product.toml keeps `features.hw-decode =
//     false` under rule F2 (exposure requires a passing implementation). So the
//     control renders VISIBLE, BOUND to the schema key, and DISABLED WITH THE
//     HONEST REASON — per the 4.E3 capability pattern, never a dead control
//     that looks alive, never hidden as if the policy did not exist.
//     w26.gfix-B (g12-P1-6): the control is the settings-row grammar's CLOSED
//     compact Dropdown (320×36), never the retired always-expanded Values-1
//     radio stack; the row-level "Advanced" badge retired with it (the
//     honest status is the gate note under the row).
//   • cache: performance.cache-demux-mib / cache-network-mib are real schema
//     keys (advanced-flagged). NOTE: their schema placement currently also
//     renders them on the generated "performance" screen — flagged to the
//     schema owner in ui/NEXT-STEPS.md to move placement here (D5/D6 safe).
//   • Codec family toggles / decoder picking: NO schema entries → NOTHING.

import type { ReactElement } from "react";
import { Dropdown } from "../../../components/menu/Dropdown";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as seedKeys } from "../../../i18n/keys/w26.f01";
import { SettingField } from "../SettingField";
import {
  SETTING_PERFORMANCE_CACHE_DEMUX_MIB,
  SETTING_PERFORMANCE_CACHE_NETWORK_MIB,
  SETTING_PLAYBACK_HW_DECODE_POLICY,
} from "../generated/schema";
import { useSettings } from "../SettingsProvider";
import { makeAreaMsg } from "./shared";

/** Pre-registration seam: b01 + b02 + f01 lane keys (w26.k13: the shared
 * makeAreaMsg factory — the j13 ruling 1 dedup). */
const useAreaMsg = makeAreaMsg(registryKeys, laneKeys, seedKeys);

/**
 * Hardware-decode policy control — bound to playback.hw-decode-policy, the
 * settings-row closed Dropdown grammar. Disabled with the honest ADR-001/F2
 * reason while the feature is not exposed (product.toml [features]
 * hw-decode = false → Phase 7).
 */
export function HwDecodePolicyControl(): ReactElement {
  const m = useAreaMsg();
  const { value } = useSettings();
  const entry = SETTING_PLAYBACK_HW_DECODE_POLICY;
  const disabled = !entry.featureExposed; // F2: not exposed while unimplemented
  const label = m(entry.description as MessageKey);
  return (
    <div
      className="settings-field"
      data-kind="enum"
      data-setting={entry.key}
      data-setting-gated={disabled ? entry.featureGate : undefined}
      data-gated-disabled={disabled || undefined}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
        </div>
        <div className="settings-field__control">
          <Dropdown
            className="settings-field__dropdown"
            density="compact"
            label={label}
            options={entry.enumeration.map((o) => ({
              value: o.value,
              labelKey: m(o.msgid as MessageKey),
            }))}
            value={String(value(entry.key))}
            onChange={() => {
              /* inert by design: the F2-gated policy cannot be changed yet */
            }}
            disabled={disabled}
          />
        </div>
      </div>
      {disabled ? (
        <p className="settings-field__badges" role="note" data-gate-reason={entry.featureGate}>
          {/* gate note (not a restart badge) — the quiet warm pill; the
              gold accent chip is reserved for restart-required states */}
          <span className="settings-badge">{m("settings.codecs.hw-decode-unavailable")}</span>
        </p>
      ) : null}
    </div>
  );
}

/** Input & Codecs settings area (hand-authored; see module header). */
export function CodecsArea(): ReactElement {
  const m = useAreaMsg();
  return (
    <div className="settings-sections" data-settings-area="codecs" data-testid="codecs-area">
      <section className="settings-section" data-section="decode">
        <h2 className="settings-section__title">{m("settings.section.codecs.decode")}</h2>
        <HwDecodePolicyControl />
        {/* codec family toggles: no schema keys → nothing rendered (see header). */}
      </section>
      <section className="settings-section" data-section="cache">
        <h2 className="settings-section__title">{m("settings.section.codecs.cache")}</h2>
        <SettingField entry={SETTING_PERFORMANCE_CACHE_DEMUX_MIB} />
        <SettingField entry={SETTING_PERFORMANCE_CACHE_NETWORK_MIB} />
      </section>
    </div>
  );
}
