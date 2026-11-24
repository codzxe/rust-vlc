# AUDIT: scripts/quality/test_hygiene.py
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / functions | 183 / 3 (strip_code, body_of, main) |
| File verdict | REAL (executed over the real tree via a redirect-patched copy; output redirected into audit/ — repo untouched) |
| Reachable from shipped entry point? | YES (CI tooling: scripts/quality/13_test_hygiene.sh:29 runs it; 99_run_all.sh static/quality tiers call 13_test_hygiene.sh; audit.yml static/quality jobs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 probe copy on crafted .rs fixtures (2 runs, exit 0); python3 exec-driver over real repo (exit 0); rg/sed quotes; rg ignore-attr counts |

## 1. One-paragraph verdict
A real, executable test-quality census — and I executed it: over the actual repo it
reports 4637 tests, 170 assertion-free candidates, 20 sleep-in-test, 1 empty body,
50 ignored (all carrying reasons), 0 should_panic. Its comment-stripping claim is
verified: raw `rg '#\[ignore'` counts 67 occurrences in source dirs but 17 of those
are doc-comment MENTIONS (`tools/xtask/src/cmd/rebrand.rs:34` `//! ... #[ignore]d
placeholders`) and the stripped census correctly counts only the 50 real attributes.
The one real defect, proven by an executed probe: raw strings containing an embedded
double quote (r#"..."#) break the hand-rolled stripper — a fake test whose body only
mentions `assert!(...)` inside such a string is counted as genuinely asserting. The
docstring's "Not a heuristic regex over raw text" overstates the lexer's fidelity.

## 2. What actually works (verified)
- Executed census (real tree, output redirected to audit/scratch/test-hygiene.json): `tests found: 4637 / assertion-free: 170 / sleeps: 20 / empty body: 1 / ignored: 50 (no reason: 0)`. Exit 0.
- Comment-stripping differentiator PROVEN: rg `#[ignore` = 67 (crates 53 + apps 2 + tests 8 + tools 3, incl. doc-comment mentions like `tools/bench-vs-vlc/tests/lumen_pcm.rs:12`), census = 50 real attributes — the exact false-positive class its docstring claims to remove.
- The 170 assertion-free candidates are printed with file:line + test name (e.g. `tests/rebrand/rebrand_skeleton.rs:618 window_title_renders_nova`) — a genuine, usable fake-test watchlist; honestly labeled "candidate".
- Probe 1 (raw string WITHOUT inner quotes): `r#"we write assert_eq!(1, 1) in a string"#` -> correctly classified assertion_free. The basic stripping path works.
- Brace matching (body_of) operates on stripped source where string contents are whitespace — bracketed test bodies extract correctly in all observed output rows.

## 3. Findings
F-001 [S3]
Where: scripts/quality/test_hygiene.py:30-80 (strip_code) vs docstring L4-6
Code (docstring):
```
Not a heuristic regex over raw text: comments and string literals are stripped
first (they contain the word #[test] in doc comments), then every #[test] body
is brace-matched and inspected for a real failure mechanism.
```
Code (the hole — probe, executed):
```
    #[test]
    fn fake_via_rawstring_embedded_quote() {
        let s = r#"fake "assert!(1 == 1)" string"#;
        let _ = s;
    }
```
Probe output: `assertion-free: 0` — the fake test was counted as HAVING a failure
mechanism. What is wrong: the `"` branch of strip_code scans for the next unescaped
`"` regardless of the `r#"` raw-string prefix — an embedded `"` terminates the
"string" early and leaks the remainder (incl. `assert!(` text) into the inspected
code. Braces leaked the same way can also corrupt body_of's brace matching.
Why it matters: the tool's value is exactly "not fooled by text"; this class of
string defeats it, producing false negatives (missed fake tests) and potentially
wrong bodies. What correct looks like: detect `r#*"` openings and scan for the
matching `"#+` terminator (and `b"`, `c"`) before falling into the plain-string
scanner. Confidence: HIGH — Verification: ran (probe under audit/scratch, output
pasted above).
F-002 [S4]
Where: scripts/quality/test_hygiene.py:111-114
Code:
```
            try:
                raw = open(p, encoding="utf-8", errors="replace").read()
            except Exception:
                continue
```
What is wrong: unreadable files vanish silently — neither counted nor listed; a
whole crate could drop out of the census without a trace. `errors="replace"`
already tolerates bad bytes, so the except is nearly dead, which makes it worse:
it hides real I/O problems (permissions). Confidence: HIGH — read.
F-003 [S4]
Where: scripts/quality/test_hygiene.py:104,18-23
Code:
```
    walk_skip = {".git", "target", "node_modules", "download", "fuzz"}
```
What is wrong: `fuzz/` is skipped entirely — the 35 fuzz targets are outside the
hygiene census; also `ui/` (.ts) is naturally out of scope while `tools/` is in —
the skip-set encodes decisions that are documented nowhere in the file. Cosmetic
transparency gap. Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| strip_code | 30-80 | REAL | S3 (F-001) | handles //, nested /* */, plain strings w/ escapes, char-vs-lifetime; raw strings with embedded quotes break it |
| body_of | 83-98 | REAL | — | naive brace match over stripped src; correct given stripping worked |
| main | 101-183 | REAL | — | os.walk with skip-set, 6 counters, JSON+stdout output, per-crate rollup (top 6), first 15 rows per category |

## 5. Test honesty
N/A (census tool). Mutation reasoning: kill the ASSERT_RE and the census reports
everything assertion-free — output visibly implodes; kill the strip and doc-comment
`#[ignore]` mentions inflate counts — I verified both directions with the rg-vs-census
comparison (67 raw vs 50 stripped).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Outputs docs/audit/reports/test-hygiene.json and a human summary on stdout" | L8-9 | REAL | executed (redirected); 13_test_hygiene.sh:29 tees it to $OUT/test-hygiene.txt |
| "Not a heuristic regex over raw text" | L4 | SHALLOW-in-part | comment-stripping verified; raw-string-with-quotes hole proven (F-001) |
| "170 candidate fake tests" class exists | my executed run | REAL | printed rows with file:line; note many are helper-asserting tests (census cannot see into helpers) — the tool itself says "candidate" |

## 7. Dead / duplicated / unreachable code
None. `download` in walk_skip guards an optional corpus dir; harmless.

## 8. Dependencies & flags
stdlib only (json/os/re/sys/collections). Runs under python3 (validated: the
common.sh PY launcher's discipline). Consumer: 13_test_hygiene.sh (static +
quality tiers), transitively audit.yml.

## 9. Open questions for the next agent
- The 170-row assertion-free list (audit/scratch/test-hygiene.json has the full set)
  is raw material for the fake-test hunting waves — the census cannot distinguish
  helper-asserting tests from truly assertion-free ones; spot-audit the top crates
  (codecs:56, tests:38, demux:26) before citing the number as fake-test count.
- Should F-001 be fixed, re-run and diff the counts — a drop in
  assertion-free would directly measure the false-negative rate.
