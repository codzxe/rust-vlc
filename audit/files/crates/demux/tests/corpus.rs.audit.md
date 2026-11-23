# AUDIT: crates/demux/tests/corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 540 / 8 (3 #[test]] all #[ignore]+env-gated, 5 helpers) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — integration test; and in the DEFAULT test lane all three tests are `#[ignore]`d no-ops (baseline cargo-test-workspace.txt:2202-2204 "ignored"). Reachable only via tools/corpus/run_harness.py:578-594 which sets the LUMEN_CORPUS env trio and runs `cargo test --release --test corpus -- --ignored`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `rg` over tools/corpus/run_harness.py (env wiring at 578-594, unlink of results at 576); `rg` over docs/corpus/report-20261022-*.md (recorded executions: report-20261022-1334.md:6 and -1610.md:6 both show `demux=0`); baseline citation for ignored status. |

## 1. One-paragraph verdict
A harness-activated measurement suite, not a self-contained test: all three
tests are `#[ignore]`d and env-gated, and in the ordinary `cargo test` lane
they print "no-op" and prove nothing. The gating is honest — the activation
command is documented in the module header, the harness
(tools/corpus/run_harness.py) exists, drives exactly that command, and
recorded runs exist (`docs/corpus/report-20261022-1334.md:6` and
`-1610.md:6`: `demux=0`, i.e. the suite executed over a 400+-file corpus with
no panic/contract violation). What it measures it measures well (typed
terminals, per-kind counts, damage, wall time, oracle judgment delegated to
the aggregator). The defects are hygiene: silently dropped TSV rows, an
unrepresentable-PTS→0 conflation, admitted dead field, and a 12-tuple return
hidden behind clippy allows. I did not execute the harness myself (corpus
regen ~1.3 GB + release build = outside budget), so the execution claim rests
on the recorded reports — hence REAL-UNPROVEN.

## 2. What actually works (verified)
- read tools/corpus/run_harness.py:570-594 — the harness sets
  LUMEN_CORPUS/CASES/RESULTS, unlinks the results file first, and runs
  `cargo test --release -p demux --test corpus -- --ignored --nocapture`:
  the env-gate contract in the module docs matches the real driver.
- read docs/corpus/report-20261022-1334.md:6 / -1610.md:6: `suite exit
  codes: probe=0 demux=0 mjpeg=0 pcm=0` — the demux corpus suite ran to
  exit 0 on the 2026-10-22 corpus instances (execution evidence, recorded).
- baseline cargo-test-workspace.txt:2202-2204: the three tests are reported
  ignored with the harness reason string — gating is loud, not silent.

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/corpus.rs:61-65
Code:
```
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 {
            continue;
        }
```
What is wrong: a malformed case-list row (fewer than 6 fields) is silently
dropped from the corpus run — the measured TSV then lacks that file with no
signal, and the "empty handed case list" assert at 254 only fires when ALL
rows are bad. A corrupt row should assert, not vanish.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/tests/corpus.rs:79-87
Code:
```
fn pts_nanos(packet: &Packet) -> i128 {
    match &packet.pts {
        media_types::Stamp::Present(ts) => match ts.as_nanos() {
            Ok(ns) => i128::from(ns.0),
            Err(_) => 0,
        },
```
What is wrong: an unrepresentable (overflow-domain) PTS is recorded as 0,
indistinguishable from an absent PTS — the first/last video PTS columns of
the results TSV then lie about a file with a huge tick count, and the
aggregator's duration judgement is silently skewed. A distinct sentinel (or
the error) should be recorded.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/corpus.rs:316-317
Code:
```
    /// The record class (kept for the results TSV's future columns;
    /// the parser only emits `avi-av` records).
    #[allow(dead_code)]
    class: String,
```
What is wrong: an admitted dead field kept "for future columns" — one of the
121 `#[allow]`s in tree; either emit the column or drop the field.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/tests/corpus.rs:90-106, 258-263
Code:
```
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn walk_one(
    path: &Path,
) -> (
```
What is wrong: a 12-element positional tuple return papered over with clippy
allows, and every run appends a header row to the results TSV
(`OpenOptions::append` + unconditional `writeln!` header at 263) — a manual
re-run against the same LUMEN_CORPUS_RESULTS stacks duplicate header rows
(the harness masks this by unlinking first at run_harness.py:576). A struct
return would make the columns self-describing.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_via_input | 43-47 | REAL | — | real access stack, propagated refusal |
| parse_cases | 59-76 | REAL | S4 | F-001 silent row drops |
| pts_nanos | 79-87 | REAL | S4 | F-002 Err→0 |
| walk_one | 91-238 | REAL | S4 | F-004 tuple; measures terminal/counts/pts/wall |
| corpus_avi_walk_records_every_file | 241-309 | REAL-UNPROVEN | — | harness-run, report-1334/1610 demux=0 |
| parse_manifest_clean_avi_av | 324-365 | REAL | — | brittle std-only JSON scan, documented |
| unquote | 368-378 | REAL | — | no escape handling (manifest is flat) |
| corpus_avi_audio_matrix_recognizes_every_pcm_format | 387-467 | REAL-UNPROVEN | — | real asserts (track+packet per codec) |
| walk_audio_track | 471-484 | REAL | — | |
| corpus_dirty_zero_mid_walks_recover_the_tail_to_oracle_counts | 507-540 | REAL-UNPROVEN | — | pinned to 2026-10-22 instance; loud skip on regen |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_avi_walk_records_every_file | NO (measurement only) | records TSV rows; judgment is the aggregator's — by design |
| corpus_avi_audio_matrix_recognizes_every_pcm_format | YES | asserts every clean avi-av file yields audio track + packets, per codec |
| corpus_dirty_zero_mid... | YES (on the pinned instance) | exact oracle counts + recovered>0 + eos terminal |
| All three in the default lane | NO | #[ignore]d no-ops without the env trio |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this suite fails only on contract violations (panic, missing corpus file, empty case list)" | corpus.rs:12-14 | REAL | asserts at 254-257, 266; no value judgments in-test |
| Activation command | corpus.rs:22-25 | REAL | run_harness.py:593 matches it verbatim |
| W2-07 oracles "the durable judgment stays with the harness's aggregate oracle comparison" | corpus.rs:504-506 | REAL-UNPROVEN | harness + reports exist; aggregate comparison logic not re-executed by me |

## 7. Dead / duplicated / unreachable code
F-003's `class` field. `walk_one`'s `kind_of` closure returns 0 for unknown
kinds — currently unreachable (tracks are only Audio/Video) but harmless.

## 8. Dependencies & flags
Integration test gated on env vars; `#[ignore]` strings name the driver.
No feature gates. Uses input::FileAccess (real open path — good fidelity).

## 9. Open questions for the next agent
- The 2026-10-22 20:02 report ran `--skip-suites` (oracle-only). When did
  the demux suite last run to completion on the CURRENT code? If never since
  late corpus changes, the `demux=0` evidence is stale.
- tools/corpus/run_harness.py timeout is 7200s per suite — has the demux
  suite ever hit it on the 485-file corpus (wall-time budget)?
