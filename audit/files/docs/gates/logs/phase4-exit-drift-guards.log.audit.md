# AUDIT: docs/gates/logs/phase4-exit-drift-guards.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines | 15 (npm-run transcript of 3 drift-guard checks) |
| File verdict | REAL — re-executed on the current tree: all three checks still exit 0 |
| Reachable from shipped entry point? | N/A — Phase-4 gate evidence log, cited by docs/gates/phase4-exit.md:53 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | re-ran all three guards TODAY from ui/: `node scripts/gen-tokens.mjs --check` -> rc=0; `node scripts/gen-settings-screens.mjs --check` -> rc=0; `node scripts/l10n-check.mjs` -> rc=0 (outputs pasted below) |

## 1. One-paragraph verdict
A genuine npm-run transcript whose three drift guards I re-executed on the current tree — all three still exit 0, and the output format matches the log byte-for-shape, which is the strongest authenticity signal a data log can offer. The embedded numbers are Phase-4-era (70 semantic tokens, 5/21/27 settings, 472 i18n keys) and today's tree has grown well past them (86 tokens, 10/102/121, 1448 keys) — era-stale but honest, and the cited "ALL THREE GREEN (exit 0)" claim remains TRUE today.

## 2. What actually works (verified)
— verified by RE-EXECUTION on the current tree (node v24.22.3, zero-dep scripts — imports are node: builtins only, verified at gen-tokens.mjs:19-22 / gen-settings-screens.mjs:21-23 / l10n-check.mjs:41-43):
```
gen-tokens --check: OK (86 semantic tokens, 2 modes, 9 accents, sha256 26c55ab2b319…)   rc=0
gen-settings-screens --check: OK (10 ui areas, 102 ui entries, 121 total)                rc=0
PASS l10n:check — 1448 MessageKeys, 1448 en-US strings: missing 0, duplicates 0, orphans 0,
      key-grammar ok, placeholders 13 ok, allowlist 6 entries (0 stale), dead keys 125/1448
      grandfathered (ui/i18n-dead-keys.json)                                              rc=0
```
— verified citation: phase4-exit.md:53 quotes the log's numbers EXACTLY ("5 ui areas, 21 ui entries, 27 total"; "472 MessageKeys, 472 en-US strings: missing 0, duplicates 0, orphans 0, key-grammar ok, placeholders 0 ok, allowlist 5 entries") — log-to-doc transcription is faithful.
— verified package.json:23/25/28 wiring: the three npm scripts are exactly the commands in the log (`node scripts/gen-tokens.mjs --check` etc.).

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase4-exit-drift-guards.log:1-3 (whole header)
Code: `> lumen-ui@0.1.0 tokens:check
> node scripts/gen-tokens.mjs --check`
What is wrong: no capture header — the log records no command preamble, no date, and no exit code; "exit 0" lives only in the citing doc (phase4-exit.md:53). The file even starts with a blank line. Same S4 class as the other phase logs (W17-A05's log audits).
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: docs/gates/logs/phase4-exit-drift-guards.log:5, 10, 15
Code: `gen-tokens --check: OK (70 semantic tokens, 2 modes, 9 accents, sha256 832da9d0a9cb…)` / `gen-settings-screens --check: OK (5 ui areas, 21 ui entries, 27 total)` / `PASS l10n:check — 472 MessageKeys, 472 en-US strings: missing 0, duplicates 0, orphans 0, key-grammar ok, placeholders 0 ok, allowlist 5 entries`
What is wrong: era-stale numbers — today's re-run gives 86 semantic tokens (sha256 26c55ab2b319…), 10 ui areas / 102 ui entries / 121 total, 1448 MessageKeys (placeholders 13 ok, allowlist 6). The evidence describes a Phase-4-era tree, not the current one; the substantive claim (all three drift guards green) is STILL true today (my re-run), so this is staleness, not fraud.
Confidence: HIGH — Verification: ran (both outputs pasted).

## 4. Test honesty
The three underlying scripts ARE the drift guards; I ran them against the committed artifacts and they passed (exit 0 each). This is a live verification, stronger than the usual baseline citation.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ALL THREE GREEN (exit 0)" | phase4-exit.md:53 | REAL (re-verified today) | my three rc=0 runs |
| "tokens:check OK (sha-pinned generator, 4.A2)" | phase4-exit.md:32 | REAL | re-run OK; generator output sha-pinned per its own line |
| quoted numbers 70/5-21-27/472 | log + phase4-exit.md:53 | REAL-at-the-time / STALE-now | today: 86 / 10-102-121 / 1448 (F-002) |

## 6. Dead / duplicated / unreachable code
N/A (data artifact).

## 7. Dependencies & flags
node builtins only (zero npm deps) — re-runnable anywhere node exists; the log's commands map to ui/package.json:23/25/28.

## 8. Open questions for the next agent
- None. No fabrication indicators; the transcript format (npm banner + OK lines) matches today's script output format exactly.
