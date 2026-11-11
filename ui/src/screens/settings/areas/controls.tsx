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
import { Button } from "../../../components/button/Button";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as shellKeys } from "../../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as laneKeys } from "../../../i18n/keys/w26.b07";
import { GENERATED_SCREENS } from "../generated/Screens";
import { HotkeysEditor } from "./hotkeys/HotkeysEditor";
import { localStorageHotkeyOverrides } from "./hotkeys/overrides";

/** Skip Amounts + Mouse — the w26.b02-seeded [[setting]] rows (controls.skip*
 * duration-ms inputs, controls.mouse-* enums + reverse-scroll bool). */
const GeneratedControlsScreen = GENERATED_SCREENS.controls;

type LaneKey = keyof typeof laneKeys;
type ShellKey = keyof typeof shellKeys;
type RegistryKey = keyof typeof registryKeys;
/** Pre-registration seam: registry (b02) + shell (b01) + this lane's keys —
 * identical output once the Lead registers them post-wave (b19). */
const laneCatalog: Readonly<Record<string, string>> = { ...registryKeys, ...shellKeys, ...laneKeys };

/** useAreaMsg — useMsg + the settings lanes' pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | RegistryKey | ShellKey | LaneKey): string =>
    laneCatalog[key] ?? m(key as MessageKey);
}

/**
 * One hand-authored action row (frame §C8: label left, full-width 320×32 r6
 * action button right with a centered label — the standard settings row grid
 * with the Button stretched by the `settings-field__special` wrapper).
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
    <div className="settings-field" data-controls-action={action}>
      <div className="settings-field__row">
        <div className="settings-field__lead">
          <span className="settings-field__label">{label}</span>
        </div>
        <div className="settings-field__control">
          <div className="settings-field__special">
            <Button variant="default" {...buttonRest}>
              {buttonLabel}
            </Button>
          </div>
        </div>
      </div>
    </div>
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
          {t("settings.section.controls.keyboard-shortcuts")}
        </h2>
        <ActionRow
          action="edit-shortcuts"
          label={t("settings.controls.defaults")}
          buttonLabel={t("settings.controls.edit-shortcuts")}
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
      {/* Remotes (frame §C8 y762: "Web remote control" toggle + "Port"
       * input): NO schemas/settings.toml entries exist, and control-server
       * is Phase 5+ [PROPOSAL] with the feature off (product.toml
       * [features]) → rendered as NOTHING, never dead controls that look
       * alive (config-never-invents). Handoff note for the schema owner:
       * docs/waves/w26/logs/w26.b07.md. */}
    </div>
  );
}
