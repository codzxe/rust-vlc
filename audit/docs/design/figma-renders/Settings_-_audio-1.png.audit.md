# CLAIM AUDIT: design/figma-renders/Settings_-_audio-1.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 73,048 B / c0fe5963a4cf2e5fdc506f5c465004d75ea774b21c823b3a320ad76875bd34a7 |
| Verdict | REAL (render; the audio settings Effects-expanded state) |

## 1. Blunt verdict
Genuine Figma render, standard 900×743 Settings-family geometry, the "-1" (fold-expanded)
state of the audio settings frame. Its consumer docs use it precisely as the
expanded-state oracle and publish honest GAPS against it (EQ embed faithful, row pitch
68px vs frame 42–46, section above the fold). Distinct file from the default-state
Settings_-_audio.png (sha256/size differ — verified this run).

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Effects scrolled into view (`audio-effects.png`)" state of /settings/audio | whole | REAL | w26.g12.md:27 maps `Settings_-_audio-1.png` -> "Effects scrolled into view"; w26.e07.md:7,121 compare `/settings/audio` against both `Settings_-_audio.png` / `Settings_-_audio-1.png`. |
| 2 | "audio-1 = Effects section expanded (diff + tree)" — distinct state, not duplicate | w26.g12.md:12 | REAL | sha256 c0fe5963… ≠ db3d3a7e… (default audio.png); sizes 73,048 vs 79,899 B — two states (contrast: the video.png/video-1.png pair IS byte-identical, md5-verified this run). |

## 3. Contradictions
None. w26.g12.md:27's GAPS row (row pitch 68px vs 42–46) is an implementation finding,
not an asset-integrity claim.
