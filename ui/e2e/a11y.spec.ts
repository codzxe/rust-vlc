import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

// Card 4.F4 — axe/Playwright a11y suite + keyboard-navigation audit.
//
// Rule set: the enforced tags are wcag2a + wcag2aa — the WCAG 2.1 A/AA
// subset this pass is accountable for (doc 08 §9.4: "zero critical
// violations"). `best-practice`-only rules (region, page-has-heading-one…)
// are NOT enforced; screens that gain landmarks/headings as they harden
// will simply pass the stricter future set.
//
// Exclusions are NEVER silent: any rule id accepted with a justification is
// listed in JUSTIFIED_EXCLUSIONS with the reason, and the assertion prints
// the justified hits so a stale entry surfaces on the next run. Route- and
// node-scoped acceptances live in ROUTE_EXCLUSIONS (same discipline, see
// there).
//
// w26.k19 (j19 J19-1/J19-2/J19-3) — the suite's coverage claims now match
// its coverage:
//  - ROUTES is the FULL routable inventory (App.tsx route tree: 24 product
//    screens + the /dev/tokens dev fixture), not a representative sample.
//  - Every audit runs in BOTH theme modes — light mode (including the h05
//    F1 focus-ring re-point) had zero e2e coverage before.
//  - The tab audit cannot truncate silently anymore: tabThrough returns an
//    exhaustion signal and the test FAILS when the per-route stop budget
//    runs out mid-walk. The two 400-press dialog screens
//    (/keyboard-shortcuts, /open-network-stream) are IN the audit with a
//    trap-aware wrap rule (a wrap is legal only as a jump back to the
//    TRAPPING CONTAINER's first focusable).

/** Every routable screen in the App.tsx route tree — 12 shell/dialog/
 * full-bleed routes + all 13 settings areas + onboarding + the /dev/tokens
 * dev fixture = 25. (w26.k19, j19 J19-2: this header used to claim "every
 * routable screen" while listing 11 paths; the inventory and the claim now
 * agree. Sources: doc 08 §1.6 + the CONTRACT §3 frameless routes
 * discovery/convert/media-info/404.) The redirect-only paths (/, /settings)
 * render no screen of their own — the smoke suite owns their redirect
 * behavior. `onboarded` routes sit behind the 4.F1 first-run gate and get a
 * seeded completed-wizard store. */
const ROUTES: ReadonlyArray<{ name: string; path: string; onboarded: boolean }> = [
  { name: "player", path: "/player", onboarded: true },
  { name: "player-music", path: "/player/music", onboarded: true },
  { name: "miniplayer", path: "/miniplayer", onboarded: true },
  { name: "playlist", path: "/playlist", onboarded: true },
  { name: "media-info", path: "/media-info", onboarded: true },
  // w26.k17 (j17 P3-9): the frameless working screens join the sweep —
  // the harness existed; these three were absent.
  { name: "discovery", path: "/discovery", onboarded: true },
  { name: "convert", path: "/convert", onboarded: true },
  { name: "not-found", path: "/no-such-route", onboarded: true },
  // w26.k19 (j19 J19-1/J19-2): the two dialog routes and the remaining
  // eight settings areas — the known-worst keyboard surface
  // (/keyboard-shortcuts: 149 tab stops inside the i17 Dialog trap) was
  // previously outside every audit.
  { name: "keyboard-shortcuts", path: "/keyboard-shortcuts", onboarded: true },
  { name: "open-network-stream", path: "/open-network-stream", onboarded: true },
  { name: "settings-global", path: "/settings/global", onboarded: true },
  { name: "settings-interface", path: "/settings/interface", onboarded: true },
  { name: "settings-privacy", path: "/settings/privacy", onboarded: true },
  { name: "settings-playback", path: "/settings/playback", onboarded: true },
  { name: "settings-video", path: "/settings/video", onboarded: true },
  { name: "settings-audio", path: "/settings/audio", onboarded: true },
  { name: "settings-subtitles", path: "/settings/subtitles", onboarded: true },
  { name: "settings-controls", path: "/settings/controls", onboarded: true },
  { name: "settings-library", path: "/settings/library", onboarded: true },
  { name: "settings-network", path: "/settings/network", onboarded: true },
  { name: "settings-performance", path: "/settings/performance", onboarded: true },
  { name: "settings-recording", path: "/settings/recording", onboarded: true },
  { name: "settings-extensions", path: "/settings/extensions", onboarded: true },
  { name: "onboarding", path: "/onboarding", onboarded: false },
  // Dev fixture (TokenSandbox) — a real routable screen; axe'd like the rest.
  { name: "dev-tokens", path: "/dev/tokens", onboarded: false },
];

