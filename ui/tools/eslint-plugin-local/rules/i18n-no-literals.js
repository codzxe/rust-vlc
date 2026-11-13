// local/i18n-no-literals — promoted verbatim from ui/eslint.config.js
// (issue #36; previously inline). Rule semantics are unchanged; the only
// adaptations for the move are the module's export form and the anchor of
// the allowlist path, which now resolves relative to this file
// (tools/eslint-plugin-local/rules/ → ui/).
//
// User-visible copy must come from useMsg() + MessageKey (doc 07 §6.5).
// v3 (w26.k20, j20 P2-7 + P3-14) covers: JSX text nodes, no-substitution
// template literals, string-literal expression containers in text position
// ({"Copy"}), and literal/expression values of the user-visible attrs
// aria-label, alt, title, placeholder, aria-placeholder, aria-valuetext and
// aria-description. (`placeholder` was a v2 miss with a live untranslated
// string — OnboardingScreen "en, de" — when j20 reviewed; the aria-* trio
// were the documented v2 limits. `data-*` attrs are NOT covered on purpose:
// they are test/dev markers, never rendered.) Still NOT covered:
// interpolated templates (`{`Hi ${name}`}` needs flow awareness) and string
// literals passed to DOM-building helpers — document, don't fake.
// Exemptions: ui/i18n-allowlist.json — every entry is FILE-SCOPED since v3:
// { text, files: ["src/…"], reason, owner } exempts that exact text ONLY in
// the listed files (paths relative to ui/). v2 matched text globally, so a
// stray "/" breadcrumb in any file sailed through; an entry without a
// non-empty `files` array is now a load-time config error.

import { readFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, "../../..");

// --- allowlist loading (i18n-allowlist.json) -----------------------------
const allowlistRaw = JSON.parse(
  readFileSync(resolve(here, "../../../i18n-allowlist.json"), "utf8"),
);
if (!Array.isArray(allowlistRaw)) {
  throw new Error("ui/i18n-allowlist.json must be a JSON array of { text, files, reason, owner }");
}
/** text → set of files (ui/-relative, forward slashes) allowed to use it. */
const allowlistedTexts = new Map();
for (const entry of allowlistRaw) {
  if (typeof entry !== "object" || entry === null || typeof entry.text !== "string") {
    continue; // l10n-check reports the shape error with file/line context
  }
  if (
    !Array.isArray(entry.files) ||
    entry.files.length === 0 ||
    entry.files.some((f) => typeof f !== "string" || f.trim().length === 0)
  ) {
    throw new Error(
      `ui/i18n-allowlist.json entry "${entry.text}" must carry a non-empty files: string[] ` +
        "(v3 file scoping, w26.k20 — a text-only entry would exempt every file)",
    );
  }
  allowlistedTexts.set(entry.text, new Set(entry.files));
}

const isTestFile = (filename) => /\.test\.[cm]?[jt]sx?$/.test(filename);

/** ui/-relative, forward-slashed path for a linted file (RuleTester passes
 * bare relative paths; real runs pass absolutes). */
const relToUi = (filename) =>
  relative(uiRoot, filename).split(/[\\/]/).join("/");

const i18nNoLiterals = {
  meta: {
    type: "problem",
    docs: { description: "user-visible copy must come from useMsg() + MessageKey" },
    schema: [],
    messages: {
      literal:
        "User-visible string literal is not allowed here — use useMsg() with a MessageKey (doc 07 §6.5), or add a file-scoped entry in ui/i18n-allowlist.json.",
    },
  },
  create(context) {
    const filename = context.filename ?? "";
    if (isTestFile(filename)) {
      return {};
    }
    const relFile = relToUi(filename);
    const check = (node, text) => {
      const trimmed = text.trim();
      if (trimmed.length === 0) {
        return;
      }
      const files = allowlistedTexts.get(trimmed);
      if (files !== undefined && files.has(relFile)) {
        return; // allowlisted for THIS file only (v3 scoping)
      }
      context.report({ node, messageId: "literal" });
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
        if (
          name !== "aria-label" &&
          name !== "alt" &&
          name !== "title" &&
          name !== "placeholder" &&
          name !== "aria-placeholder" &&
          name !== "aria-valuetext" &&
          name !== "aria-description"
        ) {
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
