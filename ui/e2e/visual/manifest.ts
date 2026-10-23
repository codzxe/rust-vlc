// Screen manifest for the screenshot-diff harness (card 4.G1, doc 08 §6).
//
// The matrix dimensions come from doc 08 §6.1/§6.3 where it names them
// (dark + light, default accent at 1×, DPI/accent expansion in the nightly
// lane) and from the 4.G1 card's [PROPOSAL] minimal set where it does not
// (viewports 1280×720 + 1920×1080, DPI 1 + 2). The route set for G1 is the
// four representative routes the card names (player, playlist, settings,
// onboarding) — 4.G2 grows this to the full doc 08 §1.6 inventory (~40
// compositions) when it captures the FROZEN baseline matrix after F4
// merges, and adds the doc §6.1 accent ×9 sampling on onboarding +
// settings (nightly only).

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
  /** Accent id from tokens (doc 08 §3.3); G1 pins the default only. */
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

/** G1 representative routes (card 4.G1); 4.G2 extends to doc 08 §1.6. */
export const ROUTES: readonly string[] = [
  "/player",
  "/playlist",
  "/settings/global",
  "/onboarding",
] as const;

/** The default accent (doc 08 §3.3 product default). */
export const DEFAULT_ACCENT = "orange";

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

/** Full matrix; the spec filters it down to the PR-lite lane. */
export const MANIFEST: readonly VisualCase[] = VIEWPORTS.flatMap(([w, h]) =>
  DPIS.flatMap((dpi) =>
    THEMES.map((theme) => ({
      route: "",
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