/** Both shipped theme modes are audited (w26.k19, j19 J19-3: light mode —
 * including the h05 F1 focus-ring fix — was never axe'd or tab-audited).
 * Dark is the product default; light boots through the persisted
 * interface.theme override (lumen.settings.overrides.v1 — the same seam
 * App.test.tsx uses, hydrated by ThemeProvider on every fresh load). */
const MODES = ["dark", "light"] as const;
type Mode = (typeof MODES)[number];

/** Axe rule ids accepted with a documented reason, on EVERY route. Empty as
 * of 4.F4 — the whole wcag2a/2aa set passes on every route. To accept a
 * finding, add `ruleId: "why this is OK (e.g. third-party iframe, tooling
 * false positive)"` — the assertion prints hits so stale entries surface. */
const JUSTIFIED_EXCLUSIONS: Readonly<Record<string, string>> = {};

/** Route-scoped acceptance of SPECIFIC pre-existing findings, found by the
 * w26.k19 inventory extension and logged for their owning lanes (contract
 * §4.2 — the files are not this lane's to fix). This is NOT a license to
 * skip a route: acceptance is scoped to route + axe rule + theme mode +
 * node marker, every OTHER node of the same rule still fails the suite,
 * and the accepted hits are printed on every run — the moment the owning
 * lane fixes the finding, the entry reports "accepted: none (STALE —
 * delete the entry)" and gets removed. */
/** One accepted finding: WHERE (modes), WHICH nodes (markers — a node is
 * accepted only if its axe target contains one of these markers), and the
 * documented reason. */
interface RouteExclusion {
  modes: readonly Mode[];
  markers: readonly string[];
  reason: string;
}

const ROUTE_EXCLUSIONS: Readonly<Record<string, Readonly<Record<string, RouteExclusion>>>> = {
  // Settings lane (SettingsScreen staged rows, both themes): the staged
  // "pending change" rows paint .settings-field__label #353535 on #0e0e0e
  // (1.57:1, axe serious) — measured w26.k19 on /settings/library (2
  // nodes) and /settings/network (4 nodes), dark AND light.
  "settings-library": {
    "color-contrast": {
      modes: ["dark", "light"],
      markers: ['[data-staged-row="library.'],
      reason: "staged-row labels #353535 on #0e0e0e (1.57:1) — logged for the settings lane (w26.k19)",
    },
  },
  "settings-network": {
    "color-contrast": {
      modes: ["dark", "light"],
      markers: ['[data-staged-row="network.'],
      reason: "staged-row labels #353535 on #0e0e0e (1.57:1) — logged for the settings lane (w26.k19)",
    },
  },
  // Settings lane (values rows, light only): the selected values-row label
  // paints the accent #d37918 on the tinted row (1.75:1) — w26.i19's own
  // rule is that light selected labels use content-primary on tinted fills
  // (w26.d2). Dark passes (accent on the dark tint is 6+:1).
  "settings-audio": {
    "color-contrast": {
      modes: ["light"],
      markers: ["settings-values__row--selected"],
      reason:
        "light selected values-row label paints accent on the tinted row (1.75:1) — logged for the settings lane (w26.k19)",
    },
  },
};

test.use({ viewport: { width: 1280, height: 720 } });

/** Seed a completed onboarding wizard (4.F1 gate) and the theme mode, then
 * open the route. */
async function openRoute(page: Page, path: string, onboarded: boolean, mode: Mode): Promise<void> {
  const seeds: [string | null, string | null] = [
    onboarded
      ? JSON.stringify({
          complete: true,
          choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
        })
      : null,
    mode === "light" ? JSON.stringify({ "interface.theme": "light" }) : null,
  ];
  await page.addInitScript(([onboarding, theme]) => {
    if (onboarding !== null) {
      window.localStorage.setItem("lumen.onboarding.v1", onboarding);
    }
    if (theme !== null) {
      window.localStorage.setItem("lumen.settings.overrides.v1", theme);
    }
  }, seeds);
  await page.goto(path);
}

