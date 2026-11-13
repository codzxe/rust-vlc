import type { SettingsArea, SettingEntry } from "./generated/schema";

// Settings search (card 4.E1, doc 07 §5): filters fields by their label
// key's resolved text, ACROSS areas — the result is the honest subset of
// schema entries whose labels match; nothing is invented. Labels that the
// catalog has not resolved yet (mid-wave lane keys await Lead registration)
// read as empty and can never match — a catalog gap degrades to "no hit",
// never a crash.
//
// w26.k12 (j12 P2-2): the in-area filter context is DELETED. It could never
// execute: while a query is live, SettingsScreen renders the cross-area
// results INSTEAD of the current area, so every would-be consumer
// (SettingsSections' partition, the composed areas' loops, the hand rows'
// `keywords`) was unmounted — ~40 lines of filtering API that read as a
// live feature. The design keeps the tested W26 contract unchanged (results
// replace the column; the current screen is hidden while searching), and
// the honest limits are documented here instead of simulated in code:
//   * search walks SCHEMA entries only — hand rows (Clear history, Reset
//     picture, the capability-backed Output device) are actions, not
//     [[setting]] rows, and stay invisible to search;
//   * no per-area filter survives navigation: the query clears on area
//     change (j12 P1-1, SettingsScreen) and the target area renders clean.

export interface SearchHit {
  readonly area: SettingsArea;
  readonly sectionTitle: string;
  readonly entries: readonly SettingEntry[];
}

const normalize = (text: string | undefined): string =>
  (typeof text === "string" ? text : "").trim().toLowerCase();

/** All matching sections across all areas, in schema order. Empty = no match. */
export function searchSettings(
  areas: readonly SettingsArea[],
  query: string,
  labelOf: (entry: SettingEntry) => string,
): readonly SearchHit[] {
  const q = normalize(query);
  if (q === "") return [];
  const hits: SearchHit[] = [];
  for (const area of areas) {
    for (const section of area.sections) {
      const entries = section.entries.filter((entry) => entry.featureExposed && normalize(labelOf(entry)).includes(q));
      if (entries.length > 0) {
        hits.push({ area, sectionTitle: section.title, entries });
      }
    }
  }
  return hits;
}
