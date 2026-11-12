import { expect, test } from "@playwright/test";

// Card 4.A2 AC: theme-switch E2E. The ThemeProvider must drive the root
// element attributes (doc 08 §3.3) and the generated token CSS must actually
// cascade in a real browser — asserted through computed styles on the
// /dev/tokens sandbox (card 4.A2 dev fixture, not a product screen).
//
// W25/W26 re-pin (2026-11-12): expectations re-aligned to the design-
// authoritative default, dark + ORANGE (#d37918) — the W25 Figma redesign
// accent (docs/waves/w26/CONTRACT.md §2). The earlier yellow #f2c41d was
// the dead Solstice skin. Light-mode accent-bright values follow the
// w26.f15 WCAG contrast repin (e.g. orange light bright #b35f0d, 4.2:1)
// and the yellow/gray ramps were re-mined from the Figma frames. Truth
// source: themes/generated/tokens.css (pipeline output of themes/tokens.json).

test("default theme lands on <html> as dark + orange", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).toHaveAttribute("data-accent", "orange");
});

test("switching mode + accent swaps the CSS variable cascade", async ({ page }) => {
  await page.goto("/dev/tokens");
  const swatch = page.getByTestId("swatch-accent");

  // dark + orange default: accent resolves to the product default #d37918
  // (W25 redesign accent — the :root value; no [data-accent="orange"] block).
  await expect(swatch).toHaveCSS("background-color", "rgb(211, 121, 24)");

  // light mode: surface + accent-bright re-point (doc 08 §3.3; W25 light
  // surface-raised is the frame's warm gray #dedede — the g03/g05 Lead
  // light-ramp repin; the WCAG-repinned light bright #b35f0d is proven by
  // the matrix below).
  await page.getByRole("button", { name: "light", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.getByTestId("swatch-surface-raised")).toHaveCSS(
    "background-color",
    "rgb(222, 222, 222)",
  );

  // violet accent: the accent role re-points to #8e42ff.
  await page.getByRole("button", { name: "violet", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-accent", "violet");
  await expect(swatch).toHaveCSS("background-color", "rgb(142, 66, 255)");

  // back to dark: mode and accent are independent axes.
  await page.getByRole("button", { name: "dark", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(swatch).toHaveCSS("background-color", "rgb(142, 66, 255)");
});

test("auto mode follows the emulated OS color scheme", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/dev/tokens");
  await page.getByRole("button", { name: "auto", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");

  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

// Card 4.A2 closure AC: the FULL visual matrix — every accent (9) × every
// mode (2), 18 combinations, asserted through computed styles on the
// /dev/tokens sandbox swatches. Expected values are the semantic accent
// roles as resolved from themes/tokens.json via the committed pipeline
// output (`themes/generated/tokens.css` → `src/tokens/generated/`):
// `--color-accent` is mode-independent (each accent's ramp sets one
// primary; light only re-points bright/icon/tinted), while
// `--color-accent-bright` differs per mode — so the pair proves both the
// accent axis AND the mode axis cascade for every combination.
// [accent, accentDark, accentLight, brightDark, brightLight] — w26 Lead
// pipeline change: dark accent-bright is the contract §2 hover (derived
// hsl+0.02/+0.045 per accent); orange light-bright is the f15 WCAG repin.
const ACCENT_MATRIX = [
  ["orange", "#d37918", "#d37918", "#e8851a", "#b35f0d"],
  ["red", "#e15032", "#e15032", "#e66044", "#e66044"],
  ["yellow", "#c49e23", "#c49e23", "#daaf24", "#daaf24"],
  ["green", "#19a234", "#19a234", "#1ab839", "#1ab839"],
  ["cyan", "#18aad3", "#18aad3", "#1abbe8", "#1abbe8"],
  ["blue", "#185dd3", "#185dd3", "#1a66e8", "#1a66e8"],
  ["violet", "#8e42ff", "#8e42ff", "#9c59ff", "#9c59ff"],
  ["pink", "#f052b3", "#f052b3", "#f365bd", "#f365bd"],
  ["gray", "#afafaf", "#afafaf", "#bababa", "#bababa"],
] as const;

function hexToRgb(hex: string): string {
  const n = Number.parseInt(hex.slice(1), 16);
  return `rgb(${(n >> 16) & 0xff}, ${(n >> 8) & 0xff}, ${n & 0xff})`;
}

for (const [accent, accentDark, accentLight, brightDark, brightLight] of ACCENT_MATRIX) {
  for (const mode of ["dark", "light"] as const) {
    test(`matrix: ${accent} × ${mode} resolves the accent cascade`, async ({
      page,
    }) => {
      await page.goto("/dev/tokens");
      const swatch = page.getByTestId("swatch-accent");
      const bright = page.getByTestId("swatch-accent-bright");

      await page.getByRole("button", { name: mode, exact: true }).click();
      await page.getByRole("button", { name: accent, exact: true }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", mode);
      await expect(page.locator("html")).toHaveAttribute("data-accent", accent);

      // --color-accent is the mode-independent accent primary…
      const expectedAccent = mode === "dark" ? accentDark : accentLight;
      await expect(swatch).toHaveCSS(
        "background-color",
        hexToRgb(expectedAccent),
      );
      // …--color-accent-bright proves the per-mode block layered on top
      // (dark = the contract §2 hover since the w26 Lead pipeline change).
      const expectedBright = mode === "dark" ? brightDark : brightLight;
      await expect(bright).toHaveCSS(
        "background-color",
        hexToRgb(expectedBright),
      );
    });
  }
}
