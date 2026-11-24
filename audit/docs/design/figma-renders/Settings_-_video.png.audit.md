# CLAIM AUDIT: design/figma-renders/Settings_-_video.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A08 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 78,981 B / 8cde030e8b74e3f4547ba68bd6080a24c92fa049015d90055666e92d8b87f6c9 |
| md5 | 6380b30e0d444d504debc4c3e2485c13 — byte-identical to Settings_-_video-1.png (verified) |
| Verdict | REAL (render; duplicate-export pair with -1) |

## 1. Blunt verdict
Genuine Figma render, standard 900×743 Settings-family geometry. The notable verified
fact: this PNG and `Settings_-_video-1.png` are the SAME image re-exported (md5 identical),
exactly as w26.g12.md discloses — the "video-1" state does not exist as a distinct render.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "`Settings_-_video.png` vs `-1` = byte-identical (md5 `6380b30e…` both) — duplicate export, one state only" | w26.g12.md:12 | REAL | md5sum both files -> `6380b30e0d444d504debc4c3e2485c13` for BOTH (executed). |
| 2 | Oracle for /settings/video default | whole | REAL | w26.g12.md:24-25 maps video.png + video-1 (same capture) to the default route; w26.h09.md consumer (rg hit). |
| 3 | Standard Settings render family geometry | whole | REAL | `file` -> 900x743 RGBA like controls/audio/library/global/privacy siblings (verified this run). |

## 3. Contradictions
None. The duplicate-export fact is DISCLOSED by the consumer doc, not hidden — evidence the
w26 loop audits its own inputs honestly.
