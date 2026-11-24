# DATA AUDIT: design/figma/Settings - interface-1.svg
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 522 lines, 456,569 bytes; 1048x865 canvas; 3 rx14 frames; 1 backdrop-filter foreignObject |
| File verdict | REAL (duplicate export) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cmp/md5sum on SVG+PNG pairs; python3 geometry+id comparison; rg consumers |

## 1. What it depicts
The Settings floating-window "Interface" screen (ids: "Settings - interface", "Interface
Settings", "Search"): 1048x865 floating window (rx14, backdrop-filter:blur via
foreignObject) — the same window grammar tools-settings.md measured for the family.
Consumers: w26.e05.md:27/266, w26.g10.md, w26.h08.md:7, w26.i13.md logs; render twin
Settings_-_interface-1.png (900x743, measured).

## 2. Duplicate status (measured)
- `Settings - interface.svg` vs `Settings - interface-1.svg`: same byte size (456,569) but
  md5 DIFFERS (e8319b9f227d vs a41bba7e95ec); 183,640 bytes differ.
- Geometry: rect width/height/rx multisets are EQUAL (42 rects) — identical layout;
  only export-session ids differ (clip0_3046_10057 vs clip0_3062_11379).
- PNG render pair `Settings_-_interface.png` vs `Settings_-_interface-1.png`: cmp =
  BYTE-IDENTICAL.
- Consumer claims: w26.e05.md "correctly a duplicate (no state to represent)" —
  substantiated (geometric duplicate). w26.h08.md:7 "byte-identical duplicate (`cmp`
  re-verified)" — TRUE for the PNG render pair (the pair a visual lane would cmp), FALSE
  if read as the SVG pair (SVGs differ by export ids at equal size).

## 3. Findings
F-001 [S4] Redundant duplicate frame in the design package: same screen exported twice
with different export ids — design/README's 102-file census counts both; no state
difference exists (e05 confirmed). Confidence: HIGH — Verification: ran (cmp, md5,
python3 geometry comparison).

## 4. Verdict
REAL asset, semantically a duplicate (SVG geometry equal, PNG render byte-identical); the
only defect is the redundant export itself.
