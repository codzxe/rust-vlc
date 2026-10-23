// ESLint flat config for the ui/ workspace (doc 08 §8, card 4.A1).
//
// Includes two scaffold-local rules (v2, closure session — still inline in
// this file, promotion to tools/eslint-plugin-lumen/ remains NEXT-STEPS #2):
//   * local/i18n-no-literals — user-visible copy must come from useMsg() +
//     MessageKey (doc 07 §6.5). v2 covers: JSX text nodes, no-substitution
//     template literals, string-literal expression containers in text
//     position ({"Copy"}), and literal/expression values of the
//     user-visible attrs aria-label/alt/title. NOT yet covered:
//     interpolated templates (`{`Hi ${name}`}` needs flow awareness) and
//     string literals passed to DOM-building helpers — document, don't fake.
//     Exemptions: ui/i18n-allowlist.json { text, reason, owner } entries.
//   * local/boundary-imports — files under src/ may not relatively import
//     outside ui/src (the generated client lives inside src/generated, so it
//     stays reachable; doc 08 §8 "no Tauri APIs outside bindings").

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
    // String content of a literal-ish node: "..." or `...` (no substitutions).
    const literalText = (node) => {
      if (node.type === "Literal" && typeof node.value === "string") {
        return node.value;
      }
      if (
        node.type === "TemplateLiteral" &&
        node.expressions.length === 0 &&
        node.quasis.length === 1
      ) {
        return node.quasis[0].value.cooked ?? "";
      }
      return null;
    };
    return {
      JSXText(node) {
        check(node, node.value);
      },
      // {"Copy"} in text position (child of a JSX element, not an attribute).
      JSXExpressionContainer(node) {
        if (node.parent?.type !== "JSXElement") {
          return;
        }
        const text = literalText(node.expression);
        if (text !== null) {
          check(node, text);
        }
      },
      JSXAttribute(node) {
        const name = node.name.name;
        if (name !== "aria-label" && name !== "alt" && name !== "title") {
          return;
        }
        const value =
          node.value?.type === "JSXExpressionContainer"
            ? node.value.expression // alt={"..."}
            : node.value; // alt="..."
        const text = value !== null && value !== undefined ? literalText(value) : null;
        if (text !== null) {
          check(node, text);
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

const local = { rules: { "i18n-no-literals": i18nNoLiterals, "boundary-imports": boundaryImports } };

export default [
  { ignores: ["dist/", "node_modules/", "coverage/", "playwright-report/", "test-results/"] },
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
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "react-refresh/only-export-components": "warn",
    },
  },
];
