# CLAIM AUDIT: docs/guides/settings.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 6 |
| REAL 3 / REAL-UNPROVEN 1 / FALSE 2 (stale numbers) | |

## 1. Blunt verdict
The guide's architecture spine is true (one schema, 14 areas — verified by tomllib parse) but two concrete numbers are stale relics of pre-w26/pre-w28 waves: the "21 UI entries" claim (the schema now carries 102 `ui = true` placement rows after the w26 settings reshape — worklog w27 records "settings:check OK 10 areas/102 entries") and the defaults table's window-min 480×320 (product.toml says min_width = 940 / min_height = 600 since w28's min-size fix). A user reading this guide today gets wrong defaults.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "`schemas/settings.toml` `[areas]` known ... The 14 areas" | §The 14 areas | REAL | tomllib: areas.known = 14 exactly |
| 2 | "## The keys and their defaults (21 UI entries + engine/extension rows)" | :46 | FALSE (stale) | `grep -c 'ui = true' schemas/settings.toml` -> 102 placement rows; worklog w27 "settings:check OK 10 areas/102 entries" — the w26 reshape passed this guide by |
| 3 | "`interface.window-min-width` / `-height` | int | `product.toml window.min_width` (480) / `min_height` (320)" | :55 | FALSE (stale) | product.toml:70-71 `min_width = 940`, `min_height = 600` — w28 Fix 5 (min-size UI floor) never propagated to this guide |
| 4 | "`playback.hw-decode-policy` ... **feature-gated** (`features.hw-decode = false` ...)" | :63 | REAL | product.toml:168 `hw-decode = false` with the ADR-001 Phase-7 rule comment |
| 5 | "The UI settings screens, the CLI `--help` text and the CLI flags are all generated from it" | intro | REAL-UNPROVEN | generator artifacts exist (ui/src/screens/settings/generated/schema.ts per w28 worklog; xtask settings codegen); generation run not executed by me |
| 6 | "Ranges are reviewed safety bounds (`product.schema.json`): out-of-range value rejected ... never silently clamped" | :80-82 | REAL-UNPROVEN | schema + validator exist (config-validator baseline runs on product.toml); rejection behavior not executed by me |

## 3. Contradictions
| This doc says | Repo says | Which is true |
|---|---|---|
| window min 480×320 | product.toml 940×600 | product.toml (w28 fix) — guide stale |
| 21 UI entries | 102 ui=true placements | schema — guide stale (pre-w26 count) |

## 4. Fabrication / inflation indicators
None — the errors are omission-staleness in a fast-moving surface, not overclaiming; the doc's feature-gating honesty (why some settings do nothing yet) is itself correct and unusually candid.
