# AUDIT: ui/src/components/button/useSwitchLabels.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 21 / 1 (useSwitchLabels) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumed by button/switchCore.tsx (the shared switch core) -> every Switch/Toggle in settings areas |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg "switch.on"/"switch.off" in messages.ts (registered, union:1492-1493 + copy:2975) and w26.k01.ts lane (15-16) |

## 1. One-paragraph verdict
The i18n seam for the ON/OFF micro labels. Both sources are present and the
documented copy is identical ("ON"/"OFF"), so the catalog-first ordering is
currently indistinguishable from the lane fallback. One operator nit: `||`
instead of `??`.

## 2. What actually works (verified)
- "switch.on": "ON" registered in the catalog (messages.ts:2975) AND lane
  (w26.k01.ts:15) — identical copy; "switch.off" likewise. The seam is
  consistent today.
- Consumers: switchCore -> Switch/Toggle -> settings rows (chain verified in
  Switch.tsx audit).

## 3. Findings
F-001 [S4]
Where: useSwitchLabels.ts:18-19
Code:
```
    on: m("switch.on" as MessageKey) || laneKeys["switch.on"],
    off: m("switch.off" as MessageKey) || laneKeys["switch.off"],
```
What is wrong: `||` — a catalog entry whose copy is the empty string falls
back to the lane value, silently overriding a deliberate empty. useMsg never
returns "" for a registered non-empty entry, so this is a style-level
footgun only (the sibling seams use `??`).
Confidence: HIGH — Verification: read.

## 4. Per-function table (<100 LOC)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| useSwitchLabels | 15-21 | REAL | see F-001 |

## 5. Test honesty
Covered via switchCore/Switch tests (labels render); the seam ordering is
data-pinned by the identical copy today.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "lane keys win while unregistered, the shared catalog wins after" | useSwitchLabels.ts:6-7 | REAL | m() catalog-first (verified semantics in shared.tsx audit) |
| "identical en-US copy either way" | useSwitchLabels.ts:7-8 | REAL | both sources read: "ON"/"OFF" |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
i18n messages + w26.k01 lane keys. None.

## 9. Open questions for the next agent
- None.
