import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

// Card 4.F4 — axe/Playwright a11y suite + keyboard-navigation audit across
// EVERY routable screen (doc 08 §4.5, doc 13 §7 row 4.F4).
//
// Rule set: the enforced tags are wcag2a + wcag2aa — the WCAG 2.1 A/AA
// subset this pass is accountable for (doc 08 §9.4: "zero critical
// violations"). `best-practice`-only rules (region, page-has-heading-one…)
// are NOT enforced; screens that gain landmarks/headings as they harden
// will simply pass the stricter future set.
//
// Exclusions are NEVER silent: any rule id accepted with a justification is
// listed in JUSTIFIED_EXCLUSIONS with the reason, and the assertion prints
// the justified hits so a stale entry surfaces on the next run.

/** Every routable screen (doc 08 §1.6 inventory). `onboarded` routes are
 * behind the 4.F1 first-run gate and get a seeded completed-wizard store. */
const ROUTES: ReadonlyArray<{ name: string; path: string; onboarded: boolean }> = [
  { name: "player", path: "/player", onboarded: true },
  { name: "player-music", path: "/player/music", onboarded: true },
  { name: "miniplayer", path: "/miniplayer", onboarded: true },
  { name: "playlist", path: "/playlist", onboarded: true },
  { name: "media-info", path: "/media-info", onboarded: true },
  { name: "settings-global", path: "/settings/global", onboarded: true },
  { name: "settings-interface", path: "/settings/interface", onboarded: true },
  { name: "settings-privacy", path: "/settings/privacy", onboarded: true },
  { name: "settings-playback", path: "/settings/playback", onboarded: true },
  { name: "settings-performance", path: "/settings/performance", onboarded: true },
  { name: "onboarding", path: "/onboarding", onboarded: false },
];

/** Axe rule ids accepted with a documented reason. Empty as of 4.F4 — the
 * whole wcag2a/2aa set passes on every route. To accept a finding, add
 * `ruleId: "why this is OK (e.g. third-party iframe, tooling false
 * positive)"` — the assertion prints hits so stale entries surface. */
const JUSTIFIED_EXCLUSIONS: Readonly<Record<string, string>> = {};

test.use({ viewport: { width: 1280, height: 720 } });

/** Seed a completed onboarding wizard (4.F1 gate) then open the route. */
async function openRoute(page: Page, path: string, onboarded: boolean): Promise<void> {
  if (onboarded) {
    await page.addInitScript(() => {
      window.localStorage.setItem(
        "lumen.onboarding.v1",
        JSON.stringify({
          complete: true,
          choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
        }),
      );
    });
  }
  await page.goto(path);
}

for (const route of ROUTES) {
  test(`axe: zero wcag2a/2aa violations on ${route.name}`, async ({ page }) => {
    await openRoute(page, route.path, route.onboarded);
    await page.waitForLoadState("networkidle");
    const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();

    const unjustified = results.violations.filter((v) => !(v.id in JUSTIFIED_EXCLUSIONS));
    const format = (v: (typeof results.violations)[number]) =>
      `${v.id} (${v.help}): ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`;
    expect(
      unjustified.map(format),
      `unjustified axe violations on ${route.path}\njustified hits: ${results.violations
        .filter((v) => v.id in JUSTIFIED_EXCLUSIONS)
        .map((v) => `${v.id} — ${JUSTIFIED_EXCLUSIONS[v.id]}`)
        .join("; ") || "none"}`,
    ).toEqual([]);
  });
}

/** One keyboard-stop descriptor collected in-page per Tab press. */
interface FocusStop {
  tag: string;
  label: string;
  domIndex: number;
  outlineWidth: string;
  outlineStyle: string;
  outlineColor: string;
  visible: boolean;
}

/** Read one keyboard-stop descriptor for the current focus in-page. */
async function readFocusStop(page: Page): Promise<FocusStop | null> {
  return page.evaluate(() => {
    const el = document.activeElement;
    if (el === null || el === document.body) return null;
    const tabbables = Array.from(
      document.querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    );
    const cs = getComputedStyle(el);
    const rect = el.getBoundingClientRect();
    return {
      tag: el.tagName.toLowerCase(),
      label: el.getAttribute("aria-label") ?? (el.textContent ?? "").trim().slice(0, 40),
      domIndex: tabbables.indexOf(el),
      outlineWidth: cs.outlineWidth,
      outlineStyle: cs.outlineStyle,
      outlineColor: cs.outlineColor,
      visible: rect.width > 0 && rect.height > 0 && cs.visibility === "visible",
    };
  });
}

