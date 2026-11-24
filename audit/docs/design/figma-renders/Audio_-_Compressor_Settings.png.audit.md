# CLAIM AUDIT: design/figma-renders/Audio_-_Compressor_Settings.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A04 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 1847, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 75,566 B / c0013787a4432223f998f6d4901d497f0ee80f82d78455bf9f1c1206404a2f20 |
| Verdict | REAL (render of the Compressor tool panel; the audio-tools frame family) |

## 1. Blunt verdict
Genuine Figma render — the tallest of the audio-tools family at 900×1847 (the compressor
panel is a long scrolling surface; contrast the 900×743 Settings frames). Used as the
compressor oracle in the w26 tools lane; consumer verdicts include an honest [VERIFY
VLM] marker where the vision pass was the only evidence.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | One of the 8 audio-tool frames (CONTRACT §3 list) | whole | REAL | docs/waves/w26/CONTRACT.md:53 enumerates `Audio_-_Compressor_Settings.png` among the audio-tools renders; w26.e10.md:11 includes it in the audio-tools scope set. |
| 2 | Compressor oracle: "7 rows @ head pitch 76 (frame-exact); PLAIN values #6B6B6B 15px … thumb 8×24; flat fill; struct-sim 77%" — verdict MATCH [VERIFY VLM] | w26.e10.md:41 | REAL (frame side) | The row/pitch/ink facts are cited FROM this frame by the consumer (its verdict cell honestly marks the VLM-dependent check); w26.i09.md:89 records the vision call against this PNG. |

## 3. Contradictions
None. The consumer's MATCH verdict is tagged [VERIFY VLM] — an honestly-uncertain
verdict against this asset, not an overclaim.
