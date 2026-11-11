// local/i18n-no-literals — promoted verbatim from ui/eslint.config.js
// (issue #36; previously inline). Rule semantics are unchanged; the only
// adaptations for the move are the module's export form and the anchor of
// the allowlist path, which now resolves relative to this file
// (tools/eslint-plugin-local/rules/ → ui/).
//
// User-visible copy must come from useMsg() + MessageKey (doc 07 §6.5).
// v2 covers: JSX text nodes, no-substitution template literals,
// string-literal expression containers in text position ({"Copy"}), and
// literal/expression values of the user-visible attrs aria-label/alt/title.
// NOT yet covered: interpolated templates (`{`Hi ${name}`}` needs flow
// awareness) and string literals passed to DOM-building helpers —
// document, don't fake. Exemptions: ui/i18n-allowlist.json
// { text, reason, owner } entries.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

// --- allowlist loading (i18n-allowlist.json) -----------------------------
const allowlistRaw = JSON.parse(
  readFileSync(resolve(here, "../../../i18n-allowlist.json"), "utf8"),
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

export default i18nNoLiterals;
