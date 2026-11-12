// Settings area: Extensions — "Add-ons & Scripts"
// (Settings - extensions.svg, analysis tools-settings.md §C13).
//
// w26.b02 SIBLING CONTRACT: this module owns the WHOLE area screen and
// composes, in Figma frame order, its hand-authored section WITH the
// schema-static section generated from the w26.b02 seed:
//   Add-ons (hand — §C13 y83: the add-on LIST well (y110: 782×192 rx5,
//     #090909 fill + 2px #191919 stroke — the settings well family, NOT the
//     surface Well/card; w26.gfix-B, g14-P1-1) + the toolbar row (y311:
//     search 208×34 rx5 in the same well skin, "Find more extensions"
//     185×32, "Add"/"Remove" 100×32 rx6 #191919 — ≈15px labels, g14-P2-1/
//     P2-2). Dynamic data, NOT [[setting]] rows — the same class of surface
//     as the hotkey bindings that live in areas/hotkeys) →
//   "Load add-ons on startup" (SCHEMA — <GENERATED_SCREENS.extensions />;
//     the registry REPLACED the generated record for this id, so the embed
//     below is the seeded row's only on-route render path).
//
// HONEST SURFACE RULES (config-never-invents, the sibling-area discipline):
//   • The add-on list is dynamic data. NO extensions command exists in the
//     command catalog and product.toml [features] wasm-extensions = false
//     (Phase 7, rule F2), so the list seam below is EMPTY — the well
//     renders the honest quiet empty state, never fabricated rows. When
//     the extensions command card lands, that seam becomes the live store
//     (handoff note: docs/waves/w26/findings/w26.g14.md — the b08 log
//     never landed, agent died mid-wave).
//   • The toolbar actions ("Find more extensions" / "Add" / "Remove") have
//     no command to call and no store to mutate: they render
//     disabled-from-data while wasm-extensions is off (the codecs lane's
//     F1/F2 disabled-with-reason pattern — the schema entry's feature gate
//     IS the data), never dead controls that look alive.
//   • The toolbar search is a real local filter over the list data.
//   • Toggle anatomy: the form API's Switch (ON/OFF micro labels). Every
//     settings toggle in the frames measures 68×32 (components/button/
//     states.md) — the task brief's "32×16" figure does not exist in the
//     frames, so the frame-measured form Switch is used.

import { useState } from "react";
import type { ReactElement } from "react";
import { Switch } from "../../../components/form/Switch";
import { TextField } from "../../../components/form/TextField";
import { ListRow } from "../../../components/list/Row";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as laneKeys } from "../../../i18n/keys/w26.b08";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTING_EXTENSIONS_LOAD_ON_STARTUP } from "../generated/schema";
import { useStagedMsg } from "./staged";
import "./extensions.css";

/** Add-ons — the w26.b02-seeded load-on-startup row (gate wasm-extensions). */
const GeneratedExtensionsScreen = GENERATED_SCREENS.extensions;

/** The add-on surface's feature gate (product.toml → the generated schema). */
const ADDONS_GATE = SETTING_EXTENSIONS_LOAD_ON_STARTUP;

/** One installed add-on — dynamic data, never [[setting]] rows. */
export interface ExtensionInfo {
  readonly id: string;
  /** Row label (also the enable Switch's accessible name). */
  readonly name: string;
  /** Right-aligned readout in the ListRow tabular slot. */
  readonly version: string;
  readonly enabled: boolean;
}

/**
 * The add-on list seam — EMPTY today, honestly: no extensions command
 * exists in the catalog and wasm-extensions is not exposed (Phase 7, rule
 * F2), so there is no data to list. Never fabricated rows; the well renders
 * the quiet empty state until the extensions command card feeds this seam
 * (handoff note: docs/waves/w26/logs/w26.b08.md).
 */
const INSTALLED_EXTENSIONS: readonly ExtensionInfo[] = [];

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
/** Pre-registration seam: registry (b02) + this lane's keys — identical
 * output once the Lead registers them post-wave (b19). */
const laneCatalog: Readonly<Record<string, string>> = { ...registryKeys, ...laneKeys };

