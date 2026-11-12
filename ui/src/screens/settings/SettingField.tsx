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
import { keys as fixerKeys } from "../../i18n/keys/w26.gfixB";
import type { SettingEntry } from "./generated/schema";
import { SPECIAL_FIELD_WIDGETS } from "./areas";
import { WIDGET_GROUPED_KEYS } from "./SettingWidgets";
import { isSliderFieldKey, isValuesFieldKey } from "./SettingWidgets";
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
//   duration-ms 50) · path → the 208×34 well + Browse companion (§C0; the
//   frame shows Browse ONLY on the path rows — global.log-file,
//   interface.skin-file, recording.directory) · url/list<string>/string →
//   plain TextField well (the privacy frame's Last.fm rows carry no Browse,
//   w26.gfix-B url-vs-path split).
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
type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam: b01 + b02 (registry/seed) + f02 + gfixB lane keys —
 * identical output once the Lead registers them post-wave (b19). */
const laneCatalog: Readonly<Record<string, string>> = {
  ...registryKeys,
  ...laneKeys,
  ...f02Keys,
  ...fixerKeys,
};

/** useFieldMsg — useMsg + the settings lanes' pre-registration keys. */
function useFieldMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey | F02Key | FixerKey): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

function numberStep(entry: SettingEntry): number {
  if (entry.type === "float") return 0.05;
  if (entry.type === "duration-ms") return 50;
  return 1;
}

/**
 * Duration display (g13-P2-7): the controls frame shows the skip rows as
 * human seconds ("3 s"), the performance frame keeps raw ms ("5000 ms") —
 * so the format is keyed: controls.skip* render seconds, every other
 * duration-ms row keeps its millisecond spelling.
 */
const secondsDuration = (key: string): boolean => key.startsWith("controls.skip");

const formatDurationMs = (key: string, value: number): string =>
  secondsDuration(key) ? `${value / 1000} s` : `${value} ms`;

const parseDurationMs = (key: string, text: string): number | null => {
  const digits = text.replace(/[^0-9.]/g, "");
  if (digits === "") return null;
  const parsed = Number(digits);
  if (!Number.isFinite(parsed)) return null;
  return secondsDuration(key) ? parsed * 1000 : parsed;
};

/** Float display (audio frame: "0.0"): one decimal minimum, natural beyond. */
const formatFloatValue = (value: number): string =>
  Number.isInteger(value) ? value.toFixed(1) : String(value);

export interface SettingFieldProps {
  entry: SettingEntry;
  /**
   * ⓘ hint msgid (§D2 "optional ⓘ") — rendered as the info affordance after
   * the row label. `undefined` (today's seed carries no hint text) renders
   * no ⓘ — honest absence, never a fabricated description.
   */
  hint?: MessageKey | LaneKey | RegistryKey | F02Key | FixerKey;
}

/** SettingField — one schema entry as a labeled, dirty-tracked control row. */
export function SettingField({ entry, hint }: SettingFieldProps) {
  const m = useFieldMsg();
  const { value, setValue, isDirty, resetField, restartPending } = useSettings();
  const [hintOpen, setHintOpen] = useState(false);
  // Feature not exposed in product.toml [features] → hidden + inert (F1/F2).
  if (!entry.featureExposed) return null;
  // Compound-widget keys (g12-P1-4): rendered INSIDE another key's special
  // widget — never a standalone row (search included).
  if (WIDGET_GROUPED_KEYS.has(entry.key)) return null;

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
  // Every branch below assigns; no dead initializer (eslint no-useless-assignment).
  let kind: string;
  const Special = SPECIAL_FIELD_WIDGETS[entry.key];
  if (Special !== undefined) {
    control = <Special entry={entry} />;
    kind = isSliderFieldKey(entry.key) ? "slider" : isValuesFieldKey(entry.key) ? "values" : "special";
  } else if (entry.type === "bool") {
    kind = "switch";
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
    kind = "enum";
    control = (
      <Dropdown
        className="settings-field__dropdown"
        density="compact"
        label={rowLabel}
        options={entry.enumeration.map((o) => ({
          value: o.value,
          labelKey: m(o.msgid as MessageKey | LaneKey | RegistryKey | F02Key | FixerKey),
        }))}
        value={String(current)}
        onChange={(next) => setValue(entry.key, next)}
      />
    );
  } else if (entry.type === "duration-ms") {
    // Duration rows (g13-P2-7): the frame shows formatted durations ("3 s",
    // "5000 ms") in the standard 318×34 well — a text field with a numeric
    // parse, not a bare number input.
    kind = "duration";
    const numeric = typeof current === "number" ? current : (entry.min ?? 0);
    control = (
      <TextField
        className="settings-field__bare"
        label={rowLabel}
        hideLabel
        inputMode="numeric"
        value={formatDurationMs(entry.key, numeric)}
        onChange={(e) => {
          const next = parseDurationMs(entry.key, e.target.value);
          if (next !== null) {
            const min = entry.min ?? -Infinity;
            const max = entry.max ?? Infinity;
            setValue(entry.key, Math.min(max, Math.max(min, next)));
          }
        }}
      />
    );
  } else if (entry.type === "float") {
    // Float rows (audio.replay-gain-preamp — frame value "0.0"): a formatted
    // text well, not a bare number input — the frame spells one decimal
    // ("0.0", the EQ chip grammar "12.0 dB"); the steppers are display:none
    // in the settings skin anyway. Parse + clamp keeps the schema range.
    kind = "number";
    const numeric = typeof current === "number" ? current : (entry.min ?? 0);
    control = (
      <TextField
        className="settings-field__bare"
        label={rowLabel}
        hideLabel
        inputMode="decimal"
        value={formatFloatValue(numeric)}
        onChange={(e) => {
          const next = Number(e.target.value);
          if (Number.isFinite(next) && e.target.value.trim() !== "") {
            const min = entry.min ?? -Infinity;
            const max = entry.max ?? Infinity;
            setValue(entry.key, Math.min(max, Math.max(min, next)));
          }
        }}
      />
    );
  } else if (entry.type === "int") {
    kind = "number";
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
  } else if (entry.type === "path") {
    // Path rows (g10-P2-2): 208×34 well + Browse 100×32 r6 (§C0 control
    // inventory — the frame shows the companion on every PATH row: log
    // file, skin file, recording directory). The input is LIVE (a typed
    // path is a real value); the Browse ACTION has no file-dialog command
    // in the catalog → disabled with the honest reason (visible, inert,
    // never a dead control that looks alive). URL rows (privacy's Last.fm
    // submission URL) fall through to the plain well below — their frame
    // carries no Browse.
    kind = "path";
    const browseNoteId = `${entry.key}-browse-note`;
    control = (
      <div className="settings-field__path">
        <TextField
          className="settings-field__bare"
          label={rowLabel}
          hideLabel
          value={String(current)}
          onChange={(e) => setValue(entry.key, e.target.value)}
        />
        <button
          type="button"
          className="settings-action settings-field__browse-action"
          disabled
          aria-describedby={browseNoteId}
          title={m("settings.capability-unavailable")}
        >
          {m("settings.field.browse")}
        </button>
        <span id={browseNoteId} className="settings-vh">
          {m("settings.capability-unavailable")}
        </span>
      </div>
    );
  } else if (entry.type === "list<string>") {
    kind = "text";
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
    kind = "text";
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
      data-kind={kind}
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
