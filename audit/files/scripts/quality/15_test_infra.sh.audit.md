# AUDIT: scripts/quality/15_test_infra.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 65 / 1 script (sections 15a-15d) |
| File verdict | REAL (with a false-positive census bug) |
| Reachable from shipped entry point? | NO — dev quality discovery script; never edits Cargo.toml by design |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Replicated 15a/15b greps read-only (no tee into repo); ls docs/audit/tasks/; rg 00_install_tools.sh |

## 1. One-paragraph verdict
A discovery-only census of modern Rust test primitives that produces one
genuinely useful fact — the repo uses NONE of the 16 WANT tools (all
MISSING from every manifest; property-based/snapshot/trybuild testing is
absent, the suite is plain #[test] + hand-rolled harnesses) — but its 15b
usage census is broken by an unanchored `use ${m}` grep alternative that
counts substrings inside prose: it reports "insta: 4 usages" where all 4
matches are the word "instantiation" in unreachable! messages and an
`InstallReceipt` pub-use line. The Lead's "ready-to-apply plan" (15c) is
thus fed corrupted usage data.

## 2. What actually works (verified)
— 15a replication (my run): all 16 WANT entries MISSING — nextest, insta,
insta-cmd, rstest, proptest, test-strategy, trybuild, assert_cmd,
assert_fs, predicates, escargot, fail, serial_test, arbitrary, dhat,
mockall — zero manifests carry any.
— 15a's manifest key pattern `^\s*${crate//-/[-]}\s*=` correctly matches
both `assert_cmd =` and `assert-cmd =` spellings.
— 15d degrades gracefully: docs/audit/tasks/T15-examples.md does not exist
(ls) -> "(examples file not present)" fallback.
— 15c's plan pins concrete versions and names the AGENTS.md 3.2
constraint ("none of these may enter the shipped graph").

## 3. Findings
F-001 [S3]
Where: scripts/quality/15_test_infra.sh:36
Code:
```
  n=$(grep -rn --include=*.rs -E "\b${m}::|#\[${m}\]|use ${m}" crates apps tools tests 2>/dev/null | wc -l)
```
What is wrong: the third alternative `use ${m}` is unanchored at the end —
"refuse instantiation" contains "use insta", so prose/doc strings are
counted as usages. Demonstrated: the script's insta count is 4, and all 4
real matches are `unreachable!("...must refuse instantiation")` lines
(crates/plugin-host-wasm/tests/plugin_sandbox_security.rs:400,423) and
`pub use install::{...}` (lib.rs:96-97) — zero actual insta usage. The
census misinforms 15c's plan ("primitive is used where").
What correct looks like: `use ${m}(\b|::)` or `use ${m}\b` word-boundary.
Confidence: HIGH — Verification: ran replicated grep + inspected all 4 hits.
F-002 [S4]
Where: scripts/quality/15_test_infra.sh:30
Code:
```
  hits=$(grep -rl --include=Cargo.toml -E "^\s*${crate//-/[-]}\s*=" crates apps tools tests 2>/dev/null | wc -l)
```
What is wrong: `\s` in grep -E is a GNU extension, not POSIX ERE — works
on the stated platforms (Linux/Git-Bash) but is not portable to strict
grep implementations. Cosmetic given the declared runtime.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: scripts/quality/15_test_infra.sh:65
Code:
```
cat "$ROOT/docs/audit/tasks/T15-examples.md" 2>/dev/null || say "(examples file not present)"
```
What is wrong: the referenced examples file was never created
(docs/audit/tasks/ does not exist) — half of section 15d is a permanent
fallback line.
Confidence: HIGH — Verification: ls.
F-004 [S4]
Where: scripts/quality/15_test_infra.sh:7-24
Code:
```
WANT=(
  "nextest:nextest"
```
What is wrong: the census lists 16 tools but reports nothing about
CARGO_NEXTEST absence interacting with 12_mutants.sh's hardcoded
`--test-tool nextest` — a discovery script whose job is exactly to surface
such infra gaps missed the one cross-script dependency (mutation testing
requires nextest; nextest is absent locally and only CI-installed).
Confidence: MEDIUM — Verification: read + rg 00_install_tools.sh (CI
installs cargo-nextest@0.9.152).

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 15a census | 26-32 | REAL | — | replicated: all 16 MISSING |
| 15b usage | 34-38 | SHALLOW | S3 | false positives (F-001) |
| 15c plan | 40-62 | REAL | — | static text; version pins; constraint noted |
| 15d examples | 64-65 | REAL | S4 | permanent fallback (file absent) |

## 5. Test honesty
Discovery script — its "test" is the census output; the 15b numbers are
provably wrong for insta (4 false positives, 0 real).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Discovery only: never edits Cargo.toml" | :3 | REAL | no write to any manifest in the script |
| (implied) 15b counts real usages | :34-38 | FALSE in effect | F-001 demonstration |

## 7. Dead / duplicated code
15d's cat is a permanent fallback; nothing else dead.

## 8. Dependencies & flags
grep/printf/tee only. Outputs to $OUT (docs/audit/reports, gitignored) —
like all quality scripts, results are never committed.

## 9. Open questions
Was any action ever taken from 15c's plan? Zero of the 16 primitives are
in-tree today, so the "ready-to-apply plan" remains unapplied.
