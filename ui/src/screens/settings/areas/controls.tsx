// Settings area: Controls — "Mouse, Keyboard & Remote Controls"
// (Settings - controls.svg, analysis tools-settings.md §C8).
//
// w26.b02 SIBLING CONTRACT: this module owns the WHOLE area screen and
// composes, in Figma frame order, its hand-authored sections WITH the
// schema-static sections generated from the w26.b02 seed:
//   Keyboard Shortcuts (hand — the hotkeys fold; the retired /settings/
//     hotkeys slug redirects here, so the HotkeysEditor is this area's edit
//     surface) → Skip Amounts + Mouse (SCHEMA — <GENERATED_SCREENS.controls
//     />; the registry REPLACED the generated record for this id, so the
//     embed below is the seeded rows' only on-route render path) →
//   Remotes (hand — see the honest-surface note at the bottom).
//
// HONEST SURFACE RULES (config-never-invents, same discipline as the sibling
// areas): the two Keyboard Shortcuts actions are real behaviors (open the
// hotkeys editor / clear the overrides store), never config options; every
// option row renders through the schema (the generated screen) or renders
// NOTHING.

import { useState } from "react";
import type { ButtonHTMLAttributes } from "react";
import type { ReactElement } from "react";
import { Switch } from "../../../components/form/Switch";
import { TextField } from "../../../components/form/TextField";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as shellKeys } from "../../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as laneKeys } from "../../../i18n/keys/w26.b07";
import { keys as fixerKeys } from "../../../i18n/keys/w26.gfixB";
import { GENERATED_SCREENS } from "../generated/Screens";
import { HotkeysEditor } from "./hotkeys/HotkeysEditor";
import { localStorageHotkeyOverrides } from "./hotkeys/overrides";

/** Skip Amounts + Mouse — the w26.b02-seeded [[setting]] rows (controls.skip*
 * duration-ms inputs, controls.mouse-* enums + reverse-scroll bool). */
const GeneratedControlsScreen = GENERATED_SCREENS.controls;

type LaneKey = keyof typeof laneKeys;
type ShellKey = keyof typeof shellKeys;
type RegistryKey = keyof typeof registryKeys;
type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam: registry (b02) + shell (b01) + b07 + this lane's
 * keys — the gfixB casing fixes (g13-P2-9: "Keyboard Shortcuts" / "Edit
 * Keyboard Shortcuts", frame-verbatim, consistent with the sibling title
 * case sections) win over b07's sentence-case exports until the Lead
 * registers the retitle. */
const laneCatalog: Readonly<Record<string, string>> = {
  ...registryKeys,
  ...shellKeys,
  ...laneKeys,
  ...fixerKeys,
};

/** useAreaMsg — useMsg + the settings lanes' pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | RegistryKey | ShellKey | LaneKey | FixerKey): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

/**
 * One hand-authored action row (frame §C8: label left, full-width 320×32 r6
 * action button right with a centered label — the standard settings row
 * grid; the flat #191919 settings-action skin, g13-P3-10).
 */
