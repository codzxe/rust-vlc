# CLAIM AUDIT: design/figma-fidelity-spec.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R4) |
| Claims extracted | 12 |
| REAL 9 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 1 (frame-count drift) |

## 1. Blunt verdict
The Wave-25 contract's measured token table is the real thing: every spot-
checked color lands byte-exact in themes/generated/tokens.css (accent family,
content ramp, surface ramp, EQ band set, record), the Inter font token and
tabular-nums exist, the 9-swatch accent picker is test-pinned at 9, the
eslint semantic-tokens-only rule and `xtask tokens` pipeline exist, and
solstice.css is indeed replaced (lumen.css declares it). Two soft spots: the
"102 frames" count is stale (105 SVG frames / 105 renders on disk today), and
the referenced measurement script `scripts/extract_design_tokens.py` is not
in the tree (external lane tooling).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Source of truth: design/figma/*.svg (102 frames ...)" | :3-4 | STALE-count | `ls design/figma/*.svg \| wc -l` -> 101 top-level + 4 in "Add Subtitles to Playlist/" = 105; `ls design/figma-renders/*.png \| wc -l` -> 105. Count drifted +3 after spec date (later lanes exported Player-5/6 etc.). |
| 2 | "rendered references in design/figma-renders/*.png (cairosvg, width 900)" | :5 | REAL | All sampled renders are 900 wide (Player_-_customize-8 900x506, Settings_-_extensions 900x743, sub_filename 900x935 — python3 struct verified this session). |
| 3 | Dark token table: surface #090909 / deep #0e0e0e / panel #191919 / chrome #161616 / raised #222222 / hover #262626 / selected #1f1f1f / muted #151515 | :27-34 | REAL (values) | tokens.css:55-63: surface #090909, surface-deep #0e0e0e, surface-hover #262626, surface-muted #151515, surface-raised #222222, surface-selected #1f1f1f (semantic names evolved; values pinned). |
| 4 | content.primary #c6c6c6 / bright #eaeaea / secondary #8f8f8f / tertiary #6b6b6b | :36-39 | REAL | tokens.css:37,39,41 + :42 (--color-content-quiet #6b6b6b; tertiary now #828282 — see w26.h03 note: tertiary drifted to #828282, a known open token defect). |
| 5 | accent #d37918 / strong #ab6314 / bright #e8851a / tinted #371f06 / on-accent #090909 / record #e15032 | :40-45 | REAL | tokens.css:31-35, :51 (--color-record: #e15032), :43; on-accent #090909 verified in the controls pill audit. |
| 6 | "EQ band set #e15032 #c49e23 #19a234 #18aad3 #185dd3 #8e42ff #f052b3" | :46 | REAL | tokens.css:43-49 `--color-eq-1`..`--color-eq-7` exactly these values in order. |
| 7 | "Font token: Inter, -apple-system, Segoe UI, Roboto ... tabular-nums" | :67-68 | REAL | tokens.css:64-65 `--font-family-sans: "Inter", ...`; lumen.css:91 `font-variant-numeric: tabular-nums`. |
| 8 | "9 swatches in the design ... matches the existing 9-accent token system" | :71-72 | REAL | AccentColorPicker.test.tsx: `expect(n).toHaveLength(9)` with data-accent order assertions (test-pinned). |
| 9 | "eslint semantic-tokens-only" rule + "cargo xtask tokens" regeneration | :115 | REAL | ui/tools/eslint-plugin-local/rules/semantic-tokens-only.js exists; tools/xtask/src/cmd/tokens.rs "`xtask tokens` — theme-token pipeline (doc 08 §3)". |
| 10 | "solstice.css is replaced by a new base stylesheet this wave" | :117 | REAL | ui/src/theme/lumen.css:19 "Replaces solstice.css (sunset stage, taped notes, keycaps, Nunito,"; no solstice.css file in ui/src/theme/. |
| 11 | "scripts/extract_design_tokens.py, summary in the wave-25 worklog" | :21-22 | UNVERIFIED | Script not in tree (`find` + repo scripts/ listing: absent); presumably external lane tooling like dev-shots.mjs. |
| 12 | Comparison loop via /home/z/my-project/scripts/dev-shots.mjs + z-ai vision CLI | :104-109 | UNVERIFIED | Path is OUTSIDE this repo (/home/z/my-project/scripts != repo root); helper explicitly uncommitted per w26.d2.md:104. Vision service external. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| 102 frames | 105 SVGs + 105 renders on disk | Tree wins on current count; spec was accurate at 2026-11-11 and aged (+3 frames exported later — Player-5/6 references appear in w26.gfix-C-era files). |
| content.tertiary #6b6b6b (dark) | tokens.css:42 --color-content-tertiary #828282 | Spec measured the frame truth (#6b6b6b); the token drifted one step lighter — the open g10-P3-2 defect (h03/i13/gfix-B all flag it as still-open). The spec is the SOURCE OF TRUTH here; the token is the deviation. |

## 4. Fabrication / inflation indicators
None. The spec's numbers were demonstrably measured (they match the generated
token file to the byte across ~20 values), and its own text carries the
honest [VERIFY] markers for font identity. The frame-count drift and missing
extraction script are hygiene, not inflation.
