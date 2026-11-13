// Settings areas — the shared layer (w26.k13, closing j13 P2-4 "missing
// shared abstraction"). The 20-parallel-agent wave protocol (contract §4.2:
// each lane imports only its own keys file) manufactured ten copies of the
// pre-registration seam hook and six near-identical schema-section render
// loops across the 13 area files — same shape, same fallback order, no drift
// yet. This module is the additive consolidation the j13 review asked for;
// every helper lives INSIDE areas/ (the areas own their layer) and nothing
// outside areas/ may import it.
//
// w26.k12 coordination (j12 P2-2, landed in-flight on the shell side): the
// in-area search-filter context is GONE — while a query is live the shell
// renders cross-area results instead of the current area, so no area-side
// filter could ever execute. The section views below therefore filter by
// feature gate (+ the frame-composition hiddenKeys) only, and the former
// hand-row keyword helpers are deleted rather than deduplicated.
//
// Exported here: pure functions + hooks only (no components) — the file
// stays react-refresh-clean (the wave's existing warnings do not grow).

import { useMsg } from "../../../i18n/messages";
import type { MessageKey } from "../../../i18n/messages";
import type { SettingEntry } from "../generated/schema";
import { schemaSectionsOf } from "../SettingsSections";
import type { SchemaSection } from "../SettingsSections";

/** One lane's pre-registration key file (§4.5: `ui/src/i18n/keys/<task>.ts`). */
export type LaneKeyFile = Readonly<Record<string, string>>;

/**
 * makeAreaMsg — build an area's `useAreaMsg` seam hook from its lane key
 * files (the ten-copy dedup, j13 ruling 1):
 * `const useAreaMsg = makeAreaMsg(registryKeys, laneKeys);`
 *
 * The returned hook resolves CATALOG-FIRST — `useMsg()` wins once the Lead
 * registers the key post-wave, the merged lane files are the fallback
 * reference — exactly the ordering every per-file copy already used (the
 * areas' files have no retitle keys, so unlike the shell's lane-first
 * settingsMsg precedence there is no ordering that can diverge). The key
 * union is inferred from the passed files, so call sites keep their typo
 * safety: only MessageKeys and the lanes' own keys type-check.
 */
export function makeAreaMsg<K extends string>(
  ...files: Readonly<Partial<Record<K, string>>>[]
): () => (key: MessageKey | K) => string {
  const catalog: LaneKeyFile = Object.assign({}, ...files) as LaneKeyFile;
  return function useAreaMsg(): (key: MessageKey | K) => string {
    const m = useMsg();
    return (key: MessageKey | K): string => m(key as MessageKey) ?? catalog[key];
  };
}

/** One schema section, ready to render: the section plus its visible rows. */
export interface SchemaSectionView {
  /** The schema section (id, title, unfiltered entries — the seed's truth). */
  readonly section: SchemaSection;
  /** Visible rows in schema order: feature-gated + optional hiddenKeys.
   * (No search filter — the shell swaps in cross-area results while a query
   * is live; an area-side filter could never execute. w26.k12, j12 P2-2.) */
  readonly visible: readonly SettingEntry[];
  /** The visible rows NOT flagged `advanced` (subset, schema order). */
  readonly normal: readonly SettingEntry[];
  /** The visible rows flagged `advanced` (subset, schema order). */
  readonly advanced: readonly SettingEntry[];
}

/**
 * schemaSectionViews — one schema area's sections as render-ready views (the
 * six-copy loop dedup, j13 ruling 1). Flat areas (interface, playback,
 * privacy, video, audio) map `visible`; disclosure areas (global,
 * performance) take `partitionAdvanced`'s split for the Advanced Options
 * bar. Pure: no hook state, the seed is static per build.
 */
export function schemaSectionViews(
  area: string,
  options?: {
    /** Rows the frame's composition does not carry (playback's language rows). */
    readonly hiddenKeys?: readonly string[];
  },
): readonly SchemaSectionView[] {
  const hidden = options?.hiddenKeys;
  return schemaSectionsOf(area).map((section) => {
    const visible = section.entries.filter(
      (entry) => entry.featureExposed && (hidden === undefined || !hidden.includes(entry.key)),
    );
    return {
      section,
      visible,
      normal: visible.filter((entry) => !entry.advanced),
      advanced: visible.filter((entry) => entry.advanced),
    };
  });
}

/** The split a disclosure area needs: sections with normal rows + the flat
 * advanced list (schema order across sections, no headers — the
 * SettingsSections "Advanced rows" grammar, Settings - global-1.svg). */
export function partitionAdvanced(views: readonly SchemaSectionView[]): {
  readonly sections: ReadonlyArray<{ readonly section: SchemaSection; readonly entries: readonly SettingEntry[] }>;
  readonly advanced: readonly SettingEntry[];
} {
  return {
    sections: views
      .filter((view) => view.normal.length > 0)
      .map((view) => ({ section: view.section, entries: view.normal })),
    advanced: views.flatMap((view) => view.advanced),
  };
}
