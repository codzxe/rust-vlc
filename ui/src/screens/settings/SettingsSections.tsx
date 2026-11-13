import { useEffect, useRef, useState } from "react";
import type { ReactElement, ReactNode } from "react";
import type { MessageKey } from "../../i18n/messages";
import { Icon } from "../../components/icon/Icon";
import { SETTINGS_AREAS } from "./generated/schema";
import type { SettingEntry } from "./generated/schema";
import { WIDGET_GROUPED_KEYS } from "./SettingWidgets";
import { SettingField } from "./SettingField";
import { useSettingsMsg } from "./settingsMsg";

/**
 * A hand-authored row inside a schema section — no [[setting]] key exists for
 * it (an ACTION: Clear history, Reset picture; or a capability-backed
 * control: the audio Output device enumeration). Rendered in section order
 * through the same row grammar.
 * w26.k12 (j12 P2-2): the `keywords` field is GONE — it fed an in-area
 * search filter whose every consumer was unmounted while a query was live
 * (the shell renders cross-area results instead of the current area), so
 * it could never match. Hand rows now always render in the normal flow;
 * they stay invisible to cross-area search (it walks schema entries only —
 * see search.ts).
 */
export interface SettingRowNode {
  /** Row marker — unique within its section (DOM/tests). */
  readonly rowId: string;
  readonly node: ReactElement;
}

/** One row of a section: a schema entry, or a hand {@link SettingRowNode}. */
export type SettingSectionRow = SettingEntry | SettingRowNode;

export interface GeneratedSection {
  readonly id: string;
  readonly title: MessageKey;
  readonly entries: readonly SettingSectionRow[];
}

/** A section of PURE schema entries — {@link schemaSectionsOf}'s output type
 * (w26.i12 relaunch). Narrower than GeneratedSection on purpose: the seed's
 * sections carry ONLY SettingEntry rows (hand rows enter later, via withRows
 * or the composed screens). The areas' hand-rendered compositions (areas'
 * GlobalAreaScreen / PrivacyArea) iterate these entries straight off the
 * generated schema — `entry.featureExposed`, `entry.advanced`, `entry.key`,
 * without re-narrowing a union, so the precise type is the contract; the
 * union widening of the dead attempt broke every such call site (tsc
 * TS2339) and is reverted here at the type level only: the runtime shape
 * never changed (a seed section never contained a hand row). */
export interface SchemaSection {
  readonly id: string;
  readonly title: MessageKey;
  readonly entries: readonly SettingEntry[];
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
export const schemaSectionsOf = (id: string): readonly SchemaSection[] => {
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

/**
 * The "Advanced Options" disclosure — in flow at the content column's
 * bottom (NOT a pinned footer): collapsed shows the › chevron, expanded the
 * ∨ chevron and renders the advanced rows below the bar.
 *
 * w26.k12 (j12 P2-6/P3-4): exported — ONE implementation. It used to exist
 * 3× (this module unexported, a hand copy in GlobalAreaScreen, a hand copy
 * in CodecsFold WITHOUT the scrollIntoView — so the codecs fold still
 * shipped the h08#1 "expanding reveals nothing" defect the shell half had
 * fixed). GlobalAreaScreen and CodecsFold consume this export; the rows
 * region's id is ONE exported constant (the two hand copies used to define
 * it twice — no live collision, mutually exclusive screens, but a trap).
 */
export const ADVANCED_ROWS_ID = "settings-advanced-rows";

export function AdvancedDisclosure({
  open,
  onToggle,
  children,
}: {
  open: boolean;
  onToggle: () => void;
  /** w26.k12: ReactNode (was ReactElement) — consumers pass mapped arrays
   * (k13's area compositions) as well as single elements; widening is
   * signature-compatible for every existing single-child usage. */
  children: ReactNode;
}) {
  const t = useSettingsMsg();
  const rowsRef = useRef<HTMLDivElement | null>(null);
  // w26.i12 (h08 defect 1, shell half): expanding a disclosure whose bar sat
  // near the fold pushed the revealed rows BELOW the window with scrollTop
  // unmoved — the user got a chevron flip and an empty gap. Scroll the rows
  // into view on expand (minimal movement, "nearest"; the bar keeps its
  // frame position). jsdom implements no scrollIntoView — guarded like the
  // Dropdown's own call.
  useEffect(() => {
    if (!open) return;
    rowsRef.current?.scrollIntoView?.({ block: "nearest" });
  }, [open]);
  return (
    <div className="settings-advanced">
      <button
        type="button"
        className="settings-advanced__bar"
        aria-expanded={open}
        aria-controls={ADVANCED_ROWS_ID}
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
        <div className="settings-advanced__rows" id={ADVANCED_ROWS_ID} ref={rowsRef}>
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

/** Split a section's rows into (normal, advanced), feature-gate filtered.
 * w26.k12 (j12 P2-2): no search filter here — see SettingRowNode. */
function partition(
  rows: readonly SettingSectionRow[],
): { normal: readonly SettingSectionRow[]; advanced: readonly SettingEntry[] } {
  const visible = rows.filter((row) => isSettingEntry(row) && row.featureExposed);
  return {
    normal: visible.filter((row) => !isSettingEntry(row) || !row.advanced),
    advanced: visible.filter(isSettingEntry).filter((entry) => entry.advanced),
  };
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
  const t = useSettingsMsg();
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const split = sections.map((section) => ({ section, ...partition(section.entries) }));
  const visible = split.filter(({ normal }) => normal.length > 0);
  // Advanced rows keep schema order across sections (flat list, no headers).
  const advancedEntries = split.flatMap(({ advanced }) => advanced);

  return (
    <div className="settings-sections" data-settings-area={area}>
      {visible.map(({ section, normal }) => (
        <section key={section.id} className="settings-section" data-section={section.id}>
          <h2 className="settings-section__title">{t(section.title)}</h2>
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