type AxeViolations = Awaited<ReturnType<AxeBuilder["analyze"]>>["violations"];

/** Split a run's violations into `unjustified` (fail the suite) and
 * `accepted` (printed, with reasons) + `stale` notes for configured
 * exclusions that matched nothing (fix landed — delete the entry). */
function triageViolations(
  routeName: string,
  mode: Mode,
  violations: AxeViolations,
): { unjustified: AxeViolations; accepted: string[]; stale: string[] } {
  const unjustified: AxeViolations = [];
  const accepted: string[] = [];
  for (const violation of violations) {
    const globalReason = JUSTIFIED_EXCLUSIONS[violation.id];
    if (globalReason !== undefined) {
      accepted.push(`${violation.id} — ${globalReason}`);
      continue;
    }
    const scoped = ROUTE_EXCLUSIONS[routeName]?.[violation.id];
    if (scoped === undefined || !scoped.modes.includes(mode)) {
      unjustified.push(violation);
      continue;
    }
    const novel = violation.nodes.filter(
      (node) => !scoped.markers.some((marker) => node.target.join(" ").includes(marker)),
    );
    if (novel.length > 0) {
      unjustified.push({ ...violation, nodes: novel });
    } else {
      accepted.push(`${violation.id} on ${routeName} (${mode}, ${violation.nodes.length} nodes) — ${scoped.reason}`);
    }
  }
  const stale = Object.entries(ROUTE_EXCLUSIONS[routeName] ?? {})
    .filter(
      ([ruleId, entry]) =>
        entry.modes.includes(mode) && !violations.some((violation) => violation.id === ruleId),
    )
    .map(([ruleId]) => `${ruleId} (${mode}) — no matching hits, delete the entry`);
  return { unjustified, accepted, stale };
}

for (const mode of MODES) {
  for (const route of ROUTES) {
    test(`axe: zero wcag2a/2aa violations on ${route.name} (${mode})`, async ({ page }) => {
      await openRoute(page, route.path, route.onboarded, mode);
      await page.waitForLoadState("networkidle");
      const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();

      const { unjustified, accepted, stale } = triageViolations(route.name, mode, results.violations);
      const format = (v: AxeViolations[number]) =>
        `${v.id} (${v.help}): ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`;
      expect(
        unjustified.map(format),
        `unjustified axe violations on ${route.path} (${mode})\naccepted: ${
          accepted.join("; ") || "none"
        }\n${stale.length > 0 ? `STALE EXCLUSIONS (delete): ${stale.join("; ")}` : ""}`,
      ).toEqual([]);
    });
  }
}

/** One keyboard-stop descriptor collected in-page per Tab press. */
interface FocusStop {
  tag: string;
  label: string;
  domIndex: number;
  /** Identity of the trapping container (nearest `[role="dialog"]` /
   * `.settings-window` ancestor) — null when the stop sits in open surface. */
  trapRoot: string | null;
  /** domIndex of the trapping container's FIRST focusable — the only legal
   * wrap landing (null when trapRoot is null). */
  trapFirstIndex: number | null;
  outlineWidth: string;
  outlineStyle: string;
  outlineColor: string;
  visible: boolean;
}

const TABBABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Read one keyboard-stop descriptor for the current focus in-page. */
async function readFocusStop(page: Page): Promise<FocusStop | null> {
  return page.evaluate((selector) => {
    const el = document.activeElement;
    if (el === null || el === document.body) return null;
    const tabbables = Array.from(document.querySelectorAll(selector));
    // Trap-aware (w26.k19, j19 J19-1): the wrap rule compares against the
    // TRAPPING CONTAINER's own first focusable, so every stop records it.
    const trapRoot = el.closest('[role="dialog"], .settings-window');
    let trapFirstIndex: number | null = null;
    if (trapRoot !== null) {
      const first = trapRoot.querySelector(selector);
      trapFirstIndex = first === null ? null : tabbables.indexOf(first);
    }
    const cs = getComputedStyle(el);
    const rect = el.getBoundingClientRect();
    return {
      tag: el.tagName.toLowerCase(),
      label: el.getAttribute("aria-label") ?? (el.textContent ?? "").trim().slice(0, 40),
      domIndex: tabbables.indexOf(el),
      trapRoot:
        trapRoot === null
          ? null
          : `${trapRoot.tagName.toLowerCase()}.${trapRoot.getAttribute("class") ?? ""}`,
      trapFirstIndex,
      outlineWidth: cs.outlineWidth,
      outlineStyle: cs.outlineStyle,
      outlineColor: cs.outlineColor,
      visible: rect.width > 0 && rect.height > 0 && cs.visibility === "visible",
    };
  }, TABBABLE_SELECTOR);
}

