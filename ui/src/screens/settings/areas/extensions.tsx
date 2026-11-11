// Settings area: Extensions — "Add-ons & Scripts"
// (Settings - extensions.svg, analysis tools-settings.md §C13).
//
// w26.b02 SIBLING CONTRACT: this module owns the WHOLE area screen and
// composes, in Figma frame order, its hand-authored section WITH the
// schema-static section generated from the w26.b02 seed:
//   Add-ons (hand — §C13 y83: the add-on LIST well (y110: 782×192 rx5,
//     empty in the frame — no rows are exported) + the toolbar row (y311:
//     search 208×34 rx5, "Find more extensions" 185×32, "Add"/"Remove"
//     100×32 rx6, right-aligned to the content edge). Dynamic data, NOT
//     [[setting]] rows — the same class of surface as the hotkey bindings
//     that live in areas/hotkeys) →
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
//     (handoff note: docs/waves/w26/logs/w26.b08.md).
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
import { ListEmpty } from "../../../components/list/Empty";
import { ListRow } from "../../../components/list/Row";
import { Well } from "../../../components/surface/Surface";
import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import { keys as registryKeys } from "../../../i18n/keys/w26.b02";
import { keys as laneKeys } from "../../../i18n/keys/w26.b08";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTING_EXTENSIONS_LOAD_ON_STARTUP } from "../generated/schema";

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
    laneCatalog[key] ?? m(key as MessageKey);
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
 */
export function ExtensionsArea(): ReactElement {
  const t = useAreaMsg();
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
      data-settings-area="extensions"
      data-testid="extensions-area"
    >
      <section className="settings-section" data-section="add-ons">
        <h2 className="settings-section__title">
          {t("settings.section.extensions.add-ons")}
        </h2>
        {/* The frame's list well (§C13: 782×192 rx5 #090909 + 2px #191919
            stroke, empty in the frame). Composed from the surface family:
            Well (recessed deep fill, r4 — later in surface.css, so it wins
            the shared background/radius) + the card hairline (1px
            stroke-border). The frame's exact 2px #191919 stroke and 192px
            min-height need a settings.css class — handoff note in the lane
            log. Rows render from the (empty) list seam. */}
        <Well className="lumen-card" data-testid="extensions-list">
          {visible.length > 0 ? (
            visible.map((info) => (
              <ExtensionRow
                key={info.id}
                info={info}
                onToggle={() => {
                  /* inert by design: the extensions store does not exist
                   * yet (Phase 7) — the list seam above owns the handoff. */
                }}
              />
            ))
          ) : (
            <ListEmpty
              title={t("settings.extensions.empty-title")}
              body={t("settings.extensions.empty-body")}
              data-testid="extensions-list-empty"
            />
          )}
        </Well>
        {/* Toolbar (§C13 y311): search left, the three frame actions
            right-aligned to the content edge — laid out on the surface
            family's header-row grammar (.lumen-section-header: flex row; the
            trailing slot's auto start-margin pushes the actions right; the
            sheet ships with the Well import). The settings sheet carries no
            toolbar row of its own — a proper .settings-toolbar class is a
            settings.css handoff (see the lane log). Actions
            disabled-from-data while wasm-extensions is off — the codecs F1/F2
            disabled-with-reason pattern. */}
        <div className="lumen-section-header" data-testid="extensions-toolbar">
          <TextField
            type="search"
            label={t("settings.extensions.search")}
            hideLabel
            placeholder={t("settings.extensions.search")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="lumen-section-header__trailing">
            {/* §C0 control inventory: "Small action 100×32 rx6 #191919
                (#8F8F8F) — 'Browse', 'Clear', 'Add', 'Remove'" — the frame's
                own Add/Remove anatomy. The lumen-browse__action class IS that
                settings small-action button (form.css, the BrowseField
                action); "Find more extensions" is the same family at 185px
                (auto width ≥ the 100px minimum). Plain <button>s carry the
                class (the BrowseField's own pattern), disabled-from-data. */}
            <div className="lumen-browse">
              <button
                type="button"
                className="lumen-browse__action"
                disabled={actionsDisabled}
                aria-describedby={actionsDisabled ? "extensions-gate-note" : undefined}
                onClick={() => {
                  /* inert by design: no discovery surface exists yet */
                }}
              >
                {t("settings.extensions.find-more")}
              </button>
              <button
                type="button"
                className="lumen-browse__action"
                disabled={actionsDisabled}
                aria-describedby={actionsDisabled ? "extensions-gate-note" : undefined}
                onClick={() => {
                  /* inert by design: no install command exists yet */
                }}
              >
                {t("settings.extensions.add")}
              </button>
              <button
                type="button"
                className="lumen-browse__action"
                disabled={actionsDisabled}
                aria-describedby={actionsDisabled ? "extensions-gate-note" : undefined}
                onClick={() => {
                  /* inert by design: no removal command exists yet */
                }}
              >
                {t("settings.extensions.remove")}
              </button>
            </div>
          </div>
        </div>
        {actionsDisabled ? (
          <p
            className="settings-field__hint"
            id="extensions-gate-note"
            role="note"
            data-gate-reason={ADDONS_GATE.featureGate}
          >
            {t("settings.capability-unavailable")}
          </p>
        ) : null}
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
