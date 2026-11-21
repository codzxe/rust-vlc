# AUDIT: themes/tokens.json#part1 (lines 1–485)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / tokens | 485 / 99 (all primitive layer: color 63, font 17, radius 13, space 6 — `space.32` and `space.48` fall to part2; plus root `$description` + `$extensions.lumen.tokens` meta block) |
| File verdict | REAL (data) — 2 FALSE sub-claims (F-001, F-002) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri`: no crate depends on theme-tokens (rg over every Cargo.toml: only comment mentions; W15-A02 concur). Real chain: `cargo xtask tokens` → tokens-gen bin → parse(this file) → themes/generated/tokens.css + tokens.rs (diff-gated, baseline rows 3 & 24) → ui bridge copy → ui/src/main.tsx import — dead-ends at W14 (tauri host crate does not compile; no GUI binary exists). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | python3 json.load + full structural walk (295 tokens, refs/layers/per-mode/$type/dupes); python3 WCAG recompute of all numeric contrast claims in range; `rg -o`/`rg -l` corpus counts over design/figma (101 top-level SVGs); `find design/figma` (105 SVGs); `rg` for cited files (extract_design_tokens.py, analysis/tools-settings.md); `sed -n` verbatim verification of every quoted line; `rg -n on-accent themes/generated/tokens.css` (rule emission cross-check) |

## 1. One-paragraph verdict
This is the canonical design-token source and it is overwhelmingly what it claims: valid JSON, 295 tokens (123/86/86 — exactly matching the generated-file header), every `{reference}` resolves, the three-layer grammar is clean, all 24 per-mode values carry exactly `{dark, light}`, and — the part I attacked hardest — the mining provenance is real: I re-derived 26 of the claimed corpus counts with plain `rg` over the 101 top-level Figma SVGs and every single one reproduces exactly (560/72 fills/frames for #090909, 381 for #D37918, 1011/36 for #404040, 501/63, 1716/84, 3464 stroke-width-1.5, rx counts 1027/188/163/559/51/180/98/9/9 and 10=8+2 consolidated, 169 fill-opacity-0.95, even the long-tail #1E1E1E=4 / #202020=1). The WCAG contrast arithmetic in the descriptions is also exact everywhere I recomputed (60+ numbers, Δ0.00–0.01). Two claims are nevertheless FALSE: gray.100's "menu labels measure #E8E8E8" has zero occurrences in the cited corpus, and the gray ramp's "lightness-ranked" convention is violated by 39 step pairs. Reachability is the familiar W14 ceiling: the pipeline that consumes this file is dev/CI tooling; no shipped binary reads it.

## 2. What actually works (verified)
— verified by python3 json.load → parses; layer census 123 primitive / 86 alias / 86 semantic = 295, matching `themes/generated/tokens.css` header "Tokens: 123 primitive / 86 alias / 86 semantic" (answering W15-A05's open question).
— verified by python walk → all `{dotted.path}` references resolve (0 dangling); layer grammar holds (primitive refs nothing, alias→primitive/alias, semantic→alias — though the semantic layer itself is part3/A03); 0 duplicate token paths; 0 own-vs-inherited `$type` conflicts (confirms W15-A02 F-001 latency); all 24 per-mode `$value` objects have exactly the registered mode keys.
— verified by `rg -o 'fill="#090909"' … | wc -l` over the 101 top-level SVGs → 560 (claim l.52 "560 fills, 72 frames"; 72 files contain it — both exact). Same exact reproduction for: #0E0E0E 97/50 (l.56), #151515 (l.60 "36 fills"), #191919 298 (l.72), #1F1F1F 13 + #1E1E1E 4 + #202020 1 (l.76 "consolidates #1E1E1E (4) and #202020 (1)" — exact), #222222 73 (l.80), #262626 50 (l.84), #404040 1011 fills / 36 frames (l.92), #6B6B6B 501/63 (l.96), #8F8F8F 1716/84 (l.108; 84 = files with `fill="#8F8F8F"`, 88 contain the string anywhere), #C6C6C6 86 (l.120), #EAEAEA 54/24 (l.144), #D37918 381 (l.231), rx counts 1027/188/163/559/51/180/98/9/9 (l.405-456; radius.19 "(10)" = rx19×8 + consolidated rx20×2), fill-opacity 0.95 ×169 (l.566), stroke-width 1.5 ×3464 (l.501). Frame metric = files whose fills include the color; exact for all 6 frame counts spot-checked.
— verified by python WCAG recompute → every numeric contrast claim in range is exact to ±0.01: gray.450 #6B6B6B = 3.74 surface / 3.62 deep / 3.30 panel / 3.43 muted / 3.23 chrome / 2.99 raised / 2.84 hover (l.96); gray.425 #828282 = 5.18/4.58/5.02 ≥4.5 on surface/panel/deep (l.100); #6B6B6B on #191919 = 3.30 (l.100); gray.325 = 2.55/2.66 (l.116); gray.460 #5F5F5F = 3.58/3.74/3.29/3.86/3.34/2.78/2.88 (l.192); gray.475 = 4.67/3.62/4.29/4.35 (l.196); gray.465 = 5.22/4.79/5.44/5.61/6.91 + NOT-certified 4.05 chrome / 4.48 hover / 4.19 selected (l.200); gray.470 = 6.39/5.86/5.94/6.66/6.87/4.95/8.45 (l.204); orange.450 #B35F0D = 2.59/2.70/2.50/3.42 (l.223); orange.425 #824505 = 3.26/3.38/3.61/3.86/4.21/4.39/4.52/5.57 (l.227); accent #D37918 = 6.18 on #090909, 1.40–2.39 on light surfaces (l.227, l.790 part2); gray.300 vs chrome = 1.23:1, ΔRGB 17 (l.112).
— verified by python + `rg -n on-accent themes/generated/tokens.css` → the accent-derivation rule (l.40) is implemented exactly: violet #8E42FF measures 3.77/4.15/3.99/4.80 vs candidates (#161616/#090909/#EAEAEA/#FFFFFF) — reaches 4.5:1 against none but white; emitted per-accent on-accent values are #161616 ×6, #eaeaea (blue, 4.92), #ffffff (violet) — first-candidate-≥4.5 exactly as documented; the default-orange on-accent #090909 is the documented "mined family" exemption.
— verified by `rg -c '#1B1B1B' design/figma/Player.svg` → 3 hits; `rg -n 'Frame 21'` → group exists with 2 `fill="#090909"` — consistent with the surface.chrome / dark-seek-track claims (l.64, l.634-part2).
— verified by `sed -n '118p' "design/figma/Player - Light.svg"` → `<rect x="254" y="1039" width="1238" height="28" rx="6" fill="#DEDEDE"/>` — the "lone #DEDEDE rect (1238×28 r6 at (254,1039))" claim at l.634 (part2) is byte-accurate.

## 3. Findings
### F-001 [S3]
Where: themes/tokens.json:144
Code:
```
          "$description": "strongest text: headings, active labels (54 fills, 24 frames); light on-accent candidate; menu labels measure #E8E8E8 — consolidated [PROPOSAL, W25 §1]"