function ActionRow({
  action,
  label,
  buttonLabel,
  ...buttonRest
}: {
  /** Row marker for tests/inspection — hand action, not a schema setting. */
  action: "edit-shortcuts" | "reset-shortcuts";
  label: string;
  buttonLabel: string;
} & ButtonHTMLAttributes<HTMLButtonElement>): ReactElement {
  return (
    <div className="settings-field" data-kind="action" data-controls-action={action}>
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
        </div>
        <div className="settings-field__control">
          <button type="button" className="settings-action settings-action--wide" {...buttonRest}>
            {buttonLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * The Remotes section (frame §C8 y762: "Web remote control" → OFF · "Port"
 * → input, empty). STAGED inert rows — the subtitles-area pattern (w26.f04):
 * no controls.web-remote / controls.port [[setting]] keys exist and the
 * control server is Phase 5+ [PROPOSAL] (product.toml [features]
 * control-server = false), so the frame-verbatim rows render inert with the
 * honest staged note — visible, full-strength, never a dead control that
 * looks alive (g13-P2-8).
 */
function RemotesSection(): ReactElement {
  const t = useAreaMsg();
  const staged = t("settings.controls.remotes-staged");
  const rows: ReadonlyArray<{
    key: string;
    label: string;
    kind: "switch" | "text";
  }> = [
    { key: "controls.web-remote", label: t("settings.controls.web-remote"), kind: "switch" },
    { key: "controls.port", label: t("settings.controls.port"), kind: "text" },
  ];
  return (
    <section className="settings-section" data-section="remotes">
      <h2 className="settings-section__title">{t("settings.section.controls.remotes")}</h2>
      {rows.map((row) => (
        <div
          key={row.key}
          className="settings-field"
          data-kind={row.kind}
          data-setting-key={`pending.${row.key}`}
          data-staged="control-server"
          title={staged}
        >
          <div className="settings-field__row">
            <div className="settings-field__lead">
              <span className="settings-field__label">{row.label}</span>
            </div>
            <div className="settings-field__control">
              {row.kind === "switch" ? (
                <Switch
                  aria-label={row.label}
                  checked={false}
                  disabled
                  onChange={() => undefined}
                />
              ) : (
                <TextField label={row.label} hideLabel value="" disabled onChange={() => undefined} />
              )}
            </div>
          </div>
        </div>
      ))}
      <p className="settings-area-staged-note" role="note">
        {staged}
      </p>
    </section>
  );
}

/**
 * Controls settings area (route /settings/controls; hosts the retired
 * hotkeys area's editor as its Keyboard Shortcuts fold).
 */
export function ControlsArea(): ReactElement {
  const t = useAreaMsg();
  const [hotkeysOpen, setHotkeysOpen] = useState(false);
  // Remount epoch: "Reset" clears the overrides store, and the editor (which
  // loads the store on mount) re-reads the cleared state — every binding
  // falls back to its keymap default.
  const [hotkeysEpoch, setHotkeysEpoch] = useState(0);

  const resetAllShortcuts = (): void => {
    localStorageHotkeyOverrides.save({});
    setHotkeysEpoch((epoch) => epoch + 1);
  };

  return (
    <div className="settings-sections" data-settings-area="controls" data-testid="controls-area">
      <section className="settings-section" data-section="keyboard-shortcuts">
        <h2 className="settings-section__title">
          {t("settings.section.controls.shortcuts")}
        </h2>
        <ActionRow
          action="edit-shortcuts"
          label={t("settings.controls.defaults")}
          buttonLabel={t("settings.controls.edit-shortcuts-action")}
          aria-expanded={hotkeysOpen}
          aria-controls="controls-hotkeys-fold"
          onClick={() => setHotkeysOpen((open) => !open)}
        />
        <ActionRow
          action="reset-shortcuts"
          label={t("settings.controls.reset-all-shortcuts")}
          buttonLabel={t("settings.controls.reset-shortcuts")}
          onClick={resetAllShortcuts}
        />
        {hotkeysOpen ? (
          <div id="controls-hotkeys-fold" data-testid="controls-hotkeys-fold">
            {/* The hotkeys edit surface (w26 merge: hotkeys → controls). The
                editor owns its bindings table, per-binding reset and its own
                apply bar; it re-mounts on epoch change so "Reset" takes
                effect immediately. */}
            <HotkeysEditor key={hotkeysEpoch} />
          </div>
        ) : null}
      </section>
      {/* Skip Amounts + Mouse — schema-seeded, rendered by the generated
          screen (its own section headers/rows; search + feature gates apply
          inside it as on every generated area). */}
      <GeneratedControlsScreen />
      {/* Remotes (frame §C8 y762) — staged inert rows while the control
          server is Phase 5+ (see RemotesSection above; g13-P2-8). */}
      <RemotesSection />
    </div>
  );
}
