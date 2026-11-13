// Settings area: Subtitles — `Settings - subtitles.svg` (analysis
// tools-settings.md §C7; w26.e07 F1). The frame carries 3 sections / 14
// rows; `Settings - subtitles-1.svg` is a byte-duplicate export (e07 frame
// facts) — no second state to reproduce.
//
// w26.f04 COMPOSITION (frame truth, top → bottom):
//   Tracks & Languages — Enable subtitles (Switch, frame ON) · Preferred
//     language (Dropdown "System language") · Text encoding (Dropdown
//     "Automatic")
//   Loading Subtitles — Load subtitle files automatically (Switch, frame
//     ON) · File name matching (Dropdown "Title with extra characters")
//   Appearance — Font (Dropdown "System, sans-serif") · Size values
//     (SegmentedControl Relative | Pixels, frame: Relative active) · Font
//     size (Dropdown "Automatic") · Bold (Switch, frame OFF) · Text color
//     (hex field "#FFFFFF" + 22×22 swatch) · Text opacity (Slider at 50% +
//     0/50/100% scale) · Outline thickness (Dropdown "Normal") · Outline
//     color (hex field "#000000" + swatch) · Shadow (Switch, frame ON).
//   The frame carries NO subtitle-appearance preview element (e07 frame
//   facts) — none is rendered. ⓘ info glyphs follow the 7 labels the frame
//   marks (Tracks & Languages ×3, Loading ×2, Font + Size values).
//
// HONEST SURFACE RULES (config-never-invents, the effects-area staged
// pattern — visible, inert, never a dead control that looks alive):
//   • NO `subtitles`-screen [[setting]] rows exist in schemas/settings.toml
//     (the b02 seed grew controls/recording/extensions only), so the
//     SettingsProvider carries ZERO subtitle keys and every row here is a
//     STAGED hand row: control rendered DISABLED with the frame-verbatim
//     value, `data-staged="subtitles-settings-seed"` +
//     `data-setting-key="pending.<future-key>"` markers, and one quiet
//     honest note per section naming the awaited seam.
//   • The schema's two subtitle-ish keys — playback.preferred-subtitle
//     -languages + playback.subtitle-mode — are placed on the PLAYBACK
//     screen (doc 07 §5.2 D-rules): they stay there, never defined twice.
//     This area's "Enable subtitles" / "Preferred language" rows await
//     their own `subtitles.*` placements.
//   • The command catalog's `subtitle.add-file` / `subtitle.set-sync` /
//     `subtitle.set-encoding` / `subtitle.set-scale` are SESSION playback
//     commands (StateSnapshot SubtitleSettings) — no persisted-default seam
//     and no enumeration command (inventing font/encoding lists would
//     violate MP §5), so no row binds them. Zero transport calls (asserted
//     in subtitles.test.tsx).
//   • When the schema owner seeds `subtitles.*` entries, E1's generated
//     screen takes the rows over (same migration note as audio.tsx) and
//     these staged rows retire. Handoff note:
//     docs/waves/w26/logs/w26.f04.md.

import type { CSSProperties, ReactElement } from "react";
import { useId, useState } from "react";
import { Switch } from "../../../components/form";
import { SegmentedControl } from "../../../components/form";
import { TextField } from "../../../components/form";
import { Dropdown } from "../../../components/menu";
// Direct-module import (the ShortcutsScreen/ConvertScreen pattern): the
// area only needs the base Slider, not the slider barrel's EQ tooling.
import { Slider } from "../../../components/slider/Slider";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.f04";
import { SettingInfoDot } from "../SettingField";
import { makeAreaMsg } from "./shared";
import "./subtitles.css";

type LaneKey = keyof typeof laneKeys;

/** Keys resolvable by `useSubtitlesMsg` (the w26.c1 useEqMsg seam-first
 * pattern: central catalog wins once the Lead registers the keys, this
 * lane file is the fallback reference). */
export type SubtitlesLabelKey = MessageKey | LaneKey;

/** w26.k13: the shared makeAreaMsg factory (the j13 ruling 1 dedup). */
export const useSubtitlesMsg = makeAreaMsg(laneKeys);

