# CLAIM AUDIT: design/figma-renders/sub_filename.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R4) |
| Claims extracted | 2 (integrity + fidelity-evidence use) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine render: `file` -> "PNG image data, 900 x 935, 8-bit/color RGBA,
non-interlaced"; python3 struct confirms 900x935, color-type 6 (RGBA), 43,615
bytes. SVG source pair `design/figma/sub filename.svg` present; this asset is
also the F6 harness-frame source (362px dropdown control) used by the f06
lane's subtitles harness.css.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Render of the filename-match dialog frame (closed+open states grammar) | whole file | REAL | `rg -ln 'sub_filename' docs/` -> w26.g19.md:14 (frame row), :86 (GAPS verdict, P3-3); w26.b11.md:7 (filename-conflict resolution); w26.e09.md:7,13 (frame inventory); w26.h17.md (rating). |
| 2 | Fidelity-evidence usage (oracle + harness frame) | whole file | REAL | g19.md:86 grades `1440-07-filename-open` against it (GAPS, P3-3 selected-label); f06 lane's harness css `.w26b11-frame--filename` comment cites "frame: sub filename.svg dropdown width" (playlist subtitles/harness.css:43-55, verified in the w26.f06 audit). |

## 3. Contradictions / 4. Fabrication indicators
None. Consumers grade the impl against the frame (GAPS verdict recorded with a
P3 defect) — oracle use, honest direction.
