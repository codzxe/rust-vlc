# AUDIT: themes/generated/tokens.css
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 262 / n/a (generated artifact; 86 semantic decls in :root, 24 in light block, 8×6 accent blocks + 8×3 light+accent blocks) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the artifact itself ships. Chain: themes/generated/tokens.css → (byte-identical copy, `diff` verified) ui/src/tokens/generated/tokens.css → ui/src/main.tsx:27 `import "./tokens/generated/tokens.css";` → ui/src/theme/ThemeProvider.tsx:103-106 sets `root.dataset.theme` / `root.dataset.accent` on `<html>` → cascade switches blocks. 25 files under ui/src reference tokens.css (Grep). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `target/debug/deps/diff-081ea81a32cdde49` (theme-tokens dir) -> committed_outputs_match_regeneration ok, exit 0 (artifact byte-matches regeneration from themes/tokens.json); `diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css` -> IDENTICAL; python3 token-name count: :root = 86 decls = header's "86 semantic"; sortedness verified against render_block sort in emit.rs; Grep ui/src/theme/ThemeProvider.tsx for dataset.theme/dataset.accent. |

## 1. One-paragraph verdict
A faithful, byte-stable, correctly-structured emission of the semantic token layer: I re-ran the drift test myself (exit 0), the committed file matches regeneration exactly, the UI copy is byte-identical, the 86 declarations match the header's count, every block is name-sorted, the CSS cascade design is sound (the 3 per-mode accent roles get a higher-specificity `:root[data-theme][data-accent]` block per accent, so the equal-specificity `[data-theme]`/`[data-accent]` overlap is always resolved correctly), and spot-checks of all 9 accent ramps against the generated Rust constants show zero divergence. The one real defect is inherited from themes/tokens.json data: `--color-focus-ring` is hard-pinned to the orange primitives, so under any of the 8 alternate accents the focus ring stays orange — contradicting doc 08 §5's "focus = 2px `accent.primary` ring".

