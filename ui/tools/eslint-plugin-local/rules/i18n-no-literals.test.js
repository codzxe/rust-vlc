// Unit tests for local/i18n-no-literals (issue #36) — the rule is exercised
// through ESLint's RuleTester with the REAL ui/i18n-allowlist.json (loaded by
// the rule module itself), so the allowlist contract is covered too.

import { RuleTester } from "eslint";
import { describe, expect, it } from "vitest";
import plugin from "../index.js";
import rule from "./i18n-no-literals.js";

const ruleTester = new RuleTester({
  languageOptions: {
    ecmaVersion: "latest",
    sourceType: "module",
    parserOptions: { ecmaFeatures: { jsx: true } },
  },
});

describe("local/i18n-no-literals", () => {
  it("is registered under its historical name in the plugin index", () => {
    expect(plugin.rules).toHaveProperty("i18n-no-literals", rule);
  });

  it("rejects raw user-visible copy and honors the allowlist + test exemption", () => {
    ruleTester.run("i18n-no-literals", rule, {
    valid: [
      // Resolved copy — the sanctioned useMsg()/MessageKey seam.
      { code: "const el = <p>{m('player.title')}</p>;" },
      // Allowlisted JSX text ("404" — ui/i18n-allowlist.json, owner A).
      { code: "const el = <strong>404</strong>;" },
      // Whitespace-only text nodes are layout, not copy.
      { code: "const el = <div>   </div>;" },
      // Non-literal expression containers pass through untouched.
      { code: "const el = <span>{count}</span>;" },
      // Attribute resolved via a call expression, not a literal.
      { code: "const el = <input aria-label={m('a11y.open')} />;" },
      // Edge: test files are exempt (fixtures may pin raw copy).
      {
        code: "const el = <p>Completely raw fixture copy</p>;",
        filename: "src/screens/Foo.test.tsx",
      },
    ],
    invalid: [
      // JSX text node.
      {
        code: "const el = <p>Save changes</p>;",
        errors: [{ messageId: "literal" }],
      },
      // User-visible attribute as a plain string literal.
      {
        code: "const el = <img alt='Movie poster' />;",
        errors: [{ messageId: "literal" }],
      },
      // {"Copy"} — string-literal expression container in text position.
      {
        code: "const el = <b>{'Copy that'}</b>;",
        errors: [{ messageId: "literal" }],
      },
      // Edge: no-substitution template literal as an attribute value.
      {
        code: "const el = <div title={`Raw title`} />;",
        errors: [{ messageId: "literal" }],
      },
    ],
    });
  });
});
