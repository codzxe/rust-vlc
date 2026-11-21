# AUDIT: crates/codecs/examples/stage_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 483 / 8 (+9 `#[path]` module includes) |
| File verdict | UNREACHABLE (dev instrument by design — a cargo example, not in any shipped payload; but REAL as a tool: committed measurement records prove execution) |
| Reachable from shipped entry point? | NO — `.github/workflows/release.yml` builds only `-p lumen-cli -p desktop` + the tauri manifest (lines 193/219 and repeats); `rg -n "example" .github/workflows/release.yml` → no hits. Examples are linted (`cargo clippy --workspace --all-targets`, baseline run #5/6 CLEAN) but never shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `sed -n` quote verification (73-78, 130-131, 296-300, 346-350); `rg -n "STAGEBENCH\|stage_bench" docs/ audit/runs/baseline/` → executed-evidence records in docs/worklog/archive-2026-10.md (:8285, :9804, :9886) and docs/perf/algorithm-audit.md (:15-17); `rg -n "example" .github/workflows/release.yml` → none |

## 1. One-paragraph verdict
A well-built MJPEG stage-attribution micro-benchmark: it `#[path]`-includes the live mjpeg sources (so it measures shipped code by construction), validates the staging seam with a byte-equality gate before timing anything, interleaves ablation arms within each pass to spread machine drift, and prints an honest `STAGEBENCH` line with medians, minima, a closure residual, and surrounding loadavg. It has real, committed execution evidence: the worklog records concrete A/B measurements made with it (9.19 → 6.04 ms/frame on corpus lane 0052, archive-2026-10.md:9886). It is not shipped and is not meant to be. The four findings are hygiene-grade: a biased even-N median, swallowed decode errors inside timed arms (mitigated by the pre-timing validation gate), NaN/Inf division on degenerate zero timings, and a frame-splitter edge that cannot see markers in the final three bytes. No fakeness: nothing here pretends to be production code.

## 2. What actually works (verified)
— Executed with real output, on the record: docs/worklog/archive-2026-10.md:9886 ("full decode 9.19 → 6.04 ms/frame (1.52x, −34%); assemble stage 4.4 → 1.3 ms/frame (3.4x)" on corpus lane 0052, medians of 5 passes) and docs/perf/algorithm-audit.md:15-17 (binaries/tools table naming this harness). Committed measurement records, not baseline runs — cited as such.
— Linted clean in baseline runs #5/#6 (`cargo clippy --workspace --all-targets [-D warnings]` exit 0 — this example is an --all-targets target).
— Validation gate read and verified (lines 244-270): staged-full vs production walk byte-equality per frame before any timing — the seam's license; an honest instrument that refuses to print numbers it cannot trust ("seam invalid — do not trust these numbers").

## 3. Findings

### F-001 [S4]
Where: crates/codecs/examples/stage_bench.rs:346-350
Code:
```rust
    let median = |v: &[f64]| {
        let mut s = v.to_vec();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        s[s.len() / 2]
    };
```
What is wrong: for an even sample count this returns the upper-middle element, not the median (the average of the two middle elements). `--passes 4` yields `s[2]` of a sorted 4-element vector — biased high by construction.
Why it matters: every headline number the tool prints is a "median"; with even N all shares and speedups skew. Default 15 (odd) and the worklog's 5-pass runs are unaffected; only user-supplied even N hits it.
What correct looks like: average `s[n/2 - 1]` and `s[n/2]` for even n.
Confidence: HIGH — Verification: read.

### F-002 [S4]
Where: crates/codecs/examples/stage_bench.rs:296-331 (all seven timed arms)
Code:
```rust
        full_v.push(timed_pass(|| {
            for frame in &spans {
                let _ = ctx.decode(frame);
            }
        }));
```
What is wrong: every timed arm swallows decode/assemble errors (`let _ =`). A frame that fails inside a timed arm is silently excluded from that arm's work while the other arms still time it — stage deltas then attribute a missing frame's cost to the wrong stage.
Why it matters: mitigated but not eliminated — the pre-timing validation gate (244-270) runs the production and staged scans with `?` on every frame, so any frame that fails the gated paths aborts the run; only a failure mode unique to a stub arm (or to `ctx.decode` vs `decode_scan`+`assemble`) could desynchronize arms mid-timing. Bench-only tool, so S4.
What correct looks like: count successes per arm and assert equal counts across arms after each pass.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/codecs/examples/stage_bench.rs:378, 416-418
Code:
```rust
    let pct = |x: f64| 100.0 * x / full;
```
and
```rust
        full / scalar,
        100.0 * full / scalar,
```
What is wrong: divisions by `full` and `scalar` with no zero guard. A degenerate measurement (all passes ≈ 0.0 — e.g. a 1-frame tiny fixture on a fast host rounding to 0.0 ms, or a stub-arm anomaly) prints `inf`/`NaN` shares rather than a diagnostic.
Why it matters: garbage numbers with no hint; no crash (f64). Bench-only.
What correct looks like: guard `if full <= 0.0 { return Err("degenerate timing") }` after the pass loop.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/codecs/examples/stage_bench.rs:130-144
Code:
```rust
        while p + 3 < data.len() {
            if data[p] != 0xff {
                p += 1;
                continue;
            }
            let code = data[p + 1];
            match code {
```
What is wrong: the outer marker walk requires 4 readable bytes (`p + 3 < len`), so a marker in the last three bytes of the file — including the canonical trailing `FF D9` EOI — is never seen by this loop; a frame whose only terminator is a final-position EOI is dropped (`end = None` → `pos = start + 2`). In practice the SOS entropy-scan inner loop (151-171) terminates nearly every real frame before that, and its `end = Some(p)` excludes the EOI marker bytes from the span (the decoder tolerates EOI-less frames — proven by the committed 40-frame measurements), so measured corpora are unaffected.
Why it matters: a silent frame-drop class in the harness's splitter for exotic inputs (EOI-terminated frame with no following bytes, no SOS fallback hit); bench-only.
What correct looks like: `while p + 1 < data.len()` with marker-payload reads individually bounds-checked.
Confidence: HIGH — Verification: read + trace; worklog :9886 shows 40/40 frames measured on 0052 (no drops in practice).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-55 | REAL | — | method table verified against code (arms, passes, warm-up, loadavg all match) |
| `#[path]` includes ×9 | 73-105 | REAL | — | live-source inclusion verified; scoped `#[allow(dead_code)]` with documented rationale (lib target still lints — worklog :8284) |
| `use field::FieldPolarity` | 92-93 | REAL | — | re-import for the include tree's `super::` resolution (worklog :9054 E0432 fix) |
| `USAGE` const | 110 | REAL | — | |
| `FrameSpan` | 114-117 | REAL | — | |
| `split_frames` | 119-194 | REAL | S4 | F-004 last-3-bytes blind spot; always advances (no hang); SOS entropy scan terminates frames |
| `loadavg` | 196-200 | REAL | — | /proc read with .ok() — portable degrade to "?" |
| `ScalarIdctStages` + `ScanHooks::transform` | 208-215 | REAL | — | the SIMD-ablation arm; worklog :8285 |
| `timed_pass` | 219-223 | REAL | — | |
| `run_file` | 225-441 | REAL | S4×3 | validation gate honest; F-002 swallowed errors; F-003 zero-division; even-N median F-001; shared-ctx reuse justified by the gate + comment 233-237 |
| `main` | 443-483 | REAL | — | arg parse: --passes ≥ 3 enforced, unknown -- rejected, exit 64/70 |

## 5. Test honesty
No tests in this file (it is a measurement harness; the byte-equality gate at 244-270 is a runtime self-check that would abort the run on divergence — the correct instrument-honesty mechanism). The staged-walk bit-exactness it relies on is pinned by the mjpeg decoder's in-file test (cross-ref: crates/codecs/src/video/mjpeg/decoder.rs, another agent's file — module doc :10-12 claims it; not re-verified here).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the example always measures the exact shipped code" (`#[path]` includes) | stage_bench.rs :44-45 | REAL | includes point at ../src/video/mjpeg/*.rs — the lib's own sources |
| "`--passes N` (default 15) after 2 warm-up passes; per-pass arm times on stderr, one STAGEBENCH summary line per file on stdout" | stage_bench.rs :49-52 | REAL | code 445, 273-277, 332-342, 420-439 |
| "/proc/loadavg sampled around the measured window" | stage_bench.rs :51-52 | REAL | code 287, 344 (sampled before/after the pass window) |
| measurements made with this tool (9.19→6.04 ms/frame etc.) | docs/worklog/archive-2026-10.md:9886, docs/perf/algorithm-audit.md:15 | REAL-UNPROVEN | committed measurement records with per-run caveats (load, host) stated at :9804; not re-executed in this audit (no cargo rule) |

## 7. Dead / duplicated / unreachable code
None within the file — every item is used by the harness. The file as a whole is unshipped (by design). The `split_frames` outer-loop `0xd9`/`0xda`/default arms are near-dead for real MJPEG (the SOS inner loop terminates frames first) but serve header-only frames — not a defect.

## 8. Dependencies & flags
std-only. No cargo features involved. Example target: built only by `--examples`/`--all-targets` invocations (clippy baseline covers it); never in release payloads.

## 9. Open questions for the next agent
- mjpeg/decoder.rs auditor: confirm the in-file bit-exactness test pinning `FullStages` ≡ production `decode` (this harness's attribution license) actually exists and runs in the default test set.
- Whoever owns docs/perf: the even-N median bias (F-001) means any recorded run with even passes has skewed shares — re-check the committed medians' pass counts (worklog used 5, odd — safe).
