# CLAIM AUDIT: docs/waves/w26/analysis/tools-settings.md (725 lines)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 7 / 0 / 1 / 0 / 0 / 1 / 0 |

## 1. Blunt verdict
A measurement spec whose numbers actually come out of the SVGs it cites. Spot-verifying
its most falsifiable "SVG wins" corrections against the source files: tool-card rx=6,
settings top bar 50px, floating window 1048×865 rx14 #0E0E0E@95%, #AB6314 slider handle —
all byte-verified. One palette claim is stale/false (#969696 "NOT in themes/tokens.json
yet" — it is there, line 112, icon-role). Per-row measurement claims (hundreds of them) are
derived by the same documented method; I verified the method, not every number.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Tool card radius is r6, NOT r8" | L24 | REAL | rg -c 'rx="6"' design/figma/Video - Essential.svg -> 10 occurrences |
| 2 | "Slider handle is #AB6314 ... 8×24 rx4" | L27 | REAL (color) | rg '#AB6314' in Video - Essential.svg -> present |
| 3 | "Settings top bar is 50px (not 36-40)" | L31 | REAL | Settings - global-1.svg line 6: `<rect width="1048" height="50" fill="#090909"/>` |
| 4 | "Settings renders as a floating window 1048×865 rx14 #0E0E0E@95%" | L39-40 | REAL | Settings - global-1.svg line 1+4: `width="1048" height="865"`, `<rect width="1048" height="865" rx="14" fill="#0E0E0E" fill-opacity="0.95"` |
| 5 | "#1A1611 ... NOT in themes/tokens.json yet" | L47-48 | REAL | rg '1A1611' themes/tokens.json -> no match |
| 6 | "#969696 (row labels) are NOT in themes/tokens.json yet" | L47-48 | FALSE (stale) | themes/tokens.json:112 contains #969696 — added as "light-mode icon strokes ... icon role only since w26.f15, NOT text"; doc claim superseded |
| 7 | "PNG renders ... 900px-wide ≈ 2.26× for 400px frames" | L8-9 | REAL | python3 struct read: Settings_-_video.png 900×743, media.png 900×544; 900/400 = 2.25 |
| 8 | "SVG XML in design/figma/ is ground truth (parsed: every rect, path bbox ...)" | L6-7 | REAL-UNPROVEN→method REAL | the method is evidenced by verified outputs (rows 1-4); full parse not re-executed |
| 9 | "z-ai vision used to confirm labels" | L9-10 | REAL-UNPROVEN | external tool run, not re-runnable from repo; labels quoted (EQ preset names) are consistent with frame names present in design/figma (EQ presets.svg, HDR Options.svg exist) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| #969696 not in tokens.json "yet" | tokens.json:112 has #969696 (icon role) | tokens.json wins — doc written before w26.f15/i19 token additions |
| Font weights [VERIFY] (can't prove 400 vs 500 from outlines) | — | honest self-flag, no contradiction |

## 4. Fabrication / inflation indicators
None. The doc's discipline is exemplary: corrections flagged against a prior spec with
"SVG wins" arbitration; uncertainties marked [VERIFY] (font weights).

## 5. Matrix rows
Indirect: feeds ui-screen-av-tools and settings-area matrix rows (other agents' scope).
Commands: rg on design/figma/*.svg; sed Settings - global-1.svg:1-6; python3 PNG header
parse; rg themes/tokens.json.
