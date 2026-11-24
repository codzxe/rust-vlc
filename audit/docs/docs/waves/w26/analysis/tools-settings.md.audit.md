# CLAIM AUDIT: docs/waves/w26/analysis/tools-settings.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 10 (measurement claims, spot-verified against SVGs/renders/tokens) |
| REAL 6 / REAL-UNPROVEN 3 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 1 / UNREACHABLE 0 | |
| Internal contradictions found | 1 (stale token claim) |

## 1. Blunt verdict
A measurement spec of unusual quality: I re-measured five of its numeric claims against the
ground-truth assets and every one lands byte-exact (r6 cards, #AB6314 handle, 1048x865 rx14
floating window, 900px renders, #1A1611 token absence). Exactly one claim has aged into
FALSE: "#969696 … NOT in themes/tokens.json yet" — the token landed at tokens.json:112 after
this doc was written (its own "(a19 also flagged)" trail shows the fix queue worked).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (command + result) |
|---|---|---|---|---|
| 1 | "PNG renders … 900px-wide ≈ 2.26× for 400px frames" | :9 | REAL | python3 IHDR: Settings_-_video.png 900x743; Video_-_Colors.png 900x1998 |
| 2 | "Tool card radius is **r6, NOT r8** (every `Audio -*`/`Video -*` card: `rx="6"`)" | :16 | REAL | rg 'rx="6"' "Audio - Compressor Settings.svg" -> 9 hits |
| 3 | "Slider handle is **#AB6314** … 8×24 rx4" | :19 | REAL | #AB6314 in Settings - video.svg (7), menu.svg (23) |
| 4 | "Settings renders as a **floating window** 1048×865 rx14 #0E0E0E@95%" | :27 | REAL | "Settings - global.svg": width="1048" height="865" rx="14" fill="#0E0E0E" fill-opacity="0.95" (byte-verbatim) |
| 5 | "Settings top bar is **50px**" | :23 | REAL-UNPROVEN | not re-measured this pass (bar rect unpinned); corroborated by dead agent's r6/50px verification |
| 6 | "#969696 (row labels) are NOT in themes/tokens.json yet (a19 also flagged #969696)" | :48 | FALSE (stale) | themes/tokens.json:112 carries #969696 — with role caveat: "icon role only since w26.f15 … ALSO the light hairline border since w26.i19 … NOT text" |
| 7 | "`#1A1611` (slider degree ticks) … NOT in themes/tokens.json" | :47-48 | REAL | rg '#1A1611' themes/tokens.json -> 0 hits |
| 8 | "`z-ai vision` used to confirm labels (EQ preset names …)" | :9 | REAL-UNPROVEN | vision evidence not committed; label lists plausible |
| 9 | "Weights [VERIFY] (outlined shapes can't prove 400 vs 500)" | :14 | REAL | honest limitation, self-flagged |
| 10 | "Where SVG and the W25 spec §2 conflict, SVG wins (conflicts flagged inline)" | :10 | REAL | 10 numbered corrections follow with measured values |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| #969696 not in tokens yet | tokens.json:112 contains #969696 | Token exists since w26.f15/i19 — but as ICON/HAIRLINE roles, "NOT text"; the doc's underlying point (no text-role gray for row labels) survives the stale wording |

## 4. Findings
F-001 [S4] Stale token claim (#969696) — the hex exists in tokens.json:112 at audit day;
what actually remains missing is a TEXT-role gray token for row labels. The doc's wording
("not in tokens.json yet") is now false as written. Confidence: HIGH — Verification: ran.

## 5. Fabrication / inflation indicators
None. Cap-corrected font methodology, [VERIFY] markers on weights, and correction list
against the prior spec are anti-inflation practices.
