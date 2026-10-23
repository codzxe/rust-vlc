// Settings area: Network — card 4.E4 (Privacy & Network group), doc 07 §5.4
// screen "network".
//
// CONFIG-NEVER-INVENT AUDIT (the honest result):
//   • Network proxies (http/socks per-stream or global), live555 / tcp
//     tuning, caching-for-network-streams toggles: NO entries exist in
//     schemas/settings.toml → rendered as NOTHING. Handoff note in
//     ui/NEXT-STEPS.md for the schema owner.
//   • The PRIVACY half of the card's "Privacy & Network" group is already
//     schema-backed and renders via E1's generated "privacy" screen
//     (privacy.telemetry / crash-reports / usage-stats / update-checks —
//     all default OFF per product.toml §2.6 opt-in rule; asserted in
//     privacy-network.test.tsx, not duplicated here — no double definition).
// When network schema entries land, the generator emits this screen.

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";

/** Network settings area — honest empty state, zero controls. */
export function NetworkArea(): ReactElement {
  const m = useMsg();
  return (
    <div className="settings-sections" data-settings-area="network" data-testid="network-area">
      <p className="settings-screen__empty" data-testid="settings-area-empty" role="note">
        {m("settings.area.empty")}
      </p>
      {/* proxies / network tuning: absent from the schema → deliberately
          nothing (never dead controls that look alive). */}
    </div>
  );
}