```
What is wrong: "menu labels measure #E8E8E8" — #E8E8E8 occurs ZERO times in the entire committed corpus (rg -io over all of design/figma/, case-insensitive: 0 hits; typo candidates E7E7E7/E9E9E9 also 0). The "measured" value has no corpus basis; only #EAEAEA (54 fills) exists.
Why it matters: this file's entire contract (l.45 "extraction-accurate values") is provenance; a "measure" claim contradicted by the cited corpus is the exact class this file's own history condemns (l.116: "the w26.f15 claim … was arithmetically false"). A future re-miner trusting the sentence will hunt for a phantom value or "re-consolidate" against it.
What correct looks like: either the actual measured value (e.g. #EAEAEA with its count) or drop the clause.
Confidence: HIGH — Verification: ran (rg over design/figma/, multiple patterns).

### F-002 [S3]
Where: themes/tokens.json:45
Code:
```
    "$description": "Layer 1 of 3 (doc 08 §3.2): raw, extraction-accurate values. Never consumed by components; exists for re-derivation and diffing against the future Figma variables. Gray step numbers: lightness-ranked, 900=#090909 down to 0=#FFFFFF — the W25 neutral ramp (fidelity spec §1); …
```
What is wrong: "lightness-ranked" is false. Pairwise check of all 39 gray steps: 39 violating pairs across 24 of 39 steps — e.g. 750 #161616 is DARKER than 775 #1B1B1B; 700 #191919 darker than 775; 400 #6E6E6E darker than 425 #828282; 325 #767676 darker than 350 #8F8F8F; 115 #C9C9C9 darker than 125 #E6E6E6; 135 darker than 150; 170 #ABABAB darker than 172 #ACACAC (Δ1, itself disclosed at l.112); 470 #3A3A3A darker than 475 #4E4E4E.
Why it matters: the step number is the ramp's only ordering API; descriptions already reason with it ("one step below deep", l.124). Every such inference is unreliable — 62% of steps participate in at least one inversion. The inversions are historical layering (w26-appended 115–172/460–475 steps, the 775 chrome repin), i.e. the convention silently died as the ramp grew.
What correct looks like: either renumber at the next re-pin or delete "lightness-ranked" and state the truth (mined steps with historical numbers; ordering must be read from the hex).
Confidence: HIGH — Verification: ran (python pairwise scan; endpoints 900/0 are correct).

### F-003 [S4]
Where: themes/tokens.json:2 and :41
Code:
```
      "provenance": "W25 spec §1 (design/figma-fidelity-spec.md, 2026-11-11): … hex frequency re-mined 2026-11 from design/figma/ (102 SVG exports) with scripts/extract_design_tokens.py methodology (doc 08 §1.2-§1.4)"
