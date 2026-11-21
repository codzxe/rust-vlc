# AUDIT: ui/src/tokens/generated/tokens.css
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 262 / 0 (generated data artifact: 18 selector blocks, 86 semantic custom properties) |
| File verdict | REAL (verbatim bridge copy of the crate output; byte-verified in sync; 10/86 tokens unconsumed) |
| Reachable from shipped entry point? | Loaded at SPA bootstrap: main.tsx style chain imports the token sheet (every component resolves `var(--…)` against it); also byte-identical to themes/generated/tokens.css (the crate's committed output). No shipped binary at HEAD (ui-wide fact). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css` → IDENTICAL (262/262); python3 consumer scan: all 86 `--*` tokens vs `rg -l 'var(' ui/src` (excluding generated/tests) → 76 consumed, 10 unconsumed (listed in F-002); declaration count check: :root block = 86 declarations (:29-114) = header's "86 semantic"; block-structure check: 2 modes × (1 + 9 accents) = 18 blocks, all 8 alternate accents carry both dark and light blocks (:144-262) ✓; `rg -rn 'color-mix\(in srgb, var\(--color-surface-panel\) 95%' ui/src --glob '*.css'` → ≥4 files hardcode 95%; head ui/src/assets/fonts/fonts.css (Inter v4.1 bundled — the font-family token's "Inter" is real, not phantom); baseline run #24 (tokens:check PASS) + run #3 (crate diff tests). |

## 1. One-paragraph verdict
A clean generated token bridge: byte-identical to the crate's committed output (diff-verified), structurally consistent (86 semantic declarations matching the header count; every one of the 9 accents has its dark+light block pair), backed by real drift gates on both sides (crate diff test in `cargo test --workspace`, npm tokens:check). The hostile scan still lands two hits: 10 of 86 semantic tokens (12%) have ZERO consumers anywhere in ui/src — including `--font-size-scale-label`, which a recent wave ADDED and never used — and `--opacity-panel` (0.95) is dead while at least four stylesheets hardcode the very value it exists to carry (`color-mix(… 95%, transparent)`), so the "components consume semantic tokens only" discipline is bypassed for panel opacity.

## 2. What actually works (verified)
— Sync: `diff` against themes/generated/tokens.css → identical; drift gates: crates/theme-tokens/tests/diff.rs (byte-equality, runs in baseline run #3) + `npm run tokens:check` (baseline run #24 PASS).
— Internal consistency: header claims (2 modes, 9 accents, 123/86/86 tokens) verified against the artifact's own structure (:root = 86 declarations; 8 alternate accents × 2 blocks + default + light = 18 blocks).
— Value sanity: light theme overrides every content/surface/stroke token (no dark-on-dark residue); on-accent contrast flips correctly per accent (dark ink on orange/red/yellow/green/cyan/pink/gray, light ink on blue #185dd3 and violet #8e42ff).
— The font stack's "Inter" is real: ui/src/assets/fonts/fonts.css bundles Inter v4.1 variable (OFL), with a documented visual-parity fix history.

## 3. Findings

### F-001 [S3]
Where: ui/src/tokens/generated/tokens.css:88 (token) vs settings.css:74, :230, variants.css, network-stream.css, shortcuts.css (consumers)
Code:
```
  --opacity-panel: 0.95;
```
```
  background: color-mix(in srgb, var(--color-surface-deep) 95%, transparent);
```
What is wrong: The semantic token for panel opacity exists but is consumed NOWHERE (script-verified: 0 usages of `var(--opacity-panel)`). Instead, at least four stylesheets hardcode the same design value as a literal percentage inside `color-mix()`: settings.css:74 (window #0E0E0E@95%), settings.css:230 (sidebar panel 95%), plus the same pattern in variants.css, network-stream.css, shortcuts.css.
Why it matters: The design value now lives in five places (one token + four literals). Changing `--opacity-panel` (or its light-mode value, which the light block overrides to 0.8 :141) silently does nothing to the real UI — the token layer and the rendered UI can drift apart, defeating the whole "semantic tokens only" contract this file's header declares (:8-9). The light-mode `--opacity-panel: 0.8` override is doubly dead: no consumer follows it, so light panels render at hardcoded 95% instead of the token's 80%.
What correct looks like: Consumers write `color-mix(in srgb, var(--color-surface-deep) calc(var(--opacity-panel) * 100%), transparent)` — or the token is deleted and the design value stays a frame constant.
Confidence: HIGH — Verification: script consumer scan + rg of the color-mix pattern; consumer CSS files belong to other agents (cross-ref: settings.css → my part1 audit; variants/network-stream/shortcuts → other agents).

### F-002 [S4]
Where: ui/src/tokens/generated/tokens.css:70-71, 77, 82-83, 84, 89, 93, 114
Code:
```
  --font-size-micro: 6px;
  --font-size-scale-label: 12px;
```
What is wrong: Ten of the 86 semantic tokens have zero consumers in ui/src (excluding generated files and tests): `--font-size-micro`, `--font-size-scale-label`, `--font-weight-extrabold`, `--motion-duration-slow`, `--motion-duration-step`, `--opacity-chrome`, `--opacity-panel` (F-001), `--opacity-scrim-subtle`, `--radius-app`, `--stroke-window`. Notably `--font-size-scale-label` was ADDED in wave w26.k18 (its log: "+font.size.scale-label primitive … repinned") and still has no consumer — added-for-nothing.
Why it matters: Dead tokens inflate the semantic surface every audit/gate must reason about, and two of them (micro 6px, extrabold) suggest designed-but-unbuilt UI. Generated-file status makes removal cheap (edit tokens.json, regenerate).
What correct looks like: Prune unconsumed tokens in themes/tokens.json (the same prune discipline icons.tsx w26.k04 applied to icon names), or build their consumers.
Confidence: HIGH — Verification: python3 scan over all `var(--…)` usages (76/86 consumed; exclusions: generated + test files; the TS twin tokens.ts mirrors the same names, so no hidden dynamic consumer exists — TokenSandbox renders a fixed SWATCH_ROLES subset, not all tokens).

## 4. Per-function table (data artifact: per-block rows)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header contract comment | 1-26 | REAL | — | Claims verified: counts, modes, accents, diff-test pointer. |
| `:root` (default dark + orange) | 28-115 | REAL | F-002 | 86 semantic declarations; sorted; hex/px/ms/percent well-formed. |
| `:root[data-theme="light"]` | 117-142 | REAL | F-001 | Full content/surface/stroke/focus override set; opacity-chrome 0.75 / panel 0.8 (panel dead — F-001). |
| `:root[data-accent="red"]` + light pair | 144-157 | REAL | — | 6 + 3 declarations, consistent shape. |
| yellow pair | 159-172 | REAL | — | Same shape. |
| green pair | 174-187 | REAL | — | Same shape. |
| cyan pair | 189-202 | REAL | — | Same shape. |
| blue pair | 204-217 | REAL | — | on-accent #eaeaea (light ink) — correct for this accent's darkness. |
| violet pair | 219-232 | REAL | — | on-accent #ffffff — correct. |
| pink pair | 234-247 | REAL | — | Same shape. |
| gray pair | 249-262 | REAL | — | Same shape; gray accent = neutral theme variant. |

## 5. Test honesty
Sibling ui/src/tokens/tokens.test.ts pins values/SHA (w26.k18 log: "tokens.test.ts sha + 86-name set + new value pins dark/light") — passed in baseline run #23. Crate-side: tests/diff.rs byte-equality (run #3). The tests are honest pins of a generated artifact (regeneration updates them, drift fails them).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Components consume SEMANTIC tokens only, via `var(--<name>)`" | :8-9 | REAL with exceptions | 76/86 consumed via var(); opacity-panel bypassed (F-001) — the color token itself IS consumed, the opacity literal is not. |
| "Tokens: 123 primitive / 86 alias / 86 semantic" | :24-25 | REAL | :root = 86 declarations (counted); w26.k18 log's 123/86/86 matches. |
| "the committed copy is diff-tested by crates/theme-tokens/tests/diff.rs" | :5-6 | REAL | diff.rs read (file 8 audit); baseline run #3. |
| "Inter" font stack resolves to a bundled face | :64 | REAL | ui/src/assets/fonts/fonts.css @font-face Inter v4.1 (read). |

## 7. Dead / duplicated / unreachable code
10 unconsumed semantic tokens (F-002). Duplicated value-lives-twice: --opacity-panel vs hardcoded 95% (F-001).

## 8. Dependencies & flags
Generated by cargo xtask tokens (tokens_gen — my file 8) → themes/generated/tokens.css → verbatim copy here + tokens.ts twin (same content as TS string + 86-name union — triple representation, all drift-gated). Consumed by every ui stylesheet.

## 9. Open questions for the next agent
- themes/tokens.json owner (W15-A03/A01/A04 chunk splits): the 10 dead tokens should be pruned at the SOURCE; also verify the 123 primitives' re-derivation claim ("exist for re-derivation from the Figma variables").
- variants.css / network-stream.css / shortcuts.css owners: same hardcoded-95% bypass as F-001 — record in those audits.
- Does any engine-side consumer read the Rust constants twin (crates/theme-tokens/src/generated/tokens.rs)? If not, that output is dead weight too.
