// Settings area: Input & Codecs — card 4.E4, doc 07 §5.4.
//
// HONEST SURFACE RULES (same as 4.E3):
//   • hw-decode policy (playback.hw-decode-policy — a REAL schema key): ADR-001
//     is ACCEPTED (hw-first H.264/HEVC via OS decoders), but the implementation
//     is Phase 7 (cards 7.B) and product.toml keeps `features.hw-decode =
//     false` under rule F2 (exposure requires a passing implementation). So the
//     control renders VISIBLE, BOUND to the schema key, and DISABLED WITH THE
//     HONEST REASON — per the 4.E3 capability pattern, never a dead control
//     that looks alive, never hidden as if the policy did not exist.
//     E1's SettingField hides gated entries (doc 05 §6.4); this hand-authored
//     control is the card-mandated disabled-with-reason presentation of the
//     same schema entry. If F2 exposure ever flips true, `disabled` follows it.
//   • cache: performance.cache-demux-mib / cache-network-mib are real schema
//     keys (advanced-flagged). NOTE: their schema placement currently also
//     renders them on the generated "performance" screen — flagged to the
//     schema owner in ui/NEXT-STEPS.md to move placement here (D5/D6 safe).
//   • Codec family toggles / decoder picking: NO schema entries → NOTHING.

import type { ReactElement } from "react";
import { RadioGroup } from "../../../components/form/RadioGroup";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { SettingField } from "../SettingField";
import {
  SETTING_PERFORMANCE_CACHE_DEMUX_MIB,
  SETTING_PERFORMANCE_CACHE_NETWORK_MIB,
  SETTING_PLAYBACK_HW_DECODE_POLICY,
} from "../generated/schema";
import { useSettings } from "../SettingsProvider";

/**
 * Hardware-decode policy control — bound to playback.hw-decode-policy.
 * Disabled with the honest ADR-001/F2 reason while the feature is not
 * exposed (product.toml [features] hw-decode = false → Phase 7).
 */
export function HwDecodePolicyControl(): ReactElement {
  const m = useMsg();
  const { value } = useSettings();
  const entry = SETTING_PLAYBACK_HW_DECODE_POLICY;
  const disabled = !entry.featureExposed; // F2: not exposed while unimplemented
  return (
    <div
      className="settings-field"
      data-setting={entry.key}
      data-setting-gated={disabled ? entry.featureGate : undefined}
      data-gated-disabled={disabled || undefined}
    >
      <div className="settings-field__head">
        <span className="settings-field__label">{m(entry.description as MessageKey)}</span>
        <span className="settings-badge">{m("settings.advanced-badge")}</span>
      </div>
      <RadioGroup
        label={m(entry.description as MessageKey)}
        options={entry.enumeration.map((o) => ({ value: o.value, label: m(o.msgid as MessageKey) }))}
        value={String(value(entry.key))}
        onChange={() => {
          /* inert by design: the F2-gated policy cannot be changed yet */
        }}
        disabled={disabled}
      />
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
  const m = useMsg();
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