```
What is wrong: two provenance defects in one sentence. (a) The corpus contains 105 SVGs (101 top-level + 4 in `design/figma/Add Subtitles to Playlist/`); "102" matches neither the full corpus nor the 101-file set that reproduces every count exactly. (b) `scripts/extract_design_tokens.py` does not exist anywhere in the tree (find over the whole repo: 0 hits) — the cited methodology tool is absent (doc 08 §15 says the mining was actually done with `rg`).
Why it matters: "102 exports" is the denominator of every "N fills / M frames" claim; a wrong corpus size makes the (otherwise exact) counts formally unreproducible as stated. The missing script is a dangling citation in the canonical source (also dangled from design/figma-fidelity-spec.md:21).
What correct looks like: pin the corpus (path + file count + the exact rg patterns, which DO reproduce — I verified 26/26 counts), and cite the real tool or drop the script name.
Confidence: HIGH — Verification: ran (find/rg counts as listed in §2).

### F-004 [S4]
Where: themes/tokens.json:64
Code:
```
          "$description": "dark window chrome — Player.svg topbar + control strip (g03-P2-1, h03 #3: the frame paints #1B1B1B on both bands; the long-tail note keeps it distinct from #191919 panels). Added w26.i02. ALSO the dark surface.elevated since w26.i19 (i09 +2 ask: the tools frames paint the expanded card #1B1B1B on the #191919 panel — analysis/tools-settings.md; …
```
What is wrong: `analysis/tools-settings.md` does not exist at the repo root (the only match is `docs/waves/w26/analysis/tools-settings.md`). The same dangling path recurs at l.683 (part2).
Why it matters: mixed citation bases in one file — `design/figma-fidelity-spec.md` (l.41) resolves root-relative, this one does not; a reader following it lands on nothing.
What correct looks like: cite the full path `docs/waves/w26/analysis/tools-settings.md`.
Confidence: HIGH — Verification: ran (find over repo).

### F-005 [S4]
Where: themes/tokens.json:315, 352-354
Code:
```
      "$description": "Type scale from outlined-text cap-heights (doc 08 §1.3): font-size = cap/0.72, rounded [VERIFY-DESIGN]. …
