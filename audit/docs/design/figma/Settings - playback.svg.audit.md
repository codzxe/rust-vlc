# CLAIM AUDIT: design/figma/Settings - playback.svg
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 4 |
| REAL 3 / UNVERIFIED 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid SVG design asset, correctly inventoried and cross-referenced. It is a Figma export
(text-outlined, no live text), a design source of truth — not shipped code, no executable
claim to break.

## 2. Claim ledger
| # | Claim | Location (in other docs) | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Asset is a well-formed SVG | — | REAL | `file` -> "SVG Scalable Vector Graphics image, ASCII text"; python3 ElementTree parse OK, root `svg` |
| 2 | Dimensions 1048×865 | doc 08 settings family | REAL | python3: width="1048" height="865", viewBox="0 0 1048 865" |
| 3 | Cited as one of the 13 settings areas (`playback(-1)` pair) | docs/08-ui-figma-fidelity.md:160; design/README.md Settings row | REAL | rg: `playback(-1)` listed in the 20-file settings family; this file is the `-0` member; `Playback` label present in file content (python3 `'Playback' in s` -> True) |
| 4 | settings-schema UI placement maps to it | doc 08:160 "Maps to the settings-schema UI placement (MP §5, doc 07)" | UNVERIFIED | requires tracing settings-schema placement wiring in crates (not this asset's job; flagged for the settings-domain agent) |

## 3. Contradictions
None. (doc 08:160's "14 areas vs 13 found" discrepancy is a doc-08 finding about the
folder, not this file.)
