import react from "@vitejs/plugin-react";
import pkg from "./package.json";
import { defineConfig } from "vitest/config";

// Doc 08 §8 toolchain: Vite (dev/build) + vitest with jsdom + RTL for unit
// tests. Playwright lives in playwright.config.ts (e2e).
// Build-info defines (card 4.E4 About panel): the UI package manifest is read
// HERE at config load and injected as constants — UI code never imports
// outside ui/src (boundary rule) and never hardcodes a version that could
// drift from the package it ships in.

export default defineConfig({
  plugins: [react()],
  define: {
    __UI_PACKAGE_NAME__: JSON.stringify(pkg.name),
    __UI_PACKAGE_VERSION__: JSON.stringify(pkg.version),
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    // src/**: component + logic tests; tools/eslint-plugin-local/**: unit
    // tests for the promoted local ESLint rules (issue #36).
    include: [
      "src/**/*.test.{ts,tsx}",
      "tools/eslint-plugin-local/**/*.test.js",
    ],
  },
});