/** Tab through the whole screen from the body start until the walk is
 * EXHAUSTED, and report HOW it ended:
 *  - "body": focus returned to the body (open-surface screens — the walk
 *    visited every tabbable);
 *  - "wrap": a focus trap cycled back (modal surfaces — the walk visited
 *    the full cycle up to the wrap);
 *  - null: the stop budget ran out mid-walk — a silent PREFIX audit. The
 *    old harness ended here quietly (j19 J19-1: /keyboard-shortcuts
 *    measures 149 stops, so maxStops=40 "passed" while auditing ~25% of
 *    the dialog); the test now FAILS on null so budgets are raised, never
 *    trusted. */
async function tabThrough(
  page: Page,
  maxStops: number,
): Promise<{ stops: FocusStop[]; exhausted: "body" | "wrap" | null }> {
  const stops: FocusStop[] = [];
  await page.evaluate(() => {
    (document.activeElement as HTMLElement | null)?.blur();
    document.body.focus();
  });
  for (let i = 0; i < maxStops; i += 1) {
    await page.keyboard.press("Tab");
    const stop = await readFocusStop(page);
    if (stop === null) return { stops, exhausted: "body" }; // wrapped back to the body
    const prev = stops[stops.length - 1];
    if (prev !== undefined && stop.domIndex <= prev.domIndex) {
      stops.push(stop); // a trap wrap — the cycle is complete
      return { stops, exhausted: "wrap" };
    }
    stops.push(stop);
  }
  return { stops, exhausted: null };
}

/** Keyboard-traversal audit routes. `maxStops` is the AUDIT BUDGET, not a
 * cap: the walk must END (body return or validated trap wrap) inside it or
 * the test fails loudly — budgets sit ~2.5× above the measured unique stop
 * counts (w26.k19: player/playlist/settings/onboarding ≤ 40 stops,
 * open-network-stream 4, keyboard-shortcuts 149 inside the i17 Dialog
 * trap) so growth trips the failure instead of silently truncating. The
 * player covers the topbar→canvas→controls→panels order doc 08 §4.5
 * specifies; playlist/settings/onboarding cover the panel/form/wizard
 * patterns; the dialogs cover the Surface focus-trap contract (i17). */
const TAB_ROUTES: ReadonlyArray<{ name: string; path: string; onboarded: boolean; maxStops: number }> = [
  { name: "player", path: "/player", onboarded: true, maxStops: 80 },
  { name: "playlist", path: "/playlist", onboarded: true, maxStops: 80 },
  { name: "settings-global", path: "/settings/global", onboarded: true, maxStops: 80 },
  { name: "onboarding", path: "/onboarding", onboarded: false, maxStops: 80 },
  // w26.k19 (j19 J19-1): the two dialog screens join the tab audit —
  // /keyboard-shortcuts is the known-worst keyboard surface (400+ presses
  // never return to the body; the trap cycles ~149 unique stops).
  { name: "keyboard-shortcuts", path: "/keyboard-shortcuts", onboarded: true, maxStops: 400 },
  { name: "open-network-stream", path: "/open-network-stream", onboarded: true, maxStops: 80 },
];

