# AUDIT: scripts/quality/10_api_surface.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 45 / 3 sub-checks (10a dump, 10b semver, 10c external types) |
| File verdict | REAL reporter — honest not-run ledger; two count/swallow defects |
| Reachable from shipped entry point? | NO — quality tool; `J 10_api_surface.sh` in 99_run_all.sh `quality` tier, driven by .github/workflows/audit.yml:127 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `ls -d .git` -> exists; `command -v cargo-public-api cargo-semver-checks cargo-check-external-types` -> none installed (so on this host the fallback/not_run paths are the live ones) |

## 1. One-paragraph verdict
A three-part API-surface reporter whose honest parts are genuinely honest (not_run ledger entries with reasons and install commands, a grep fallback when cargo-public-api is absent, an explicit .git-baseline precondition for semver-checks). Two defects: the same `grep -c ... || echo 0` double-zero bug as 01_clippy.sh (reproduced there), and per-crate `cargo public-api` failures are swallowed into "0 pub items" index rows instead of a not-run entry — a failing dump looks like an empty crate.

## 2. What actually works (verified)
— `need`/`not_run` semantics verified in common.sh:26-31 (timestamped ledger, "NEVER invent a result").
— 10b precondition: `[ ! -d .git ]` -> not_run "no .git in this working copy (blocker B1)" — honest; on this host .git EXISTS and the tools don't, so the not_run path is live.
— Fallback 10a (L21-24) counts pub items with grep when the EVAL-tier tool is absent — present and coherent.

## 3. Findings
F-001 [S4] — `grep -c ... || echo 0` double-zero (same bug class as 01_clippy.sh:41).
Where: scripts/quality/10_api_surface.sh:14
Code:
```
    n=$(grep -c '^pub' "$OUT/api/$name.txt" 2>/dev/null || echo 0)
    printf '%-24s %5d pub items\n' "$name" "$n" >> "$OUT/api/index.txt"
```
What is wrong: when the dump has no `^pub` lines, grep prints 0 AND exits 1, `|| echo 0` appends a second 0 — `$n` becomes two lines and the index row is corrupted (executed proof in the 01_clippy.sh audit: `A[0\n0]B`). Confidence: HIGH — Verification: ran (bug class reproduced on 01_clippy's identical construct).
F-002 [S4] — Failed public-api dumps are recorded as "0 pub items", not as failures.
Where: scripts/quality/10_api_surface.sh:13-15
Code:
```
    cargo public-api --simplified -p "$name" > "$OUT/api/$name.txt" 2>/dev/null
```
What is wrong: stderr discarded, exit ignored; a crate whose dump fails (compile error, tool bug) truncates the output file and the index prints `0 pub items` — indistinguishable from an empty public API. The not-run/failure ledger is bypassed for per-crate failures. Why it matters: the sweep's evidence file can misstate a real crate as empty. Correct: check the exit and record a failure row. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Sub-check | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 10a public API dump | 8-25 | REAL | S4 ×2 | fallback path is the live one on tool-less hosts |
| 10b semver-checks | 27-34 | REAL | — | .git precondition honest |
| 10c external types | 36-44 | REAL | — | not_run honest |

## 5. Test honesty
N/A (reporter). As with 01_clippy.sh: the script always exits 0 (reporter design; the J contract's rc-recording is inert).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "EVAL tier" tools documented with install commands | L19, 33, 43 | REAL (honest gaps) | none installed here; ledger rows would record it |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
cargo-public-api@0.52.0 / cargo-semver-checks / cargo-check-external-types (EVAL tier, none in sandbox); crates_list from common.sh (needs cargo metadata — absent here).

## 9. Open questions for the next agent
- Does the `quality` tier ever run with the EVAL tools installed in CI (audit.yml quality job)? If not, 10a's real dump path is CI-dead too and only the grep fallback ever runs.
