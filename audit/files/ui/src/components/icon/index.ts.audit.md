# AUDIT: ui/src/components/icon/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A08 (recovered by W15-R3) |
| Date | 2026-11-24 |
| Lines / functions | 6 / barrel (6 export statements, 7 symbols) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: 12 screen files import { Icon } via this barrel (e.g. PlaylistScreen.tsx:42, DiscoveryScreen.tsx:64, NetworkStreamScreen.tsx:45, StageOverlays.tsx:42), all mounted from App.tsx routes; also re-exported wholesale by ui/src/components/index.ts:8 (consumed by PlayerScreen.tsx:89) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg export-source checks (exit 0); rg per-symbol consumers (exit 0/1); rg 'idden\]' corruption scan on icon/ (exit 1 = clean); python3 glyph-key count -> 76 (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
A clean value+type barrel: all 7 re-exported symbols exist at their sources (Icon.tsx:30/38, icons.tsx:94/824/826, IconWrapper.tsx:28/30). The registry behind it is real and exhaustive — 76 glyph keys in `const glyphs: Record<IconName, ReactElement>` (icons.tsx:239-822, counted by script), matching Icon.test.tsx:93's `toBe(76)`. The barrel is itself test-covered (Icon.test.tsx:4 imports "./index" and renders all 76 glyphs). Not clean: 3 of 7 exports have no consumer importing them through the barrel, and the one production consumer of IconWrapper bypasses the barrel entirely.

## 2. What actually works (verified)
— rg export checks: all 7 symbols present at sources.
— python3 key count over icons.tsx:240-822 -> 76 keys, equal to `Object.keys(glyphs)` (iconNames) and enforced exhaustive by `Record<IconName, ReactElement>`.
— Barrel seam test: Icon.test.tsx:4 `import { Icon, iconNames, IconWrapper } from "./index";` + L39-44 renders every glyph and asserts non-empty svg.
— Corruption scan: rg 'idden\]' ui/src/components/icon/ -> no matches (the W15-A08 tree-wide "[h" deletion did NOT touch this directory).

## 3. Findings
F-001 [S4]
Where: ui/src/components/icon/index.ts:2-3,6
Code:
```
export type { IconVariant } from "./Icon";
export { iconNames, getIconGlyph } from "./icons";
export type { IconWrapperSize } from "./IconWrapper";
```
What is wrong: `getIconGlyph`, `IconVariant`, and `IconWrapperSize` have zero consumers importing them via the barrel — getIconGlyph/IconVariant are consumed only by Icon.tsx's direct sibling import (Icon.tsx:2,32), IconWrapperSize only inside IconWrapper.tsx:37. Dead re-exports on a frozen surface. What correct looks like: prune or document as intentional public seam. Confidence: HIGH — Verification: ran (rg per-symbol, no barrel-import matches).
F-002 [S4]
Where: ui/src/components/appshell/AppShell.tsx:64
Code:
```
import { IconWrapper } from "../icon/IconWrapper";
```
What is wrong: the only production consumer of IconWrapper bypasses the barrel while 12 other files use it for Icon — two import paths for the same module defeat the single-funnel purpose of a barrel. Confidence: HIGH — Verification: ran (rg).

## 4. Per-function table
N/A (<100 LOC barrel): each of the 7 symbols verified against source and consumer list.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Icon.test.tsx:39-45 "renders every registered glyph" | YES | Imports via the barrel itself; an export typo in index.ts breaks it before rendering; asserts every glyph produces svg children |
| Icon.test.tsx:93 `expect(iconNames.length).toBe(76)` + L94 uniqueness | YES | Verified against an actual count of glyphs keys (76, script above); adding/removing a glyph fails it |
Baseline: audit/runs/baseline UI vitest run passes (1705 passed, 1 skipped).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Primitive library barrel (cards 4.B1–4.B4)" | components/index.ts:1 (consumer) | TRUE for the icon leg | `export * from "./icon"` at components/index.ts:8 |

## 7. Dead / duplicated / unreachable code
F-001 (3 dead re-exports). Note: icons.tsx exports getTwoToneGlyph/hasTwoToneVariant which are deliberately NOT in the barrel (consumed by Icon.tsx:2 + tests only) — correct scoping.

## 8. Dependencies & flags
None. No side effects, no cycles (Icon.tsx imports ./icons directly, never via the barrel).

## 9. Open questions for the next agent
- IconWrapper.tsx and icons.tsx were not in this recovery batch's scope; Icon.tsx and icon.css were audited by W15-A08 (REAL, zero findings).
