// Staged rows — the shared honest-empty surface for the schema-blocked
// settings areas (w26.i15, closing h11/h12): the Figma frames (Settings -
// library/network/recording/extensions.svg) carry real rows; no [[setting]]
// keys exist behind them yet (schemas/settings.toml is the Lead's — see the
// lane log for the exact rows needed). Per the wave-I fix directive the
// frames' rows render in the frame's own visual grammar, STAGED INERT — the
// Remotes precedent (w26.gfix-B g13-P2-8, pixel-verified by h11): visible,
// full-strength, disabled, marked `data-staged`, the honest tooltip on the
// row — never a dead control that looks alive, never invented rows.
//
// w26.i15 (vision loop 2, the §C0 "optional ⓘ"): where the frame's row
// carries the r7.5 #404040 info circle (library 7/7, network 5/9, recording
// 4/10 — SVG circle census), the staged row renders it through i12's
// SettingInfoDot grammar, OPERABLE like i14's subtitles ⓘ: aria-expanded +
// the .settings-field__hinttext reveal carries the staged reason, so
// keyboard/AT users reach it without a hover. The dimmed rows ride the
// frame's own @0.3 group, ⓘ included.
//
// w26.k13 (j13 P2-2, staged-reason keyboard reachability): the reason on
// ⓘ-less rows rode ONLY a `title` — on a non-focusable div or on a disabled
// button — so a keyboard/AT user could never learn why the control is dead.
// Every staged row now also carries the reason as a visually hidden note in
// the row's lead (`.settings-vh`, announced in browse mode; the SettingField
// Browse-companion precedent) and the disabled controls point at it through
// `aria-describedby`. The mouse tooltip stays (title); nothing visible
// changes — the frames carry no note line and none is invented.
//
// Two presentations:
//   • staged rows — full-strength disabled controls (settings.css already
//     un-dims form controls under [data-staged]; staged.css extends the same
//     grammar to actions and the Advanced bar);
//   • dimmed rows — the frames themselves ship some rows as disabled groups
//     @ opacity 0.3 (§C0 control inventory: "Disabled row — library/network"):
//     those rows add `data-staged-dimmed` (var(--opacity-muted) = 0.3).
//
// When the schema keys land, E1's generated screens render the rows live and
// these staged compositions yield (see each area file).

import { useId, useState } from "react";
import type { ReactElement } from "react";
import { SettingInfoDot } from "../SettingField";
import { Dropdown } from "../../../components/menu/Dropdown";
import { Icon } from "../../../components/icon/Icon";
import { Switch } from "../../../components/form/Switch";
import { TextField } from "../../../components/form/TextField";
import { keys as stagedKeys } from "../../../i18n/keys/w26.i15";
import { makeAreaMsg } from "./shared";
import "./staged.css";

/** useStagedMsg — useMsg + this lane's pre-registration keys (§4.5 seam;
 * w26.k13: the shared makeAreaMsg factory — the j13 ruling 1 dedup). */
export const useStagedMsg = makeAreaMsg(stagedKeys);

/** The staged control variants (one per §C0 control-inventory row kind). */
export type StagedControl =
  | { kind: "switch"; on: boolean }
  | { kind: "text"; value: string }
  | { kind: "enum"; value: string }
  | { kind: "path"; value: string; browse: string };

/**
 * One staged row — the standard settings row grammar (label left, control
 * right in the 320px column) with the control rendered inert at full
 * strength. `dimmed` marks the frame's own disabled-row groups (@0.3).
 * `info` renders the frame's ⓘ affordance (SettingInfoDot), operable —
 * the reveal carries the staged reason.
 */