```
```
      "display": {
        "$value": "32px",
        "$description": "cap 23.6px (onboarding welcome)"
```
What is wrong: the stated rule does not reproduce: cap/0.72 gives 32.78 for display (→ 32 = truncated, not "rounded" — round would be 33), while caption 7.8/0.72 = 10.83 → 11 (rounded up) and micro 4.4/0.72 = 6.11 → 6, body 14.31 → 14, time 15.42 → 15, title 18.06 → 18 (all truncated). The scale mixes floor and round; label (16.39 nominal) is re-valued to 15px by documented CONTRACT §2 deviation (l.346 — disclosed, fine).
Why it matters: minor — but a stated derivation rule that fits only some values is a small false claim in a file that trades on derivation accuracy.
What correct looks like: "cap/0.72, floored (caption rounded up; label re-pinned by CONTRACT §2)".
Confidence: HIGH — Verification: ran (python arithmetic on all 10 sizes).

### F-006 [S4]
Where: themes/tokens.json:186-189 (example; full list below)
Code:
```
        "172": {
          "$value": "#ACACAC",
          "$description": "w26 light-frame ramp (Player_-_Light.png pixel-measured, g03/g05 Lead repin 2026-11-12)"
        },
```
What is wrong: 20 of 123 primitive tokens (16%) have ZERO inbound references — 12 of 39 gray steps (525, 400, 325, 225, 200, 150, 125, 75, 50, 25, 172, 475), 4 of 13 radii (2, 10, 16, 20), 4 of 8 spaces (12, 16, 24, 32). Notably radius.16/20 — the "miniplayer floating card r16-20" pair — never became alias roles, and gray.172 is superseded as light border by gray.300 (l.112) with only prose to say so.
Why it matters: the primitive layer's charter ("exists for re-derivation and diffing", l.45) makes orphans legitimate-as-record, but nothing distinguishes "live source of a role" from "mining museum piece": gray.325's description had to talk a future author out of reuse (h20 F1) — the same trap exists unmarked for 19 siblings. Counterweight, verified: 0 of 86 alias tokens are orphaned — every role maps to the semantic layer, so the dead supply is confined to the record layer.
What correct looks like: a "record-only" marker convention (or a separate `$record: true` extension key) on the 20 steps, or pruning at the next re-pin.
Confidence: HIGH — Verification: ran (python inbound-reference census over all $values + $extensions references).

## 4. Per-token-group table (data file: groups are the units; all values verified against corpus/contrast where claimed)
| Group (lines) | Tokens | Verdict | Severity | Notes |
|---|---|---|---|---|
| root `$description` (2) | meta | REAL | S4 | F-003 (102 count) |
| `$extensions.lumen.tokens` (3–43) | meta | REAL | — | modes/accents consistent: 9 accents = 9 accent-primaries (all resolve); on-accent-candidates ×4 resolve; default-mode dark + default-accent orange match emitted :root; per-mode extension claim matches parser classify_value (W15-A02 §2); violet fallback arithmetic TRUE (4.80 vs max 4.15) |
| primitive.color.gray (49–206) | 39 | REAL | S3 | F-001 (E8E8E8), F-002 (lightness-ranked), F-006 (172/325 orphans); 20+ count claims exact; endpoint values verified |
| primitive.color.orange (207–241) | 8 | REAL | — | #D37918 381 fills exact; 450/425 WCAG numbers exact (§2) |
| primitive.color.accent (242–276) | 8 | REAL | — | 8 alternate primaries; values duplicated by eq.1–7/record BY DESIGN (independence documented, l.243, 282) |
| primitive.color.record (277–280) | 1 | REAL | — | = accent.red #E15032 (mined same value) |
| primitive.color.eq (281–311) | 7 | REAL | — | fixed EQ palette, documented as accent-independent |
| primitive.font sizes (313–355) | 10 | REAL | S4 | F-005 (rounding rule) |
| primitive.font.family (356–378) | 2 | REAL | — | sans ≡ time arrays — deliberate seam, ruling recorded in part2 F-007/§7 |
| primitive.font.weight (379–401) | 5 | REAL | — | [PROPOSAL] markers present; extrabold consumed nowhere downstream (part2 F-004) |
| primitive.radius (403–458) | 13 | REAL | — | 10 rx counts exact incl. consolidation arithmetic (10 = 8×rx19 + 2×rx20) |
| primitive.space.2–24 (459–485) | 6 | REAL | — | [PROPOSAL] grid steps; values sane; 12/16/24 are among the 20 record-only orphans (F-006) |

## 5. Test honesty (no tests in-file; the gates that execute this file)
| Gate | Would it fail if this file were broken? | Reason |
|---|---|---|
| crates/theme-tokens/tests/diff.rs (baseline row 3, 1681 passed) | YES | Re-parses this file, re-emits, byte-compares both committed copies + mutation-sensitivity test (W15-A02/A05 verified the test bodies) |
| crates/theme-tokens/tests/pipeline.rs (same run) | YES | Parses committed source; asserts shape/values; recomputes per-accent WCAG ≥4.5 |
| ui `npm run tokens:check` (baseline row 24, 4/4 PASS) | YES (staleness only) | Verifies the ui bridge copy matches regeneration — does NOT validate values |
| Mutation reasoning (F-001): change #EAEAEA → #E8E8E8 | NO test fails | Descriptions are not asserted anywhere — the false "measure" claim is invisible to every gate. Same for F-002/F-004/F-005: prose is unguarded by construction. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "canonical source of the theme-token pipeline (doc 08 §3.1)" | l.2 | REAL | parse/emit/diff chain green in baseline rows 3/24; single generator (W15-A01 lib.rs audit, corrected reachability: dev-lane only) |
| "mined from design/figma/ — 102 SVG exports" | l.2, l.41 | FALSE (count) | 105 SVGs in tree; exact-count set = 101 top-level (F-003) |
| "scripts/extract_design_tokens.py methodology" | l.41 | FALSE (dangling) | script absent from repo (F-003); methodology still reproducible via rg (§2, 26/26 counts) |
| "Gray step numbers: lightness-ranked" | l.45 | FALSE | 39 violating pairs (F-002) |
| "menu labels measure #E8E8E8" | l.144 | FALSE | 0 corpus occurrences (F-001) |
| "on-accent glyph chosen as the first candidate with WCAG contrast >= 4.5:1" + violet white fallback | l.40 | REAL | recomputed + emitted values match (§2) |
| All numeric fill/frame/rx/opacity/stroke counts in range (26 spot-checked) | various | REAL | rg reproduction, all exact (§2) |
| All numeric WCAG claims in range (60+ numbers) | l.96–227 | REAL | python recompute, all exact (§2) |
| "Never consumed by components" (primitives) | l.45 | REAL | primitives are not emitted to CSS (W15-A05 §6); ui hardcode sweep in part2 audit |

## 7. Dead / duplicated / unreachable code
- 20 zero-inbound-reference primitives (12 gray steps, 4 radii, 4 spaces) — record-layer orphans (F-006); by contrast 0 of 86 alias tokens are orphaned.
- record ≡ accent.red ≡ eq.1 (#E15032 ×3), eq.2–7 ≡ accent.yellow/green/cyan/blue/violet/pink: intentional, documented duplication (independent-of-accent EQ palette).
- font.family.sans ≡ font.family.time: intentional seam (see part2 F-007 note).
- Whole file unreachable from shipped binaries (see header) — dev/CI lane only.

## 8. Dependencies & flags
None (data file). Consumed by crates/theme-tokens (serde_json-only crate; no feature flags). Modes ["dark","light"], 9 accents, default-mode dark / default-accent orange — all consumed as data by parse.rs read_meta (W15-A02).

## 9. Open questions for the next agent
- part3 (A03): does the semantic layer's "enforced by the parser" claim (l.1075) hold? (parse.rs check_layer_ref says yes — verify from the semantic side.) Do the 6–8 zero-consumer semantic tokens (part2 F-004) include any semantic-only ones emitted in :root?
- Whoever owns design/: is `design/figma/Add Subtitles to Playlist/` (4 SVGs, Folder*.svg) post-pin? If yes, the corpus grew after the 2026-11-11 re-mining and "re-pinned" claims no longer cover the full corpus.
- ui owner: --font-size-micro (6px) is emitted but unconsumed — A05 flagged it as an a11y hazard if adopted; consider deleting at the source (this file, l.316-318).
