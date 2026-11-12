// Settings area: Library — "Playlists, Folders & Files"
// (Settings - library.svg, analysis tools-settings.md §C9).
//
// w26.i15 (closing h11): the §C9 frame rows render STAGED INERT — the wave-I
// fix directive (the honest-empty pattern in the frame's own visual grammar,
// the Remotes precedent): no schemas/settings.toml [[setting]] keys exist
// behind them yet, so every row renders full-strength, disabled, marked
// data-staged="schema-seed" with the honest tooltip — visible frame rows,
// never dead controls that look alive. When the Lead seeds the library rows
// (exact list in docs/waves/w26/logs/w26.i15.md), E1's generated screen
// renders them live and this composition yields to it (the registry embeds
// GENERATED_SCREENS.library at that point — see library.test.tsx).
//
// Frame-disabled rows: §C9 ships "Apply to files opened from the file
// manager" and "Add to the playlist instead of replacing it" as disabled
// groups @0.3 (§C0 "Disabled row") — those two rows carry
// data-staged-dimmed (var(--opacity-muted)); the rest are staged full
// strength like Remotes.
//
// w26.b02 SIBLING CONTRACT: this module owns the WHOLE area screen. Library
// is the merge home of the retired playlist-services area — the fold mounts
// here (playlist-services.tsx, this lane's file); it renders nothing until
// playlist-services schema rows exist (the seam) and no longer claims the
// area root (w26.i15: exactly ONE [data-settings-area="library"] per route).

import type { ReactElement } from "react";
import { useMsg } from "../../../i18n/messages";
import { PlaylistServicesArea } from "./playlist-services";
import { StagedAdvancedBar } from "./staged";
import { StagedRow } from "./staged";
import { useStagedMsg } from "./staged";
import "./staged.css";

/**
 * Library settings area (route /settings/library; Media group) — the §C9
 * frame: Opening Files + Metadata & Artwork + the Advanced bar, staged;
 * the playlist-services fold hosts its future schema rows.
 */
export function LibraryArea(): ReactElement {
  const t = useStagedMsg();
  const m = useMsg();
  const staged = t("settings.library.staged");
  const seam = "schema-seed";
  const advancedLabel = m("settings.advanced-options");

  return (
    <div
      className="settings-sections"
      data-settings-area="library"
      data-testid="library-area"
      data-advanced-staged="true"
    >
      {/* The playlist-services fold (w26 merge: playlist-services → library).
          Renders nothing today — the seam for its future schema rows; no
          duplicate area root (w26.i15). */}
      <PlaylistServicesArea />
      <section className="settings-section" data-section="opening-files">
        <h2 className="settings-section__title">{t("settings.section.library.opening-files")}</h2>
        <StagedRow
          rowKey="library.single-window"
          label={t("settings.library.single-window")}
          control={{ kind: "switch", on: false }}
          seam={seam}
          reason={staged}
          info
        />
        <StagedRow
          rowKey="library.file-manager"
          label={t("settings.library.file-manager")}
          control={{ kind: "switch", on: false }}
          seam={seam}
          reason={staged}
          info
          dimmed
        />
        <StagedRow
          rowKey="library.playlist-add"
          label={t("settings.library.playlist-add")}
          control={{ kind: "switch", on: false }}
          seam={seam}
          reason={staged}
          info
          dimmed
        />
        <StagedRow
          rowKey="library.folder-open"
          label={t("settings.library.folder-open")}
          control={{ kind: "enum", value: t("settings.library.value.folder.include-subfolders") }}
          seam={seam}
          reason={staged}
          info
        />
        <StagedRow
          rowKey="library.show-hidden"
          label={t("settings.library.show-hidden")}
          control={{ kind: "switch", on: false }}
          seam={seam}
          reason={staged}
          info
        />
      </section>
      <section className="settings-section" data-section="metadata-artwork">
        <h2 className="settings-section__title">{t("settings.section.library.metadata-artwork")}</h2>
        <StagedRow
          rowKey="library.fetch-metadata"
          label={t("settings.library.fetch-metadata")}
          control={{ kind: "enum", value: t("settings.library.value.metadata.file-only") }}
          seam={seam}
          reason={staged}
          info
        />
        <StagedRow
          rowKey="library.use-media-library"
          label={t("settings.library.use-media-library")}
          control={{ kind: "switch", on: true }}
          seam={seam}
          reason={staged}
          info
        />
      </section>
      {/* §C9 y490: the collapsed Advanced bar, staged (nothing behind it). */}
      <StagedAdvancedBar reason={t("settings.advanced.staged")} label={advancedLabel} />
    </div>
  );
}
