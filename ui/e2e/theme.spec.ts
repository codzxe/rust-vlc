import { expect, test } from "@playwright/test";

// Card 4.A2 AC: theme-switch E2E. The ThemeProvider must drive the root
// element attributes (doc 08 §3.3) and the generated token CSS must actually
// cascade in a real browser — asserted through computed styles on the
// /dev/tokens sandbox (card 4.A2 dev fixture, not a product screen).

test("default theme lands on <html> as dark + orange", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).toHaveAttribute("data-accent", "orange");
});

test("switching mode + accent swaps the CSS variable cascade", async ({ page }) => {
  await page.goto("/dev/tokens");
  const swatch = page.getByTestId("swatch-accent");

  // dark + orange default: accent resolves to the product default #d37918.
  await expect(swatch).toHaveCSS("background-color", "rgb(211, 121, 24)");

  // light mode: surface + accent-bright re-point (doc 08 §3.3).
  await page.getByRole("button", { name: "light", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.getByTestId("swatch-surface-raised")).toHaveCSS(
    "background-color",
    "rgb(190, 190, 190)",
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