for (const mode of MODES) {
  for (const route of TAB_ROUTES) {
    test(`keyboard: tab order is DOM order, focus visible on ${route.name} (${mode})`, async ({ page }) => {
      await openRoute(page, route.path, route.onboarded, mode);
      await page.waitForLoadState("networkidle");
      const { stops, exhausted } = await tabThrough(page, route.maxStops);

      // Every screen must have at least one interactive keyboard stop.
      expect(stops.length, `tab stops on ${route.path}`).toBeGreaterThan(0);

      // The walk must have ENDED — body return (open surface) or a trap
      // wrap (modal). A null exhaustion means the budget truncated the
      // audit: raise maxStops; the suite never passes on a prefix (J19-1).
      expect(
        exhausted,
        `tab walk on ${route.path} (${mode}) ran out of the ${route.maxStops}-stop budget without ending — the screen has more keyboard stops than budgeted; raise maxStops`,
      ).not.toBeNull();

      // DOM order == focus order (no positive-tabIndex jumps). One documented
      // exception: a MODAL FOCUS TRAP (w26.f03, e06 F8 — the settings window
      // and the i17 dialogs are floating surfaces whose Tab wraps within
      // them). A wrap is legal ONLY as a jump back to the TRAPPING
      // CONTAINER's first focusable (w26.k19, j19 J19-1: compared against
      // the container's own DOM-first tabbable, not the page minimum —
      // /keyboard-shortcuts wraps to dialog stop 21 while the walk entered
      // at 23) — anything else is a real order violation.
      for (const [i, stop] of stops.entries()) {
        if (i === 0) continue;
        const prevStop = stops[i - 1];
        if (prevStop === undefined) break; // unreachable (i > 0)
        const prev = prevStop.domIndex;
        const curr = stop.domIndex;
        if (curr <= prev) {
          expect(
            stop.trapRoot,
            `focus wrap on ${route.path} (${mode}) (stop ${i}, "${stop.label}") happens outside any focus-trap container ([role=dialog] / .settings-window)`,
          ).not.toBeNull();
          expect(
            curr,
            `focus wrap on ${route.path} (${mode}) (stop ${i}, "${stop.label}") must return to the trapping container's first focusable (domIndex ${stop.trapFirstIndex}), not jump randomly`,
          ).toBe(stop.trapFirstIndex);
          break; // traversal exhausted — the trap cycles from here
        }
        expect(
          curr,
          `focus order left DOM order between stops ${i - 1} ("${stops[i - 1]?.label}") and ${i} ("${stop.label}") on ${route.path} (${mode})`,
        ).toBeGreaterThan(prev);
      }

      // Every stop is on-screen and paints the token focus ring (2px solid).
      for (const [i, stop] of stops.entries()) {
        expect(stop.visible, `invisible tab stop #${i} (${stop.tag} "${stop.label}") on ${route.path} (${mode})`).toBe(true);
        expect(
          `${stop.outlineWidth} ${stop.outlineStyle}`,
          `stop #${i} (${stop.tag} "${stop.label}") has no visible focus ring on ${route.path} (${mode})`,
        ).toMatch(/^([2-9]|[1-9]\d*)(\.\d+)?px solid$/);
      }
    });
  }
}

// The ring paints a THEME-APPROPRIATE token, never a hardcoded color
// (w26.k19, j19 J19-3: previously dark-only). Dark: the accent token
// (#d37918, 6.18:1 on #090909). Light: --color-focus-ring (#824505, the
// h05 F1 re-point in theme/lumen.css — the accent ring measures 1.40:1 on
// light chrome, so light MUST NOT resolve to the accent; measured
// rgb(130,69,5) ≠ accent rgb(211,121,24)). Probe-compare: a throwaway
// element painted with the token var must match the focused outline.
for (const expectation of [
  { mode: "dark" as const, cssVar: "--color-accent" },
  { mode: "light" as const, cssVar: "--color-focus-ring" },
]) {
  test(`focus ring paints the ${expectation.cssVar} token in ${expectation.mode} mode (not a hardcoded color)`, async ({
    page,
  }) => {
    await openRoute(page, "/onboarding", false, expectation.mode);
    await page.waitForLoadState("networkidle");
    await page.keyboard.press("Tab");
    const colors = await page.evaluate((cssVar) => {
      const el = document.activeElement;
      const probe = document.createElement("span");
      probe.style.color = `var(${cssVar})`;
      document.body.appendChild(probe);
      const token = getComputedStyle(probe).color;
      probe.remove();
      return {
        outline: el === null ? "" : getComputedStyle(el).outlineColor,
        token,
      };
    }, expectation.cssVar);
    // The ring color must resolve to the same paint as the token.
    expect(colors.outline).toBe(colors.token);
    expect(colors.token).not.toBe(""); // token actually resolved
  });
}