/** useAreaMsg — useMsg + the settings lanes' pre-registration keys. */
function useAreaMsg() {
  const m = useMsg();
  return (key: MessageKey | RegistryKey | LaneKey): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

/**
 * One add-on row — name + version left, enable Switch right (the settings
 * label-left/control-right grammar). Anatomy is [VERIFY]: §C13's list well
 * exports no rows, so this composes the shared ListRow grammar (compact
 * density, tabular version slot) + the form Switch's ON/OFF toggle.
 */
export function ExtensionRow({
  info,
  onToggle,
}: {
  info: ExtensionInfo;
  /** Enable/disable — wired to the extensions store when it exists. */
  onToggle: (enabled: boolean) => void;
}): ReactElement {
  return (
    <ListRow
      density="compact"
      title={info.name}
      duration={info.version}
      data-extension={info.id}
      trailing={
        <Switch aria-label={info.name} checked={info.enabled} onChange={onToggle} />
      }
    />
  );
}

/**
 * Extensions settings area (route /settings/extensions; System group): the
 * hand add-on list + toolbar, then the schema-seeded load-on-startup row
 * via the generated screen.
 *
 * w26.i15 relaunch (h12 #2/#3/#4): the toolbar renders STAGED — full-
 * strength while inert (the [data-staged] action rule in staged.css), the
 * honest reason on each button's tooltip; the invented 11px gate note and
 * the invented empty-state copy are gone (content the §C13 frame does not
 * carry — the frame's well is empty, its toolbar lively); the well and the
 * toolbar search ride the recessed settings-input family in BOTH themes
 * (extensions.css). The wrapper no longer carries data-settings-area — the
 * embedded generated screen's root is the area's single root (g14-P3-4).
 */
export function ExtensionsArea(): ReactElement {
  const t = useAreaMsg();
  const staged = useStagedMsg()("settings.extensions.staged");
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const visible =
    needle === ""
      ? INSTALLED_EXTENSIONS
      : INSTALLED_EXTENSIONS.filter(
          (info) =>
            info.name.toLowerCase().includes(needle) ||
            info.version.toLowerCase().includes(needle),
        );
  const actionsDisabled = !ADDONS_GATE.featureExposed;

  return (
    <div
      className="settings-sections"
      data-testid="extensions-area"
    >
      <section className="settings-section" data-section="add-ons">
        <h2 className="settings-section__title">
          {t("settings.section.extensions.add-ons")}
        </h2>
        {/* The frame's list well (§C13: 782×192 rx5 #090909 + 2px #191919
            stroke, EMPTY in the frame — w26.gfix-B, g14-P1-1; w26.i15: the
            invented empty-state copy is gone, the well renders exactly the
            frame's empty surface). Rows render from the (empty) list seam. */}
        <div className="settings-extensions-list" data-testid="extensions-list">
          {visible.map((info) => (
            <ExtensionRow
              key={info.id}
              info={info}
              onToggle={() => {
                /* inert by design: the extensions store does not exist
                 * yet (Phase 7) — the list seam above owns the handoff. */
              }}
            />
          ))}
        </div>
        {/* Toolbar (§C13 y311): search left (208×34, the settings input
            well skin — g14-P2-1), the three frame actions right-aligned to
            the content edge with 10px gaps (frame x619/x814/x924; ≈15px
            labels, min-width 185px on the primary action — g14-P2-2).
            w26.i15 (h12 #4): staged full-strength while wasm-extensions is
            off — the honest reason rides each tooltip, never a greyed-out
            apology. */}
        <div className="settings-extensions-toolbar" data-testid="extensions-toolbar">
          <TextField
            type="search"
            label={t("settings.extensions.search")}
            hideLabel
            placeholder={t("settings.extensions.search")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="settings-extensions-toolbar__actions">
            <button
              type="button"
              className="settings-action settings-extensions-toolbar__find"
              disabled={actionsDisabled}
              data-staged="wasm-extensions"
              title={staged}
              onClick={() => {
                /* inert by design: no discovery surface exists yet */
              }}
            >
              {t("settings.extensions.find-more")}
            </button>
            <button
              type="button"
              className="settings-action"
              disabled={actionsDisabled}
              data-staged="wasm-extensions"
              title={staged}
              onClick={() => {
                /* inert by design: no install command exists yet */
              }}
            >
              {t("settings.extensions.add")}
            </button>
            <button
              type="button"
              className="settings-action"
              disabled={actionsDisabled}
              data-staged="wasm-extensions"
              title={staged}
              onClick={() => {
                /* inert by design: no removal command exists yet */
              }}
            >
              {t("settings.extensions.remove")}
            </button>
          </div>
        </div>
      </section>
      {/* "Load add-ons on startup" — schema-seeded, rendered by the
          generated screen (feature-gated wasm-extensions → hidden + inert
          until the feature ships; this embed is the seeded row's only
          on-route render path). KNOWN Phase-7 debt: when the gate flips,
          the generated screen renders its own "Add-ons" section header
          below this hand one — fold the row into the hand section
          (SettingField direct, the codecs pattern) or re-seed placement;
          see docs/waves/w26/logs/w26.b08.md. */}
      <GeneratedExtensionsScreen />
    </div>
  );
}
