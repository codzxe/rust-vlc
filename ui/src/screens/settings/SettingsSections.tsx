import { useState } from "react";
import type { ReactElement } from "react";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../i18n/keys/w26.b02";
import { keys as fixerKeys } from "../../i18n/keys/w26.gfixB";
import { Icon } from "../../components/icon/Icon";
import { SETTINGS_AREAS } from "./generated/schema";
import type { SettingEntry } from "./generated/schema";
import { WIDGET_GROUPED_KEYS } from "./SettingWidgets";
import { SettingField } from "./SettingField";
import { useSettingsSearch } from "./search";

/**
 * A hand-authored row inside a schema section — no [[setting]] key exists for
 * it (an ACTION: Clear history, Reset picture; or a capability-backed
 * control: the audio Output device enumeration). Rendered in section order
 * through the same row grammar; `keywords` are its resolved search terms
 * (matched against the live query, the same filter schema rows go through).
 */
export interface SettingRowNode {
  /** Row marker — unique within its section (DOM/tests). */
  readonly rowId: string;
  /** Search terms (already-resolved labels); absent = hidden while searching. */
  readonly keywords?: readonly string[];
  readonly node: ReactElement;
}

/** One row of a section: a schema entry, or a hand {@link SettingRowNode}. */
export type SettingSectionRow = SettingEntry | SettingRowNode;

export interface GeneratedSection {
  readonly id: string;
  readonly title: MessageKey;
  readonly entries: readonly SettingSectionRow[];
}

const isSettingEntry = (row: SettingSectionRow): row is SettingEntry => "key" in row;

/**
 * One schema area's sections, ready for <SettingsSections> (w26.gfix-B): the
 * hand compositions (areas/video · areas/audio · areas/privacy) interleave
 * the frame's non-schema rows into these arrays — the seed stays the source
 * of truth for every schema row. Compound-widget keys (WIDGET_GROUPED_KEYS)
 * are filtered out: they render inside their widget's row, never standalone.
 * Section titles flow through the msgid seam like the generated screens'
 * own helper (generated/Screens.tsx).
 */
export const schemaSectionsOf = (id: string): readonly GeneratedSection[] => {
  const area = SETTINGS_AREAS.find((candidate) => candidate.id === id);
  return area === undefined
    ? []
    : area.sections.map((section) => ({
        ...section,
        title: section.title as MessageKey,
        entries: section.entries.filter((entry) => !WIDGET_GROUPED_KEYS.has(entry.key)),
      }));
};

/** Splice hand rows into a schema section (frame order) — pure helper. */
export const withRows = (
  sections: readonly GeneratedSection[],
  sectionId: string,
  rows: readonly { after?: string; row: SettingRowNode }[],
): readonly GeneratedSection[] => {
  const take = (entries: readonly SettingSectionRow[]): readonly SettingSectionRow[] => {
    const out = [...entries];
    for (const { after, row } of rows) {
      const at = after === undefined ? -1 : out.findIndex((e) => "key" in e && e.key === after);
      out.splice(at < 0 ? out.length : at + 1, 0, row);
    }
    return out;
  };
  return sections.map((section) =>
    section.id === sectionId ? { ...section, entries: take(section.entries) } : section,
  );
};

// SettingsSections — the generated screens' content half, in the W26 Figma
// grammar (tools-settings.md §D2): section headers + label-left/control-right
// rows, closed by the IN-FLOW "Advanced Options" disclosure (Settings -
// global.svg Frame 265: 784×38 r2 bar, label + › chevron; global-1.svg is
// the expanded state — ∨ chevron, advanced rows render BELOW the bar, no
// extra section header). Schema entries flagged `advanced` are segregated
// behind that disclosure instead of carrying badges in the normal flow.
//
// w26.gfix-B: sections may interleave hand rows (SettingRowNode) between the
// schema entries — the frame inventories carry rows no [[setting]] key can
// express (audio's capability-backed Output device between Enable audio and
// Speaker setup; privacy's "Clear history now" action; video's "Reset
// picture"). `beforeAdvanced` renders between the visible sections and the
// disclosure (global's About fold — g10-P2-3: the bar stays the column's
// last element).

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
type FixerKey = keyof typeof fixerKeys;
/** Pre-registration seam: b01 + b02 + this lane's keys (see w26.gfixB.ts). */
const laneCatalog: Readonly<Record<string, string>> = {
  ...registryKeys,
  ...laneKeys,
  ...fixerKeys,
};

