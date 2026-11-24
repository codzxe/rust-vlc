# CLAIM AUDIT: design/figma/EQ.svg (text asset)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | SVG, ASCII text with very long lines (`file`) |
| Size / Lines | 112,688 B / 235 lines |
| Dimensions | 824×628 (`width="824" height="628" viewBox="0 0 824 628"`, :1) |
| sha256 | a860ff0f021ea796346ad85e2a597cb347a952a5700952354abddfa9b2838c13 |
| Verdict | REAL (the EQ frame: Default collapsed bar + Variant2 expanded panel) |

## 1. Blunt verdict
Genuine Figma-export vector source with TWO variant states (`id="Property 1=Default"`,
`id="Property 1=Variant2"`) and the full 11-fader anatomy: `id="Preamp"`, `id="60 Hz"`,
`170 Hz`, `310 Hz`, `600 Hz`, `1 KHz`, `3 KHz` … — Preamp + 10 VLC bands, exactly the
fader set the implementation owns. Static paint only; no script/foreignObject/href.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "EQ.svg (824×628 — frame A1: Default collapsed bar + Variant2 expanded panel, the vertical-fader anatomy)" | w26.g15.md:5 | REAL | `sed -n 1p` -> 824×628; both `Property 1=` variant groups present (id extraction this run). |
| 2 | "EQ.svg Variant2 — one 784px panel, 11 columns visible at 68px pitch" | w26.g15.md:29 | REAL | 11 fader label ids (Preamp + 10 band labels + value rows, rg extraction); consistent with k03's live-measured "all 11 faders" flush verification. |
| 3 | "EQ.svg label paths `#Preamp`/`#60 Hz`… fill `#6B6B6B`" | w26.g15.md:35 | REAL | `rg -c 'fill="#6B6B6B"'` -> 11 hits; `#60 Hz` etc. exist as element ids (extraction above). |
| 4 | Inventory row "Audio Effects → Equalizer … `EQ.svg` (Preamp 12 dB, 10 VLC bands, preset dropdown)" | ui-inventory.md:370 | REAL | `id="Preamp"`, `id="12.0 dB"` present; 10 band labels counted. |

## 3. Contradictions
None. g15's ink-delta finding (implementation #828282 vs frame #6B6B6B, Δ23/255) is an
implementation gap measured against this asset — the frame itself is coherent.
