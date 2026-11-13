// local/semantic-tokens-only (card 4.A2, doc 08 §3.2/§3.4) — promoted
// verbatim from ui/eslint.config.js (issue #36; previously inline). No
// adaptations were needed: the rule is path-free.
//
// TS source may express colors and font-size/radius dimensions ONLY through
// semantic tokens — `var(--…)` (inline) or the generated token CSS classes.
// Raw `#hex`, raw `rgb()/rgba()`, and raw `px` values on font-size/radius
// keys are rejected. Exemptions: test files and generated files (the token
// CSS itself is emitter output); plain .css files are Stylelint territory
// (doc 08 §3.4 — ui/stylelint.config.js since issue #36). v1: literal +
// property-context checks; see NEXT-STEPS.md for the typed-rule promotion.
//
// v3 (w26.k20, j20 P3-15): the rule now covers `.ts` files too — v2 skipped
// everything but .tsx, so raw hex in .ts helpers (canvas paint, data
// modules) escaped both lint systems. Frame-measured ports are exempt per
// contract §4.4: a literal preceded by (or sharing a line with) a
// `frame:`-citing block comment naming a frame is allowed — that is the
// documented marker for "this exact value was measured in this exact
// frame" (e.g. src/screens/player/tools/video/frameData.ts, the
// Video - Colors.svg threshold hexes). The frame citation exempts the
// literal's nearest commented ancestor — a data object with one citation
// may carry several frame-measured values, but a stray hex with no citation
// still fails.
//
// The message-catalog surface (src/i18n/**) is exempt from the COLOR check:
// catalog VALUES are displayed copy, not paint — e.g. the subtitles
// text-color field legitimately shows its current hex ("#FFFFFF") as the
// value text. The discipline is enforced at the CONSUMPTION site (component
// code), which is what this rule reads; the catalog is data.

const HEX_OR_RGB_RE = /#[0-9a-fA-F]{3,8}\b|rgba?\(/;
const PX_RE = /\b\d+(?:\.\d+)?px\b/;
const DIMENSION_KEY_RE = /(?:font-?size|radius)/i;
const FRAME_MARKER_RE = /frame\s*:/i;
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
        "Raw color literal — components consume SEMANTIC tokens only (var(--…), doc 08 §3.2). Frame-measured ports must carry a `frame:` block-comment citation (contract §4.4).",
      rawDimension:
        "Raw px on a font-size/radius property — use a semantic token (var(--…), doc 08 §3.2).",
    },
  },
  create(context) {
    const filename = context.filename ?? "";
    if (
      isTestFile(filename) ||
      /([\\/])generated\1/.test(filename) ||
      /([\\/])i18n\1/.test(filename) ||
      !(filename.endsWith(".tsx") || filename.endsWith(".ts"))
    ) {
      return {}; // tests + generated emitter output are exempt; css = stylelint;
      // src/i18n/** is catalog data (displayed copy), not paint-site code
    }
    const sourceCode = context.sourceCode ?? context.getSourceCode();
    /** Contract §4.4 escape: a `frame:`-citing comment on the literal's
     * own line, or as the nearest leading comment up the ancestor chain
     * (stopped at Program — a file-header comment never exempts the file).
     * The nearest ancestor that HAS leading comments decides: a citation
     * naming a frame exempts, any other comment does not. */
    const isFrameMarked = (node) => {
      const line = sourceCode.lines?.[node.loc?.start.line - 1] ?? "";
      if (FRAME_MARKER_RE.test(line)) {
        return true;
      }
      for (let cur = node; cur !== null && cur !== undefined && cur.type !== "Program"; cur = cur.parent) {
        const comments = sourceCode.getCommentsBefore(cur);
        if (comments.length > 0) {
          return comments.some((c) => FRAME_MARKER_RE.test(c.value));
        }
      }
      return false;
    };
    const checkColorLiteral = (node, text) => {
      if (
        typeof text === "string" &&
        HEX_OR_RGB_RE.test(text) &&
        !isFrameMarked(node)
      ) {
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
