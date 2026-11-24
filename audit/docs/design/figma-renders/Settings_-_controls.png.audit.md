# CLAIM AUDIT: design/figma-renders/Settings_-_controls.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 84,040 B / b30de1cd8b4bd5b70190ee34f1b5bd21e42aa86d9e21a5f426e196e1af7e3422 |
| Verdict | REAL (render of Settings>Controls) |

## 1. Blunt verdict
Genuine Figma render, standard 900×743 Settings-family geometry (identical to
Settings_-_video/library/global/privacy siblings — same `file` output). PNG signature +
IHDR parsed by `file`; no anomalies. Nothing in the repo claims pixel content that the
file contradicts.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | One of the 13-area Settings render set | whole | REAL | w26.g13.md:4 lists it as frame (`Settings_-_controls.png`); w26.h11.md:6 uses `Settings_-_controls|library|network.png` as vision-compare oracles (verbatim line verified). |
| 2 | Used as fidelity oracle in w26 loop | whole | REAL | w26.h11.md:6 (z-ai vision compare each shot vs renders); w26.f02.md:78 placeholder cites it as intended comparison target. |

## 3. Contradictions
None. w26.f02.md:78 is a "[TO BE FILLED]" placeholder that names this PNG — honest gap
disclosure, not a false claim about the asset.
