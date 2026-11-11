// Settings area: Updates & About — card 4.E4, doc 07 §5.4.
//
// HONEST SURFACE RULES:
//   • Version data: REAL values only — injected at build time from
//     ui/package.json via vite.config.ts `define` (__UI_PACKAGE_VERSION__),
//     so the About panel always shows the version of the package it ships
//     in and never a hardcoded literal that can drift. Runtime engine/
//     protocol versions come from system.get-engine-info (doc 05 §2.12) on
//     surfaces that have a client; this static About panel sticks to the
//     build-injected package data.
//   • Update checks: privacy.update-checks is a REAL schema key (bool,
//     product default false — opt-in, product.toml §2.6) rendered via E1's
//     SettingField. NOTE: its schema placement also renders it on the
//     generated "privacy" screen — flagged to the schema owner in
//     ui/NEXT-STEPS.md to consolidate placement (D5/D6 safe).
//   • No "check now" action exists in the command catalog (no update
//     command) → no button is rendered. Inventing one would violate MP §5.

import type { ReactElement } from "react";
import { UI_PACKAGE_NAME, UI_PACKAGE_VERSION } from "../../../build-info";
import { useMsg } from "../../../i18n/messages";
import { SettingField } from "../SettingField";
import { SETTING_PRIVACY_UPDATE_CHECKS } from "../generated/schema";

/** About panel — version facts from imported package data (never hardcoded).
 * w26.d2 a11y sweep: the field head (a plain span label) moved OUT of the
 * <dl> — axe definition-list requires every direct <div> child of a <dl> to
 * hold an ordered dt/dd group; the head wrapper held only the span. */
export function AboutPanel(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-field" data-testid="about-panel" data-about-package={UI_PACKAGE_NAME}>
      <div className="settings-field__head">
        <span className="settings-field__label">{m("settings.about.product")}</span>
      </div>
      <dl className="settings-about__rows">
        <div className="settings-about__row">
          <dt>{m("settings.about.ui-package")}</dt>
          <dd data-testid="about-package-name">{UI_PACKAGE_NAME}</dd>
        </div>
        <div className="settings-about__row">
          <dt>{m("settings.about.version")}</dt>
          <dd data-testid="about-version">{UI_PACKAGE_VERSION}</dd>
        </div>
      </dl>
    </div>
  );
}

/** Updates & About settings area. */
export function AboutArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="about" data-testid="about-area">
      <section className="settings-section" data-section="updates">
        <h2 className="settings-section__title">{m("settings.section.about.updates")}</h2>
        <SettingField entry={SETTING_PRIVACY_UPDATE_CHECKS} />
        {/* "Check for updates now": no command in the catalog → nothing. */}
      </section>
      <section className="settings-section" data-section="about">
        <h2 className="settings-section__title">{m("settings.section.about.about")}</h2>
        <AboutPanel />
      </section>
    </div>
  );
}
