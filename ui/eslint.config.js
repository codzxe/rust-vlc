// ESLint flat config for the ui/ workspace (doc 08 §8, card 4.A1).
//
// Includes two scaffold-local rules (v1: AST-literal + regex-approximate;
// refine into proper typed rules in a follow-up session — see NEXT-STEPS.md):
//   * local/i18n-no-literals — JSX text nodes and literal `aria-label` /
//     `alt` / `title` props must not carry user-visible copy; use useMsg() +
//     MessageKey (doc 07 §6.5). Exemptions live in ui/i18n-allowlist.json as
//     { text, reason, owner } entries.
//   * local/boundary-imports — files under src/ may not relatively import
//     outside ui/src (the generated client lives inside src/generated, so it
//     stays reachable; doc 08 §8 "no Tauri APIs outside bindings").
//   * local/semantic-tokens-only — card 4.A2: no raw #hex/rgb() and no raw px
//     on font-size/radius keys in TSX; var(--semantic-token) only (doc 08
//     §3.2/§3.4). Generated + test files exempt.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

const here = dirname(fileURLToPath(import.meta.url));

// --- allowlist loading (i18n-allowlist.json) -----------------------------
const allowlistRaw = JSON.parse(
  readFileSync(resolve(here, "i18n-allowlist.json"), "utf8"),
);
if (!Array.isArray(allowlistRaw)) {
  throw new Error("ui/i18n-allowlist.json must be a JSON array of { text, reason, owner }");
}
const allowlistedTexts = new Set(
  allowlistRaw
    .filter((e) => typeof e === "object" && e !== null && "text" in e)
    .map((e) => e.text),
);

const isTestFile = (filename) => /\.test\.[cm]?[jt]sx?$/.test(filename);

// --- local rule: i18n-no-literals ----------------------------------------
const i18nNoLiterals = {
  meta: {
    type: "problem",
    docs: { description: "user-visible copy must come from useMsg() + MessageKey" },
    schema: [],
    messages: {
      literal:
        "User-visible string literal is not allowed here — use useMsg() with a MessageKey (doc 07 §6.5), or add an allowlist entry in ui/i18n-allowlist.json.",
    },
  },
  create(context) {
    if (isTestFile(context.filename ?? "")) {
      return {};
    }
    const check = (node, text) => {
      const trimmed = text.trim();
      if (trimmed.length > 0 && !allowlistedTexts.has(trimmed)) {
        context.report({ node, messageId: "literal" });
      }
    };
    return {
      JSXText(node) {
        check(node, node.value);
      },
      JSXAttribute(node) {
        const name = node.name.name;
        if (name !== "aria-label" && name !== "alt" && name !== "title") {
          return;
        }
        if (
          node.value !== null &&
          node.value.type === "Literal" &&
          typeof node.value.value === "string"
        ) {
          check(node, node.value.value);
        }
      },
    };
  },
};

// --- local rule: boundary-imports ----------------------------------------
const srcRoot = resolve(here, "src");
const boundaryImports = {
  meta: {
    type: "problem",
    docs: { description: "src/ modules may not import outside ui/src" },
    schema: [],
    messages: {
      outsideSrc:
        "Import escapes ui/src — UI code must stay within the workspace boundary (doc 08 §8).",
      absolute:
        "Absolute-path imports are not allowed in src/ — use package specifiers or relative paths.",
    },
  },
  create(context) {
    const filename = context.filename ?? "";
    return {
      ImportDeclaration(node) {
        const spec = node.source.value;
        if (typeof spec !== "string") {
          return;
        }
        if (spec.startsWith("/")) {
          context.report({ node, messageId: "absolute" });
          return;
        }
        if (!spec.startsWith(".")) {
          return; // bare package specifier — allowed
        }
        const target = resolve(dirname(filename), spec);
        if (target !== srcRoot && !target.startsWith(srcRoot + "/")) {
          context.report({ node, messageId: "outsideSrc" });
        }
      },
    };
  },
};

// --- local rule: semantic-tokens-only (card 4.A2, doc 08 §3.2/§3.4) ------
// TSX source may express colors and font-size/radius dimensions ONLY through
// semantic tokens — `var(--…)` (inline) or the generated token CSS classes.
// Raw `#hex`, raw `rgb()/rgba()`, and raw `px` values on font-size/radius
// keys are rejected. Exemptions: test files and generated files (the token
// CSS itself is emitter output); plain .css files are Stylelint territory
// (doc 08 §3.4, follow-up session). v1: literal + property-context checks;
// see NEXT-STEPS.md for the typed-rule promotion.
const HEX_OR_RGB_RE = /#[0-9a-fA-F]{3,8}\b|rgba?\(/;
const PX_RE = /\b\d+(?:\.\d+)?px\b/;
const DIMENSION_KEY_RE = /(?:font-?size|radius)/i;

const semanticTokensOnly = {
  meta: {
    type: "problem",
    docs: {
      description:
        "colors and font-size/radius dimensions must come from semantic tokens — var(--…) only",
    },
    schema: [],
    messages: {
      rawColor:
        "Raw color literal — components consume SEMANTIC tokens only (var(--…), doc 08 §3.2).",
      rawDimension:
        "Raw px on a font-size/radius property — use a semantic token (var(--…), doc 08 §3.2).",
    },
  },
  create(context) {
    const filename = context.filename ?? "";
    if (
      isTestFile(filename) ||
      /([\\/])generated\1/.test(filename) ||
      !filename.endsWith(".tsx")
    ) {
      return {}; // tests + generated emitter output are exempt
    }
    const checkColorLiteral = (node, text) => {
      if (typeof text === "string" && HEX_OR_RGB_RE.test(text)) {
        context.report({ node, messageId: "rawColor" });
      }
    };
    const checkDimensionLiteral = (node, keyName, value) => {
      if (keyName !== null && typeof value === "string" && PX_RE.test(value)) {
        context.report({ node, messageId: "rawDimension" });
      }
    };
    const literalValue = (expr) =>
      expr !== null && expr.type === "Literal" && typeof expr.value === "string" ? expr.value : null;
    return {
      Literal(node) {
        if (typeof node.value === "string") {
          checkColorLiteral(node, node.value);
        }
      },
      TemplateLiteral(node) {
        checkColorLiteral(node, node.quasis.map((q) => q.value.raw).join(""));
      },
      Property(node) {
        if (node.key.type !== "Identifier" && node.key.type !== "Literal") {
          return;
        }
        const keyName = node.key.type === "Identifier" ? node.key.name : String(node.key.value);
        if (!DIMENSION_KEY_RE.test(keyName)) {
          return;
        }
        checkDimensionLiteral(node.value, keyName, literalValue(node.value));
      },
      JSXAttribute(node) {
        const name = node.name.name;
        if (typeof name !== "string" || !DIMENSION_KEY_RE.test(name)) {
          return;
        }
        checkDimensionLiteral(node.value, name, literalValue(node.value));
      },
    };
  },
};

const local = {
  rules: {
    "i18n-no-literals": i18nNoLiterals,
    "boundary-imports": boundaryImports,
    "semantic-tokens-only": semanticTokensOnly,
  },
};

export default [
  { ignores: ["dist/", "node_modules/", "coverage/", "playwright-report/", "test-results/"] },
  {
    // node-flavored scripts (token generator) — plain globals, no DOM.
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: { console: "readonly", process: "readonly" } },
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
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
