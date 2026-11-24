# AUDIT: ui/src/screens/settings/areas/shared.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 107 / 3 (makeAreaMsg, schemaSectionViews, partitionAdvanced) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — 12+ area files import it (playback, extensions, audio, subtitles, privacy, effects, interface, video, performance, controls, codecs, staged, index) -> SettingsScreen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers; read useMsg fallback semantics (messages.ts:3011-3016) to verify the seam's fallback actually fires |

## 1. One-paragraph verdict
The j13 consolidation layer for the 13 settings areas: a lane-key seam hook
factory, a feature-gate/hidden-key section view builder, and an
advanced-disclosure partition. All pure/hooks-only as advertised. The critical
seam behavior (catalog-first, lane-fallback) is CORRECT because useMsg returns
undefined for unregistered keys (messages.ts:3016 `catalog[key] ??
messages[key]`) — the `?? catalog[key]` fallback genuinely fires. Note this
depends on useMsg's undefined-for-unknown behavior, which the cast hides.

## 2. What actually works (verified)
- makeAreaMsg fallback chain: `m(key as MessageKey) ?? catalog[key]` — verified useMsg returns undefined for keys absent from the reference catalog, so lane keys resolve (not rendered as raw ids).
- schemaSectionViews filters `entry.featureExposed` (line 83) — the generator's feature-gate resolution is enforced AT RENDER TIME here, answering gen-settings-screens' open question: gated entries (e.g. playback.hw-decode-policy, features.hw-decode=false) are excluded from the UI by this filter.
- 12+ consumers (rg) — the claimed ten-copy/six-copy dedup landed.

## 3. Findings
F-001 [S4]
Where: shared.tsx:48
Code:
```
    return (key: MessageKey | K): string => m(key as MessageKey) ?? catalog[key];
```
What is wrong: if a lane key file and the catalog ever both miss a key, the
resolver returns undefined and callers render nothing/`undefined` — no
type-level or runtime guard forces the last-resort fallback to a visible
string. Today the i18n coverage test (generated.test.ts) pins coverage for
schema labels, and area-specific keys are lane-file-typed, so the hole is
theoretical. Confidence: HIGH — Verification: read.
(Note: the settings-screen's own settingsMsg uses `?? key` as last resort;
this seam lacks that.)

## 4. Per-function table (file ~107 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LaneKeyFile type | 27 | REAL | - | |
| makeAreaMsg | 42-50 | REAL | S4 | catalog-first verified; see F-001 |
| SchemaSectionView interface | 53-64 | REAL | - | |
| schemaSectionViews | 73-92 | REAL | - | featureExposed + hiddenKeys filter — the render-time gate |
| partitionAdvanced | 97-107 | REAL | - | |

## 5. Test honesty
No tests in this file; the area files' tests exercise the views through their
screens (areas.test.tsx exists). The seam's fallback ordering is tested
indirectly by the i18n coverage suite. UNVERIFIED: a direct unit test of
makeAreaMsg's fallback ordering — none found in shared's vicinity.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "resolves CATALOG-FIRST ... the merged lane files are the fallback reference" | shared.tsx:33-38 | REAL | useMsg undefined-for-unknown verified (messages.ts:3016) |
| "no area-side filter could ever execute" (search) | shared.tsx:11-14 | REAL-UNPROVEN | depends on SettingsScreen's cross-area search swap — that file's audit owner should confirm |

## 7. Dead / duplicated / unreachable code
None — this file IS the dedup.

## 8. Dependencies & flags
i18n messages, generated schema types, SettingsSections. No gates (it
CONSUMES featureExposed rather than gating itself).

## 9. Open questions for the next agent
- SettingsScreen.tsx auditor: confirm the cross-area search behavior claim
  (shell renders results view while a query is live).
