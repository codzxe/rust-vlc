# CLAIM AUDIT: docs/waves/w26/CONTRACT.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R3) |
| Claims extracted | 12 |
| REAL 7 / STALE-drift 1 / UNVERIFIED 4 (process/network claims) | |
| Internal contradictions found | 0 direct; 1 drift vs themes/tokens.json |

## 1. Blunt verdict
The wave-26 binding contract's verifiable artifacts all check out: all 22 spot-checked
reference renders exist (105 PNG renders in design/figma-renders/), the PNG↔SVG
space→underscore naming rule holds, the palette hexes land in themes/tokens.json, the
eslint semantic-tokens-only rule and the i18n keys/<task-id>.ts lane both exist, and the W25
spec exists. One quiet-text hex has drifted from the token lane's later WCAG arbitration.
Process claims (GitHub REST push, dev server) are unverifiable from this sandbox.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "design/figma/* = source of truth ... PNG renders at design/figma-renders/" | :10, :40-41 | REAL | 105 renders present; space→underscore rule verified (Video - Colors.svg ↔ Video_-_Colors.png) |
| 2 | reference-frames table (Onboarding 11, Player-1..6, customize 9 states, Settings 13 areas, ...) | :43-61 | REAL | spot-census 22/22 present incl. Player-6, customize-8, Settings-extensions, Designs.png; count 105 total |
| 3 | palette: "#090909 surfaces, #191919 panels, #404040 hairline, #c6c6c6 quiet, ONE orange #d37918 (pressed #ab6314, hover #e8851a, glow #371f06)" | :30-32 | REAL | themes/tokens.json: orange.500 #D37918, orange.600 #AB6314 (accent.strong "pressed fills"), orange.300 #E8851A, orange.700 #371F06 (accent.tinted), primitives #090909/#191919/#404040/#c6c6c6 all present |
| 4 | "#8f8f8f quiet text" | :31 | STALE-drift | shipped content.quiet = gray.450 #6B6B6B dark (tokens.json:96 documents the w26.i19/h20 WCAG arbitration); #8f8f8f not in tokens.json |
| 5 | "Semantic tokens only ... (eslint semantic-tokens-only). Raw hex ONLY in themes/tokens.json ... /* frame: <Frame.svg> */" | :76-79 | REAL | ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js exists; w26.e15.md:13 census found exactly the frame-provenance pattern in ui css |
| 6 | "Genuinely new keys go in a NEW file ui/src/i18n/keys/<task-id>.ts" | :81-83 | REAL | ls ui/src/i18n/keys -> 54 w26.*.ts files + README |
| 7 | "design/figma-fidelity-spec.md (the W25 spec)" is an authority | :4-5 | REAL | design/figma-fidelity-spec.md present; tokens.json:41 cites "W25 spec §1 ... 2026-11-11" |
| 8 | "Status: ACTIVE (Lead, 2026-11-11)" | :3 | REAL-UNPROVEN | no contradiction anywhere in w26 logs (agents treat it as binding); authorship unverifiable |
| 9 | §8 self-push via GitHub REST to codzxe/rust-vlc, token never in files | :131-166 | UNVERIFIED | no network in audit sandbox; token-discipline statement is honest-by-construction |
| 10 | "Dev server: Lead-owned at http://127.0.0.1:5173" | :170-171 | UNVERIFIED | not running during audit |
| 11 | "z-ai vision may 429 ... fall back permanently to measurement ... g-lane verifiers proved this path works" | :174-178 | REAL | w26.g17.md:6 records exactly that fallback ("429 on the single attempted call" -> SVG/DOM measurement) |
| 12 | "Work in /home/z/my-project/rust-vlc (shared checkout)" | :65 | UNVERIFIED (env) | dir absent from audit sandbox (repo snapshot lives at audit-workspace); addresses the dev env, not this copy |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| quiet text #8f8f8f (:31) | tokens.json content.quiet = #6B6B6B (gray.450) | the token lane won (i19/h20 WCAG arbitration, tokens.json:96); the contract's prose palette is pre-arbitration drift |

## 4. Fabrication / inflation indicators
None. The contract's honesty rules (§4.9 "No fabricated claims ... Unknown facts → [VERIFY]")
are themselves enforced by later review waves (w26.j/h lanes graded exactly this).

## 5. Reachability
N/A (process/binding contract document). The artifacts it governs are the shipped UI.
Commands: frame census loop (22 OK), ls counts (105 renders, 54 key files), python3 token
dump (orange.500/600/300/700, gray.450 hexes), rg '#hex' tokens.json, file existence checks.
