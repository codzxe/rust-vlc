// local/boundary-imports — promoted verbatim from ui/eslint.config.js
// (issue #36; previously inline). Rule semantics are unchanged; the only
// adaptations for the move are the module's export form and the anchor of
// the src root, which now resolves relative to this file
// (tools/eslint-plugin-local/rules/ → ui/src — the same directory the
// inline definition pointed at).
//
// Files under src/ may not relatively import outside ui/src (the generated
// client lives inside src/generated, so it stays reachable; doc 08 §8
// "no Tauri APIs outside bindings").

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

const srcRoot = resolve(here, "../../../src");
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

export default boundaryImports;