/** One staged row shell — label (+ optional ⓘ) left, control right in the
 * 320px column, the frame's row grammar (settings.css owns the classes).
 * w26.gfix-B (g13-P2-3): rows render FULL-STRENGTH while inert (the
 * frame's live-looking controls; the honesty lives in the disabled state +
 * the ⓘ note), and carry data-kind so the row pitch follows the frame's
 * per-control heights (g13-P1-1).
 * w26.k13 (j13 P2-3 + ruling 1): the ⓘ is the SHARED SettingInfoDot
 * button (the local 14px twin is gone — one ⓘ grammar across the areas),
 * and every staged row carries the shared data-staged-row marker; the
 * reason on ⓘ-less rows rides a visually hidden note in the lead (j13
 * P2-2 — keyboard/AT users can read why the row is inert). */
function StagedRow({
  labelKey,
  pendingKey,
  kind,
  info = false,
  note,
  children,
}: {
  labelKey: SubtitlesLabelKey;
  /** Future schemas/settings.toml key this row awaits (data marker). */
  pendingKey: string;
  /** Row control kind (frame pitch: switches 42, 36px controls 46). */
  kind: "switch" | "enum" | "text" | "slider" | "color";
  info?: boolean;
  note: string;
  children: ReactElement;
}): ReactElement {
  const m = useSubtitlesMsg();
  const label = m(labelKey);
  // w26.i14: the ⓘ is operable — it toggles the honest note under the row
  // (aria-expanded + the hint line), so keyboard/AT users reach the staged
  // reason without a mouse hover. Closed by default: the 815px single-view
  // fit (g13-P1-1) is untouched until the user asks.
  const [hintOpen, setHintOpen] = useState(false);
  const hintCaptionId = useId();
  const reasonNoteId = `${pendingKey}-staged-reason`;
  return (
    <div
      className="settings-field"
      data-kind={kind}
      data-setting-key={`pending.${pendingKey}`}
      data-staged="subtitles-settings-seed"
      data-staged-row={`subtitles.${pendingKey}`}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
          {info ? (
            <SettingInfoDot
              note={note}
              label={note}
              expanded={hintOpen}
              onToggle={() => setHintOpen((open) => !open)}
              controlsId={hintCaptionId}
            />
          ) : (
            <span id={reasonNoteId} className="settings-vh">
              {note}
            </span>
          )}
        </div>
        <div className="settings-field__control">{children}</div>
      </div>
      {info && hintOpen ? (
        <p id={hintCaptionId} role="note" className="settings-field__hinttext">
          {note}
        </p>
      ) : null}
    </div>
  );
}

/** A staged toggle row — the frame's ON/OFF state, inert (the effects-area
 * pattern: disabled means never fired). */
function StagedToggleRow({
  labelKey,
  checked,
  pendingKey,
  info = false,
  note,
}: {
  labelKey: SubtitlesLabelKey;
  checked: boolean;
  pendingKey: string;
  info?: boolean;
  note: string;
}): ReactElement {
  const m = useSubtitlesMsg();
  const label = m(labelKey);
  return (
    <StagedRow labelKey={labelKey} pendingKey={pendingKey} kind="switch" info={info} note={note}>
      <Switch
        checked={checked}
        onChange={() => {
          /* staged — disabled means never fired */
        }}
        disabled
        aria-label={label}
      />
    </StagedRow>
  );
}

/** A staged dropdown row — one honest frame-verbatim option (the frame
 * exports no open list; no invented engine modes). */
function StagedDropdownRow({
  labelKey,
  optionKey,
  pendingKey,
  info = false,
  note,
}: {
  labelKey: SubtitlesLabelKey;
  optionKey: SubtitlesLabelKey;
  pendingKey: string;
  info?: boolean;
  note: string;
}): ReactElement {
  const m = useSubtitlesMsg();
  const label = m(labelKey);
  return (
    <StagedRow labelKey={labelKey} pendingKey={pendingKey} kind="enum" info={info} note={note}>
      <Dropdown
        options={[{ value: optionKey, labelKey: optionKey }]}
        value={optionKey}
        onChange={() => {
          /* staged — disabled means never fired */
        }}
        label={label}
        resolveLabel={m}
        density="compact"
        disabled
      />
    </StagedRow>
  );
}