export function StagedRow({
  rowKey,
  label,
  control,
  seam,
  reason,
  dimmed = false,
  info = false,
}: {
  /** Row marker for tests/inspection (data-staged-row). */
  rowKey: string;
  label: string;
  control: StagedControl;
  /** The awaited seam id (data-staged) — the schema seed / feature gate. */
  seam: string;
  /** The honest tooltip (the row's title). */
  reason: string;
  /** Frame-disabled row group @0.3 (§C0). */
  dimmed?: boolean;
  /** Frame carries the ⓘ circle on this row (§C0 "optional ⓘ"). */
  info?: boolean;
}): ReactElement {
  // w26.i15: operable ⓘ — closed by default (the 815px fit is untouched
  // until the user asks); the reveal carries the staged reason.
  const [hintOpen, setHintOpen] = useState(false);
  const hintCaptionId = useId();
  // w26.k13 (j13 P2-2): the visually hidden reason twin for ⓘ-less rows —
  // the `title` alone is unreachable for keyboard/AT users (it sits on a
  // non-focusable row div and on disabled controls). The id is referenced
  // by the disabled control's aria-describedby where the control accepts it.
  const reasonNoteId = `${rowKey}-staged-reason`;
  const built: ReactElement =
    control.kind === "switch" ? (
      <Switch
        aria-label={label}
        aria-describedby={reasonNoteId}
        checked={control.on}
        disabled
        onChange={() => undefined}
      />
    ) : control.kind === "text" ? (
      <TextField label={label} hideLabel value={control.value} disabled onChange={() => undefined} />
    ) : control.kind === "enum" ? (
      <Dropdown
        className="settings-field__dropdown"
        density="compact"
        label={label}
        options={[{ value: control.value, labelKey: control.value }]}
        // w26.k13 (j13 P3-12): the option's labelKey carries ALREADY-RESOLVED
        // copy — the identity resolver, made explicit instead of relying on
        // the documented @default identity fallback.
        resolveLabel={(value) => value}
        value={control.value}
        disabled
        onChange={() => undefined}
      />
    ) : (
      <div className="settings-field__path">
        <TextField label={label} hideLabel value={control.value} disabled onChange={() => undefined} />
        <button
          type="button"
          className="settings-action"
          disabled
          aria-describedby={reasonNoteId}
          title={reason}
        >
          {control.browse}
        </button>
      </div>
    );
  return (
    <div
      className="settings-field"
      data-kind={control.kind}
      data-staged={seam}
      data-staged-dimmed={dimmed || undefined}
      data-staged-row={rowKey}
      title={reason}
    >
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label" aria-hidden="true">
            {label}
          </span>
          {info ? (
            <SettingInfoDot
              note={reason}
              label={reason}
              expanded={hintOpen}
              onToggle={() => setHintOpen((open) => !open)}
              controlsId={hintCaptionId}
            />
          ) : null}
          {/* w26.k13 (j13 P2-2): the sr-only reason — keyboard/AT users can
           * read why the row is inert (the ⓘ rows reach it through their own
           * operable reveal + the dot's description twin). */}
          {!info ? (
            <span id={reasonNoteId} className="settings-vh">
              {reason}
            </span>
          ) : null}
        </div>
        <div className="settings-field__control">{built}</div>
      </div>
      {info && hintOpen ? (
        <p id={hintCaptionId} role="note" className="settings-field__hinttext">
          {reason}
        </p>
      ) : null}
    </div>
  );
}

/**
 * The staged "Advanced Options" bar — the frames close every one of these
 * areas with the collapsed disclosure (784×38 r2, › chevron); with no
 * advanced [[setting]] rows behind it the bar renders inert at full
 * strength, exactly as collapsed, never a disclosure that opens to nothing.
 *
 * w26.k13 (j13 P2-2): the reason rides the disabled bar's sr-only twin too
 * (aria-describedby) — a disabled button is not focusable, so the title
 * alone was keyboard-unreachable.
 */
export function StagedAdvancedBar({ reason, label }: { reason: string; label: string }): ReactElement {
  const reasonNoteId = useId();
  return (
    <div className="settings-advanced" data-staged="schema-seed" data-testid="settings-advanced-staged" title={reason}>
      <button
        type="button"
        className="settings-advanced__bar"
        disabled
        aria-describedby={reasonNoteId}
        title={reason}
      >
        <span className="settings-advanced__label">{label}</span>
        <Icon name="chevron-right" size={14} className="settings-advanced__chevron" />
      </button>
      <span id={reasonNoteId} className="settings-vh">
        {reason}
      </span>
    </div>
  );
}
