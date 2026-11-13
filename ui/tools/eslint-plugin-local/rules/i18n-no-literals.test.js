// Unit tests for local/i18n-no-literals (issue #36) — the rule is exercised
// through ESLint's RuleTester with the REAL ui/i18n-allowlist.json (loaded by
// the rule module itself), so the allowlist contract is covered too.
//
// v3 (w26.k20, j20 P2-7 + P3-14): placeholder-family attribute coverage +
// FILE-SCOPED allowlist entries (an allowlisted text is exempt only in the
// files its entry names — the "00:02:10" customize-band timecodes are the
// live fixture for that contract).

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

  it("rejects raw user-visible copy and honors the file-scoped allowlist + test exemption", () => {
    ruleTester.run("i18n-no-literals", rule, {
    valid: [
      // Resolved copy — the sanctioned useMsg()/MessageKey seam.
      { code: "const el = <p>{m('player.title')}</p>;" },
      // Allowlisted JSX text — but ONLY in the file its entry scopes to
      // ("00:02:10" → src/screens/player/customize/CustomizePanels.tsx).
      {
        code: "const el = <span>00:02:10</span>;",
        filename: "src/screens/player/customize/CustomizePanels.tsx",
      },
      // Whitespace-only text nodes are layout, not copy.
      { code: "const el = <div>   </div>;" },
      // Non-literal expression containers pass through untouched.
      { code: "const el = <span>{count}</span>;" },
      // Attribute resolved via a call expression, not a literal.
      { code: "const el = <input aria-label={m('a11y.open')} />;" },
      // placeholder resolved through the seam (v3: the attribute IS covered,
      // this must stay the only sanctioned shape for it).
      { code: "const el = <input placeholder={m('playlist.search.placeholder')} />;" },
      // data-placeholder is a dev/test marker, never rendered — uncovered
      // by design (v3 doc note).
      { code: 'const el = <div data-placeholder="engine-wiring-4.H1" />;' },
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
      // v3 (j20 P2-7): placeholder is user-visible copy — the live j20 miss
      // was OnboardingScreen's placeholder="en, de".
      {
        code: 'const el = <input placeholder="en, de" />;',
        errors: [{ messageId: "literal" }],
      },
      // v3: template-literal placeholder value.
      {
        code: "const el = <input placeholder={`Search everything`} />;",
        errors: [{ messageId: "literal" }],
      },
      // v3 (j20 P3-14): an allowlisted text in a file its entry does NOT
      // scope to is still a violation — file scoping is the whole point.
      {
        code: "const el = <span>00:02:10</span>;",
        filename: "src/screens/playlist/PlaylistScreen.tsx",
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
