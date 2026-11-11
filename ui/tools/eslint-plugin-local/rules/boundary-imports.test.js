// Unit tests for local/boundary-imports (issue #36). The rule resolves
// relative specifiers against context.filename and compares with the REAL
// ui/src root, so the cases below use absolute filenames under ui/src.

import { RuleTester } from "eslint";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, it } from "vitest";
import rule from "./boundary-imports.js";

const here = dirname(fileURLToPath(import.meta.url));
// tools/eslint-plugin-local/rules/ → ui/src (the boundary the rule enforces).
const uiSrc = resolve(here, "../../../src");

const ruleTester = new RuleTester({
  languageOptions: { ecmaVersion: "latest", sourceType: "module" },
});

describe("local/boundary-imports", () => {
  it("keeps src/ imports inside ui/src and rejects escapes + absolute paths", () => {
    ruleTester.run("boundary-imports", rule, {
      valid: [
        // Relative import that stays inside ui/src.
        {
          code: "import { helper } from './util';",
          filename: join(uiSrc, "components/Foo.tsx"),
        },
        // Bare package specifier — always allowed.
        {
          code: "import { useState } from 'react';",
          filename: join(uiSrc, "components/Foo.tsx"),
        },
        // Edge: deep relative climb that still lands inside ui/src.
        {
          code: "import { useClient } from '../../../client/context';",
          filename: join(uiSrc, "screens/settings/areas/advanced.tsx"),
        },
        // Edge: up-and-back-in specifier normalizes back inside the boundary.
        {
          code: "import { x } from '../src/b';",
          filename: join(uiSrc, "a.ts"),
        },
      ],
      invalid: [
        // Relative import escaping ui/src (…/themes is Rust-owned territory).
        {
          code: "import { t } from '../../themes/tokens';",
          filename: join(uiSrc, "client/Foo.ts"),
          errors: [{ messageId: "outsideSrc" }],
        },
        // Absolute-path import.
        {
          code: "import { x } from '/home/z/secret';",
          filename: join(uiSrc, "components/Foo.tsx"),
          errors: [{ messageId: "absolute" }],
        },
      ],
    });
  });
});