/** useSectionsMsg — useMsg + the settings lanes' pre-registration keys. */
function useSectionsMsg() {
  const m = useMsg();
  return (key: MessageKey | LaneKey | RegistryKey | FixerKey): string =>
    m(key as MessageKey) ?? laneCatalog[key];
}

/** Split a section's rows into (normal, advanced), search + gate filtered.
 * Hand rows always render in the normal flow (their own keyword filter). */
function partition(
  rows: readonly SettingSectionRow[],
  matches: (entry: SettingEntry) => boolean,
  rowMatches: (row: SettingRowNode) => boolean,
): { normal: readonly SettingSectionRow[]; advanced: readonly SettingEntry[] } {
  const visible = rows.filter((row) =>
    isSettingEntry(row) ? row.featureExposed && matches(row) : rowMatches(row),
  );
  return {
    normal: visible.filter((row) => !isSettingEntry(row) || !row.advanced),
    advanced: visible.filter(isSettingEntry).filter((entry) => entry.advanced),
  };
}

/**
 * The "Advanced Options" disclosure — in flow at the content column's
 * bottom (NOT a pinned footer): collapsed shows the › chevron, expanded the
 * ∨ chevron and renders the advanced rows below the bar.
 */
function AdvancedDisclosure({
  open,
  onToggle,
  children,
}: {
  open: boolean;
  onToggle: () => void;
  children: ReactElement;
}) {
  const t = useSectionsMsg();
  return (
    <div className="settings-advanced">
      <button
        type="button"
        className="settings-advanced__bar"
        aria-expanded={open}
        aria-controls="settings-advanced-rows"
        onClick={onToggle}
      >
        <span className="settings-advanced__label">{t("settings.advanced-options")}</span>
        <Icon
          name={open ? "chevron-down" : "chevron-right"}
          size={14}
          className="settings-advanced__chevron"
        />
      </button>
      {open ? (
        <div className="settings-advanced__rows" id="settings-advanced-rows">
          {children}
        </div>
      ) : null}
    </div>
  );
}

/** Advanced rows — one SettingField per entry, flat under the bar (global-1). */
function AdvancedRows({ entries }: { entries: readonly SettingEntry[] }) {
  return (
    <>
      {entries.map((entry) => (
        <SettingField key={entry.key} entry={entry} />
      ))}
    </>
  );
}

export function SettingsSections({
  area,
  sections,
  beforeAdvanced,
}: {
  area: string;
  sections: readonly GeneratedSection[];
  /** Rendered between the sections and the Advanced disclosure (g10-P2-3). */
  beforeAdvanced?: ReactElement;
}) {
  const t = useSectionsMsg();
  const { query, matches } = useSettingsSearch();
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const needle = query.trim().toLowerCase();
  const rowMatches = (row: SettingRowNode): boolean =>
    needle === "" ||
    (row.keywords ?? []).some((keyword) => keyword.toLowerCase().includes(needle));

  const split = sections.map((section) => ({
    section,
    ...partition(section.entries, matches, rowMatches),
  }));
  const visible = split.filter(({ normal }) => normal.length > 0);
  // Advanced rows keep schema order across sections (flat list, no headers).
  const advancedEntries = split.flatMap(({ advanced }) => advanced);

  return (
    <div className="settings-sections" data-settings-area={area}>
      {visible.map(({ section, normal }) => (
        <section key={section.id} className="settings-section" data-section={section.id}>
          <h2 className="settings-section__title">{t(section.title as MessageKey | LaneKey | RegistryKey | FixerKey)}</h2>
          {normal.map((row) =>
            isSettingEntry(row) ? (
              <SettingField key={row.key} entry={row} />
            ) : (
              <div key={row.rowId} data-setting-row={row.rowId}>
                {row.node}
              </div>
            ),
          )}
        </section>
      ))}
      {beforeAdvanced ?? null}
      {/* The frames show the bar only where advanced content exists
       * (global.svg carries it; interface/controls.svg do not) — so the
       * disclosure renders only when the schema actually flags advanced
       * entries for this area. */}
      {advancedEntries.length > 0 ? (
        <AdvancedDisclosure open={advancedOpen} onToggle={() => setAdvancedOpen((v) => !v)}>
          <AdvancedRows entries={advancedEntries} />
        </AdvancedDisclosure>
      ) : null}
    </div>
  );
}