## 2. What actually works (verified)
— verified by running the prebuilt diff test binary (2 passed, exit 0): this exact file is the deterministic output of themes/tokens.json via emit_css.
— verified by `diff` against ui/src/tokens/generated/tokens.css: IDENTICAL (7852 bytes) — the shipped UI consumes exactly these bytes.
— verified by python3 count: :root carries exactly 86 `--*` declarations = the header claim "86 semantic"; light block 24; 8 accents × (6 + 3) block decls + orange's 6 in :root + 3 in light block = consistent totals.
— verified by cross-referencing every accent block value against crates/theme-tokens/src/generated/tokens.rs ACCENT_* constants (e.g. red: #e15032/#e66044/#c83a1d/#351008/#161616 = ACCENT_RED_* at tokens.rs:184-191): zero mismatches.
— cascade correctness: for light+accent users the `:root[data-theme="light"][data-accent="X"]` blocks (specificity 0,3,0) always override both 0,2,0 blocks for the 3 per-mode roles; `--color-accent`/`-strong`/`-content-on-accent` correctly carry no per-mode variants.

## 3. Findings
F-001 [S3]
Where: themes/generated/tokens.css:50 and :127 (root cause themes/tokens.json `alias.color.focus.ring`)
Code:
```
  --color-focus-ring: #d37918;
```
and (light block)
```
  --color-focus-ring: #824505;
```
What is wrong: The focus-ring token is hard-wired to the ORANGE primitives (`alias.color.focus.ring => {"dark": "{primitive.color.orange.500}", "light": "{primitive.color.orange.425}"}` in themes/tokens.json) and is NOT part of the accent family, so no `[data-accent]` block overrides it. Doc 08 §5 (docs/08-ui-figma-fidelity.md:307) specifies "focus = 2px `accent.primary` ring, 2px offset". A user who selects the red accent gets red accents everywhere (buttons, seek bar, selection fills — all from the accent family) but every focus-visible outline stays ORANGE (#d37918 dark / #824505 light), because ui/src/theme/lumen.css:203 `outline-color: var(--color-focus-ring);` reads this token. 8 of 9 accents render an off-accent focus ring — a visible a11y/consistency defect in any non-default accent. The emitter is not at fault; the token source mis-parameterizes the ring. What correct looks like: `semantic.color.focus.ring` resolves through the accent family (or the ring joins the 6 accent roles). Confidence: HIGH — Verification: read (tokens.json + cascade blocks + lumen.css consumer + doc 08:307).
F-002 [S4]
Where: themes/generated/tokens.css:1-26 (header) vs doc 08 §3.4
Code:
```
 * Tokens: 123 primitive / 86 alias / 86 semantic (source:
 * themes/tokens.json).
```
What is wrong: Nothing in the counts — but the header's consumer contract ("Components consume SEMANTIC tokens only ... primitive and alias layers are never emitted") is enforced only by lint rule + review; doc 08 §3.4's companion "dead-token detector" (every semantic token consumed ≥ once) does not exist (see emit.rs audit F-002), so nothing proves all 86 of these declarations have a consumer. Verified consumption of a sample (focus-ring, panel opacity, blur, shadows, eq colors) shows real usage; full 86/86 coverage is UNVERIFIED. Confidence: HIGH on the missing detector, UNVERIFIED on full coverage.

## 4. Per-function table
N/A (generated artifact — no functions). Block inventory verified instead:
| Block | Lines | Decls | Check |
|---|---|---|---|
| :root | 28-115 | 86 | sorted, matches 86-semantic count, orange accent family inlined |
| :root[data-theme="light"] | 117-142 | 24 | sorted; per-mode semantic + orange's 3 per-mode accent roles |
| 8 × :root[data-accent] | 144-256 (interleaved) | 6 each | accent/strong/on-accent + 3 per-mode roles at dark values; verified vs ACCENT_* constants |
| 8 × :root[data-theme="light"][data-accent] | interleaved | 3 each | bright/icon/tinted light values; verified |

## 5. Test honesty
No tests in this file. Its guards live in crates/theme-tokens/tests/diff.rs (committed-output byte-compare — I ran it, exit 0) and ui/src/tokens/tokens.test.ts (pinned sha256 — not audited here, noted for the UI auditor).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Regeneration is idempotent; the committed copy is diff-tested" | tokens.css:4-6 | REAL | diff binary run by me, exit 0 |
| "Modes (2): dark (default), light" | tokens.css:22 | REAL | exactly one [data-theme] block, dark values in :root |
| "Accents (9): orange (default) + 8" | tokens.css:23 | REAL | 8 [data-accent] blocks; orange inlined in :root |
| "Tokens: 123 primitive / 86 alias / 86 semantic" | tokens.css:24 | REAL | 86 :root decls counted; source shape matches generated/tokens.rs:21 |
| "focus = 2px accent.primary ring" | doc 08:307 | FALSE for 8/9 accents | focus-ring pinned to orange primitives (F-001) |
| alpha 0.95/0.8 panel + blur 25/13.75 (doc 08 §3.3) | doc 08:238 | REAL | --opacity-panel 0.95/light 0.8; --blur-backdrop 25px/-small 13.75px (lines 29-30, 88, 141) |
| shadows dy 4.76/blur 4.76/38% + 70% variant (doc 08 §3.3) | doc 08:239 | REAL | --shadow-card / --shadow-overlay lines 104-105 |

## 7. Dead / duplicated / unreachable code
- The `themes/generated/tokens.css` original vs `ui/src/tokens/generated/tokens.css` copy: deliberate bridge duplication (gen-tokens.mjs `--check` guards drift; verified identical today). Not dead.
- 86/86 semantic consumption coverage: UNVERIFIED (no dead-token detector exists — F-002).

## 8. Dependencies & flags
- Pure CSS, no imports, no @media, no @supports (doc 08 §3.3 promised an `@supports` fallback for backdrop-filter — the token file carries only the raw blur radius; the fallback responsibility is pushed to ui CSS, which I did not audit here).

## 9. Open questions for the next agent
- Does ui/src CSS implement the doc 08 §3.3 `@supports (backdrop-filter)` fallback to solid fill, or do unsupported webviews lose translucency silently?
- Audit ui/src/tokens/tokens.test.ts sha256 pin: does it reference this file's hash (drift guard on the UI side)?
