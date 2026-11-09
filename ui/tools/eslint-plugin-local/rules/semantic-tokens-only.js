// local/semantic-tokens-only (card 4.A2, doc 08 §3.2/§3.4) — promoted
// verbatim from ui/eslint.config.js (issue #36; previously inline). No
// adaptations were needed: the rule is path-free.
//
// TSX source may express colors and font-size/radius dimensions ONLY through
// semantic tokens — `var(--…)` (inline) or the generated token CSS classes.
// Raw `#hex`, raw `rgb()/rgba()`, and raw `px` values on font-size/radius
// keys are rejected. Exemptions: test files and generated files (the token
// CSS itself is emitter output); plain .css files are Stylelint territory
// (doc 08 §3.4 — ui/stylelint.config.js since issue #36). v1: literal +
// property-context checks; see NEXT-STEPS.md for the typed-rule promotion.

const HEX_OR_RGB_RE = /#[0-9a-fA-F]{3,8}\b|rgba?\(/;
const PX_RE = /\b\d+(?:\.\d+)?px\b/;
const DIMENSION_KEY_RE = /(?:font-?size|radius)/i;
const isTestFile = (filename) => /\.test\.[cm]?[jt]sx?$/.test(filename);

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

export default semanticTokensOnly;
