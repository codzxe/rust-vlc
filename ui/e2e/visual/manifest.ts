// Screen manifest for the screenshot-diff harness (card 4.G1, doc 08 §6).
//
// The matrix dimensions come from doc 08 §6.1/§6.3 where it names them
// (dark + light, default accent at 1×, DPI/accent expansion in the nightly
// lane) and from the 4.G1 card's [PROPOSAL] minimal set where it does not
// (viewports 1280×720 + 1920×1080, DPI 1 + 2).
//
// 4.G2 grew the route set to the full routed inventory of ui/src/App.tsx
// (every <Route> has a case: player, player/music, miniplayer, playlist,
// media-info, discovery, convert, ALL 13 settings areas, onboarding,
// keyboard-shortcuts, open-network-stream, the catch-all not-found,
// /dev/tokens). Doc 08 §1.6's not-yet-routed compositions (player
// customize/hide-ui, context menus, audio/video tools, the playlist/
// subtitles flow) are in-panel states, not routes — they belong to the
// in-flow state harnesses (PlayerScreen seam tests, e2e dialogs), not this
// route × viewport grid.
//
// w26.k20 (j20 P2-9): the inventory lagged the app by 9 routes (convert +
// 8 of the 13 settings areas). ROUTES is now the full 25-route truth
// (12 non-settings renderable routes + 13 settings areas — the same
// inventory e2e/a11y.spec.ts walks since w26.k19; App.tsx's / and
// /settings redirect-only paths render no screen of their own).
//
// 4.G2 scaling decision (recorded in MATRIX-20261024.md): the accent ×9
// sampling of doc 08 §6.1 runs at one pinned combo (1280×720 @1× dark) on
// the two routes the doc names — onboarding + settings/global — for the 8
// non-default accents; the default accent already covers the full
// route × viewport × DPI × mode grid. Full matrix = 25 × 2 × 2 × 2 + 16
// = 216 nightly cases; PR lane = 25 routes × {dark, light} = 50.

export type ThemeMode = "dark" | "light";

/** One screenshot case: route × viewport × DPI × theme-mode × accent. */
export interface VisualCase {
  /** App route to navigate to (rendered under the stubbed/mock dev shell). */
  readonly route: string;
  readonly width: number;
  readonly height: number;
  /** deviceScaleFactor (DPI). 1 = 1×, 2 = 2×. */
  readonly dpi: number;
  readonly theme: ThemeMode;
  /** Accent id from tokens (doc 08 §3.3); the grid pins the default. */
  readonly accent: string;
  /** Include in the PR-lite lane (doc 08 §9.3: "every PR (1× dark/light)"). */
  readonly pr: boolean;
}

/** [PROPOSAL] minimal viewport set (card 4.G1) until the Lead widens it. */
export const VIEWPORTS: ReadonlyArray<readonly [number, number]> = [
  [1280, 720],
  [1920, 1080],
] as const;

/** DPI set: 1× canonical (doc 08 §6.1) + 2× (nightly DPI matrix, §6.3). */
export const DPIS: readonly number[] = [1, 2] as const;

export const THEMES: readonly ThemeMode[] = ["dark", "light"] as const;

/** Full routed surface of ui/src/App.tsx (card 4.G2, doc 08 §1.6;
 * w26.k20 full 25-route truth — see the header note above). */
export const ROUTES: readonly string[] = [
  "/player",
  "/player/music",
  "/miniplayer",
  "/playlist",
  "/media-info",
  "/discovery",
  "/convert",
  // All 13 settings areas (w26.k20): /settings/:area is one route, but
  // each area is its own screenful with its own baselines — enumerated,
  // not sampled.
  "/settings/global",
  "/settings/interface",
  "/settings/privacy",
  "/settings/playback",
  "/settings/video",
  "/settings/audio",
  "/settings/subtitles",
  "/settings/controls",
  "/settings/library",
  "/settings/network",
  "/settings/performance",
  "/settings/recording",
  "/settings/extensions",
  "/onboarding",
  "/keyboard-shortcuts",
  "/open-network-stream",
  // No literal /not-found route exists: navigating to an unmatched path
  // renders the catch-all `*` placeholder (App.tsx).
  "/not-found",
  "/dev/tokens",
] as const;

