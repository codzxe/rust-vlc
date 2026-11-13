import type { ReactElement } from "react";
import { Slider } from "../../components/slider/Slider";
import { Switch } from "../../components/form/Switch";
import { useRadiogroupKeyboard } from "../../components/form/useRadiogroupKeyboard";
import { useSettingsMsg } from "./settingsMsg";
import type { SettingEntry } from "./generated/schema";
import { SETTING_AUDIO_STARTUP_VOLUME_MODE } from "./generated/schema";
import { useSettings } from "./SettingsProvider";

// SettingWidgets — the per-key special controls that have no primitive
// mapping (w26.gfix-B, findings g11-P2-3 / g12-P1-3 / g12-P1-4). Registered
// in the specialWidgets registry (w26.k12 leaf module, formerly
// areas/index.tsx SPECIAL_FIELD_WIDGETS); SettingField renders them inside
// the standard row (label left, 320px control column right).
//
// w26.k12 (j12 P3-5): the lane-seam resolver is the shared settingsMsg
// module (was this file's private useWidgetMsg copy).
//
// Frame truth (tools-settings.md §C0 "Slider" + a19 B5 "Values-1"):
//   Slider   track 320×16 rx6 #090909 · fill 12 rx4 #371F06 inset 2 · thumb
//            8×24 rx4 #AB6314 · optional degree ticks (#1A1611) and scale
//            labels ≈12px #6B6B6B under the track (playback speed, maximum
//            volume, hide-after). The base Slider primitive carries the
//            geometry; the settings skin (setting-field.css) restyles it.
//   Values-1 320×78 rx6 #090909 · rows 316×36 rx4 (2px gaps) · selected
//            #371F06 + label #D37918 · unselected #191919 + label #6B6B6B ·
//            nested input 182×32 rx2 @ 0.4 opacity while its row is
//            unselected.

/** One scale label: text anchored at `at` (0..1 across the track). */
interface ScaleMark {
  readonly at: number;
  readonly label: string;
}

/** Per-key slider anatomy (min/max/step + optional ticks and scale labels). */
interface SliderSpec {
  readonly min: number;
  readonly max: number;
  readonly step: number;
  /** Track tick count (degree marks — decorative, aria-hidden). */
  readonly ticks?: number;
  readonly scale?: readonly ScaleMark[];
  /** aria-valuetext formatter. */
  readonly format: (value: number) => string;
}

const numberLabel = (value: number): string => String(value);

/** Frame-measured slider specs (tools-settings.md §C4/C5/C6/C2). */
const SLIDER_SPECS: Readonly<Record<string, SliderSpec>> = {
  // Picture rows (video.svg y349–517): INLINE layout — no scale labels. The
  // ranges track the schema (w26.gfix-B probe fix): saturation 0..300 puts
  // the default 100 at fill 107 ≈ frame 105 (thumb x806); gamma 0..1000
  // puts γ 1.0 (100) at fill 32 = frame exact (thumb x730). Tick counts are
  // the frame's degree marks (zero-bbox marker paths, y+5 below each
  // track): hue/brightness/contrast ONE tick at x864 (the 50% mark),
  // saturation two (x811/x917 = values 100/200 on 0..300), gamma nine
  // (x737.6–990.4, pitch 31.6) — the (i+1)/(n+1) spread reproduces every
  // frame set.
  "video.hue": { min: -180, max: 180, step: 1, ticks: 1, format: numberLabel },
  "video.brightness": { min: 0, max: 200, step: 1, ticks: 1, format: numberLabel },
  "video.contrast": { min: 0, max: 200, step: 1, ticks: 1, format: numberLabel },
  "video.saturation": { min: 0, max: 300, step: 1, ticks: 2, format: numberLabel },
  "video.gamma": { min: 0, max: 1000, step: 5, ticks: 9, format: numberLabel },
  // Playback speed (playback.svg y210): scale 0.25–2 at 0.25 steps (8 marks,
  // pitch 45.15 — the frame's slider spans 0.25–2 even though the schema
  // validation domain is 0.25–4; committed values stay inside the schema
  // range, g11-P2-3).
  "playback.default-rate": {
    min: 0.25,
    max: 2,
    step: 0.25,
    ticks: 6,
    scale: [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2].map((at) => ({
      at: (at - 0.25) / 1.75,
      label: numberLabel(at),
    })),
    format: (v) => `${v}×`,
  },
  // Maximum volume (audio.svg y315): scale 100 % / 200 % / 300 %.
  "audio.maximum-volume": {
    min: 100,
    max: 300,
    step: 5,
    ticks: 7,
    scale: [
      { at: 0, label: "100%" },
      { at: 0.5, label: "200%" },
      { at: 1, label: "300%" },
    ],
    format: (v) => `${v}%`,
  },
  // Hide after (interface.svg y714): scale "0 s" / "5 s" / "10s" (the
  // frame's right-most label is space-less, y741 — frame-verbatim).
  "interface.hide-controls-after": {
    min: 0,
    max: 10000,
    step: 500,
    ticks: 19,
    scale: [
      { at: 0, label: "0 s" },
      { at: 0.5, label: "5 s" },
      { at: 1, label: "10s" },
    ],
    format: (v) => `${v / 1000} s`,
  },
};

