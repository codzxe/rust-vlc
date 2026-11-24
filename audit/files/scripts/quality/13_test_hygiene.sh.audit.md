# AUDIT: scripts/quality/13_test_hygiene.sh
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 38 / 4 sections (13a-13d) |
| File verdict | REAL (measurement reporter; real census logic, honest not_run protocol) |
| Reachable from shipped entry point? | YES (CI evidence lane): 99_run_all.sh:39,48 runs it as `J 13_test_hygiene.sh` (reporter tier, invoked by audit.yml via `bash scripts/quality/99_run_all.sh <tier>`); never gates |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `bash -n` -> OK; read common.sh/99_run_all.sh/test_hygiene.py; `ls docs/audit/reports` -> absent; sed quotes verified |

## 1. One-paragraph verdict
A genuine measurement harness, not a gate — and it says so: it runs in the
`J` (reporter) tier of 99_run_all.sh whose exit code "is recorded but does
not fail the tier", inside audit.yml which self-describes as "evidence ... not
a green light" while ci.yml is the merge gate. The census logic is real
(13c delegates to test_hygiene.py, a comment-stripping, brace-matching
structural analyzer — read, not fake). The defects: 13d silently excludes
three crates (platform-linux, platform-macos, input) from flaky detection with
no recorded reason, and 13b's fallback astgrep invocation discards all its
output/stderr, so a failed scan degrades to an ambiguous "0 hits (or scan
not run)" line.

## 2. What actually works (verified)
- `bash -n scripts/quality/13_test_hygiene.sh` -> exit 0.
- common.sh protocol verified: `not_run` appends timestamped rows to
  docs/audit/reports/not-run.txt and "NEVER invent a result" (common.sh:29-34).
- 13c: test_hygiene.py is a real parser (strips comments/string contents
  with offset preservation, brace-matches test bodies, counts failure
  mechanisms) — read the implementation, not just the docstring.
- J/JG split verified in 99_run_all.sh:16-24: this script is `J` (lines 39,
  48) — its non-gating status is declared, not hidden.

## 3. Findings
F-001 [S3] — Flaky-detection coverage holes, unexplained
Where: scripts/quality/13_test_hygiene.sh:33-34
Code:
```
  cargo nextest run --workspace --all-features --retries 3 --no-fail-fast \
    --exclude platform-linux --exclude platform-macos --exclude input \
```
What is wrong: three crates are excluded from the flakiness census with no
comment anywhere in the file. Whatever the reason (sandbox/hardware
sensitivity), the exclusions are silent: a flaky platform-linux test would
never be counted. Why it matters: this is the repo's only flaky detector,
and it is blind on 3 of 39 crates. What correct looks like: `--exclude` with
an inline reason + the exclusion mirrored in the census output.
Confidence: HIGH — Verification: read; no comment exists in file.
F-002 [S4] — Fallback scan errors discarded; "0 hits" can mean "not measured"
Where: scripts/quality/13_test_hygiene.sh:18-24
Code:
```
  [ -f "$OUT/astgrep/$id.json" ] || bash "$ROOT/scripts/quality/03_astgrep.sh" "$id" >/dev/null 2>&1
  f="$OUT/astgrep/$id.json"
  if [ -s "$f" ]; then
    n=$($PY -c "import json,sys;print(len(json.load(open(sys.argv[1],encoding='utf-8'))))" "$f" 2>/dev/null || echo '?')
    say "$id: $n hits"
  else
    say "$id: 0 hits (or scan not run)"
  fi
```
What is wrong: the fallback 03_astgrep.sh run is fully silenced
(`>/dev/null 2>&1`); if it fails, the section prints "0 hits (or scan not
run)" — a number and a non-measurement rendered in the same shape. The
parenthetical is honest, but a scoreboard consumer diffing "hits" counts
cannot tell. What correct looks like: propagate the fallback's rc into a
`not_run` record (the protocol already exists in common.sh).
Confidence: HIGH — Verification: read.
F-003 [S4] — 13a test count is a grep heuristic
Where: scripts/quality/13_test_hygiene.sh:10
Code:
```
    && say "tests: $(grep -c '"name"' "$OUT/tests.json" 2>/dev/null || echo '?')" \
```
What is wrong: counts lines containing the literal `"name"` in nextest's JSON
instead of parsing it; any extra field or error text containing "name"
corrupts the count, and failure prints `tests: ?`. The next section (13b)
does parse JSON properly with $PY — inconsistent rigor. Cosmetic census drift.
Confidence: HIGH — Verification: read.

## 4. Section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 13a nextest inventory | 6-14 | REAL | — | F-003 count heuristic; not_run fallback honest |
| 13b AST heuristics | 16-26 | REAL | — | F-002 silenced fallback |
| 13c census | 28-29 | REAL | — | test_hygiene.py real parser; exit ignored (reporter tier) |
| 13d flaky detection | 31-38 | REAL | — | F-001 silent exclusions |

## 5. Test honesty
Not a test file. Its own runs leave no in-repo evidence: `ls
docs/audit/reports/` -> directory absent — reports live only in CI artifacts
(audit.yml runs 99_run_all.sh). Current-tree execution UNVERIFIED
(reason: requires cargo-nextest + full workspace build; load discipline).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T13 - test hygiene: fake-test heuristics, ignores, sleeps, flaky detection, timings" (L2) | header | REAL (heuristic part) / SHALLOW (flaky part) / FALSE (timings) | 13b/13c real; 13d real command but with F-001 holes; "timings": `grep -n 'tim\|dur\|elapsed' scripts/quality/test_hygiene.py` -> only SLEEP_RE (a regex, not timing); no timing measurement anywhere in the script or census — claim inflation |
| audit.yml is "evidence, not a green light"; ci.yml is the merge gate | .github/workflows/audit.yml:17-18 | REAL | read; J-reporter placement in 99_run_all.sh:39,48 |

## 7. Dead / duplicated / unreachable code
None. `need cargo-nextest` gating means 13a/13d degrade to not_run when the
tool is absent — by design, recorded.

## 8. Dependencies & flags
cargo-nextest (13a/13d), 03_astgrep.sh + ast-grep (13b fallback),
python3 (test_hygiene.py), common.sh env (OUT=docs/audit/reports).

## 9. Open questions for the next agent
- Who decided the 13d exclusions (platform-linux/macos/input)? Check
  worklog/docs for the rationale; if the reason is "they fail in CI", then
  the flaky census is excluding the crates most likely to BE flaky.
- The header claims "timings" — nothing in the file measures timing; is
  timing in test_hygiene.py? (I read its head only — its tail may or may not
  time tests; the census I read counts, not times. UNVERIFIED beyond head.)
