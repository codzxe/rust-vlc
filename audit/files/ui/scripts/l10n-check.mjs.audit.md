# AUDIT: ui/scripts/l10n-check.mjs
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 289 (top-level script; ~12 rule blocks) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (tooling) — wired as `npm run l10n:check` (ui/package.json:28) and EXECUTED in CI: .github/workflows/ui.yml:116-118 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `node ui/scripts/l10n-check.mjs` -> exit 0, `PASS l10n:check — 1448 MessageKeys, ... dead keys 125/1448 grandfathered`; python3 test-only-key probe; `rg -n "m\('[a-z]" ui/src` (empty) |

## 1. One-paragraph verdict
A real, working, honest gate — not security theater. I executed it: exit 0 on
the current tree, with a two-way dead-key gate (new dead keys FAIL, stale
grandfather entries FAIL) that actually holds the line at 125/1448. Rules 1-6b
(parse, coverage, duplicates, orphans, key grammar, placeholder grammar,
allowlist shape + staleness) are all implemented with real parsing of
messages.ts. Two documented-over-marking weaknesses: test-file references count
as "usage", and the exact-literal scan only matches double-quoted strings.

## 2. What actually works (verified)
— Ran it: exit 0 with the PASS line quoted in the header row. — Rule 7 scan
universe (src, e2e, scripts, tools minus messages.ts + keys/**) re-implemented
independently by my python3 probe: my dead count matches the grandfather file
(125) and finds exactly 3 keys kept alive only by test references (below).
— CI wiring verified real: ui.yml:116-118 runs `npm run l10n:check` (the
file's own header comment "CI proposal in .github/workflows/ui.yml" is stale —
it is no longer a proposal).

## 3. Findings
F-001 [S4]
Where: ui/scripts/l10n-check.mjs:165, 227-232
Code:
```
const keyUsed = (k) => {
  if ([...dynPrefixes].some((p) => k.startsWith(p + "."))) return true;
  for (const t of sourceTextsByPath.values()) if (t.includes(`"${k}"`)) return true;
  return false;
};
```
What is wrong: `sourceTextsByPath` includes `*.test.ts(x)` and harness files,
so a key referenced ONLY by a test never counts as dead. My probe: 3 keys
survive solely on test references — `menu.file`, `player.region.title-bar`,
`playlist.options.sort.title-asc` — i.e. production-dead keys the gate reports
as used.
Why it matters: the dead-key metric (125/1448) understates production dead
weight; the gate can be gamed accidentally by fixtures.
Confidence: HIGH — Verification: ran (python3 probe + script output diff).
F-002 [S4]
Where: ui/scripts/l10n-check.mjs:229
Code: (same line) `if (t.includes(`"${k}"`)) return true;`
What is wrong: only double-quoted literals count. A single-quoted
`msg('key')` usage would be invisible and the key falsely flagged dead (then
grandfathered — rot). Today moot: `rg -n "m\('[a-z]" ui/src` -> empty
(prettier-enforced double quotes), so this is latent, not active.
Confidence: HIGH — Verification: ran rg (empty result).

## 4. Per-function table (rule blocks; script >100 LOC)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| parse messages.ts (union + catalog) | 46-89 | REAL | regex parse; error on missing value |
| rules 1-3 coverage/dup/orphan | 91-105 | REAL | verified 0/0/0 on current tree (ran) |
| rule 4 key grammar | 107-110 | REAL | 2-5 dotted kebab segments |
| rule 5 placeholder grammar | 112-132 | REAL | {{ }} escapes masked with PUA sentinels; stray-brace check |
| rule 6 allowlist shape | 134-161 | REAL | requires text/reason/owner/files |
| source scan | 163-194 | REAL | skips node_modules/dist + definition sites |
| rule 6b stale exemptions | 196-217 | REAL | text must occur in its OWN scoped files |
| rule 7 dead keys | 219-232 | REAL | S4 weaknesses F-001/F-002 |
| grandfather two-way gate | 234-275 | REAL | left-catalog / gained-reference both FAIL |
| report/exit | 277-289 | REAL | exit 1 with per-error lines |

## 5. Test honesty
The script itself is the test gate; its honesty verified by execution (PASS
matches my independent python3 recount: 1448 keys, 125 dead). Mutation
reasoning: deleting a catalog entry -> rule 1 missing FAIL; adding an
unreferenced key -> rule 7 FAIL unless grandfathered; stale grandfather ->
rule 7 FAIL. All paths exercised by reading the code; behavior confirmed by
the passing run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI proposal in .github/workflows/ui.yml" | l10n-check.mjs:38 | STALE/REAL (it IS wired) | ui.yml:118 `run: npm run l10n:check` |
| "Dynamic families ... may over-mark a family as used, never under-mark" | l10n-check.mjs:221-223 | REAL | prefix-startsWith logic L228 can only widen the used set |

## 7. Dead / duplicated / unreachable code
None. (The grandfather map value is unused by design — comment L265-266
explains the lint-driven choice.)

## 8. Dependencies & flags
node:fs/path/url only — zero-dependency as claimed (header L14). No feature
gates. Reads: ui/src/i18n/messages.ts, ui/i18n-allowlist.json,
ui/i18n-dead-keys.json.

## 9. Open questions for the next agent
- Should the rule-7 scan exclude `.test.`/harness files so the 3 test-only
  keys (menu.file, player.region.title-bar, playlist.options.sort.title-asc)
  surface as dead and get pruned with the other 125?
