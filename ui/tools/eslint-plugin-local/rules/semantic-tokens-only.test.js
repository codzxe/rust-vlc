// Unit tests for local/semantic-tokens-only (issue #36) — TS token
// discipline: no raw #hex/rgb() colors, no raw px on font-size/radius keys.
//
// v3 (w26.k20, j20 P3-15): .ts files are covered too (v2 skipped them, so
// hex in .ts helpers escaped both lint systems), with the contract §4.4
// `/* frame: <Frame.svg> */` escape for frame-measured ports.

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
const ts = "src/screens/player/tools/video/frameData.ts";

describe("local/semantic-tokens-only", () => {
  it("allows var(--token) dimensions and exempts tests/generated/non-source", () => {
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
        // Edge: plain .css files are Stylelint territory, not this rule.
        { code: "const c = '#fff';", filename: "src/foo.css" },
        // v3: .ts files are covered — token-resolved values pass there too.
        { code: "const c = 'var(--color-accent)';", filename: ts },
        // v3 (contract §4.4): frame-cited hex in a .ts data module — the
        // escape hatch the contract promises for frame-measured ports (the
        // real fixture: Video - Colors.svg threshold hexes).
        {
          code: "/* frame: Video - Colors.svg — Color Threshold row hex input */\nexport const FRAME_THRESHOLD_COLOR = '#FF0000';",
          filename: ts,
        },
        // v3: same-line frame citation.
        { code: "const c = '#FF0000'; /* frame: Colors.svg */", filename: ts },
        // v3: the message catalog is DATA (displayed copy — the subtitles
        // text-color field shows its current hex as value text), not paint
        // code; the discipline lives at the consumption site.
        {
          code: 'const m = { "settings.subtitles.text-color.value": "#FFFFFF" };',
          filename: "src/i18n/messages.ts",
        },
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
        // v3: raw hex in a .ts helper — the v2 hole, now covered.
        { code: "const c = '#fff';", filename: ts, errors: [{ messageId: "rawColor" }] },
        // v3: a NON-frame comment does not exempt (the citation must name a
        // frame — random prose before a hex is not a measurement record).
        {
          code: "/* data value, not a frame citation */\nexport const COLOR = '#FF0000';",
          filename: ts,
          errors: [{ messageId: "rawColor" }],
        },
        // v3: .ts dimension discipline matches .tsx.
        {
          code: "const s = { fontSize: '14px' };",
          filename: ts,
          errors: [{ messageId: "rawDimension" }],
        },
      ],
    });
  });
});
