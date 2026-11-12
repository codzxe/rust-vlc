// Settings area: About — card 4.E4, doc 07 §5.4 (the about→global registry
// fold). w26.i13 (h08 #2): the About panel renders BEHIND the global screen's
// Advanced Options disclosure, AFTER the advanced rows — NEITHER global frame
// (Settings - global.svg / -1.svg) shows an About section, and the previous
// in-flow placement displaced the Advanced bar 141px (y786 vs the frame's
// y645), clipping every expanded row below the window fold (h08's only NO
// verdict). Reachable and honest; out of the frame-true column.
//
// HONEST SURFACE RULES:
//   • Version data: REAL values only — injected at build time from
//     ui/package.json via vite.config.ts `define` (__UI_PACKAGE_VERSION__),
//     so the About panel always shows the version of the package it ships
//     in and never a hardcoded literal that can drift. Runtime engine/
//     protocol versions come from system.get-engine-info (doc 05 §2.12) on
//     surfaces that have a client; this static About panel sticks to the
//     build-injected package data.
//   • Update checks (w26.gfix-B, g10-P2-1): the "Check for updates
//     automatically" row MOVED to the global screen's Startup section with
//     the w26.gfix-B re-seed (key global.update-checks) — this fold no
//     longer duplicates it.
//   • No "check now" action exists in the command catalog (no update
//     command) → no button is rendered. Inventing one would violate MP §5.
//
// w26.gfix-B (g10-P2-3): the About rows render in the settings ROW grammar
// (label left, value right in the control column) — the legacy stacked
// .settings-field__head grammar is gone.

import type { ReactElement } from "react";
import { UI_PACKAGE_NAME, UI_PACKAGE_VERSION } from "../../../build-info";
import { useMsg } from "../../../i18n/messages";

/** About panel — version facts from imported package data (never hardcoded),
 * rendered as settings rows (label left, value right). */
export function AboutPanel(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-about" data-testid="about-panel" data-about-package={UI_PACKAGE_NAME}>
      <div className="settings-field" data-kind="about" data-about-row="package">
        <div className="settings-field__row">
          <div className="settings-field__lead">
            <span className="settings-field__label">{m("settings.about.ui-package")}</span>
          </div>
          <div className="settings-field__control settings-about__value" data-testid="about-package-name">
            {UI_PACKAGE_NAME}
          </div>
        </div>
      </div>
      <div className="settings-field" data-kind="about" data-about-row="version">
        <div className="settings-field__row">
          <div className="settings-field__lead">
            <span className="settings-field__label">{m("settings.about.version")}</span>
          </div>
          <div className="settings-field__control settings-about__value" data-testid="about-version">
            {UI_PACKAGE_VERSION}
          </div>
        </div>
      </div>
    </div>
  );
}

/** About fold — the About panel only (the Updates row moved to Startup). */
export function AboutArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="about" data-testid="about-area">
      <section className="settings-section" data-section="about">
        <h2 className="settings-section__title">{m("settings.section.about.about")}</h2>
        <AboutPanel />
      </section>
    </div>
  );
}
