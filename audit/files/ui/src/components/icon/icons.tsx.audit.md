# AUDIT: ui/src/components/icon/icons.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 904 / 4 (getIconGlyph, getTwoToneGlyph, hasTwoToneVariant + data registries) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — icons.tsx -> Icon.tsx:60 `getIconGlyph/getTwoToneGlyph` -> Icon component -> AppShell.tsx, SettingsScreen.tsx, OpenMediaDialog.tsx, MenuItem/Dropdown, MediaInfoPanel.tsx, playlist options menus |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg outline; awk union-count=76; per-name consumer scan (all 76 consumed); sed quote verification |

## 1. One-paragraph verdict
Pure data module: 76 SVG glyph entries + 5 two-tone variants, exported through
three lookup functions. No logic beyond dictionary lookups, no backend, no
stubbing. Exhaustiveness is compile-enforced (`Record<IconName, ReactElement>`),
the "76 names" header claim matches the union (76 members counted by awk) and
the test (Icon.test.tsx:93). Design-provenance comments are unverifiable claims
about Figma files but the geometry data itself is real rendered JSX.

## 2. What actually works (verified)
- awk '| "' count in the IconName union = 76; `glyphs` entry count = 76 -> matches header claim and Icon.test.tsx:93 `expect(iconNames.length).toBe(76)`.
- Per-name consumer scan (rg each of the 76 names in src, excluding icon dir/tests/snapshots): ZERO no-consumer names -> the W26.K04 prune claim ("9 registry names removed ... ZERO consumers") left no stragglers; no dead registry entries found.

## 3. Findings
F-001 [S4]
Where: icons.tsx:824-828
Code:
```
export const iconNames = Object.keys(glyphs) as IconName[];

export function getIconGlyph(name: IconName): ReactElement {
  return glyphs[name];
}
```
What is wrong: nothing functionally; lookup can never miss because the record
type is exhaustive. Noted only because the `as IconName[]` cast only holds
while the Record stays exhaustive — a future partial record would silently
produce `undefined` glyphs at runtime. Why it matters: cosmetic risk only.
What correct looks like: n/a. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC — data entries covered as a block)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| IconName union | 94-184 | REAL | - | 76 members, verified count |
| shared glyph consts (folder/info/wand/person/equalizer) | 191-237 | REAL | - | alias dedupe, referenced at 424/499/646/711/714 |
| glyphs record | 239-822 | REAL | - | 76 JSX path entries; consumers exist for all names |
| iconNames | 824 | REAL | - | Object.keys cast (see F-001) |
| getIconGlyph | 826-828 | REAL | - | dict lookup |
| twoToneGlyphs + folder alias | 838-889 | REAL | - | 4 dedicated variants (open-file, open-folder, playback, check) + folder alias |
| getTwoToneGlyph | 896-898 | REAL | - | fallback to mono entry |
| hasTwoToneVariant | 902-904 | REAL | - | test hook only (Icon.test.tsx:379) — acceptable |

## 5. Test honesty
Icon.test.tsx:93 asserts length 76 + uniqueness — would fail if a name were
added without a glyph (compile) or duplicated (runtime). Honest structural
test; visual fidelity of paths is untested (SVG-to-Figma comparison UNVERIFIED —
no executable comparison available; comments cite layer ids only).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Registry: 76 names (32 wave-1 frozen + 44 a07)" | icons.tsx:57 | REAL | awk union count = 76; test asserts 76 |
| "9 registry names removed after an exhaustive quoted-literal grep ... ZERO consumers" | icons.tsx:49-56 | REAL (current state) | my independent scan: 0 of 76 names unconsumed |
| Glyph paths are "verbatim design geometry" from design/figma/*.svg | icons.tsx:12 | UNVERIFIED | would require diffing against design/figma exports; not executable in budget |

## 7. Dead / duplicated / unreachable code
None. All 76 names consumed outside the icon module (verified scan). Alias
twins (folder/open-folder, info/about, person/wand) share const references —
intentional dedupe, not duplication.

## 8. Dependencies & flags
React types only (`import type { ReactElement}`). No feature flags. No
backend surface — irrelevant to the master question except negatively: this
file is pure presentation and IS wired to live UI hosts.

## 9. Open questions for the next agent
- Visual diff of glyph paths vs design/figma/Icon.svg is unclaimed territory —
  a design-fidelity agent with the Figma exports could verify or falsify the
  "verbatim geometry" claim.
