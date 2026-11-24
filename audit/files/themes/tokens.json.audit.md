# AUDIT: themes/tokens.json
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines / functions | 1457 / 0 (data file: 295 `$value` tokens in 3 layers + `$extensions`) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES at build time — `cargo xtask tokens` (tools/xtask/src/cmd/tokens.rs) → theme-tokens `tokens_gen` bin (default path `<root>/themes/tokens.json`, crates/theme-tokens/src/bin/tokens_gen.rs:88) → `themes/generated/tokens.css` → `ui/scripts/gen-tokens.mjs` → `ui/src/tokens/generated/tokens.css`, imported by `ui/src/main.tsx:27` (`import "./tokens/generated/tokens.css";`). Not read at runtime by any shipped binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `python3` (JSON parse, ref-resolution, token count, WCAG contrast math, duplicate-key check) all exit 0; `diff -q themes/generated/tokens.css ui/src/tokens/generated/tokens.css` exit 0; `sha256sum` both copies; `rg` cross-refs; baseline cargo-test-workspace.txt theme-tokens block |

## 1. One-paragraph verdict
This is a real, machine-consumed design-token source and it is the rare file in this repo whose prose claims survive hostile arithmetic. Every one of the ~30 WCAG contrast ratios asserted in `$description` strings recomputes to the stated 2 decimals (verified by script, e.g. gray.465 "4.79:1 on deep #bababa, 5.22:1 on surface" → measured 4.79/5.22); all 196 `{dotted.path}` references resolve; the claimed token census "123 primitive / 86 alias / 86 semantic" (emitted header) matches an independent count; the three-layer grammar the file claims is "enforced by the parser" is actually enforced (crates/theme-tokens/src/parse.rs:1-16 rules; 0 semantic→primitive and 0 alias→alias violations found by script); and the committed generated outputs are byte-identical to each other with sha256 `26c55ab2…` matching the hash pinned in ui/src/tokens/tokens.test.ts:54 and tokens.ts:16. The one false claim found: the gray ramp's "lightness-ranked" step numbering is violated by 8 step pairs (S4). The Wave-15 cross-ref question is answered YES: `ui/src/theme` does not read tokens.json directly (README pins it READ-ONLY for ui/) but consumes its output — ThemeProvider.tsx:44 imports `ACCENTS, AccentId, TokenMode` from `../tokens/generated/tokens` and sets `data-theme`/`data-accent` attributes that the generated tokens.css selectors key on; lumen.css composes only `var(--…)` tokens.

## 2. What actually works (verified)
- `python3 json.load` → valid JSON, no duplicate keys (object_pairs_hook check raised none). Token census: primitive=123 alias=86 semantic=86 — matches the generated header claim "Tokens: 123 primitive / 86 alias / 86 semantic".
- Reference resolution script: 196 refs, 0 unresolvable; `accent-primaries` (9/9) and `on-accent-candidates` (4/4) all resolve.
- Layer grammar: 0 semantic→primitive direct refs, 0 alias→alias refs — matches parse.rs:1-16 enforced rules ("semantic tokens reference the alias layer only — never primitives").
- WCAG contrast script (WCAG2.x relative luminance): every asserted ratio matches — gray.465 #474747: 4.79/5.22/5.44/5.61/6.91 on deep/surface/panel/muted/raised, 4.05/4.48/4.19 NOT-certified chrome/hover/selected (all exact); gray.470 #3a3a3a: 4.95/5.86/5.94/6.39/6.66/6.87/8.45 (exact); orange.425 #824505: 3.26/3.38/3.61/3.86/4.21/4.39/4.52/5.57 (exact); orange.450 #b35f0d: 2.59/2.70/2.50/3.42 (exact); gray.325 2.55/2.66; gray.460 3.58/3.74/3.29/3.86/2.78/2.88; gray.475 4.67/3.62/4.29; gray.450 dark 3.74/3.62/3.30/3.43/3.23/2.99/2.84; focus ring #d37918 light 1.40/1.81/1.89; orange on #090909 = 6.18 (claimed 6.18:1).
- Violet fallback claim (L40, L160): violet #8e42ff vs candidates = 3.77 / 4.15 / 3.99 / 4.80 — only pure white passes 4.5:1, exactly as claimed.
- Drift: `diff -q themes/generated/tokens.css ui/src/tokens/generated/tokens.css` → identical; sha256 both = `26c55ab2b3193445728537df9cbb878bc77a46b7752714f1ff45752489648a20` = pinned hash. Baseline audit/runs/baseline/cargo-test-workspace.txt (theme-tokens block, lines 6250-6331): lib 22 passed, diff.rs 2 passed ("Compared against a fresh emission from the committed source" — so committed outputs match a fresh emission from THIS tokens.json), pipeline 12 passed, semantic_lint 2 passed, exit_code=0.
- Consumers verified: crates/theme-tokens/src/parse.rs:1 ("Parser + validator for `themes/tokens.json`"); tools/xtask/src/cmd/tokens.rs; ui/src/main.tsx:27; ui/src/theme/ThemeProvider.tsx:44.