/** A staged text row — the frame's input well (no chevron; g13-P2-2: the
 * Font row is a text input, not a dropdown). */
function StagedTextRow({
  labelKey,
  valueKey,
  pendingKey,
  info = false,
  note,
}: {
  labelKey: SubtitlesLabelKey;
  valueKey: SubtitlesLabelKey;
  pendingKey: string;
  info?: boolean;
  note: string;
}): ReactElement {
  const m = useSubtitlesMsg();
  const label = m(labelKey);
  return (
    <StagedRow labelKey={labelKey} pendingKey={pendingKey} kind="text" info={info} note={note}>
      <TextField label={label} hideLabel value={m(valueKey)} disabled onChange={() => undefined} />
    </StagedRow>
  );
}

/** A staged segmented row — the frame's active segment, inert. */
function StagedSegmentedRow({
  labelKey,
  options,
  value,
  pendingKey,
  info = false,
  note,
}: {
  labelKey: SubtitlesLabelKey;
  options: ReadonlyArray<{ value: string; labelKey: SubtitlesLabelKey }>;
  value: string;
  pendingKey: string;
  info?: boolean;
  note: string;
}): ReactElement {
  const m = useSubtitlesMsg();
  const label = m(labelKey);
  return (
    <StagedRow labelKey={labelKey} pendingKey={pendingKey} kind="enum" info={info} note={note}>
      <SegmentedControl
        label={label}
        options={options.map((option) => ({ value: option.value, label: m(option.labelKey) }))}
        value={value}
        onChange={() => {
          /* staged — disabled means never fired */
        }}
        disabled
      />
    </StagedRow>
  );
}

/** A staged color row — the frame's hex field (320×32 r6 well) + 22×22
 * swatch; the value is DATA (the frame default), not a theme color, so it
 * flows from the lane catalog through a custom property. */
function StagedColorRow({
  labelKey,
  valueKey,
  pendingKey,
  note,
}: {
  labelKey: SubtitlesLabelKey;
  valueKey: LaneKey;
  pendingKey: string;
  note: string;
}): ReactElement {
  const m = useSubtitlesMsg();
  const value = m(valueKey);
  return (
    <StagedRow labelKey={labelKey} pendingKey={pendingKey} kind="color" note={note}>
      <div className="settings-area-subtitles-color" data-staged-value={value}>
        <span className="settings-area-subtitles-color-hex">{value}</span>
        <span
          className="settings-area-subtitles-color-swatch"
          aria-hidden="true"
          style={{ "--settings-area-subtitles-swatch": value } as CSSProperties}
        />
      </div>
    </StagedRow>
  );
}

/** The frame's 9 degree ticks (10%–90% step 10%, 2×6 marks — dynamics.css
 * tick grammar, ink #1A1611 ≈ surface-muted). */
const OPACITY_TICKS: readonly number[] = [10, 20, 30, 40, 50, 60, 70, 80, 90];

/** A staged opacity row — slider at the frame's 50% + the 0/50/100% scale
 * labels (values, formatDb-style — not copy). */
function StagedOpacityRow({
  labelKey,
  pendingKey,
  note,
}: {
  labelKey: SubtitlesLabelKey;
  pendingKey: string;
  note: string;
}): ReactElement {
  const m = useSubtitlesMsg();
  const label = m(labelKey);
  const percent = (value: number): string => `${value}%`;
  return (
    <StagedRow labelKey={labelKey} pendingKey={pendingKey} kind="slider" note={note}>
      <div className="settings-area-subtitles-opacity">
        <div className="settings-area-subtitles-opacity-track">
          <Slider
            value={50}
            min={0}
            max={100}
            disabled
            aria-label={label}
            formatValue={percent}
          />
          <div className="settings-area-subtitles-opacity-ticks" aria-hidden="true">
            {OPACITY_TICKS.map((tick) => (
              <span
                key={tick}
                className="settings-area-subtitles-opacity-tick"
                style={{ left: `${tick}%` } as CSSProperties}
              />
            ))}
          </div>
        </div>
        <div className="settings-area-subtitles-opacity-scale" aria-hidden="true">
          <span>{percent(0)}</span>
          <span>{percent(50)}</span>
          <span>{percent(100)}</span>
        </div>
      </div>
    </StagedRow>
  );
}

