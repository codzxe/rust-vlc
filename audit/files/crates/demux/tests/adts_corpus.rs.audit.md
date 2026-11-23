# AUDIT: crates/demux/tests/adts_corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 438 / 4 tests + 3 helpers |
| File verdict | REAL-UNPROVEN (genuine ffmpeg differential; cannot execute here — corpus absent, all 4 tests are loud-skip no-ops in this environment) |
| Reachable from shipped entry point? | N/A (integration test). The demuxer under test is reachable: `lumen open` → probe (`AdtsSignature`, signature/mod.rs:138) → containers/mod.rs:52 `"adts" => adts::open`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `target/debug/deps/adts_corpus-1f30c7673d54cbad` → 4 passed in 0.01s (SKIP-noops, corpus absent — verified `/tmp/my-project/corpus-v4/media` does not exist); `LUMEN_CORPUS=/nonexistent target/debug/deps/adts_corpus-1f30c7673d54cbad corpus_adts_lanes` → 1 FAILED (assert "corpus file missing" — proves the env override is honored and the assertions are live); baseline grep: no `corpus(adts):` output lines in audit/runs/baseline/cargo-test-workspace.txt |

## 1. One-paragraph verdict
This is a real, honestly-documented external-oracle differential: ffmpeg-measured truths (packet
counts, full-payload md5 vs `ffmpeg -f data -`, per-frame pts series, exact frame-arithmetic
durations), a dirty-lane divergence policy (ffmpeg's 351-packet short-tail acceptance vs this
demuxer's typed sticky truncation at 350 — declared, not hidden), a seek roundtrip, and a
mutation-sweep fuzz pass. BUT in this sandbox the corpus is absent and every test silently
turns green via the loud-skip path — the baseline's "4 passed" for this binary proves nothing
about ADTS demuxing here. The always-on coverage lives in the in-crate suite (29 tests in
crates/demux/src/adts/tests.rs) and the CLI lane (containers/adts.rs tests), so the format is
not test-free; this file's own evidence value is environment-conditional, and it says so in its
header. No fabrication found; one process-level finding.

## 2. What actually works (verified)
- `LUMEN_CORPUS=/nonexistent … corpus_adts_lanes` → FAILED with the missing-file assert — the
  override is honored, the skip is purely "dir absent", the assertions are live code (exit ≠ 0).
- Skip behavior verified: `target/debug/deps/adts_corpus-1f30c7673d54cbad` → 4 passed, 0.01s,
  corpus dir absent (an md5 differential + seek roundtrips + 4 KiB-stride mutation sweeps over
  two ~180 KB files cannot finish in 0.01s — these were skips).
- Assertion inventory (read): exact md5 parity (line 178-182), exact packet count (183-188),
  `frames_demuxed()` equality (189), clean-EOS + zero-damage (190-200), every-frame pts series
  `i·1024` (205-212), ffprobe spot values within 1 µs (213-225), exact duration nanos
  (226-233), dirty-lane 346-byte-exact prefix md5 at a fixed 130 920-byte cut (361),
  350-vs-351 divergence pinned (362), sticky truncation replay (377-381).

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/adts_corpus.rs:101-125 (pattern at 256-261, 324-337, 394-399)
Code:
```rust
/// The corpus media dir: the `LUMEN_CORPUS` override, else the
/// sandbox default. `None` when neither exists (the loud skip).
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}
```
```rust
#[test]
fn corpus_adts_lanes_match_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(adts): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
```
What is wrong: environment-conditional evidence. In any environment without
`/tmp/my-project/corpus-v4/media` (this sandbox; no `corpus(adts):` output in the orchestrator
baseline log either) all four tests return early and count as "passed" — the baseline's green
includes assertions that never ran. The skip is loud (eprintln, documented header) and the
always-on equivalent exists (29 in-crate adts tests + containers/adts.rs lane tests), so this
is not fraud — but any claim of the form "the ADTS corpus differential passes" is
UNPROVEN in this environment and must cite a corpus-present run. What correct looks like:
register these as `#[ignore = "corpus"]` (they would then show in the 51 ignored, not the
passed count) or gate on the env var at the harness level. Confidence: HIGH — Verification:
ran both the skip path and the override path.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CorpusTruth/TRUTHS | 52-99 | REAL | — | measured constants; aac-lc mapping documented |
| corpus_dir | 103-109 | REAL | — | env override + default; skip source (F-001) |
| open_demuxer / track_base | 111-115, 314-317 | REAL | — | real access stack (`open_via_input`) |
| corpus_adts_lanes_match_the_ffmpeg_truth | 117-254 | REAL-UNPROVEN | S3 | full differential; not executed here (F-001) |
| corpus_adts_seek_roundtrip | 256-312 | REAL-UNPROVEN | — | seek + reanchor + monotonicity; not executed here |
| corpus_dirty_tail_garbage_is_honest | 324-386 | REAL-UNPROVEN | — | divergence policy pinned; not executed here |
| corpus_adts_survive_the_fuzz_harness | 394-438 | REAL-UNPROVEN | — | truncation/flip sweeps + dirty/garbage dirs; not executed here |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 (corpus present) | YES | exact md5/count/pts/duration assertions over external-oracle truths (assertion inventory in §2) |
| all 4 (corpus absent — THIS environment) | NO | early return; green regardless of the demuxer's behavior — F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every pinned value below was measured this session from the corpus-v4 instance" | adts_corpus.rs:10-12 | REAL-UNPROVEN | constants present; no corpus in this sandbox to re-derive them (the 2026-10-24 worklog table is cited, not committed here) |
| "When the corpus is absent … this suite **skips loudly**" | adts_corpus.rs:15-17 | REAL | executed: skip path taken, eprintln present (output not captured by default harness) |
| "the one intentional divergence" (350 vs ffmpeg 351 on the dirty lane) | adts_corpus.rs:28-30 | REAL-UNPROVEN | policy pinned in code at line 362; not executed here |

## 7. Dead / duplicated / unreachable code
None. All helpers used.

## 8. Dependencies & flags
- Uses `common::{md5_hex, open_via_input}` (crates/demux/tests/common/mod.rs — the real access
  stack helper), `demux::adts::{AdtsDemuxer, SAMPLES_PER_BLOCK}`, `demux::adts::fuzz`.
- Env: `LUMEN_CORPUS` (honored — verified). No features.

## 9. Open questions for the next agent
- Did ANY CI or gate run ever execute this suite with the corpus present? The baseline log has
  no `corpus(adts):` rows; if the answer is no, the TRUTHS constants (md5s, 130 920-byte cut,
  350/351) are uncorroboratable in-tree — check the worklog artifact referenced by the header.
- `payload[..130_920]` (line 361) is a hard slice on the dirty lane — on a regenerated corpus
  with different frame sizes this panics (index out of range) instead of failing with a
  message; acceptable for a test, but it makes regeneration failures cryptic.
