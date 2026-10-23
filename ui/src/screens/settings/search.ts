import { createContext } from "react";
import { useContext } from "react";
import type { SettingsArea, SettingEntry } from "./generated/schema";

// Settings search (card 4.E1, doc 07 §5): filters fields by their label
// key's resolved text, ACROSS areas — the result is the honest subset of
// schema entries whose labels match; nothing is invented.

export interface SearchHit {
  readonly area: SettingsArea;
  readonly sectionTitle: string;
  readonly entries: readonly SettingEntry[];
}

export interface SettingsSearchContextValue {
  readonly query: string;
  setQuery(query: string): void;
  /** True when a query is active and the entry's label matches it. */
  matches(entry: SettingEntry): boolean;
}

export const SettingsSearchContext = createContext<SettingsSearchContextValue>({
  query: "",
  setQuery: () => undefined,
  matches: () => true,
});

export function useSettingsSearch(): SettingsSearchContextValue {
  return useContext(SettingsSearchContext);
}

const normalize = (text: string): string => text.trim().toLowerCase();

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
