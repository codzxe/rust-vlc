# AUDIT: ui/src/screens/playlist/subtitles/SubtitleGlyphs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 105 / 3 components (+ GlyphProps interface) |
| File verdict | REAL (presentational SVG port; geometry verified line-by-line against the actual frame SVGs; consumed by the shipped Add-Subtitles flow) |
| Reachable from shipped entry point? | YES in the UI tree (call chain: PlaylistScreen.tsx:50,391 `<AddSubtitlesDialog>` → SubtitlePickRow.tsx:22,63 `<SubtitleFileGlyph>` / :74 `<CycleGlyph>` and MatchedPairsList.tsx:18,50,61 `<MediaFileGlyph>`/`<SubtitleFileGlyph>`) — with the W14 ceiling: the tauri host cannot compile at HEAD, so no shipped binary reaches any UI. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -ln "SubtitleFileGlyph|MediaFileGlyph|CycleGlyph" ui/src` → 3 consumer files; `rg -n "MatchedPairsList|SubtitlePickRow" ui/src` → AddSubtitlesDialog flow ← PlaylistScreen.tsx:50; read of design/figma/"subtitle one.svg" (Vector 9 `M36 39H44`, Vector 10 `M36 42H40`, stroke #6B6B6B — matches the port's provenance comment exactly); read of design/figma/"subtitle multiple.svg" (Vector 42 `M473 83.5L486 83.5…`, Vector 43 `M485 72.5H472…`, stroke #8F8F8F — matches port y-origins 69→3.5/14.5, x 472–486→0–14); `rg -n "content-tertiary" themes/generated/tokens.css` → dark #828282, light #474747; read of "Add Subtitles to Playlist/Folder.svg" (ticks near x37.95/x43.8 confirmed in path data context); aspect-ratio arithmetic hand-checked. |

## 1. One-paragraph verdict
The rare clean file: three decorative SVG glyphs whose geometry I verified against the real Figma-export frames coordinate-by-coordinate (Vector 9/10 text lines, Vector 42/43 stacked arrows, the filmstrip ticks) — every claimed frame coordinate in the comments exists verbatim in design/figma, and the width/height aspect math ((size·20)/18, (size·16)/18, (size·14)/18) is correct. All three glyphs have real consumers in the Add-Subtitles flow, aria-hidden/focusable=false is right for decoration, and no raw colors leak. One defect: the header asserts the frame's #6B6B6B stroke "equals" --color-content-tertiary — the token's actual values are #828282 (dark) / #474747 (light), so the equality is false and a dark-mode render is ~10% lighter than the frame in a file whose contract (§4.4) is "pixel-perfect" ports.

## 2. What actually works (verified)
— verified by reading design/figma/"subtitle one.svg":8-9: `<path id="Vector 9" d="M36 39H44" … stroke="#6B6B6B"/>` and `Vector 10 d="M36 42H40"` — the port's `M5 10.5h8M5 13.5h4` (:45) is exactly these lines on the claimed x31/y28 icon origin (36−31=5, +8=13; +4=9), with the −0.5 stroke-center adjustment for strokeWidth 1.5.
— verified by reading design/figma/"subtitle multiple.svg":36-37: Vector 42 at y83.5 (port y14.5 = 83.5−69), Vector 43 at y72.5 (port y3.5), x-span 472–486 → 0–14 in a 14×18 viewBox — the CycleGlyph geometry is a faithful port.
— verified by `rg` on consumers: SubtitlePickRow.tsx:63/74 and MatchedPairsList.tsx:50/61 render all three glyphs; index.ts:30 re-exports them; no dead exports.
— aspect math: SubtitleFileGlyph height = size·20/18 for an 18×20 viewBox ✓; MediaFileGlyph height = size·16/18 for 18×16 ✓; CycleGlyph width = size·14/18 for 14×18 ✓.

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/playlist/subtitles/SubtitleGlyphs.tsx:2-4
Code:
```
// w26.b11 — frame-ported glyphs for the subtitle flow (per-frame
// pixel-perfect SVG ports, contract §4.4): the subtitle-file glyph
// ("subtitle one.svg" Icon — page + fold + two text lines, stroke 1.5
// #6B6B6B = --color-content-tertiary), the media-file glyph
```
What is wrong: The asserted equality "#6B6B6B = --color-content-tertiary" is false. themes/generated/tokens.css:42 defines `--color-content-tertiary: #828282` (dark, line 126: `#474747` light); the frame stroke is #6B6B6B ("subtitle one.svg":8). Since the glyphs use `stroke="currentColor"`, a host that actually applies content-tertiary renders the glyph ~10% lighter than the frame. No spec maps frame grays onto content tokens (checked design/figma-fidelity-spec.md — its icon-color row is #9a9a9a for a different frame set; no mapping rule stated).
Why it matters: The file's contract is "pixel-perfect SVG ports" and the comment asserts the color mapping is exact. A verifier reading the comment stops checking; a pixel-diff run against the frame would flag the drift. This is the same pattern as engine-info: a written equality that the data contradicts.
What correct looks like: Either add a `--color-content-frame-gray: #6B6B6B` token (or map to the closest existing token and document the delta honestly: "#6B6B6B frame → content-tertiary #828282, accepted ±10% tone shift per §…"), or paint the stroke with the frame value.
Confidence: HIGH — Verification: frame SVG read; token values read; no mapping policy found in the fidelity spec.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| GlyphProps | 14-18 | REAL | — | size + className; documented |
| SubtitleFileGlyph | 24-48 | REAL | S3 | F-001 (header equality); geometry verified vs Vector 9/10; 18×20 viewBox, fold flap path matches frame Rectangle 1 description |
| MediaFileGlyph | 56-78 | REAL | — | page outline r2 matches frame mask; two diagonal ticks at rel x≈5.5/11 match frame 37.95/43.8 on origin x33 |
| CycleGlyph | 84-105 | REAL | — | stacked ←/→ verified vs Vectors 42/43; stroke 1.5 #8F8F8F claim TRUE (frame stroke attr read) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| no direct unit test for SubtitleGlyphs | N/A | glyphs are exercised indirectly by AddSubtitlesDialog.test.tsx / MatchedPairsList.test.tsx / SubtitlePickList.test.tsx (render-level); geometry fidelity is untestable in jsdom and was verified manually (w26.e09 loop) — a broken path would crash those renders, a wrong path would pass silently |
| w26.e09 frame comparison | partially | VLM comparison failed (HTTP 429 ×8); verdicts were DOM/SVG-measurement-backed and marked [VERIFY] — geometry claims corroborated by my own frame reads |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "pixel-perfect SVG ports" (geometry) | :2 | REAL | every coordinate checked against design/figma frames (§2 above) |
| "stroke 1.5 #6B6B6B = --color-content-tertiary" | :4 | FALSE | token is #828282/#474747 (F-001) |
| "stroke 1.5 #8F8F8F" (CycleGlyph) | :7-8 | REAL | subtitle multiple.svg:36-37 stroke attr |
| "Strokes are currentColor so hosts tint via tokens; no raw colors live here" | :8-10 | REAL | zero color literals in the file |
| "x472–486, y69–87" / "x31–49, y28–48" frame bounds | :21-22, 81-82 | REAL | matches the svg path data read |

## 7. Dead / duplicated / unreachable code
None. All three components consumed (SubtitlePickRow, MatchedPairsList); re-export in subtitles/index.ts:30 has importers via the flow.

## 8. Dependencies & flags
No imports except React types. Pure presentational; no engine dependency; correct decorative a11y (aria-hidden + focusable="false" on all three).

## 9. Open questions for the next agent
- Is there a design-system decision that frame grays (#6B6B6B/#8F8F8F) intentionally snap to the nearest content token? If it exists, cite it in this header; if not, F-001 stands as a fidelity drift.
