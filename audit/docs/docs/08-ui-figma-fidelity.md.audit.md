# CLAIM AUDIT: docs/08-ui-figma-fidelity.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 16 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 11 / 2 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 0 material; 3 off-by-N counts (see findings) |

## 1. Blunt verdict
The strongest kind of planning doc: its quantitative corpus-mining claims
reproduce EXACTLY when re-measured over design/figma/*.svg — fill counts
(#090909:560, #8F8F8F:1716, #6B6B6B:501, #151515:36, #D37918:381,
#371F06:189, opacity .95:169, #EAEAEA:54), radius histogram (1:1027,
2:316, 3:188, 4:354, 5:163, 6:559, 4.5:35, 10:51, 12:180, 14:98, 21:9,
30:9), stroke-width 1.5 = 3464, zero text/font attributes, onboarding 1.1x
scale values (15.4/3.3/4.4), the menu-accelerator conflict vs doc 05
(design `Ctrl+N` vs doc 05:668 `Ctrl+Shift+O`), and the implementation
surfaces it names (crates/theme-tokens, xtask tokens, ui/src/tokens,
components/**/states.md) all exist. Sloppy edges: "102 SVG exports" (101
top-level / 105 with the Add-Subtitles folder), "7 files 3–8.5 MB" (8
measured, 3.2–8.85 MB), "20 files" for settings (19 on disk). The
pixel-exactness program itself is [PROPOSAL]-marked and only partially
landed (visual harness exists per w26 evidence; not re-run here).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "The 102 SVG exports (~77 MB; 7 files 3–8.5 MB)" | :14 | REAL (size) / off-by-N (counts) | `du -sb design/figma` -> 77,507,849 B ≈ 77 MB ✓; `find -name '*.svg'` -> 101 top-level / 105 total (doc says 102); `find -size +3M` -> 8 files (3.26–8.85 MB; doc says 7). |
| 2 | "No `<text>`, `font-family`, ... exist anywhere in the package (zero-count searches)" | :16 | REAL | `rg -c '<text\|font-family\|font-size\|letter-spacing\|font-weight' design/figma --glob '*.svg'` -> 0 matches total. |
| 3 | fill counts: #090909=560, #0E0E0E@0.95 surfaces, #151515=36, #191919=298, #303030=114, #222222=73, #404040=1011, #8F8F8F=1716, #6B6B6B=501, #EAEAEA=54 | §1.2 table | REAL | `rg -o --fixed-strings 'fill="#XXXXXX"' design/figma/*.svg \| wc -l` reproduces every listed count exactly (560/36/298/114/73/1011/1716/501/54; fill-opacity .95 = 169). |
| 4 | accent counts: #D37918=381, #AB6314=232, #371F06=189, #E8851A=2, #C2711A=3 | §1.2 | REAL | Measured: 381/232/189 — exact (E8851A/C2711A not re-counted, same method). |
| 5 | radius histogram (1:1027, 2:316, 3:188, 4:354, 5:163, 6:559, 4.5:35, 10:51, 12:180, 14:98, 21:9, 30:9) | §1.4 | REAL | Per-value `rg -o 'rx="N"' design/figma/*.svg \| wc -l` -> 1027/316/188/354/163/559/35/51/180/98/9/9 — every value exact. |
| 6 | "line icons, stroke #8F8F8F, stroke-width 1.5 (3464 uses)" | §1.4 Icon system | REAL | `rg -o 'stroke-width="1.5"'` -> 3464; stroke #8F8F8F -> 3242 (doc: "+3221 strokes" — off by 21, method-boundary). |
| 7 | Onboarding "15.4px control height, rx 3.3, 4.4px knob — exactly 1.1× the 14/3/4 values" | §1.1 | REAL | `rg -o` in Onboarding-3.svg -> `height="15.4"` ×2, `rx="3.3"` ×2, `width="4.4"` ×1; 15.4/1.1=14, 3.3/1.1=3, 4.4/1.1=4. |
| 8 | annotation chrome: "0.5-inset rect stroked #8A38F5 dashed 10 5" | §1.1 | REAL | design/figma/Icon.svg:3 `<rect x="0.5" y="0.5" width="547" height="811" rx="4.5" stroke="#8A38F5" stroke-dasharray="10 5"/>`. |
| 9 | settings "×13 areas ... design/README says '14 areas' — 13 distinct areas found on disk" | §1.6, design/README.md:20 | REAL | `ls Settings*.svg` -> 19 files, 13 distinct areas (audio, controls, extensions, global, interface, library, network, performance, playback, privacy, recording, subtitles, video); design/README.md:20 states "13 settings areas ... count confirmed by decision D-002" — README itself says 13, not 14 (doc 08's "README says 14" is stale vs the README as it stands today). |
| 10 | "menu accelerators partially conflict with doc 05 §8.2 seed (`Ctrl+Shift+O` for open URL vs design `Ctrl+N`)" | §1.5 | REAL | docs/05-command-layer.md:668 `| `Ctrl+Shift+O` | open URL |`; design/figma/menu.svg carries `id="Ctrl + N"` (+ Ctrl+O/F/D/C/R/P ids verified). |
| 11 | token pipeline: crates/theme-tokens parse/emit, `cargo xtask tokens`, ui/src/tokens/generated.css | §3.1 | REAL | crates/theme-tokens/src/{lib.rs,emit.rs,color.rs,lint.rs} present; tools/xtask/src/cmd/tokens.rs present; ui/src/tokens/ present (generated/, tokens.test.ts, TokenSandbox.tsx). |
| 12 | state tables "live in `ui/src/components/**/states.md`" | §5.5 | REAL | ui/src/components/button/states.md exists (also slider/, form/ states.md per audit/docs listing). |
| 13 | "13 distinct areas" table cell says "(20 files)" | §1.6 | S4 imprecision | 19 Settings-*.svg on disk (13 areas + 6 `-1` states); doc says 20. |
| 14 | cap-heights measured by "Python bbox script over M/L anchors" | §1.1 | UNVERIFIED | Method plausible; script not in repo (no such script found under tools/); geometry values not re-measured here. |
| 15 | screenshot harness: `cargo xtask ui-shots` + per-OS baselines | §6.3 | REAL-UNPROVEN | Visual-harness evidence exists in-repo (docs/ui/e2e/visual/MATRIX-*.md series, audit/docs/ui/...) but the doc's specific tolerances/protocol were not re-executed here. |
| 16 | `resvg` "pure-Rust rasterizer" for baselines | §6.1 | UNVERIFIED | No design-baseline xtask command found (tools/xtask/src/cmd has no design_baseline.rs); [PROPOSAL]-marked in doc — consistent, but unlanded. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "102 SVG exports" :14 | disk: 101 top-level, 105 incl. folder | The disk is true; count imprecise (S4). Color mining matched the 101-file set exactly, so the mining corpus was the same minus renames/additions. |
| "design/README says '14 areas'" §1.6 | design/README.md:20 "13 settings areas ... count confirmed by decision D-002" | README today says 13 — doc 08's quote is stale (or the README was fixed later); either way the on-disk truth is 13 = doc 08's own measured count. |
| "+3221 strokes" of #8F8F8F | measured 3242 | Off by 21 — counting-boundary difference, not a fabrication. |

## 4. Fabrication / inflation indicators
None. Every marker discipline ([VERIFY-DESIGN]/[PROPOSAL]/[VERIFY]) is
respected; the doc understates its own corpus precision. The off-by-one
file counts are the only numeric slips, and they cut against the doc's
tidiness, not its honesty.

## 5. Matrix rows
None — not a matrix chunk. Doc 08 is the design-side authority cited by
feature-matrix rows for UI surfaces (audit trail: w26 fidelity logs).

## 6. Findings
F-001 [S4] "102 exports" vs 101/105 on disk; "7 files 3–8.5MB" vs 8
(3.26–8.85MB); "(20 files)" vs 19 — three off-by-N counts in §1.1/§1.6.
Cosmetic but they seed downstream counts (e.g. baseline-set sizing).
Confidence HIGH — Verification: ran (find/ls/du).
F-002 [S4] "design/README says '14 areas'" — README as committed says 13
(design/README.md:20). Stale quote. Confidence HIGH — read.
