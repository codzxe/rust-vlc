import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Doc 08 §8 toolchain: Vite (dev/build) + vitest with jsdom + RTL for unit
// tests. Playwright lives in playwright.config.ts (e2e).
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
