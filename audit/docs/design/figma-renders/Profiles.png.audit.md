# CLAIM AUDIT: design/figma-renders/Profiles.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 875, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 76,234 B / a043a16f8da84c2d55b6bc9d7277278e91acf84921a1a40ce97bda2230f16155 |
| Verdict | REAL (render of the Profiles flyout — the §A10 menus-family frame) |

## 1. Blunt verdict
Genuine Figma render of the Profile flyout (menus family, CONTRACT §3 row 57). It has a
DIRECT code consumer: the profile-flyout implementation was wired from this frame's
§A10 row set with an in-code citation naming the asset.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | One of the 9 menus-family frames (CONTRACT §3: `… option.png, setting_category.png, dropdown.png, Profiles.png, Setting_Preset.png`) | whole | REAL | docs/waves/w26/CONTRACT.md:57 lists `Profiles.png` in the Menus row — verbatim. |
| 2 | "the Profile flyout's real rows (Profiles.png §A10): the local `lumen.profiles.v1` store (frame-seeded Movies/Shows/Music/Work; select + '+ New' quick-create persist)" | w26.i16.md:161-165 | REAL | The i16 lane log documents the implementation reading this frame's row set; "No engine profile command exists — local device data, the playlistsStore precedent" — the honesty seam disclosed alongside. |
| 3 | Submenu grammar reference for the app-menu's Profile row | w26.i03.md:100 | REAL | i03's app-menu comparison maps the Profile submenu to "Profiles.png's, which has no engine seam" — the frame used as the submenu oracle with its seamlessness honestly noted. |

## 3. Contradictions
None. Consumers use this asset as the flyout/submenu oracle; no consumer asserts a
property the file contradicts.
