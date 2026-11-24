# AUDIT: ui/src/screens/settings/areas/performance.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 155 / 2 describe blocks, 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest; baseline ui-test.txt:103 "performance.test.tsx (6 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (+1 cross-file S3 noted) |
| Commands run | rg suite in baseline; rg VLC label provenance |

## 1. One-paragraph verdict
The regression suite born from a review finding (the area had NO test, which let an ⓘ census gap slip): it pins schema-bound rendering for every non-advanced performance key (data-setting present) and the absence of advanced rows until the disclosure opens, frame-verbatim default values through the real framework (enums, spinbuttons, switch states), the advanced flat-list order derived from the schema (not pinned literals), the exact 10-circle ⓘ census (5 collapsed + 5 behind the bar — the documented regression pin), the honest absences (2 toggles + 3 resource rows carry no ⓘ), and the operable reveal behavior. Everything is derived from SETTINGS_ENTRIES except the frame-value pins.

## 2. What actually works (verified)
- Baseline ui-test.txt:103: `✓ src/screens/settings/areas/performance.test.tsx (6 tests) 403ms`.
## 3. Findings
None in this file. CROSS-FILE note (S3, belongs to the rebrand/i18n audits): the frame-value pin at :81-84 asserts the label "Give VLC real-time priority" — shipped copy that names VLC. Provenance: ui/src/i18n/keys/w26.f01.ts:104-108 documents a LEAD DECISION ("frame-verbatim C11 copy carries the VLC name … flagged, not silently changed"), so this one is flagged; but sibling messages (ui/src/i18n/messages.ts:2373 "Quit VLC", :2669 "About VLC", :2690 "VLC settings", :2885 "Only allow one VLC window") carry the same pattern — the shipped UI's About/menu copy names VLC while the rebrand row (docs/feature-matrix.toml:2207-2216) claims identity is product.toml-driven. Rebrand-surface leak for the next agent to adjudicate.
## 4. Per-function table (test blocks)
| Test | Lines | Verdict | Notes |
|---|---|---|---|
| schema rows bound + advanced hidden | 42-59 | REAL | per-entry data-setting presence/absence loop |
| frame values via framework | 61-88 | REAL | Automatic/Off/Never/0 + switch states + Process priority 0 |
| Advanced Options flat rows | 90-107 | REAL | rendered keys == schema-derived advancedKeys order |
| ⓘ census 10 (5+5) | 111-122 | REAL | count before/after disclosure — THE regression pin |
| honest absences | 124-139 | REAL | 5 keys asserted circle-free |
| ⓘ operable reveal | 141-154 | REAL | aria-expanded flip + hinttext copy |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all six | YES | binding loops fail on any unwired schema row; census counts fail on any ⓘ addition/removal; reveal test fails on a dead glyph |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the performance area had NO dedicated test file, which is exactly why its ⓘ census gap went undetected" | header :1-3 | REAL-UNPROVEN | historical claim; consistent with w26.k13 comment trail |
| circle extraction over the design SVG (Ellipse ids/cy values) | :112-115 | REAL-UNPROVEN | design/figma SVG not re-extracted here (design dir exists; not my file) |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
@testing-library/react, MemoryRouter, MockTransport (sanctioned shared transport double from client/transport), SettingsProvider + generated schema. No feature flags.
## 9. Open questions for the next agent
- The VLC-copy leak set (messages.ts:2373/2553/2669/2690/2885 + w26.f01.ts:108) vs the config-rebrand matrix row — adjudicate whether shipped i18n copy naming VLC is a rebrand-row FALSE claim or an accepted frame-verbatim exception (only realtime-priority carries the LEAD DECISION flag).