## 3. Findings
F-001 [S4]
Where: themes/tokens.json:45 (group `$description` of `primitive.color.gray`) vs the step values at L49-205
Code: (verbatim, line 45, truncated)
```
    "$description": "Layer 1 of 3 (doc 08 §3.2): raw, extraction-accurate values. Never consumed by components; exists for re-derivation and diffing against the future Figma variables. Gray step numbers: lightness-ranked, 900=#090909 down to 0=#FFFFFF — the W25 neutral ramp (fidelity spec §1); ...
```
What is wrong: "lightness-ranked" is false. Script over all 37 numeric gray steps found 8 inversion pairs where the higher step number is LIGHTER than the lower: (115 #C9C9C9,125 #E6E6E6), (135 #C2C2C2,150 #E0E0E0), (170 #ABABAB,172 #ACACAC), (172 #ACACAC,175 #DEDEDE), (325 #767676,350 #8F8F8F), (400 #6E6E6E,425 #828282), (470 #3A3A3A,475 #4E4E4E), (750 #161616,775 #1B1B1B). The w26 light-frame steps (115-172) and WCAG-adjusted steps (425,460-475) were appended with ad-hoc numbers that break the declared ranking; the ordering of keys in the file is also chaotic (900→0 then 115-172 then 460-470 appended after "0").
Why it matters: this is the self-declared "canonical source" whose group description states an invariant future contributors will rely on when picking/inserting steps; a contributor reasoning "gray.475 is darker than gray.470" gets the opposite of the truth (#4E4E4E is lighter than #3A3A3A).
What correct looks like: either renumber the appended steps to restore monotonic luminance vs step number, or amend the description to "lightness-ranked for the W25 spine (900..0); w26 light-frame and WCAG steps use ad-hoc numbers".
Confidence: HIGH — Verification: ran (python3 luma ranking script, exit 0, 8 inversions printed)

## 4. Per-section table (data file; >100 LOC so section-level in lieu of functions)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `$description` + `$extensions.lumen.tokens` (modes/accents/derivation/provenance) | 1-43 | REAL | — | modes dark/light, 9 accents, accent-primaries 9/9 refs resolve; violet/white fallback claim verified by arithmetic |
| primitive.color.gray (37 steps) | 45-206 | REAL | S4 | F-001 ranking claim false; every per-step WCAG claim recomputes exactly |
| primitive.color.orange / accent / record / eq | 207-312 | REAL | — | 8 alternate primaries match Settings swatch set; eq 1-7 present; values flow into emitted CSS verbatim (spot-checked #d37918/#e15032/…) |
| primitive.font (sizes/family/weights) | 313-402 | REAL | — | label 15px matches CONTRACT band claim; time face = same Inter stack (tabular-nums prose only, no DTCG type — disclosed) |
| primitive.radius/space/stroke/shadow/blur/opacity/motion | 403-617 | REAL | — | shadow objects are composites not per-mode values (parse.rs:6-7 rule, keys are not mode names); overlay dy "assumed 4.76 [PROPOSAL]" is honestly flagged |
| alias.* (86 tokens, per-mode where flipping) | 618-1073 | REAL | — | all refs resolve; per-mode objects cover both modes (parser enforces, parse.rs:314) |
| semantic.* (86 tokens) | 1074-1457 | REAL | — | 0 direct primitive refs; only this layer emitted to CSS (verified in generated tokens.css — only --semantic names present) |

## 5. Test honesty
No tests in this file (data). Downstream gates that would catch its breakage, verified honest: crates/theme-tokens/tests/diff.rs (2 tests, byte-equality vs fresh emission — passed in baseline), ui/src/tokens/tokens.test.ts (pins sha256 of the CSS; hash matches file on disk), e2e/theme.spec.ts (18 computed-style assertions across 9 accents × 2 modes per tokens/README). These fail if tokens.json is repinned without regeneration — mutation reasoning: editing any `$value` changes the emission, diff.rs fails. Verified as passing in audit/runs/baseline/cargo-test-workspace.txt:6250-6331 and ui-test.txt.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Gray step numbers: lightness-ranked" | tokens.json:45 | FALSE (8 inversion pairs) | python3 ranking script (F-001) |
| "Every token resolves through an alias, never directly to a primitive (enforced by the parser)" | tokens.json:1075 | REAL | script: 0 violations; parse.rs:1-16 implements it; baseline tests pass |
| "on-accent glyph chosen as the first candidate with WCAG contrast >= 4.5:1" (alternate accents) | tokens.json:40 | REAL | color.rs:184-200 select_on_accent + tests at color.rs:301-327 (blue→#EAEAEA, violet→#FFFFFF match computed ratios); default orange ramp is file-mined per the same sentence — ramp_from_file parse.rs:690-716 reads color.content.on-accent #090909 (6.18:1, still ≥4.5) |
| "violet accent reaches 4.5:1 against none of the others" | tokens.json:40,160 | REAL | computed: 3.77/4.15/3.99 vs 4.80 on white |
| "102 SVG exports" provenance | tokens.json:2,41 | UNVERIFIED | design/figma/ holds 101 top-level .svg + subdir with 4 more (105 now); mining-time count not reproducible |
| Token census "123 primitive / 86 alias / 86 semantic" | generated tokens.css header | REAL | independent count matches exactly |

## 7. Dead / duplicated / unreachable code
None. Every primitive is either referenced by an alias or is a deliberate mined record explicitly marked "retained as the mined record" (gray.325, gray.460, gray.475 — superseded as aliases but kept as provenance; their descriptions say so). No unreferenced semantic token found (86/86 emitted to CSS).

## 8. Dependencies & flags
None — pure data. Consumed by: crates/theme-tokens (parse/emit), tools/xtask tokens cmd, ui/scripts/gen-tokens.mjs, ui/src/main.tsx. No feature flags gate the pipeline.

## 9. Open questions for the next agent
- "102 SVG exports" vs 105 SVGs now on disk — was the corpus different at mining time? (Provenance only; no functional impact.)
- gray.172 #ACACAC and gray.170 #ABABAB coexist at ΔRGB 1 with inversions between them and 175; the h05/F5 note (L783) says gray.172 was dropped as light hairline — confirm no alias still points at 172 (script shows no $value references it; it is a mined record only). Confirmed unreferenced by my ref-resolution script.
