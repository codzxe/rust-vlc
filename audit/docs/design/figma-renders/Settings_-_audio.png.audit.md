# CLAIM AUDIT: design/figma-renders/Settings_-_audio.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 79,899 B / db3d3a7e9161efd1087cca2f601d23ca3d4a9ea3d8ae248fa0382310198c3cc |
| Verdict | REAL (render of Settings>Audio default state) |

## 1. Blunt verdict
Genuine Figma render, standard 900×743 Settings-family geometry. Consumers treat it as the
default-state oracle for `/settings/audio`; the companion `Settings_-_audio-1.png` is the
Effects-expanded state (rendered below it in the family, same width). No consumer claim
contradicts the file's actual type/dimensions.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Default-state audio frame (vs audio-1 = expanded state) | whole | REAL | w26.g12.md:26 maps `Settings_-_audio.png` -> `/settings/audio` default; w26.g12.md:27 maps `-1` -> Effects scrolled; w26.e07.md:7 "/settings/audio vs `Settings_-_audio.png` / `Settings_-_audio-1.png`". |
| 2 | Fidelity oracle use in w26 | whole | REAL | w26.c5.md:85 mission check "/settings/audio vs `Settings_-_audio-1.png`"; w26.h10.md references (rg hit). |
| 3 | Differs from Settings_-_audio-1.png | whole | REAL | distinct sha256 (db3d3a7e… vs c0fe5963…), distinct byte sizes 79,899 vs 73,048 — two states, not a duplicate export. |

## 3. Contradictions
None. g12's row-pitch gap findings concern the IMPLEMENTATION vs frame, not this asset's integrity.
