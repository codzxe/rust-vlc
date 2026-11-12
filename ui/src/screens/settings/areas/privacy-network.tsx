// Settings area: Network — card 4.E4 (Privacy & Network group), doc 07 §5.4
// screen "network" (Settings - network.svg, analysis tools-settings.md §C10).
//
// w26.i15 (closing h11): the §C10 frame rows render STAGED INERT — the
// wave-I fix directive (the honest-empty pattern in the frame's own visual
// grammar, the Remotes precedent): no network [[setting]] keys exist in
// schemas/settings.toml yet, so every row renders full-strength, disabled,
// marked data-staged="schema-seed" with the honest tooltip. When the Lead
// seeds the network rows (exact list in docs/waves/w26/logs/w26.i15.md),
// E1's generated screen renders them live and this composition yields.
//
// Frame-disabled rows: §C10 ships the four buffering inputs (Local files,
// Discs, Network streams, Live capture) as disabled groups @0.3 (§C0
// "Disabled row") — those rows carry data-staged-dimmed; the Buffering
// dropdown, both Proxy inputs and the Security rows are staged full
// strength.
//
// The PRIVACY half of the card's "Privacy & Network" group stays
// schema-backed on the generated privacy screen (privacy.telemetry /
// crash-reports / usage-stats / update-checks — all default OFF per
// product.toml §2.6 opt-in rule; asserted in privacy-network.test.tsx, not
// duplicated here — no double definition).

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import { StagedAdvancedBar } from "./staged";
import { StagedRow } from "./staged";
import { useStagedMsg } from "./staged";
import "./staged.css";

/**
 * Network settings area (route /settings/network; System group) — the §C10
 * frame: Buffering + Proxy + Security + the Advanced bar, all staged.
 */
export function NetworkArea(): ReactElement {
  const t = useStagedMsg();
  const m = useMsg();
  const staged = t("settings.network.staged");
  const seam = "schema-seed";
  const advancedLabel = m("settings.advanced-options");

  return (
    <div
      className="settings-sections"
      data-settings-area="network"
      data-testid="network-area"
      data-advanced-staged="true"
    >
      <section className="settings-section" data-section="buffering">
        <h2 className="settings-section__title">{t("settings.section.network.buffering")}</h2>
        <StagedRow
          rowKey="network.buffer-size"
          label={t("settings.network.buffer-size")}
          control={{ kind: "enum", value: t("settings.network.value.buffer.normal") }}
          seam={seam}
          reason={staged}
          info
        />
        <StagedRow
          rowKey="network.local-files"
          label={t("settings.network.local-files")}
          control={{ kind: "text", value: "" }}
          seam={seam}
          reason={staged}
          dimmed
        />
        <StagedRow
          rowKey="network.discs"
          label={t("settings.network.discs")}
          control={{ kind: "text", value: "" }}
          seam={seam}
          reason={staged}
          dimmed
        />
        <StagedRow
          rowKey="network.network-streams"
          label={t("settings.network.network-streams")}
          control={{ kind: "text", value: "" }}
          seam={seam}
          reason={staged}
          dimmed
        />
        <StagedRow
          rowKey="network.live-capture"
          label={t("settings.network.live-capture")}
          control={{ kind: "text", value: "" }}
          seam={seam}
          reason={staged}
          dimmed
        />
      </section>
      <section className="settings-section" data-section="proxy">
        <h2 className="settings-section__title">{t("settings.section.network.proxy")}</h2>
        <StagedRow
          rowKey="network.http-proxy"
          label={t("settings.network.http-proxy")}
          control={{ kind: "text", value: t("settings.network.value.proxy.system") }}
          seam={seam}
          reason={staged}
          info
        />
        <StagedRow
          rowKey="network.http-proxy-password"
          label={t("settings.network.http-proxy-password")}
          control={{ kind: "text", value: "" }}
          seam={seam}
          reason={staged}
          info
        />
      </section>
      <section className="settings-section" data-section="security">
        <h2 className="settings-section__title">{t("settings.section.network.security")}</h2>
        <StagedRow
          rowKey="network.verify-certificates"
          label={t("settings.network.verify-certificates")}
          control={{ kind: "switch", on: true }}
          seam={seam}
          reason={staged}
          info
        />
        <StagedRow
          rowKey="network.cipher-preference"
          label={t("settings.network.cipher-preference")}
          control={{ kind: "enum", value: t("settings.network.value.cipher.balanced") }}
          seam={seam}
          reason={staged}
          info
        />
      </section>
      {/* §C10 y669: the collapsed Advanced bar, staged (nothing behind it). */}
      <StagedAdvancedBar reason={t("settings.advanced.staged")} label={advancedLabel} />
    </div>
  );
}
