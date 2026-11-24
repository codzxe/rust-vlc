# AUDIT: ui/scripts/gen-settings-screens.mjs
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 663 / 7 (parseToml, lookupPath, buildModel, constName, tsValue, emitSchemaTs, emitScreensTs, main) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (build-time) — npm run settings:generate/settings:check (package.json:24-25); output ui/src/screens/settings/generated/Screens.tsx consumed by areas/controls.tsx:41, areas/recording.tsx:27, areas/extensions.tsx:45 -> SettingsScreen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | node scripts/gen-settings-screens.mjs --check -> exit 0 "(10 ui areas, 102 ui entries, 121 total)"; rg escape-scan of schemas/settings.toml + product.toml (no hits); read generated.test.ts (266 LOC) |

## 1. One-paragraph verdict
A zero-dependency code generator of unusual quality for this repo: real TOML
subset parser, 8 invariant checks (D1-D5), enum msgid collision guard, and a
byte-compare drift check that I executed (exit 0, no drift). The generated
screens are genuinely wired into the live settings UI. The two real nits: the
parser silently mis-parses TOML escapes it does not know, and the emitted
`msgid()` cast punches through the MessageKey type system — but the cast hole
is fenced by an i18n coverage test that fails closed.

## 2. What actually works (verified)
- `node scripts/gen-settings-screens.mjs --check` -> exit 0, "OK (10 ui areas, 102 ui entries, 121 total)" — committed generated files match schemas/settings.toml + product.toml exactly.
- Drift detection is mutation-tested, not asserted-empty: generated.test.ts:110-131 runs the generator on mutated temp copies (default change, restart flag change, entry removal) and asserts exit 1 each time. These tests would fail if the check were gutted.
- The generated constants feed live components: `GENERATED_SCREENS` is imported and rendered by areas/controls.tsx:41+49, areas/recording.tsx:27+35, areas/extensions.tsx:45 (rg verified) — generator output reaches the shipped screen.

## 3. Findings
F-001 [S3]
Where: gen-settings-screens.mjs:58-75
Code:
```
  const parseString = () => {
    i++; // opening quote
    let out = "";
    while (i < n && text[i] !== '"') {
      if (text[i] === "\\") {
        const e = text[i + 1];
        if (e === '"' || e === "\\") {
          out += e;
          i += 2;
          continue;
        }
      }
      out += text[i];
```
What is wrong: only `\"` and `\\` are honored; every other TOML escape
(`\n`, `\t`, `\uXXXX`) is passed through as a literal backslash pair instead
of either decoding it or throwing. `foo = "a\nb"` silently yields
`a\nb` (3 chars) not `a<LF>b`. The header claims the subset "suffices" —
true today (rg for backslash escapes in schemas/settings.toml and
product.toml: zero hits, verified), but the failure mode is silent data
corruption, not an error.
Why it matters: the first settings.toml string needing a real escape ships a
corrupted default/label with no error. What correct looks like: throw
TomlError on any unhandled escape (same treatment as `unsupported TOML
value` at line 146). Confidence: HIGH — Verification: read + ran rg.
F-002 [S4]
Where: gen-settings-screens.mjs:554 (emitted into generated/Screens.tsx)
Code:
```
const msgid = (id: string): MessageKey => id as MessageKey;
```
What is wrong: a blind cast that converts any schema string into a
MessageKey, defeating the type system for area/section titles. Documented as
a "seam" (lines 550-553) because titles register in messages.ts only
post-wave. Mitigation verified: generated.test.ts:207-264 asserts every
description, enum option msgid, and section title resolves in the catalog OR
the four enumerated lane-key files, and pins catalog/lane copy equality —
the hole fails closed in `npm test`. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parseToml | 39-209 | REAL | S3 | subset parser; see F-001 |
| lookupPath | 218-225 | REAL | - | correct undefined-on-miss |
| buildModel | 227-388 | REAL | - | D1-D5 enforced; enum msgid collision guard (303-311); String(defaultValue) enum compare fix (312-317) |
| constName / tsValue | 394-401 | REAL | - | |
| emitSchemaTs | 403-526 | REAL | - | deterministic; type-union fix for path/url/percent documented (421-426) |
| emitScreensTs | 528-602 | REAL | S4 | msgid() cast seam, see F-002 |
| main | 608-655 | REAL | - | --check byte-compare, --input/--out-dir overrides |

## 5. Test honesty
generated.test.ts (266 LOC): mutation-based drift tests assert FAILURE modes
(exit 1 + stderr text "collide on msgid"), not just success — these would
catch a broken generator. Data tests pin 10 areas / 102 ui entries / 121
msgids, unique keys, in-range defaults, default_ref resolution, feature
exposure. Honest suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`node scripts/gen-settings-screens.mjs --check` exit 1 on drift" | line 14 | REAL | ran: exit 0 pristine; test mutates and asserts 1 |
| "The script NEVER writes outside ui/src/screens/settings/generated/ (unless --out-dir)" | line 18-19 | REAL | read: outputs constrained to outDir/schema.ts + Screens.tsx (620-623, 649-650) |
| "both files use only tables, array-of-tables, inline tables, strings/bools/numbers/string-arrays" | lines 6-8 | REAL (today) | rg escape/multiline scan: 0 hits in both TOMLs |
| "zero deps" | line 6 | REAL | imports only node:fs/promises, node:path, node:url (21-23) |

## 7. Dead / duplicated / unreachable code
None. All exported functions consumed (parseToml/buildModel/emitters by tests
and main; --input/--out-dir used by generated.test.ts drift cases).

## 8. Dependencies & flags
Node builtins only. Reads schemas/settings.toml + product.toml (READ-ONLY
inputs — verified: no writes to repo paths except the generated dir).
Feature gates resolved from product.toml [features]; playback.hw-decode-policy
emits featureExposed=false (pinned by test at generated.test.ts:190-193).

## 9. Open questions for the next agent
- Who consumes `featureExposed` at runtime? If no runtime consumer filters
  unexposed entries, gated settings could render despite Phase 7 being off —
  check areas/*.tsx (the controls.tsx comment at line 9 suggests awareness).
