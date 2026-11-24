# CLAIM AUDIT: design/figma-renders/Settings_-_library.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A08 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 72,350 B / e0c7923ee2b96ab8237230bf76b5d0486c1160c85128026d1384a45bff13a260 |
| Verdict | REAL (render of Settings>Library; oracle for the library-lane gap verdicts) |

## 1. Blunt verdict
Genuine Figma render, standard 900×743 Settings-family geometry (matches the
controls/audio/video/global/privacy siblings). It is the oracle the w26 ratings used to
publish an HONEST FAILURE: h11's finding "Library content columns are voids — 16 of 16
frame rows + 2 Advanced bars absent" is measured against this very PNG's 7-row layout.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Settings>Library frame with 7 rows + Advanced bar at y490 | whole | REAL | w26.h11.md:83: "compare `Settings_-_library.png` (7 rows, adv bar y490)" — the render is the stated measurement reference; `file` confirms 900×743 RGBA (743px height consistent with the y-coordinate arithmetic). |
| 2 | Reference frame of the library/network lane | whole | REAL | w26.g14.md:6 lists it among reference frames; w26.b08.md:14 "hosts the playlist-services fold"; w26.h11.md:6 vision-compare set `Settings_-_controls|library|network.png`. |

## 3. Contradictions
None. The consumer that leans on it hardest (h11) uses it to prove a MISSING feature
(`schemas/settings.toml` has no library.* seeds) — the asset is used as a gap witness,
not an achievement.
