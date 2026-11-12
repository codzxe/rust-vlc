import { useState } from "react";
import type { ReactNode } from "react";
import { Dropdown } from "../../components/menu/Dropdown";
import { Icon } from "../../components/icon/Icon";
import { NumberField } from "../../components/form/NumberField";
import { Switch } from "../../components/form/Switch";
import { TextField } from "../../components/form/TextField";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../i18n/keys/w26.b02";
import { keys as f02Keys } from "../../i18n/keys/w26.f02";
import type { SettingEntry } from "./generated/schema";
import { SPECIAL_FIELD_WIDGETS } from "./areas";
import { useSettings } from "./SettingsProvider";
import "./setting-field.css";

// SettingField — one schema entry as a settings ROW (W26 Figma grammar,
// tools-settings.md §D2): label left (#8F8F8F ≈15px — EVERY row type, never a
// floating control), control right in the 320px column right-anchored to the
// content edge; pitch 42–46px. Type → control mapping (w26.f02 anatomy pass
// over card 4.E1 §5.3 G1):
//   bool → Switch · enum → compact Dropdown — a CLOSED 320×36 trigger (§D2
//   row grammar; the stacked RadioGroup was the always-expanded Values-1
//   grammar, e06/e08 F1) · int/float/duration-ms → NumberField (schema
//   min/max; step derived — the schema carries no step: int 1, float 0.05,
//   duration-ms 50) · string/list<string>/path/url/percent → TextField.
// The settings-row CONTROL skin (dropdown trigger 320×36, input well
// 318×34 rx5, number fields stretched to the column) lives in
// setting-field.css, scoped to .settings-field__control so the shared form
// primitives keep their onboarding grammar everywhere else (e06 F7, e08 F10).
// Every option is bound to a REAL schema key; gated-but-unexposed features
// render nothing (hidden + inert, doc 05 §6.4). The areas layer's special
// widgets (SPECIAL_FIELD_WIDGETS) replace the control inside the same row.
// Advanced entries are segregated behind the in-flow "Advanced Options"
// disclosure (SettingsSections); the restart badge stays (behavior contract).
//
// ⓘ hint affordance (§D2 "optional ⓘ r7.5 #404040 sw1.125 after the label",
// e06 F11): rendered ONLY where a hint text exists — the `hint` prop first,
// then the generated seed's future `hint` msgid field (read defensively; the
// current schema seed carries none — b01 log item 2). No hint → no ⓘ, honest.

const labelOf = (entry: SettingEntry): MessageKey => entry.description as MessageKey;

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
type F02Key = keyof typeof f02Keys;
/** Pre-registration seam: b01 + b02 (registry/seed) + f02 lane keys —
 * identical output once the Lead registers them post-wave (b19). */
const laneCatalog: Readonly<Record<string, string>> = {
  ...registryKeys,
  ...laneKeys,
  ...f02Keys,
};

/** useFieldMsg — useMsg + the settings lanes' pre-registration keys. */
function useFieldMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey | F02Key): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

function numberStep(entry: SettingEntry): number {
  if (entry.type === "float") return 0.05;
  if (entry.type === "duration-ms") return 50;
  return 1;
}

export interface SettingFieldProps {
  entry: SettingEntry;
  /**
   * ⓘ hint msgid (§D2 "optional ⓘ") — rendered as the info affordance after
   * the row label. `undefined` (today's seed carries no hint text) renders
   * no ⓘ — honest absence, never a fabricated description.
   */
  hint?: MessageKey | LaneKey | RegistryKey | F02Key;
}

