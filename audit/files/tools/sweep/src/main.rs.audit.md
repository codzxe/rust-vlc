# AUDIT: tools/sweep/src/main.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 429 / 8 (+3 tests) |
| File verdict | REAL-UNPROVEN (instrument real + 15/15 unit tests in baseline; never executed against its corpus in any committed run) |
| Reachable from shipped entry point? | NO — test-infra bin `lumen-sweep` (workspace member, root Cargo.toml:41) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full read; rg corpus-v4/SWEEPLINE; baseline block :3297-3321; ls /tmp/my-project |

## 1. One-paragraph verdict
A subprocess-per-file corpus sweep instrument: supervisor walks dirs deterministically, spawns itself in `--file` worker mode per file, enforces a poll-based deadline (try_wait + kill, default 30s), classifies outcomes (ok/unsupported/dirty-reject/typed-error/panic/timeout), emits one tab-separated SWEEPLINE per file + per-class tallies. The isolation rationale (L27-50) is sound and honestly argued (catch_unwind can't see aborts, can't enforce timeouts, leaks state). Usage errors typed (exit 64), spawn-failure typed (exit 70), panics/timeouts are honest per-file outcomes (exit 0 = completed). What is missing: the corpus it exists for (corpus-v4) is NOT committed — regenerated on demand (docs/corpus/corpus-v4-smoke-manifest-20261023.json: 126 files, 69,512,763 B, generator tools/corpus/gen_corpus.py; the only corpus dir candidates are /tmp/my-project/ln — absent in this sandbox — and download/, which contains a 1-line README "Here are all the generated files." = placeholder). rg SWEEPLINE over the tree: only this source + a NEXT-STEPS.md mention — no committed sweep log exists.

## 2. What actually works (verified)
- Baseline: package lumen-sweep 15 unit tests pass (audit/runs/baseline/cargo-test-workspace.txt:3297-3321, exit_code=0) — but only the outcome/worker/line-shape layer (tests at :366-428 here assert field layouts; the subprocess machinery `sweep_one`/`wait_with_timeout` has NO test).
- Honest cap: `--max-files` truncation always stated in the summary (:191-194).

## 3. Findings
F-001 [S3]
Where: tools/sweep/src/main.rs:1 ("the corpus-v4 sweep instrument") + docs/corpus/corpus-v4-smoke-manifest-20261023.json (`"count": 126, "bytes_total": 69512763`) + download/README.md ("Here are all the generated files.")
What is wrong: the tool's input corpus is never committed and no committed run output (SWEEPLINE log) exists anywhere — the differential/sweep lanes that would consume corpus-v4 are vacuous-green wherever the corpus is absent (the sibling codecs tests print "LUMEN_CORPUS unset and /tmp/my-project/ln absent — SKIP" and pass — e.g. crates/codecs/tests/flac_bitexact.rs; baseline :1070 shows `corpus_flacs_decode_byte_exact_or_reject_typed ... ok` in 0.21s, consistent with a skip, capture hidden for passing tests). The sweep instrument itself therefore has zero end-to-end evidence in the repo. Why it matters: the panic/damage census the tool exists to produce has no committed artifact. What correct looks like: commit a bounded smoke sweep output (or wire gen_corpus + sweep into a parkable workflow like compat-matrix.yml). Confidence: HIGH — Verification: ran (rg + ls + baseline reads). UNVERIFIED: actual sweep execution (cargo absent).
F-002 [S4]
Where: tools/sweep/src/main.rs:304-324 `walk_dir`
What is wrong: `path.is_dir()` follows symlinks; a symlink cycle in a swept tree recurses until stack exhaustion (no visited-set). Dot-entries skipped by name only. Blast radius: a hostile corpus tree layout, not media bytes. Confidence: MEDIUM — Verification: read (mutation reasoning).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 75-79 | REAL | — | thin entry |
| run | 82-198 | REAL | — | worker dispatch + arg parse + cap honesty |
| sweep_one | 214-263 | REAL | — | spawn/kill/read-after-exit; no unit test |
| wait_with_timeout | 268-284 | REAL | — | poll deadline; no unit test |
| parse_entry | 288-299 | REAL | — | tested (:366-378) |
| walk_dir | 304-324 | REAL | S4 | symlink cycle (F-002) |
| sweepline | 330-360 | REAL | — | tested both shapes (:380-428) |
| tests (3) | 366-428 | REAL | — | field-layout assertions |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parse_entry_handles_overrides_and_bare_paths | YES | 3-way grammar pinned |
| sweepline_for_a_timeout_has_dashes_and_the_note | YES | field idx 3/4/15 pinned |
| sweepline_for_an_ok_file_carries_the_worker_fields | YES | composite fields "10/10" pinned |
Gap: sweep_one/wait_with_timeout/classify integration untested (unit-level only) — the isolation machinery could regress silently.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the v4 smoke manifest is 126 files" (cost note L46-47) | file itself | REAL | manifest count=126 (read) |
| corpus-v4 exists for sweeping | header L1 | FALSE-in-repo | no committed corpus bytes; download/ placeholder; /tmp corpus absent — regeneration-on-demand only |
| "the summary always states the cap" (:18-19) | file itself | REAL | :191-194 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
Std-only (per tools/sweep/Cargo.toml header C5); no feature gates; workspace member.

## 9. Open questions for the next agent
- tools/corpus/run_harness.py does NOT invoke lumen-sweep (rg no match) — is any driver wired to it at all, or is the instrument orphaned by tooling?
