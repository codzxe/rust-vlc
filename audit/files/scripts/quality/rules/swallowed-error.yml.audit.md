# AUDIT: scripts/quality/rules/swallowed-error.yml
| Field | Value |
|---|---|
| Auditor | W17-A05 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 15 / 1 ast-grep rule (7 alternative patterns) |
| File verdict | REAL-UNPROVEN (near-zero live coverage; likely-invalid pattern 6 jeopardizes the whole rule) |
| Reachable from shipped entry point? | NO — CI audit lane only: 03_astgrep.sh:17-25; 99_run_all.sh:31 (static) + :44 (quality), J reporter lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | yaml.safe_load -> OK (7 patterns dumped); swallow-form census (rg): empty-arm=1, non-empty Err(_) arm=1, bare `.ok();`=8, `let _ = *.ok()`=0, non-brace Err(_) arms=98, unwrap_or_default=211, `let _ =`=930; quote bytes verified via python (display layer corrupts `$`/`[m` sequences) |

## 1. One-paragraph verdict
The repo's anti-swallow rule matches almost nothing that exists: the tree's single genuine empty-arm swallow is an Or-pattern (`Ok(_) | Err(_) => {}` in shipped opus decoder code) that the rule's exact-arm patterns cannot match, its one non-empty-arm target is ruopus's deliberate panic counter, pattern 7 has zero call sites — and the 8 real `.ok();` cleanup swallows in tools depend on pattern 6, `.ok();`, which is not valid standalone Rust and will most likely fail ast-grep pattern parsing, killing the entire rule scan (which then surfaces as a not_run row in a reporter nobody gates on).

## 2. What actually works (verified)
— verified by `python3 yaml.safe_load` -> OK, 7 patterns: `if let Err(_) = $E {}`, `if let Err($_) = $E {}`, `Err(_) => {}`, `Err(_e) => {}`, `Err(_) => { $$$ }`, `.ok();`, `let _ = $E.ok();`.
— verified by census (rg over crates/apps/tools/tests): the rule's narrow-form targets in the whole tree are exactly two sites:
  - crates/codecs/src/audio/opus/decoder.rs:834 `Ok(_) | Err(_) => {}` — an OR-pattern arm, which the exact-arm patterns do not match (see F-001);
  - tools/vta/ruopus/src/main.rs:131 `Err(_) => { failures += 1; println!("hostile ... PANIC", name); }` — the only literal `Err(_) => { ... }` arm (pattern 5's single target — and it is a deliberate, commented panic counter, not a swallow).
— verified wiring: 03_astgrep.sh:17-25 scans it over crates/apps/tools/tests with not_run on scan failure.

## 3. Findings
F-001 [S3]
Where: scripts/quality/rules/swallowed-error.yml:14 vs 03_astgrep.sh:21-24
Code: `    - pattern: '.ok();'`
What is wrong: `.ok();` is not a valid standalone Rust fragment (no receiver expression) — ast-grep patterns must parse as valid code, so this pattern most likely fails rule compilation, which aborts the ENTIRE scan for this rule (03_astgrep.sh:21-24: scan rc≠0 and empty json -> `not_run "ast-grep:swallowed-error" "scan failed rc=..."` -> zero hits, reporter-only, nothing gates). The 8 real `.ok();` swallows it was meant to catch (tools/mem-audit/src/main.rs:257, tools/sweep/src/main.rs:279, tools/xtask/src/cmd/compat_matrix.rs:1509/1528/1553, tools/bench-vs-vlc/tests/lumen_pcm.rs:473, ...) are then uncovered by ANY rule. Pattern 7 (`let _ = $E.ok();`) — the valid receiver form the author demonstrably knows how to write — has ZERO call sites in the tree.
Confidence: MEDIUM (parse-failure consequence UNVERIFIED — no ast-grep in sandbox; `/usr/bin/sg` is util-linux per W17-A02) — Verification: read + census.

F-002 [S3-adjacent, filed as S4]
Where: scripts/quality/rules/swallowed-error.yml:4-6, 13
Code: `  Error branch is empty or discards the error value: the failure is
  unobservable to the caller. Either propagate it or log it with context.` / `    - pattern: 'Err(_) => { $$$ }'`
What is wrong: pattern 5 flags every non-empty `Err(_) => { ... }` arm — including arms that DO observe the failure (counters, logging, break) — contradicting the message's "failure is unobservable". Its only tree target is exactly such a case (ruopus's panic counter). Combined with F-001: the rule's realistic hit set on this tree is one deliberate non-swallow.
Confidence: HIGH — Verification: read + census.

F-003 [S4]
Where: scripts/quality/rules/swallowed-error.yml:3, 15 (vs ignored-result.yml)
Code: `severity: error` / `    - pattern: 'let _ = $E.ok();'`
What is wrong: severity decorative (nothing consumes it — same as every sibling rule); pattern 7 overlaps the type-blind `let _ = $EXPR;` rule (ignored-result.yml, W17-A02's audit) for the zero call sites that have it; and the whole swallow family named in the audit checklist — `unwrap_or_default` (211 sites), non-brace arms like `Err(_) => continue/break/return` (98 sites) — is covered by no rule at all.
Confidence: HIGH — Verification: read + census.

## 4. Test honesty
No tests, no committed astgrep json (outputs gitignored per W17-A06's systemic finding). Rule execution UNVERIFIED (ast-grep absent).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Error branch is empty or discards the error value" | :5 | TRUE as a description of the two tree sites it can almost reach | census |
| "the failure is unobservable to the caller" | :5-6 | FALSE for pattern 5's only target (the counter observes it) | ruopus main.rs:131 read |
| (implied) guards the repo against swallowed errors | whole rule | SHALLOW | exact-form hits ≈ 0-1; shipped-code swallow at opus decoder.rs:834 unmatched (Or-pattern) |

## 6. Dead / duplicated / unreachable code
Pattern 7 (0 call sites); pattern 6 (suspect-invalid, F-001); overlap with ignored-result.yml.

## 7. Dependencies & flags
ast-grep 0.51.3 (pinned via installer/sg_run). Consumed by 03_astgrep.sh (static+quality J tiers).

## 8. Open questions for the next agent
- Does ast-grep 0.51.3 accept `.ok();` as a pattern (parse failure or match)? One run of `sg scan --rule swallowed-error.yml` on a host with ast-grep settles F-001 — UNVERIFIED here.
- The one genuine shipped-code swallow (opus decoder.rs:834 `Ok(_) | Err(_) => {}` — full failure concealment in the audio decode path) deserves a code-audit look regardless of this rule's blindness.