/** SettingField — one schema entry as a labeled, dirty-tracked control row. */
export function SettingField({ entry, hint }: SettingFieldProps) {
  const m = useFieldMsg();
  const { value, setValue, isDirty, resetField, restartPending } = useSettings();
  const [hintOpen, setHintOpen] = useState(false);
  // Feature not exposed in product.toml [features] → hidden + inert (F1/F2).
  if (!entry.featureExposed) return null;

  const rowLabel = m(labelOf(entry));
  // Seed seam (b01 log item 2): when the generator grows a `hint` msgid
  // field, it flows through here with zero further changes in this file.
  const seedHint = "hint" in entry && typeof entry.hint === "string" ? entry.hint : undefined;
  const hintKey = hint ?? (seedHint as MessageKey | undefined);
  const hintText = hintKey !== undefined ? m(hintKey) : undefined;
  const hasHint = typeof hintText === "string" && hintText !== "";

  const current = value(entry.key);
  const restartBadge =
    entry.restartRequired && (isDirty(entry.key) || restartPending.includes(entry.key));
  const canReset = current !== entry.defaultValue;

  let control: ReactNode;
  const Special = SPECIAL_FIELD_WIDGETS[entry.key];
  if (Special !== undefined) {
    control = <Special entry={entry} />;
  } else if (entry.type === "bool") {
    control = (
      <Switch
        aria-label={rowLabel}
        checked={current === true}
        onChange={(checked) => setValue(entry.key, checked)}
      />
    );
  } else if (entry.type === "enum") {
    // Closed dropdown (§D2): trigger 320×36 rx6 #090909 + inner 316×32 rx4
    // #191919, value #8F8F8F ≈15px — skinned in setting-field.css. Option
    // labels arrive pre-resolved through the labelKey + identity resolver
    // seam (the component's i18n contract stays key-driven for other hosts).
    control = (
      <Dropdown
        className="settings-field__dropdown"
        density="compact"
        label={rowLabel}
        options={entry.enumeration.map((o) => ({
          value: o.value,
          labelKey: m(o.msgid as MessageKey | LaneKey | RegistryKey | F02Key),
        }))}
        value={String(current)}
        onChange={(next) => setValue(entry.key, next)}
      />
    );
  } else if (entry.type === "int" || entry.type === "float" || entry.type === "duration-ms") {
    control = (
      <NumberField
        className="settings-field__bare"
        label={rowLabel}
        labelHidden
        min={entry.min}
        max={entry.max}
        step={numberStep(entry)}
        value={typeof current === "number" ? current : entry.min ?? 0}
        onChange={(e) => {
          const next = Number(e.target.value);
          if (Number.isFinite(next)) setValue(entry.key, next);
        }}
      />
    );
  } else if (entry.type === "list<string>") {
    const list = Array.isArray(current) ? current : [];
    control = (
      <TextField
        className="settings-field__bare"
        label={rowLabel}
        hideLabel
        value={list.join(", ")}
        onChange={(e) =>
          setValue(
            entry.key,
            e.target.value
              .split(",")
              .map((part) => part.trim())
              .filter((part) => part !== ""),
          )
        }
      />
    );
  } else {
    control = (
      <TextField
        className="settings-field__bare"
        label={rowLabel}
        hideLabel
        value={String(current)}
        onChange={(e) => setValue(entry.key, e.target.value)}
      />
    );
  }

  return (
    <div
      className="settings-field"
      data-setting={entry.key}
      data-dirty={isDirty(entry.key) || undefined}
      data-advanced={entry.advanced || undefined}
    >
      <div className="settings-field__row">
        {/* Lead cluster — label + ⓘ + badges + per-field reset in the left
            column. The label renders for EVERY row type (e06 F2 / e08 F2:
            the row grammar is label-left; text/number inputs used to float
            bare). It is aria-hidden because the control already carries the
            same accessible name (the form field's associated label, the
            Switch/Dropdown aria-labels) — no double announcement. */}
        <div className="settings-field__lead">
          <span className="settings-field__label" aria-hidden="true">
            {rowLabel}
          </span>
          {hasHint ? (
            <button
              type="button"
              className="settings-field__info"
              aria-expanded={hintOpen}
              aria-label={m("settings.field.more-info")}
              title={hintText}
              onClick={() => setHintOpen((open) => !open)}
            >
              <Icon name="info" size={14} />
            </button>
          ) : null}
          {restartBadge ? (
            <span className="settings-badge settings-badge--restart">{m("settings.restart-required")}</span>
          ) : null}
          {canReset ? (
            <button
              type="button"
              className="settings-field__reset"
              aria-label={m("settings.reset-field")}
              title={m("settings.reset-field")}
              onClick={() => resetField(entry.key)}
            >
              {m("settings.reset-field")}
            </button>
          ) : null}
        </div>
        <div className="settings-field__control">{control}</div>
      </div>
      {hintOpen && typeof hintText === "string" ? (
        <p className="settings-field__hinttext">{hintText}</p>
      ) : null}
    </div>
  );
}
