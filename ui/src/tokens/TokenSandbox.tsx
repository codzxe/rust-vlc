// TokenSandbox — card 4.A2 dev fixture (NOT a product screen; real screens
// and components land with wave 4.B+). Mounted at /dev/tokens, it proves the
// token pipeline end-to-end: mode/accent switching drives the root-element
// attributes, and every visual property below resolves through SEMANTIC
// tokens only (TokenSandbox.css + the generated tokens.css cascade).
//
// Button labels are technical identifiers (ThemeModeSetting / AccentId union
// values), rendered via expressions — not user copy, matching the technical
// exemption rationale in doc 07 §6.5. User-visible headings flow through
// useMsg() like everywhere else.

import { useMsg } from "../i18n/messages";
import { useTheme, type ThemeModeSetting } from "../theme/ThemeProvider";
import { ACCENTS } from "./generated/tokens";
import "./TokenSandbox.css";

const MODE_SETTINGS: readonly ThemeModeSetting[] = ["dark", "light", "auto"];

const SWATCH_ROLES = [
  "accent",
  "accent-bright",
  "accent-strong",
  "accent-tinted",
  "surface-raised",
  "content-primary",
] as const;

export function TokenSandbox() {
  const { modeSetting, accent, setModeSetting, setAccent } = useTheme();
  const msg = useMsg();

  return (
    <main className="ts-sandbox">
      <h1>{msg("screen.dev.tokens")}</h1>
      <div className="ts-controls">
        <section>
          <h2>{msg("tokens.mode")}</h2>
          {MODE_SETTINGS.map((mode) => (
            <button
              key={mode}
              type="button"
              aria-pressed={modeSetting === mode}
              onClick={() => setModeSetting(mode)}
            >
              {mode}
            </button>
          ))}
        </section>
        <section>
          <h2>{msg("tokens.accent")}</h2>
          {ACCENTS.map((id) => (
            <button
              key={id}
              type="button"
              aria-pressed={accent === id}
              onClick={() => setAccent(id)}
            >
              {id}
            </button>
          ))}
        </section>
      </div>
      <div className="ts-swatches">
        {SWATCH_ROLES.map((role) => (
          <span
            key={role}
            className="ts-swatch"
            data-role={role}
            data-testid={`swatch-${role}`}
          />
        ))}
      </div>
    </main>
  );
}