/**
 * GRID_ANCHOR_ACCENT — the accent pinned across the full route × viewport ×
 * DPI × mode grid (doc 08 §6.1 "default accent at 1×").
 *
 * `orange` IS the product default since the W25 redesign (2026-11-11):
 * gen-tokens.mjs (`DEFAULT_ACCENT = "orange"`), ThemeProvider's initial
 * state, the tokens.css `:root` cascade and e2e/theme.spec.ts:19-23 all
 * agree — dark + orange (the VLC orange #d37918, W25 spec §1). The OLD
 * honesty note here (issue #37, 2026-11-09) recorded a real discrepancy:
 * the grid was anchored on orange while the product still defaulted to
 * Solstice yellow. That discrepancy was RESOLVED by the W25 re-anchor —
 * grid anchor == product default == the out-of-box look, so the grid
 * baselines depict exactly what ships. The other 8 accents remain covered
 * via ACCENT_SAMPLES on the two sampling routes. Baseline provenance:
 * docs/waves/w26/baselines/MATRIX-20261112.md (w26.k20).
 */
export const DEFAULT_ACCENT = "orange";

/**
 * The 9 accent ids (doc 08 §3.3), mirroring
 * src/tokens/generated/tokens.ts ACCENTS (kept in sync by that module's
 * committed-set test; duplicated here so the e2e tsconfig stays self-contained).
 */
export const ACCENTS: readonly string[] = [
  "blue",
  "cyan",
  "gray",
  "green",
  "orange",
  "pink",
  "red",
  "violet",
  "yellow",
] as const;

/** Doc 08 §6.1 accent-sampling routes (exports Onboarding-3..10). */
export const ACCENT_SAMPLE_ROUTES: readonly string[] = [
  "/onboarding",
  "/settings/global",
] as const;

function slug(route: string): string {
  return route === "/" ? "root" : route.slice(1).replaceAll("/", "-");
}

/** Stable baseline file id: <route>-<viewport>-<dpi>x-<mode>[-<accent>].
 *  Matches the on-disk name exactly — Playwright sanitizes `@`/`_` in
 *  snapshot names to `-`, so `-` separators are used up front. */
export function caseId(c: VisualCase): string {
  const accent = c.accent === DEFAULT_ACCENT ? "" : `-${c.accent}`;
  return `${slug(c.route)}-${c.width}x${c.height}-${c.dpi}x-${c.theme}${accent}`;
}

/** Route × viewport × DPI × mode grid at the default accent. */
const GRID: readonly VisualCase[] = VIEWPORTS.flatMap(([w, h]) =>
  DPIS.flatMap((dpi) =>
    THEMES.map((theme) => ({
      width: w,
      height: h,
      dpi,
      theme,
      accent: DEFAULT_ACCENT,
      // PR-lite: one viewport, 1×, dark + light, current screens (§9.3).
      pr: w === 1280 && h === 720 && dpi === 1,
    })),
  ),
).flatMap((partial) =>
  ROUTES.map((route) => ({ ...partial, route }) satisfies VisualCase),
);

/** Nightly accent sampling (doc 08 §6.1): the 8 non-default accents on the
 *  sampling routes at one pinned combo — the default accent is already
 *  covered by GRID at every viewport × DPI × mode. */
const ACCENT_SAMPLES: readonly VisualCase[] = ACCENT_SAMPLE_ROUTES.flatMap(
  (route) =>
    ACCENTS.filter((a) => a !== DEFAULT_ACCENT).map((accent) => ({
      route,
      width: 1280,
      height: 720,
      dpi: 1,
      theme: "dark" as const,
      accent,
      pr: false,
    })),
);

/** Full matrix; the spec filters it down to the PR-lite lane. */
export const MANIFEST: readonly VisualCase[] = [...GRID, ...ACCENT_SAMPLES];
