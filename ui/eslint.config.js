// ESLint flat config for the ui/ workspace (doc 08 §8, card 4.A1).
//
// The three scaffold-local rules (i18n-no-literals, boundary-imports,
// semantic-tokens-only) now live in the tested package
// ui/tools/eslint-plugin-local/ (issue #36) — promoted verbatim from this
// file, unit-tested via vitest (rules/*.test.js). This config imports that
// package and registers it under the `local` plugin key, so rule addresses
// (local/<name>) are unchanged and there is a single source of truth.

import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";
import local from "./tools/eslint-plugin-local/index.js";

export default [
  {
    ignores: [
      "dist/",
      "node_modules/",
      "coverage/",
      "playwright-report/",
      "test-results/",
      // web-matrix staging output (card 7.I1): scripts/web-matrix-run.mjs
      // copies the committed site + the engine wasm + glue + fixture here;
      // never linted (build output, same class as dist/ — .gitignored too).
      "e2e/web/sitedist/",
    ],
  },
  {
    // node-flavored scripts (token generator) — plain globals, no DOM.
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: { console: "readonly", process: "readonly" } },
  },
  {
    // e2e/web/site/*.js — the browser-matrix harness page (card 7.I1,
    // issue #49): hand-written in-page tooling (the apps/web worker.js
    // precedent) that runs IN THE BROWSER — Worker/fetch/document
    // globals are the environment, node globals are absent.
    files: ["e2e/web/site/**/*.js"],
    languageOptions: {
      globals: {
        console: "readonly",
        window: "readonly",
        document: "readonly",
        navigator: "readonly",
        Worker: "readonly",
        fetch: "readonly",
        performance: "readonly",
        AudioContext: "readonly",
        crossOriginIsolated: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
      },
    },
  },
  {
    // web-matrix-run.mjs (card 7.I1): node-flavored driver, but it parses
    // request URLs with the WHATWG URL global (node exposes it ungated).
    files: ["scripts/web-matrix-run.mjs"],
    languageOptions: {
      globals: { console: "readonly", process: "readonly", URL: "readonly" },
    },
  },
  {
    // l10n-pseudo-probe.mjs is a Playwright driver: node-side timers +
    // a page.evaluate() callback whose body runs IN THE BROWSER (document/
    // window/getComputedStyle are browser globals there, not node). Hygiene
    // fix by P4-GATE (phase-4 gate): pre-existing no-undef errors on main
    // @ be4e4c6 blocked `npm run lint`; behavior-neutral config-only change.
    files: ["scripts/l10n-pseudo-probe.mjs"],
    languageOptions: {
      globals: {
        console: "readonly",
        process: "readonly",
        setTimeout: "readonly",
        document: "readonly",
        window: "readonly",
        getComputedStyle: "readonly",
      },
    },
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    // Generated emitter output (gen-settings-screens.mjs, gen-hotkeys-data.mjs,
    // gen-tokens.mjs) carries a blanket `/* eslint-disable */` emitted by the
    // GENERATORS and is drift-checked byte-for-byte against its sources
    // (`npm run settings:check` / `hotkeys:check` / `tokens:check`) — it may
    // not be hand-edited, and today's output triggers no rules, so the
    // blanket directive reads as "unused". Stale-directive hygiene for these
    // files belongs to the generator scripts (separate owner), not to lint
    // consumers; scoped here instead of editing generated files (issue #36).
    files: ["src/**/generated/**"],
    linterOptions: { reportUnusedDisableDirectives: "off" },
  },
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: { local, "react-hooks": reactHooks, "react-refresh": reactRefresh },
    languageOptions: {
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
    rules: {
      "local/i18n-no-literals": "error",
      "local/boundary-imports": "error",
      "local/semantic-tokens-only": "error",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "react-refresh/only-export-components": "warn",
    },
  },
];
