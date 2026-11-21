# AUDIT: ui/scripts/l10n-check.mjs
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 289 / 7 rule blocks (parse, coverage 1/2/3, grammar 4, placeholders 5, allowlist 6/6b, dead keys 7, report) |
| File verdict | REAL (executed gate: baseline #24 PASS with cited output; wired as CI step 3) |
| Reachable from shipped entry point? | N/A (dev/CI tooling, not shipped) — invoked by `npm run l10n:check` (package.json), CI .github/workflows/ui.yml:115-117 ("3. l10n catalog check"), executed in baseline run #24. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cat audit/runs/baseline/ui-l10n_check.log.txt` → PASS 1448 keys/1448 strings, 125 grandfathered dead keys, EXIT=0; `sed -n '112,120p' .github/workflows/ui.yml` → CI step wired (contradicts header's "CI proposal"); `ls ui/src/i18n/locales.ts` → absent (pseudo.ts:28 references it); `rg -n "global-language.en-us" messages.ts` → single language option "English" (:1757); read of pseudo.ts (deterministic transform, no second catalog); hand-trace of the parser regexes. |

## 1. One-paragraph verdict
This is a real, executed, CI-wired integrity gate with sound engineering: a sticky-regex catalog parser, PUA-sentinel placeholder masking that honors `{{ }}` escapes, a two-way dead-key grandfather list (new dead keys fail; grandfathered keys that gain references also fail), and stale-allowlist detection (an exemption whose text vanished from its scoped files fails). Baseline run #24 executed it PASS with zero errors over 1448 keys. Three defects: the header still calls the CI wiring a "proposal" when it has been a gate step for waves; the gate's green PASS line could be cited as "localization works" when the surface it guards does not exist — the UI ships exactly one language (English), no locales pack exists, and pseudo.ts's own comment references a `locales.ts` file that is not in the tree; and the dead-key scan only recognizes double-quoted literals, so alternate quoting styles would mint false dead-key errors.

## 2. What actually works (verified)
— verified by baseline #24 (audit/runs/baseline/ui-l10n_check.log.txt, EXIT=0): `PASS l10n:check — 1448 MessageKeys, 1448 en-US strings: missing 0, duplicates 0, orphans 0, key-grammar ok, placeholders 13 ok, allowlist 6 entries (0 stale), dead keys 125/1448 grandfathered`.
— verified by reading .github/workflows/ui.yml:115-117: the gate runs in CI as step 3 (`if: always()`), after lint and before unit tests.
— verified by hand-trace: union parse (`\|\s*"…"`), catalog parse (entry + sticky value regex, JSON.parse of both), placeholder grammar `{name}`/`{name, argstyle}` with `{{`/`}}` masked to U+E002/E003 before tokenizing — the escapes cannot be misread as placeholders.
— zero-dependency claim TRUE: imports are node:fs/node:path/node:url only.

## 3. Findings
### F-001 [S3]
Where: ui/scripts/l10n-check.mjs:40 (+ ui/src/i18n/pseudo.ts:28, ui/src/i18n/messages.ts:1757)
Code:
```
// Wired as `npm run l10n:check`; CI proposal in .github/workflows/ui.yml.
```
What is wrong: Two-sided drift. (a) The header calls the CI wiring a "proposal"; it is an active gate step (ui.yml:115-117). (b) More substantively: the gate is named and sold as the "l10n" check, but the localization surface does not exist — the catalog is en-US only, the settings language picker offers exactly one option (`"settings.value.global-language.en-us": "English"`, messages.ts:1757), no `locales.ts` exists although pseudo.ts:28 says "the language picker lists only real catalogs (locales.ts)", and the Rust localization crate is orphaned (W11 pin). A green `l10n:check` PASS proves en-US self-consistency and nothing about localization.
Why it matters: This is the engine-info disease at the tooling layer: a green signal whose name over-claims its scope. Anyone citing "l10n gate passes" as evidence of localization support is misled by the gate's own naming and this header.
What correct looks like: rename the gate output to "en-US catalog integrity" (it already says so in line 2 — finish the job), fix the stale CI-proposal comment, and delete or create the referenced locales.ts.
Confidence: HIGH — Verification: all three files read; CI workflow read; baseline log read.

### F-002 [S4]
Where: ui/scripts/l10n-check.mjs:227-231
Code:
```
const keyUsed = (k) => {
  if ([...dynPrefixes].some((p) => k.startsWith(p + "."))) return true;
  for (const t of sourceTextsByPath.values()) if (t.includes(`"${k}"`)) return true;
  return false;
};
```
What is wrong: Usage detection recognizes only double-quoted exact literals and dotted template families (`prefix.${…}`). A single-quoted (`'key'`) or backtick-full-literal usage would count as unused → false dead-key error. Today the gate passes (baseline), so no live damage — the exposure is future false failures if any file adopts a different quoting style (prettier currently enforces double quotes, which is the only thing keeping this sound).
Why it matters: The gate's correctness silently depends on the quote style of 400+ scanned files, with no assertion of that invariant.
What correct looks like: match `["'\`]${k}["'\`]` or run the scan through the ESLint AST instead of raw text.
Confidence: HIGH — Verification: read; quote-style dependence confirmed by scanning the regex.

### F-003 [S4]
Where: ui/scripts/l10n-check.mjs:58-62
Code:
```
const unionStart = src.indexOf("export type MessageKey =");
const catalogAnchor = src.indexOf("const messages:");
if (unionStart === -1) err("parse", "MessageKey union not found in messages.ts");
const unionSrc = src.slice(unionStart, catalogAnchor === -1 ? undefined : catalogAnchor);
const unionKeys = [...unionSrc.matchAll(/\|\s*"([^"]+)"/g)].map((m) => m[1]);
```
What is wrong: Parser fragility: (a) if `unionStart === -1`, the code still slices from -1 (last char) instead of bailing — the error is recorded but the run continues on garbage input (the final exit is still 1, so no false PASS is possible — but the summary line would print misleading counts); (b) the union regex requires every arm to start with `|`, so a first arm written inline (`= "key" |` on one line) would be silently dropped from unionKeys → every downstream rule (missing/orphan) reports bogus mismatches; (c) `catalogAnchor`/`catalogStart` failures similarly continue.
Why it matters: A refactor of messages.ts formatting could turn the gate into a confusing error firehose instead of one clean parse error.
What correct looks like: early `process.exit(1)` after parse errors, before any rule runs.
Confidence: HIGH — Verification: read; control flow traced.
Severity: S4 (no false-PASS path exists — errors always exit 1).

## 4. Per-function table (rule blocks)
| Rule block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header/doc | 1-40 | REAL | S3 | F-001 stale "CI proposal"; scope honestly "en-US catalog integrity" at :2 |
| parse (union + catalog) | 52-89 | REAL | S4 | F-003 fragility; sticky-regex value parse is genuinely good |
| 1/2/3 coverage (missing/dup/orphan) | 91-105 | REAL | — | correct set algebra |
| 4 key grammar | 107-110 | REAL | — | 2-5 dotted segments, [a-z0-9-]+ — matches doc 07 §6.1 |
| 5 placeholders | 112-132 | REAL | — | PUA-sentinel masking; stray-brace check; empty-string check |
| 6/6b allowlist shape + staleness | 134-217 | REAL | — | v3 file-scoped exemptions; stale detection (j18 F9) |
| 7 dead keys + grandfather two-way gate | 219-275 | REAL | S4 | F-002 quote-style dependence; two-way shrink discipline is excellent |
| report/exit | 277-289 | REAL | — | FAIL→exit 1, PASS summary with counts |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| baseline #24 execution | — | PASS 1448/1448; the gate itself is the test |
| No unit tests exist for l10n-check.mjs itself | N/A | a checker this central (1448-key catalog) has no self-tests for its parser — the F-003 fragilities are exactly what such tests would pin; the i18n-dead-keys/allowlist two-way logic is only exercised by the live files |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Zero-dependency audit" | l10n-check.mjs:4 | REAL | imports are node: builtins only |
| "CI proposal in .github/workflows/ui.yml" | l10n-check.mjs:40 | STALE | wired as step 3 (ui.yml:115-117) |
| "The per-key placeholder NAME set is reported so future catalogs can be diffed" | :14-17 | PARTIAL | placeholderCount is summed; per-key name sets are checked for grammar but NOT emitted in the summary/exit output — the "reported" diff data is not actually reported |
| doc 07 §6.4 cross-catalog comparison | :16-17 | REAL as handoff | owned by crates/localization — orphaned per W11 |

## 7. Dead / duplicated / unreachable code
The `grandfathered` Map's values are intentionally unused (documented at :265-266, keys-only iteration to keep lint green) — disclosed. No other dead code. Tooling, not shipped.

## 8. Dependencies & flags
node:fs/path/url only. No flags. Executed under Node in baseline #24 and CI. The gate depends on prettier's quote style for F-002's soundness (undocumented dependency).

## 9. Open questions for the next agent
- The 125 grandfathered dead keys (8.6% of the catalog): i18n-dead-keys.json reasons — worth a lane audit (are the reasons still true?).
- locales.ts: create it or fix pseudo.ts:28's comment (F-001c).
