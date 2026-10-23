# ui/src/tokens — the token pipeline, UI side (card 4.A2, doc 08 §3)

## Pipeline shape (the 4.A2 decision)

```
themes/tokens.json            Rust-owned source of truth (card 1.B4) — READ-ONLY for ui/
  └▶ cargo xtask tokens       crates/theme-tokens `tokens-gen` (unchanged, crate-owned)
       └▶ themes/generated/tokens.css    committed crate output — THE BRIDGE
            └▶ ui/scripts/gen-tokens.mjs      node, ZERO deps (card 4.A2)
                 ├▶ src/tokens/generated/tokens.css   verbatim copy, imported in main.tsx
                 └▶ src/tokens/generated/tokens.ts    typed accessors (TokenMode, AccentId,
                                                      SemanticTokenName, tokenVar(), sha256)
```

Rationale: the Rust crate already validates the three-layer grammar
(primitive → alias → semantic), derives the 9 accent ramps, and emits
byte-stable CSS diff-tested on the Rust side. The UI therefore consumes the
crate's committed CSS output rather than re-implementing emission — one
emitter, two consumers (CSS cascade for the webview, Rust constants for the
engine, `crates/theme-tokens/src/generated/tokens.rs`). The bridge file is
byte-copied so `local/boundary-imports` stays intact (no imports outside
`ui/src`) and drift is impossible to miss (`npm run tokens:check` +
`tokens.test.ts` pins the sha256).

## What lives here

- `generated/` — COMMITTED build artifacts. Never edit by hand.
  - `tokens.css` — semantic custom properties, switched by
    `:root[data-theme="dark|light"]` + `:root[data-accent="…"]` (9 accents).
  - `tokens.ts` — typed accessors generated from the CSS structure.
- `TokenSandbox.tsx` / `.css` — dev fixture at `/dev/tokens` proving
  end-to-end consumption (semantic tokens only, both in CSS and via the
  context). NOT a product screen; wave 4.B owns components.
- `tokens.test.ts` — snapshot: pinned sha256, byte-identity with the crate
  output, 2 modes × 9 accents matrix, per-accent role coverage, accessor
  fidelity. Update the pinned hash ONLY together with a `cargo xtask tokens`
  + `npm run tokens:generate` run in the same commit.

## Rules (lint-enforced)

- Components consume SEMANTIC tokens only (doc 08 §3.2): `var(--…)` in CSS,
  `tokenVar("--…")` for inline styles. `local/semantic-tokens-only` rejects
  raw `#hex` / `rgb()` and raw `px` on font-size/radius keys in TSX.
- Primitive and alias layers never appear here — the emitter only ships the
  semantic layer.

## Regeneration

```sh
cargo xtask tokens        # Rust side: themes/tokens.json → themes/generated/
npm run tokens:generate   # ui side: copy + type-generate (run in ui/)
npm run tokens:check      # CI drift guard
```

## Honest status / follow-ups

- Persistence + settings-schema wiring ("Accent color" setting, doc 07 /
  doc 08 §3.3) lands with the settings cards — ThemeProvider currently
  defaults to the product defaults (dark + orange) with no storage.
- The full 9-accent × contrast matrix is verified Rust-side (crate
  `TokenError::Contrast`); a UI-side visual matrix pass is a later session.
- Stylelint for plain `.css` files (doc 08 §3.4) is deferred — the TSX lint
  rule is live; sandbox CSS is var()-only by construction.
