# AUDIT: scripts/quality/08_typos.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 26 / 4 sections (typos, categories, top misspellings, spellcheck) |
| File verdict | REAL (reporter, with one broken output section) |
| Reachable from shipped entry point? | NO — CI audit lane: audit.yml:77 install + :80 `99_run_all.sh static` -> 99_run_all.sh:34 `J 08_typos.sh` (static tier; advisory — 0 JG gates, audit.yml continue-on-error per W17-A05's 99_run_all audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `bash -n` -> 0; full simulation with fake `typos` CLI + patched common.sh (OUT -> audit/scratch) -> categories work, histogram collapses to empty-string counts, double NOT-RUN record, script rc=0 despite typos exit 1; `od -c` proof of the total double-zero bug |

## 1. One-paragraph verdict
A J-reporter for the static audit tier: pinned typos-cli 1.52.2 is installed in CI and the run+categorize half works (verified by simulation), but the "top misspellings" histogram is self-defeating as written — its own `s/error: .*//` erases the entire message whenever the post-prefix text starts with `error: `, leaving awk to count empty strings (my simulation output is literally `      4 ` followed by a blank field). The spellcheck section is dead in every wired lane (cargo-spellcheck is deliberately not installed; the "EVAL tier" it names does not exist among 99_run_all's modes). Nothing here can fail anything — by design of the J lane.

## 2. What actually works (verified)
— verified by `bash -n scripts/quality/08_typos.sh` -> exit 0.
— verified by full simulation (patched common.sh with OUT redirected to audit/scratch; fake `typos` emitting 4 brief-format findings, exit 1):
```
=== typos ===
(4 findings lines)
=== categories ===
total=4  in-rust-sources=2  in-docs=2
=== top misspellings ===
      4
=== spellcheck (rustdoc only, EVAL) ===
NOT RUN [cargo-spellcheck]: not installed (run scripts/quality/00_install_tools.sh)
NOT RUN [cargo-spellcheck]: not installed (EVAL tier)
script rc=0
```
The guard path is honest: with `typos` absent, `need typos` records a not_run row and exits 0 (common.sh:36-42, verified in W17-A08's common.sh audit).

## 3. Findings
F-001 [S4]
Where: scripts/quality/08_typos.sh:17-19
Code: `sed -E 's/^[^ ]+:[0-9]+:[0-9]+ - //' "$OUT/typos.txt" 2>/dev/null \
  | sed -E 's/error: .*//' | awk '{print $1}' | sort | uniq -c | sort -rn | head -20`
What is wrong: the second sed erases EVERYTHING after `error: ` — so for any brief-format line whose message begins with `error: `, awk receives an empty line and the histogram counts blank strings (simulation output: `      4 ` + blank). The two seds are mutually incompatible: if the real brief format lacks `error: ` the second sed is dead code; if it has it, the histogram is empty-string garbage. To get the misspelling word the author needed `s/error: //` (strip the tag, not to end-of-line). The exact typos-cli 1.52.2 brief contract is UNVERIFIED (binary absent, no committed typos.txt), but no format makes both seds useful at once.
Confidence: HIGH on the self-defeating logic (executed); MEDIUM on the real-format consequence — Verification: ran (simulation pasted).

F-002 [S4]
Where: scripts/quality/08_typos.sh:12
Code: `total=$(grep -c '^' "$OUT/typos.txt" 2>/dev/null || echo 0)`
What is wrong: on an empty typos.txt, `grep -c` prints `0` AND exits 1, so `|| echo 0` appends a second `0`: total becomes the two-line string `0\n0` (od -c proof: `0 \n 0`). The say line then renders across two lines. Correct form: `total=$(grep -c '^' file 2>/dev/null); total=${total:-0}`.
Confidence: HIGH — Verification: ran (od output pasted).

F-003 [S4]
Where: scripts/quality/08_typos.sh:21-26 vs 00_install_tools.sh:23-26 and 99_run_all.sh:28-75
Code: `head1 "spellcheck (rustdoc only, EVAL)"` / installer comment: `#   cargo-careful, kani-verifier, dylint, cargo-spellcheck, cargo-call-stack,`
What is wrong: cargo-spellcheck is in the installer's "EVAL-only, installed on demand ... not installed here" list — no wired lane installs it, and 99_run_all.sh has no `EVAL` mode (modes: static/quality/dynamic/security/speed/all/report, verified :28-75). The section therefore NEVER runs anywhere automated; its label names a tier that does not exist. Honest (not_run record) but dead-by-construction.
Confidence: HIGH — Verification: read + simulation (not_run path executed).

F-004 [S4]
Where: scripts/quality/08_typos.sh:6, 22, 25
Code: `if ! need typos; then exit 0; fi` / `if need cargo-spellcheck; then` / `  not_run "cargo-spellcheck" "not installed (EVAL tier)"`
What is wrong: double not-run bookkeeping — `need cargo-spellcheck` itself appends one not-run.txt row ("not installed (run scripts/quality/00_install_tools.sh)") and the else branch appends a second ("not installed (EVAL tier)"); one skip = 2 ledger rows (both lines visible in the simulation output). Also the typos exit code is never captured anywhere in the script (findings present or not, script rc stays 0 — simulation: typos exit 1 -> script rc 0), and "in-rust-sources" (:13) counts only `crates/` — apps/, tools/, ui/ hits land in no bucket.
Confidence: HIGH — Verification: ran.

## 4. Section table (26 loc — brief)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| guard | 6 | REAL | — | honest not_run exit 0 |
| typos run | 8-9 | REAL-UNPROVEN | F-004 | rc never captured; output to $OUT/typos.txt |
| categories | 11-15 | REAL | F-002/F-004 | counts verified in simulation |
| top misspellings | 17-19 | BROKEN output | F-001 | empty-string histogram |
| spellcheck | 21-26 | DEAD in all lanes | F-003/F-004 | EVAL tier nonexistent |

## 5. Test honesty
No tests. My simulation is the test: categories arithmetic correct; histogram collapse and double not-run proven. No committed docs/audit/reports/typos.txt exists in-tree (docs/audit holds only docs-sweep-20261110.md + panic-baseline.json) — no committed evidence that the tier ever ran (UNVERIFIED; consistent with the billing-blocked CI lanes).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T08 - typos in identifiers, user-facing strings and docs" | :2 | SHALLOW | runs the CLI but the headline histogram is broken (F-001); scope buckets miss apps/tools/ui |
| pinned toolset (typos-cli@1.52.2) | 00_install_tools.sh:20 | REAL | installer pin verified |

## 7. Dead / duplicated / unreachable code
`sed -E 's/error: .*//'` half of F-001; the EVAL spellcheck section (F-003).

## 8. Dependencies & flags
typos-cli@1.52.2 (pinned, installed in CI static lane); cargo-spellcheck (never installed anywhere). Writes docs/audit/reports/typos.txt (never committed). No feature flags.

## 9. Open questions for the next agent
- Exact typos-cli 1.52.2 `--format brief` line shape (needs the binary — UNVERIFIED). Whoever has the tool should run `typos --format brief . | head -3` and reconcile with F-001's two seds.