/** Row-kind probes for SettingField's data-kind marker (row pitch follows
 * the frame's control height: sliders pitch 42, the Values-1 block ~88). */
export const isSliderFieldKey = (key: string): boolean => key in SLIDER_SPECS;
const VALUES_FIELD_KEYS: ReadonlySet<string> = new Set(["audio.startup-volume"]);
export const isValuesFieldKey = (key: string): boolean => VALUES_FIELD_KEYS.has(key);

/**
 * Compound-widget keys (g12-P1-4): rendered INSIDE another key's special
 * widget — the Values-1 startup-volume control owns the mode enum — never as
 * a standalone row. SettingField returns null for them and schemaSectionsOf
 * filters them out of the composed section arrays.
 */
export const WIDGET_GROUPED_KEYS: ReadonlySet<string> = new Set(["audio.startup-volume-mode"]);

/**
 * The settings-row slider (g11-P2-3 / g12-P1-3): the base Slider primitive
 * in the frame's horizontal anatomy, with optional degree ticks and scale
 * labels. Controlled: commits through the settings store on release/step.
 */
export function SettingsSliderField({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useSettingsMsg();
  const { value, setValue } = useSettings();
  const spec = SLIDER_SPECS[entry.key];
  const currentRaw = value(entry.key);
  const current =
    typeof currentRaw === "number"
      ? currentRaw
      : typeof entry.defaultValue === "number"
        ? entry.defaultValue
        : 0;
  const label = m(entry.description as Parameters<typeof m>[0]);
  if (spec === undefined) {
    // Unregistered key — the honest fallback is the schema range, no scale.
    return (
      <Slider
        value={current}
        min={entry.min ?? 0}
        max={entry.max ?? 100}
        step={1}
        aria-label={label}
        onCommit={(next) => setValue(entry.key, next)}
      />
    );
  }
  const ticks = spec.ticks;
  return (
    <div
      className={
        spec.scale !== undefined
          ? "settings-slider settings-slider--scaled"
          : "settings-slider"
      }
      data-slider-scale={spec.scale !== undefined || undefined}
    >
      <Slider
        value={current}
        min={spec.min}
        max={spec.max}
        step={spec.step}
        aria-label={label}
        formatValue={spec.format}
        onCommit={(next) => setValue(entry.key, next)}
      />
      {ticks !== undefined ? (
        <div className="settings-slider__ticks" aria-hidden="true">
          {Array.from({ length: ticks }, (_, i) => (
            <span
              key={i}
              className="settings-slider__tick"
              style={{ left: `${((i + 1) / (ticks + 1)) * 100}%` }}
            />
          ))}
        </div>
      ) : null}
      {spec.scale !== undefined ? (
        <div className="settings-slider__scale" aria-hidden="true">
          {spec.scale.map((mark, i) => (
            <span
              key={i}
              className="settings-slider__scale-mark"
              style={{
                left: `${mark.at * 100}%`,
                transform:
                  mark.at === 0
                    ? "none"
                    : mark.at === 1
                      ? "translateX(-100%)"
                      : "translateX(-50%)",
              }}
            >
              {mark.label}
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The inverted display switch (g11-P2-2): the persisted key keeps its
 * negative semantics (playback.start_paused) while the row renders the
 * frame's positive copy ("Start playing as soon as a file opens" → ON when
 * start_paused is false). Clicking ON→OFF persists start_paused = true.
 */
export function InvertedSwitchField({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useSettingsMsg();
  const { value, setValue } = useSettings();
  const label = m(entry.description as Parameters<typeof m>[0]);
  return (
    <Switch
      aria-label={label}
      checked={value(entry.key) !== true}
      onChange={(checked) => setValue(entry.key, !checked)}
    />
  );
}

/** The Values-1 nested input: 182×32 rx2, "N%" value, inert at 0.4 opacity
 * while the "Remember last volume" row owns the selection (a19 B5). */
function StartupVolumeInput({
  volume,
  label,
  onCommit,
  disabled,
}: {
  volume: number;
  label: string;
  onCommit: (next: number) => void;
  disabled: boolean;
}): ReactElement {
  return (
    <input
      className="settings-values__input"
      type="text"
      inputMode="numeric"
      value={`${volume}%`}
      disabled={disabled}
      aria-label={label}
      onChange={(event) => {
        const next = Number(event.target.value.replace(/[^0-9]/g, ""));
        if (Number.isFinite(next) && event.target.value.trim() !== "") {
          onCommit(Math.min(300, Math.max(0, next)));
        }
      }}
    />
  );
}

/**
 * The Startup volume Values-1 control (g12-P1-4): two radio rows in one
 * 320×78 well — "Remember last volume" (selected: #371F06 + #D37918) and
 * "Always starts at:" with the nested percent input (dimmed + inert while
 * unselected). Binds BOTH schema keys: the mode enum drives the selection,
 * the int drives the input.
 *
 * w26.k12 (j12 P2-4): the WAI-ARIA radiogroup pattern — k01's shared roving
 * keyboard grammar (useRadiogroupKeyboard: four arrows, wrap-around, skips
 * disabled options) on the group host + ROVING TABINDEX (only the checked
 * radio is a tab stop; the unselected one rides tabIndex -1). The two
 * buttons used to be two independent tab stops with no arrow keys.
 */
export function StartupVolumeField({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useSettingsMsg();
  const { value, setValue } = useSettings();
  const mode = String(value(SETTING_AUDIO_STARTUP_VOLUME_MODE.key));
  const rememberLast = mode !== "always";
  const volumeRaw = value(entry.key);
  const volume = typeof volumeRaw === "number" ? volumeRaw : 20;
  const groupLabel = m("settings.audio.startup-volume");
  const onGroupKeyDown = useRadiogroupKeyboard(
    [
      { value: "remember-last" },
      { value: "always" },
    ],
    mode,
    (next) => setValue(SETTING_AUDIO_STARTUP_VOLUME_MODE.key, next),
  );
  return (
    <div
      className="settings-values"
      role="radiogroup"
      aria-label={groupLabel}
      data-setting-composite={entry.key}
      onKeyDown={onGroupKeyDown}
    >
      <button
        type="button"
        role="radio"
        aria-checked={rememberLast}
        tabIndex={rememberLast ? 0 : -1}
        className={
          rememberLast
            ? "settings-values__row settings-values__row--selected"
            : "settings-values__row"
        }
        onClick={() => setValue(SETTING_AUDIO_STARTUP_VOLUME_MODE.key, "remember-last")}
      >
        <span className="settings-values__label">{m("settings.value.audio-startup-volume-mode.remember-last")}</span>
      </button>
      <div
        className={
          rememberLast
            ? "settings-values__row settings-values__row--with-input"
            : "settings-values__row settings-values__row--selected settings-values__row--with-input"
        }
      >
        <button
          type="button"
          role="radio"
          aria-checked={!rememberLast}
          tabIndex={rememberLast ? -1 : 0}
          className="settings-values__radio"
          onClick={() => setValue(SETTING_AUDIO_STARTUP_VOLUME_MODE.key, "always")}
        >
          <span className="settings-values__label">{m("settings.value.audio-startup-volume-mode.always")}</span>
        </button>
        <div className={rememberLast ? "settings-values__input-wrap settings-values__input-wrap--disabled" : "settings-values__input-wrap"}>
          <StartupVolumeInput
            volume={volume}
            label={groupLabel}
            disabled={rememberLast}
            onCommit={(next) => setValue(entry.key, next)}
          />
        </div>
      </div>
    </div>
  );
}
