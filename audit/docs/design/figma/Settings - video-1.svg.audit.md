# DATA AUDIT: design/figma/Settings - video-1.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 502 lines, 363,837 bytes; 1048x865 canvas; 42 rects; backdrop-filter foreignObject |
| File verdict | REAL (duplicate export) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cmp/md5sum SVG+PNG pairs; python3 rect-multiset comparison; rg consumers |

## 1. What it depicts
The Settings floating-window "Video" screen (1048x865 rx14, backdrop-filter:blur) — same
window grammar the tools-settings analysis measured (1048x865 floating window, rx14,
controls 320px right-aligned). Consumers: w26 ratings h09.md (:6, :67, :83 — the video
screen audit), findings e06.md:24 (python normalized SVG diff), logs c5.md,
analysis tools-settings.md.

## 2. Duplicate status (measured)
- SVG pair `Settings - video.svg` vs `Settings - video-1.svg`: same byte size (363,837),
  md5 DIFFERS (bf415872a1cc vs 245ce1cb0367); export ids differ
  (clip0_3073_14187 vs clip0_3073_15375); rect w/h/rx multisets EQUAL (42 rects).
- PNG render pair `Settings_-_video.png` vs `Settings_-_video-1.png`: cmp = BYTE-IDENTICAL
  (md5 6380b30e0d444d504debc4c3e2485c13 both — exactly the md5 prefix w26.h09.md:6
  quotes: "byte-identical to `Settings_-_video.png` (md5 `6380b30e…` both)").
- w26.h09's duplicate verdict and the "no distinct state exists" conclusion are
  substantiated; h09 drove nearest real states instead — correct methodology.

## 3. Findings
F-001 [S4] Redundant duplicate export in the design package (same geometry, different
export ids, byte-identical render): design/README's 102-file census counts both frames;
no state difference exists. Confidence: HIGH — Verification: ran.

## 4. Verdict
REAL asset; the duplicate export is cosmetic debt only. Downstream w26 lanes (e06/h09)
already treated it correctly as a duplicate.