/** Tab through the whole screen from the body start; returns every stop. */
async function tabThrough(page: Page, maxStops = 40): Promise<FocusStop[]> {
  const stops: FocusStop[] = [];
  await page.evaluate(() => {
    (document.activeElement as HTMLElement | null)?.blur();
    document.body.focus();
  });
  for (let i = 0; i < maxStops; i += 1) {
    await page.keyboard.press("Tab");
    const stop = await readFocusStop(page);
    if (stop === null) break; // wrapped back to the body — screen exhausted
    stops.push(stop);
  }
  return stops;
}

// Representative screens for the traversal audit (player covers the
// topbar→canvas→controls→panels order doc 08 §4.5 specifies; playlist,
// settings and onboarding cover the panel/form/wizard patterns; the other
// settings areas share the settings framework and its primitives).
const TAB_ROUTES = ROUTES.filter((r) =>
  ["player", "playlist", "settings-global", "onboarding"].includes(r.name),
);

for (const route of TAB_ROUTES) {
  test(`keyboard: tab order is DOM order, focus visible on ${route.name}`, async ({ page }) => {
    await openRoute(page, route.path, route.onboarded);
    await page.waitForLoadState("networkidle");
    const stops = await tabThrough(page);

    // Every screen must have at least one interactive keyboard stop.
    expect(stops.length, `tab stops on ${route.path}`).toBeGreaterThan(0);

    // DOM order == focus order (no positive-tabIndex jumps). One documented
    // exception: a MODAL FOCUS TRAP (w26.f03, e06 F8 — the settings window
    // is a floating surface whose Tab wraps within it). A wrap is legal ONLY
    // as a jump back to the trapped container's FIRST focusable (the lowest
    // DOM index recorded) — anything else is a real order violation.
    const firstDomIndex = Math.min(...stops.map((s) => s.domIndex));
    for (const [i, stop] of stops.entries()) {
      if (i === 0) continue;
      const prev = stops[i - 1]?.domIndex;
      const curr = stop.domIndex;
      if (prev === undefined || curr === undefined) break;
      if (curr < prev) {
        // Trap wrap: must land exactly on the window's first focusable.
        expect(
          curr,
          `focus wrap on ${route.path} (stop ${i}, "${stop.label}") must return to the trapped window's first focusable, not jump randomly`,
        ).toBe(firstDomIndex);
        break; // traversal exhausted — the trap cycles from here
      }
      expect(
        curr,
        `focus order left DOM order between stops ${i - 1} ("${stops[i - 1]?.label}") and ${i} ("${stop.label}") on ${route.path}`,
      ).toBeGreaterThan(prev);
    }

    // Every stop is on-screen and paints the token focus ring (2px solid).
    for (const [i, stop] of stops.entries()) {
      expect(stop.visible, `invisible tab stop #${i} (${stop.tag} "${stop.label}") on ${route.path}`).toBe(true);
      expect(
        `${stop.outlineWidth} ${stop.outlineStyle}`,
        `stop #${i} (${stop.tag} "${stop.label}") has no visible focus ring on ${route.path}`,
      ).toMatch(/^([2-9]|[1-9]\d*)(\.\d+)?px solid$/);
    }
  });
}

test("focus ring paints the --color-accent token (not a hardcoded color)", async ({ page }) => {
  await openRoute(page, "/onboarding", false);
  await page.waitForLoadState("networkidle");
  await page.keyboard.press("Tab");
  const colors = await page.evaluate(() => {
    const el = document.activeElement;
    const probe = document.createElement("span");
    probe.style.color = "var(--color-accent)";
    document.body.appendChild(probe);
    const accent = getComputedStyle(probe).color;
    probe.remove();
    return {
      outline: el === null ? "" : getComputedStyle(el).outlineColor,
      accent,
    };
  });
  // The ring color must resolve to the same paint as the accent token.
  expect(colors.outline).toBe(colors.accent);
  expect(colors.accent).not.toBe(""); // token actually resolved
});
