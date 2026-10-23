import type { ReactNode } from "react";
import { NumberField } from "../../components/form/NumberField";
import { RadioGroup } from "../../components/form/RadioGroup";
import { Switch } from "../../components/form/Switch";
import { TextField } from "../../components/form/TextField";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import type { SettingEntry } from "./generated/schema";
import { useSettings } from "./SettingsProvider";

// Field renderer (card 4.E1): maps schema field types → the 4.B1 form
// primitives (doc 07 §5.3 G1 control selection):
//   bool → Switch · enum → RadioGroup · int/float/duration-ms → NumberField
//   (schema min/max; step derived — the schema carries no step: int 1,
//    float 0.05, duration-ms 50) · string/list<string> → TextField.
// Every option is bound to a REAL schema key; gated-but-unexposed features
// render nothing (hidden + inert, doc 05 §6.4).

const labelOf = (entry: SettingEntry): MessageKey => entry.description as MessageKey;

function numberStep(entry: SettingEntry): number {
  if (entry.type === "float") return 0.05;
  if (entry.type === "duration-ms") return 50;
  return 1;
}

/** SettingField — one schema entry as a labeled, dirty-tracked control. */
export function SettingField({ entry }: { entry: SettingEntry }) {
  const m = useMsg();
  const { value, setValue, isDirty, resetField, restartPending } = useSettings();
  // Feature not exposed in product.toml [features] → hidden + inert (F1/F2).
  if (!entry.featureExposed) return null;

  const current = value(entry.key);
  const restartBadge =
    entry.restartRequired && (isDirty(entry.key) || restartPending.includes(entry.key));
  const canReset = current !== entry.defaultValue;

  let control: ReactNode;
  if (entry.type === "bool") {
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
        options={entry.enumeration.map((o) => ({ value: o.value, label: m(o.msgid as MessageKey) }))}
        value={String(current)}
        onChange={(next) => setValue(entry.key, next)}
      />
    );
  } else if (entry.type === "int" || entry.type === "float" || entry.type === "duration-ms") {
    control = (
      <NumberField
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
        label={m(labelOf(entry))}
        value={String(current)}
        onChange={(e) => setValue(entry.key, e.target.value)}
      />
    );
  }

  return (
    <div className="settings-field" data-setting={entry.key} data-dirty={isDirty(entry.key) || undefined}>
      {entry.type === "bool" ? (
        <div className="settings-field__head">
          <span className="settings-field__label">{m(labelOf(entry))}</span>
          {entry.advanced ? <span className="settings-badge">{m("settings.advanced-badge")}</span> : null}
          {restartBadge ? <span className="settings-badge settings-badge--restart">{m("settings.restart-required")}</span> : null}
        </div>
      ) : null}
      {control}
      {entry.type !== "bool" && (entry.advanced || restartBadge) ? (
        <p className="settings-field__badges">
          {entry.advanced ? <span className="settings-badge">{m("settings.advanced-badge")}</span> : null}
          {restartBadge ? (
            <span className="settings-badge settings-badge--restart">{m("settings.restart-required")}</span>
          ) : null}
        </p>
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
  );
}