/**
 * Subtitles settings area (route /settings/subtitles; Media group) — the
 * frame's 3 sections / 14 rows, all staged per the module header (no
 * subtitles-screen schema entries exist yet; the seeded schema rows render
 * the moment the settings seed lands).
 */
export function SubtitlesArea(): ReactElement {
  const m = useSubtitlesMsg();
  const staged = m("settings.subtitles.staged");
  return (
    <div
      className="settings-sections settings-area-subtitles"
      data-settings-area="subtitles"
      data-testid="subtitles-area"
    >
      <section className="settings-section" data-section="tracks-languages">
        <h2 className="settings-section__title">
          {m("settings.section.subtitles.tracks-languages")}
        </h2>
        {/* w26.gfix-B (g13-P1-1): the per-section staged notes are GONE — the
            honesty moved into the 7 ⓘ titles (the frame carries no notes;
            the rows must fit the 865px window). */}
        <StagedToggleRow
          labelKey="settings.subtitles.enable"
          checked
          pendingKey="subtitles.enable"
          info
          note={staged}
        />
        <StagedDropdownRow
          labelKey="settings.subtitles.preferred-language"
          optionKey="settings.subtitles.preferred-language.system"
          pendingKey="subtitles.preferred-language"
          info
          note={staged}
        />
        <StagedDropdownRow
          labelKey="settings.subtitles.text-encoding"
          optionKey="settings.subtitles.encoding.automatic"
          pendingKey="subtitles.text-encoding"
          info
          note={staged}
        />
      </section>

      <section className="settings-section" data-section="loading">
        <h2 className="settings-section__title">{m("settings.section.subtitles.loading")}</h2>
        <StagedToggleRow
          labelKey="settings.subtitles.auto-load"
          checked
          pendingKey="subtitles.auto-load"
          info
          note={staged}
        />
        <StagedDropdownRow
          labelKey="settings.subtitles.file-name-matching"
          optionKey="playlist.subtitles.match.extra"
          pendingKey="subtitles.file-name-matching"
          info
          note={staged}
        />
      </section>

      <section className="settings-section" data-section="appearance">
        <h2 className="settings-section__title">
          {m("settings.section.subtitles.appearance")}
        </h2>
        {/* w26.gfix-B (g13-P2-2): Font is the frame's TEXT input well (no
            chevron), not a dropdown. */}
        <StagedTextRow
          labelKey="settings.subtitles.font"
          valueKey="settings.subtitles.font.system-sans-serif"
          pendingKey="subtitles.font"
          info
          note={staged}
        />
        <StagedSegmentedRow
          labelKey="settings.subtitles.size-values"
          options={[
            { value: "relative", labelKey: "settings.subtitles.size-values.relative" },
            { value: "pixels", labelKey: "settings.subtitles.size-values.pixels" },
          ]}
          value="relative"
          pendingKey="subtitles.size-units"
          info
          note={staged}
        />
        <StagedDropdownRow
          labelKey="settings.subtitles.font-size"
          optionKey="settings.subtitles.font-size.automatic"
          pendingKey="subtitles.font-size"
          note={staged}
        />
        <StagedToggleRow
          labelKey="settings.subtitles.bold"
          checked={false}
          pendingKey="subtitles.bold"
          note={staged}
        />
        <StagedColorRow
          labelKey="settings.subtitles.text-color"
          valueKey="settings.subtitles.text-color.value"
          pendingKey="subtitles.text-color"
          note={staged}
        />
        <StagedOpacityRow
          labelKey="settings.subtitles.text-opacity"
          pendingKey="subtitles.text-opacity"
          note={staged}
        />
        <StagedDropdownRow
          labelKey="settings.subtitles.outline-thickness"
          optionKey="settings.subtitles.outline-thickness.normal"
          pendingKey="subtitles.outline-thickness"
          note={staged}
        />
        <StagedColorRow
          labelKey="settings.subtitles.outline-color"
          valueKey="settings.subtitles.outline-color.value"
          pendingKey="subtitles.outline-color"
          note={staged}
        />
        <StagedToggleRow
          labelKey="settings.subtitles.shadow"
          checked
          pendingKey="subtitles.shadow"
          note={staged}
        />
      </section>
    </div>
  );
}
