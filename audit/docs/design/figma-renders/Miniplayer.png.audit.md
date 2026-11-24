# CLAIM AUDIT: design/figma-renders/Miniplayer.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 2 (asset integrity + fidelity-evidence usage) |
| REAL 2 / others 0 | |

## 1. Blunt verdict
Genuine render artifact, not a placeholder: `file` -> "PNG image data, 900 x 5882,
8-bit/color RGBA, non-interlaced"; 281,369 bytes; PNG signature bytes `89504e470d0a1a0a`
(python3 struct read, dims 900x5882, bit-depth 8, color-type 6 RGBA). A tall sheet (9
states stacked) consistent with the other figma-renders exports. The 9 "parts" in the
assignment are byte-line slices of one binary — one underlying file, audited once.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Render of the miniplayer surface (companion to `Miniplayer.svg`, design/README.md:19 "Miniplayer — `Miniplayer.svg` — Compact player") | whole file | REAL | Both files exist: design/figma/Miniplayer.svg + design/figma-renders/Miniplayer.png (ls). PNG header verified above. |
| 2 | Fidelity-evidence use: "miniplayer empty state vs `Miniplayer.png`: **GAPS**, but every gap is an honest-empty-state delta" | docs/waves/w26/logs/w26.k08.md:79-80 | REAL (citation resolves) | `sed -n '75,85p'` -> exact text present; the render is the comparison oracle for the w26 visual loop (contract §5). |

## 3. Contradictions
None. The consuming log describes the *render as reference* and scores the UI against
it; it does not claim the UI matches it.

## 4. Fabrication / inflation indicators
None against this asset. (The w26 logs' "honest-empty-state delta" framing is a claim
about the UI, not about this PNG.)

## 5. Reachability
Not code. Consumer chain: design/figma-fidelity-spec.md:82 (miniplayer variant spec) ->
w26 logs (k08, a09, d2, a11, i02, i16 cite it). No build step reads it (design assets
are documentation-side evidence).
