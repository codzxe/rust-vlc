// Unit tests for local/semantic-tokens-only (issue #36) — TSX token
// discipline: no raw #hex/rgb() colors, no raw px on font-size/radius keys.

import { RuleTester } from "eslint";
import { describe, it } from "vitest";
import rule from "./semantic-tokens-only.js";

const ruleTester = new RuleTester({
  languageOptions: {
    ecmaVersion: "latest",
    sourceType: "module",
    parserOptions: { ecmaFeatures: { jsx: true } },
  },
});

const tsx = "src/components/Foo.tsx";

describe("local/semantic-tokens-only", () => {
  it("allows var(--token) dimensions and exempts tests/generated/non-tsx", () => {
    ruleTester.run("semantic-tokens-only", rule, {
      valid: [
        // Dimensions via semantic tokens.
        {
          code: "const s = { borderRadius: 'var(--radius-card)', fontSize: 'var(--font-size-body)' };",
          filename: tsx,
        },
        // Non-dimension properties are not the rule's business.
        { code: "const w = { width: '420px' };", filename: tsx },
        // Edge: test files are exempt.
        { code: "const c = '#fff';", filename: "src/screens/Foo.test.tsx" },
        // Edge: generated emitter output is exempt (token CSS mirror lives there).
        { code: "const c = '#fff';", filename: "src/tokens/generated/tokens.tsx" },
        // Edge: plain .ts / .css files are Stylelint territory, not this rule.
        { code: "const c = '#fff';", filename: "src/foo.ts" },
      ],
      invalid: [
        // Raw hex color literal.
        { code: "const c = '#fff';", filename: tsx, errors: [{ messageId: "rawColor" }] },
        // Raw px on a font-size key.
        {
          code: "const s = { fontSize: '14px' };",
          filename: tsx,
          errors: [{ messageId: "rawDimension" }],
        },
        // Edge: rgb() smuggled through a no-substitution template literal.
        {
          code: "const bg = `rgb(0 0 0 / 0.5)`;",
          filename: tsx,
          errors: [{ messageId: "rawColor" }],
        },
        // Edge: raw px on a radius-carrying JSX attribute.
        {
          code: "const el = <div borderRadius='8px' />;",
          filename: tsx,
          errors: [{ messageId: "rawDimension" }],
        },
      ],
    });
  });
});
