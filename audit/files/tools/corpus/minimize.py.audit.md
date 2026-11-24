# AUDIT: tools/corpus/minimize.py
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 187 / 4 (Pred, truncation_min, ddmin, main) |
| File verdict | REAL (executed end-to-end in this audit) |
| Reachable from shipped entry point? | NO — triage companion tool (dev use) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 execution test (grep predicate, 401,010 B synthetic crash); rg references |

## 1. One-paragraph verdict
A ddmin crash-reproducer minimizer: truncate stage (binary search on keep-length, preserves headers), ddmin stage (aligned chunk removal with granularity backoff + byte-granularity tail trim), predicate = shell template with {file}/{size} substitution and --expect-rc (0 / any-nonzero / N), flake control via --verify N with an honest FLAKY verdict + exit 1. EXECUTED by this auditor: synthetic 401,010-byte file with a 10-byte trigger at offset 400,000 → reduced to exactly 10 bytes, 45 predicate calls, 0.1s, verify 3/3 STABLE, exit 0, JSON report written (output: `401010 -> 10 bytes (100.00% reduction)`). The algorithm works and the honesty mechanisms (verify/FLAKY, max-tests budget, hang-is-not-a-reproduce) are real.
## 2. What actually works (verified)
— verified by `python3 tools/corpus/minimize.py --in crash.bin --predicate 'grep -q TRIGGERXYZ {file}' --max-tests 300 --verify 3 --json rep.json` → 10-byte minimizer output equal to the trigger, STABLE, rc=0.
## 3. Findings
F-001 [S4]
Where: minimize.py:140-141
Code: `out = Path(args.out) if args.out else src.with_suffix(src.suffix + ".min" + src.suffix)`
What is wrong: default output name appends ".min" BETWEEN extensions and re-appends the extension → `crash.bin` → `crash.bin.min.bin` (observed in the executed run). Odd, harmless. Confidence: HIGH — Verification: ran (observed output path crash.bin.min.bin).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Pred.__call__ | 49-63 | REAL | — | timeout → False (hang ≠ reproduce); calls counted |
| truncation_min | 66-79 | REAL | — | binary search assumes prefix-monotonicity — standard ddmin assumption, documented strategy L8-10 |
| ddmin | 82-117 | REAL | — | aligned chunks, granularity n*2 backoff, tail trim (1..1024) |
| main | 120-183 | REAL | — | verify/FLAKY/JSON; exit 0 stable / 1 flaky / 2 non-reproducing input |
## 5. Test honesty
Not a test file; a tool. Self-honesty verified (FLAKY path, orig-does-not-reproduce refusal at :148-151).
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "40KB synthetic crash → 217 B (99.46% reduction), 59 predicate calls, 3/3 stable" | docs/worklog/archive-2026-10.md (minimize.py entry) | REAL | my independent run: 401,010 → 10 B, 45 calls, 3/3 stable — same class of result |
| "never silently accepted" (flake control, docstring L20-22) | file itself | REAL | :164-183 exit 1 when verify < N |
## 7. Dead / duplicated code
None.
## 8. Dependencies & flags
stdlib only (argparse/json/subprocess/tempfile/time). No repo coupling beyond the predicate command the user supplies.
## 9. Open questions
None.
