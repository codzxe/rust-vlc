# AUDIT: crates/media-probe/tests/corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 180 / 1 test + 4 helpers (open_via_input, input_shim::open, parse_cases, probe_one) |
| File verdict | REAL (executed by me over real media through the real access stack; recorder-only contract as documented) |
| Reachable from shipped entry point? | Test-only file; the code under test (`media_probe::probe_stream` + `ProbeRegistry::with_builtins`) is the shipped probe surface used by the open path |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `env LUMEN_CORPUS=/tmp/w04probe/media LUMEN_CORPUS_CASES=...tsv LUMEN_CORPUS_RESULTS=...tsv ./target/debug/deps/corpus-81fd6f29d1d57bdb --ignored --nocapture corpus_probe_records_every_file` -> 1 passed; results TSV verified (3 ogg files -> format `ogg`, conf 70, bytes = exact file sizes, 30-187 µs); baseline citation cargo-test-workspace.txt:3663-3666 (ignored in default lane) |

## 1. One-paragraph verdict
This is the honest version of a corpus suite: it does not pretend to judge — it records one TSV row per file (verdict, format, confidence, bytes examined, error code, latency) and fails only on contract violations, with the accuracy judgment delegated to the external aggregator, exactly as the module docs state. I executed it for real over three ffmpeg-authored Ogg files through the genuine access stack (`input::FileAccess` → `SourceStream` → `probe_stream`): all three probed as `ogg` (matching truth), confidence 70, `bytes_examined` equal to the exact file sizes, 30–187 µs each — the recorder works end-to-end. The debts: it is `#[ignore]`d and env-trio-gated, so it contributes zero assertions to any default-lane run (baseline: `ignored, randomized-corpus harness`); malformed case rows are silently dropped (only total emptiness is asserted); and the append-mode writer re-emits the header row on a reused results path.

## 2. What actually works (verified)
- Real run (command above): `test result: ok. 1 passed` and the results file contains
  `0090-ogv-video.ogv clean ogg probed ogg 70 16116  187` / `0085-audio-only.ogg ... probed ogg 70 3384  43` / `0047-audio-only.ogg ... probed ogg 70 2406  30` — correct format identification, exact byte counts (16116/3384/2406 are the files' true sizes), real latencies.
- The open path is the real one (input::FileAccess, card 2.A1's seam) — not an in-memory stand-in (verified: `input_shim::open` → `input::FileAccess::new().open(uri)`).
- Baseline behavior confirmed: `test corpus_probe_records_every_file ... ignored, randomized-corpus harness: run via tools/corpus/run_harness.py` (cargo-test-workspace.txt:3666) — no silent green, an explicit ignore.

## 3. Findings

F-001 [S3]
Where: crates/media-probe/tests/corpus.rs:130-142
Code:
```rust
#[test]
#ust_use]  // (display artifact; raw bytes `#[ignore = ...]` — od-verified pattern)
fn corpus_probe_records_every_file() {
    let (Some(corpus), Some(cases_path), Some(results_path)) = (
        std::env::var("LUMEN_CORPUS").ok(),
```
(The raw source at 133 is `#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py"]`.)
What is wrong: the only test in this file never runs in the default lane or the baseline — the entire probe-accuracy evidence stream lives outside the tree (the aggregator's recorded runs). The gate decision is documented and deliberate ("the local gate must not depend on a generated corpus"), and unlike the demux differentials it at least surfaces as `ignored` rather than a silent green skip — but the in-repo safety net for probe identification accuracy is empty: a probe regression that misidentified every file ships green unless a human runs the harness. Confidence: HIGH — Verification: ran (my harness invocation + baseline ignore line).
Confidence: HIGH — Verification: ran

F-002 [S4]
Where: crates/media-probe/tests/corpus.rs:73-88
Code:
```rust
fn parse_cases(text: &str) -> Vec<ProbeCase> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let mut fields = line.split('\t');
        let (Some(rel), Some(truth), Some(class)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
```
What is wrong: malformed case rows are silently dropped; only total emptiness is asserted later (`!cases.is_empty()`). If the harness hands 100 cases and 3 rows are malformed, the run records 97 and stays green — a case-count contract (`assert_eq!(out.len(), expected)`) would catch driver/generator drift. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/media-probe/tests/corpus.rs:149-154
Code:
```rust
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&results_path)
        .must();
    writeln!(out, "rel\tclass\ttruth_format\t...").must();
```
What is wrong: append mode + an unconditional header write means a reused results path accumulates duplicate header rows mid-file (every harness re-run appends a fresh header). Harmless to a fresh-path driver, but nothing enforces the fresh-path assumption the writer depends on. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_via_input | 45-50 | REAL | — | real access stack; verified by my run |
| input_shim::open | 54-63 | REAL | — | FileAccess::new().open(uri) |
| parse_cases | 73-88 | REAL | S4 | F-002 silent row drop |
| probe_one | 93-128 | REAL | — | verdict/format/confidence/bytes/err/elapsed — all exercised by my run |
| corpus_probe_records_every_file | 132-180 | REAL | S3 | recorder-only contract (documented); never runs in-tree (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_probe_records_every_file | Only contract violations (by design) | panics/hangs/missing files fail; misidentification passes (judged externally); recorder verified working by my run |
The asymmetry is documented (module docs lines 12-18) — the name ("records_every_file") does not overpromise.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this test records what the probe says ... and fails only on contract violations" | corpus.rs:12-15 | REAL | my run wrote the TSV; only asserts are empty-cases/missing-file/budget-free |
| "Without the env trio the suite is a no-op (green)" | corpus.rs:27-28 | REAL | baseline: ignored; env-trio else-branch prints and returns |
| Activation command (env trio + `--ignored --nocapture`) | corpus.rs:22-25 | REAL | I ran exactly this shape and it worked |

## 7. Dead / duplicated / unreachable code
None. The input_shim indirection is a documented dev-dependency alias.

## 8. Dependencies & flags
`media_probe` (registry + probe_stream), `input` (dev-dep, FileAccess), `plugin_api`, `media_types`, `testkit`. Env trio gates the run. Binary `corpus-81fd6f29d1d57bdb` newer than source (verified).

## 9. Open questions for the next agent
- The aggregator (tools/corpus/run_harness.py) owns the accuracy judgment — its audit should confirm it actually parses this TSV and fails on misidentification, otherwise the whole chain judges nothing.
- media-probe's own 80 unit tests passed in the baseline (cargo-test-workspace.txt:3579+) — the probe library itself has real in-tree coverage; only the corpus accuracy layer is external.
