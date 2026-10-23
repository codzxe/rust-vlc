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
  doc 08 §3.3) lands with the settings cards (4.E2) — ThemeProvider currently
  defaults to the product defaults (dark + orange) with no storage;
  `ui/src/theme/` is the seam.
- The full 9-accent × light/dark visual matrix IS now verified UI-side too
  (closure session 3, `e2e/theme.spec.ts` — 18 computed-style assertions
  against the values resolved from themes/tokens.json; see below).
- Stylelint for plain `.css` files (doc 08 §3.4) is deferred — the TSX lint
  rule is live; sandbox CSS is var()-only by construction.

## Visual matrix verification (closure session, 4.A2 relaunch 2)

`e2e/theme.spec.ts` loops all 9 accents × dark/light (18 tests) on the
`/dev/tokens` sandbox and asserts, per combination, the computed
`background-color` of `swatch-accent` (`--color-accent`, mode-independent
  accent primary) and `swatch-accent-bright` (`--color-accent-bright`, which
per-mode blocks re-point) — proving BOTH axes cascade for every combo.
Expected hex values are transcribed from the committed pipeline output
(`themes/generated/tokens.css`, itself diff-tested Rust-side), i.e. from
`themes/tokens.json`. `accent-bright` was added to the sandbox
`SWATCH_ROLES` + CSS for this (it re-points in light mode; `accent` alone
could not distinguish dark from light).

## Rust-constants consumer survey (NEXT-STEPS item 2 — for the Lead, ARCH-3)

Surveyed (READ-ONLY; no engine code touched). `cargo xtask tokens` also
emits `crates/theme-tokens/src/generated/tokens.rs` (semantic-layer Rust
constants: `MODES`, `DEFAULT_MODE`, `ACCENTS`, `ACCENT_{ID}_{ROLE}` ramps,
per-mode `_{MODE}` suffixes). **Current consumers: ZERO** — no engine crate's
Cargo.toml depends on `theme-tokens`; the only mentions outside the crate are
doc comments (settings-schema, xtask). Would-be consumers identified:

1. **Renderer clear/letterbox color** (doc 08 §3.1): `renderer-wgpu`
   `backend.rs` takes `background: Rgba` as a runtime parameter and the
   default today is the hardcoded `color::BLACK` ("opaque black — the default
   letterbox background", `color.rs:83`). Token candidates:
   `--color-surface-deep` (#090909 dark / #dedede light) or intentional
   black. ARCH-3 note: the engine must NOT read current theme state — the
   correct seam already exists: the UI/host resolves the token and passes the
   concrete `Rgba` in (as `backend.rs` already does for background). Rust
   constants would only justify a dep for engine-side DEFAULTS/fallbacks.
2. **Subtitle-safe margins** (doc 08 §3.1): no margin constants exist yet
   anywhere — `plugin-api/src/subtitle.rs` is plain-text v1 (no styling),
   `media-types::frame::SubtitleCue` carries no layout. Future consumers:
   the L2 `subtitles` impl crate and/or the renderer compose pass; token
   candidates are the `--space-*` family.

Recommendation for the Lead: defer adding any engine-side `theme-tokens`
dep until a concrete consumer lands (renderer default clear color when
video-out surface params are wired); prefer value-passing over the command
API / render params so the engine stays UI-stateless.
