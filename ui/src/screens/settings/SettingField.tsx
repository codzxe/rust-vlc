import type { ReactNode } from "react";
import { NumberField } from "../../components/form/NumberField";
import { RadioGroup } from "../../components/form/RadioGroup";
import { Switch } from "../../components/form/Switch";
import { TextField } from "../../components/form/TextField";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../i18n/keys/w26.b02";
import type { SettingEntry } from "./generated/schema";
import { SPECIAL_FIELD_WIDGETS } from "./areas";
import { useSettings } from "./SettingsProvider";

// SettingField — one schema entry as a settings ROW (W26 Figma grammar,
// tools-settings.md §D2): label left (#8F8F8F ≈15px), control right in the
// 320px column right-anchored to the content edge; pitch 42–46px. Type →
// control mapping unchanged from card 4.E1 (doc 07 §5.3 G1):
//   bool → Switch · enum → RadioGroup · int/float/duration-ms → NumberField
//   (schema min/max; step derived — the schema carries no step: int 1,
//    float 0.05, duration-ms 50) · string/list<string> → TextField.
// Every option is bound to a REAL schema key; gated-but-unexposed features
// render nothing (hidden + inert, doc 05 §6.4). The areas layer's special
// widgets (SPECIAL_FIELD_WIDGETS) replace the control inside the same row.
// Advanced entries no longer carry a badge — they are segregated behind the
// in-flow "Advanced Options" disclosure (SettingsSections); the restart
// badge stays (behavior contract).

const labelOf = (entry: SettingEntry): MessageKey => entry.description as MessageKey;

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
/** Pre-registration seam: b01 + b02 (registry/seed) lane keys — identical
 * output once the Lead registers them post-wave (b19). */
const laneCatalog: Readonly<Record<string, string>> = { ...registryKeys, ...laneKeys };

/** useFieldMsg — useMsg + the settings lanes' pre-registration keys. */
function useFieldMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

function numberStep(entry: SettingEntry): number {
  if (entry.type === "float") return 0.05;
  if (entry.type === "duration-ms") return 50;
  return 1;
}

/** SettingField — one schema entry as a labeled, dirty-tracked control row. */
export function SettingField({ entry }: { entry: SettingEntry }) {
  const m = useFieldMsg();
  const { value, setValue, isDirty, resetField, restartPending } = useSettings();
  // Feature not exposed in product.toml [features] → hidden + inert (F1/F2).
  if (!entry.featureExposed) return null;

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
        aria-label={m(labelOf(entry))}
        checked={current === true}
        onChange={(checked) => setValue(entry.key, checked)}
      />
    );
  } else if (entry.type === "enum") {
    control = (
      <RadioGroup
        label={m(labelOf(entry))}
        options={entry.enumeration.map((o) => ({
          value: o.value,
          label: m(o.msgid as MessageKey | LaneKey | RegistryKey),
        }))}
        value={String(current)}
        onChange={(next) => setValue(entry.key, next)}
      />
    );
  } else if (entry.type === "int" || entry.type === "float" || entry.type === "duration-ms") {
    control = (
      <NumberField
        className="settings-field__bare"
        label={m(labelOf(entry))}
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
        label={m(labelOf(entry))}
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
        label={m(labelOf(entry))}
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
        {/* Lead cluster — label + badges + per-field reset in the left
            column. Text/number fields keep their own (associated) label
            inside the control, clipped to a screen-reader-only box. */}
        <div className="settings-field__lead">
          {entry.type === "bool" || entry.type === "enum" || Special !== undefined ? (
            <span className="settings-field__label">{m(labelOf(entry))}</span>
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
    </div>
  );
}
