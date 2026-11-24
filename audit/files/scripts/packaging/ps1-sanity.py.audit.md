# AUDIT: scripts/packaging/ps1-sanity.py
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 235 / 2 classes/fns (Walker: 7 methods, check, main) |
| File verdict | REAL (executed; 6-case mutation-tested; detects every documented class) |
| Reachable from shipped entry point? | N/A — tooling; called by scripts/packaging/gate.sh step 4 (packaging gate = full_gate.sh step 11) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | ps1-sanity.py on the 2 real .ps1 (PASS, rc 0); 6 synthetic malformed samples in /tmp (all flagged, rc 1); od BOM check |

## 1. One-paragraph verdict
A working structural checker, honestly labeled as NOT a PowerShell parser
("STRUCTURAL check, NOT a parse [the real parse runs on the Windows runner]").
I mutation-tested it: unbalanced/mismatched delimiters, unterminated
here-string, code-before-param, and non-ASCII all get flagged with exit 1; a
clean param-first file passes with exit 0; the two real packaging .ps1 files
pass. The state machine (code/blockcomment/dq/sq/hdq/hsq) handles the documented
PowerShell constructs correctly for the files it guards, including the
`"@`-must-be-line-leading here-string terminator rule. One real defect: a
param-less .ps1 ALWAYS fails with the misleading message "code precedes
'param('" (the loop cannot distinguish "no param block" from "code before
param") — harmless today because all three guarded files have param blocks,
but the checker's message lies about the actual condition.

## 2. What actually works (verified)
- `python3 scripts/packaging/ps1-sanity.py scripts/packaging/test-msi-upgrade.ps1 scripts/packaging/build-msi.ps1` -> `ps1-sanity: PASS (ASCII-only, delimiters balanced, param first)` rc 0.
- Mutation tests (all rc 1 with correct findings):
  - `param($x)` preceded by code -> "code precedes 'param(' (first statement rule)" ✓
  - `@(1,2}` -> "mismatched '}' closes '(' opened at line 1" ✓
  - unterminated `@"` -> "file ends inside a here-string @\"" ✓
  - latin-1 byte -> "unreadable as UTF-8" ✓
  - balanced `@" ... "@` here-string -> NO here-string finding (balance detection works) ✓
  - clean `param($a)` file -> PASS rc 0 ✓
- No BOM in the guarded .ps1 files (`od` first 3 bytes = `23 20 74`, not EF BB BF).

## 3. Findings
F-001 [S3]
Where: scripts/packaging/ps1-sanity.py:191-204
Code:
```
    for idx, line in enumerate(lines, start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("param("):
            break
```
What is wrong: a .ps1 with NO param() block at all hits the first code line and
reports "code precedes 'param('" — the message asserts a param block exists
elsewhere. Verified: synthetic `$y = @(1,2)` + `Write-Host $y` (no param) ->
"t2.ps1:2: code precedes 'param('". Why it matters: anyone running the checker
on a legit param-less helper script gets a false structural failure with a
wrong diagnosis. What correct looks like: track whether any `param(` was found;
if none, either skip the rule or say "no param() block found".
Confidence: HIGH — Verification: ran.

F-002 [S4]
Where: scripts/packaging/ps1-sanity.py:145-150
Code:
```
        elif mode == "blockcomment":
            if ch == "#" and nxt == ">":
                w.advance(2)
                mode = "code"
```
What is wrong: PowerShell block comments NEST (`<# <# #> #>`); this walker
treats a nested `#>` as closing the outer comment — mis-tracking position on
nested-comment files. Not exercised by the guarded files. Confidence: MEDIUM —
Verification: read.

F-003 [S4]
Where: scripts/packaging/ps1-sanity.py:89-91
Code:
```
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        return [f"{path}: unreadable as UTF-8: {exc}"]
```
What is wrong: a UTF-8 BOM (good practice for PS 5.1 files) decodes to U+FEFF
and would be flagged by the ASCII rule as "non-ASCII byte" — the one encoding
that HELPS legacy hosts is rejected. Convention-consistent (repo ships BOM-less
ASCII; verified no BOM), but the failure message misleads ("keep .ps1 source
ASCII" for a BOM). Confidence: HIGH for behavior (ord(U+FEFF)>0x7F), MEDIUM
for whether it ever matters. Verification: read + od on real files.

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Walker.__init__/eof/peek/line_text/line_start/advance/skip_to_line_end | 47-83 | REAL (exercised via mutation tests) | — | col/line bookkeeping correct incl. multi-char advances |
| check | 86-206 | REAL | S3/S4 | rules 1-5 all fire in tests; F-001 message conflation; F-002 nesting; F-003 BOM |
| main | 209-235 | REAL | — | rc 0/1/2 contract verified; findings to stderr, PASS to stdout |

## 5. Test honesty
No in-repo tests for this checker; my 6 synthetic mutations + 2 real files are
the test. Would it fail if broken? If the stack logic regressed, t5 (mismatched
brace) would stop firing — mutation direction covered.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "gate.sh step 4" caller | ps1-sanity.py:3-4 | TRUE | gate.sh step 4 ("structural check (no pwsh in this sandbox)") ran in my gate.sh invocation |
| "this is a STRUCTURAL check, NOT a parse" | ps1-sanity.py:8-10 | TRUE | no tokenizer/AST; delimiter walking only |
| "none of them can hide an unbalanced delimiter from rule 2" | ps1-sanity.py:29-30 | TRUE for dq/sq content (strings skipped wholesale); FALSE-ish for nested block comments (F-002 can mis-track) | read + tests |

## 7. Dead / duplicated / unreachable code
None found. `stripped == "]"` (line 197) is a niche multi-line-attribute
allowance — not dead, just narrow.

## 8. Dependencies & flags
Stdlib only (sys, pathlib). Python 3.7+ (from __future__ annotations). No
flags. Invoked per-file or multi-file by gate.sh step 4 and by me.

## 9. Open questions for the next agent
- gate.sh claims "balanced braces/parens/quotes per file" via this tool — note
  the tool does NOT check quote balance in the sense of paired quotes outside
  strings (dq/sq modes handle open+close within the walker, so effectively it
  does) — verified via t3/t4 here-string cases; plain `"unterminated` dq would
  be caught by "file ends inside a double-quoted string". Test that case too if
  touching this file.
