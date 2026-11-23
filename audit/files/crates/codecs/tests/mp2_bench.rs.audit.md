# AUDIT: crates/codecs/tests/mp2_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / functions | 72 / 2 (bench, mp2_decode_throughput) + 1 const |
| File verdict | REAL — an honest #[ignore]d measurement lane over the production decode path; executed live here; its printed number is profile-ambiguous |
| Reachable from shipped entry point? | Test file: not shipped. The measured code IS shipped: `decode_stream` (crates/codecs/src/audio/mp2/decoder.rs:126, imported at tests/mp2/mod.rs:14) is dispatched by the shipped pipeline's mp2 arm (pipeline_host; the engine_info() hardcode under-reporting it is the known cross-file S3, W06-A10 nal.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/mp2_bench-a1708d147ec768c5 --ignored --nocapture` -> 1 passed (exit 0): "fixture mp2_48k.mp2: 145152 inter-channel samples, median 4143096 samples/s = 43x realtime (1.512s of audio)" + corpus SKIP observed; rg/sed citation checks; worklog arithmetic cross-check |

## 1. One-paragraph verdict
A real throughput bench that measures what it claims: `decode_s16le` → the
production free function `decode_stream` (mp2/decoder.rs:126 — ID3 skip +
Layer II walk + CBR gate + engine decode), median-of-7 reps, div-by-zero
guarded (`channels.max(1)`, `rate.max(1)`), median index REPS/2 correct for
7 samples. I executed the ignored test live on the prebuilt debug binary:
the always-on fixture row ran (145,152 inter-channel samples = 63 frames ×
1152 × 2 — matches the fixture record and worklog :9023), and the corpus
rows SKIPped loudly exactly as documented. The arithmetic of the doc
comment's corpus facts cross-checks against the recorded worklog numbers
(154 frames × 1152 × 2 ch = 354,816 inter-channel == worklog :9029's figure
for 0119; 56 × 1152 = 64,512 for 0005). Two defects: the printed result
line carries no build profile, and my live debug run (43× realtime) vs the
worklog's release run (118.2 M samples/s ≈ 1231× realtime, :9029) show a
~28× gap that the line format cannot distinguish — the identical
machine-parseable shape serves both; and the corpus rows are measurable
only on a machine holding /tmp/my-project/corpus-v4 (the original sandbox),
so 2 of the 3 rows are dead weight in any fresh checkout.

## 2. What actually works (verified)
— verified by `./target/debug/deps/mp2_bench-a1708d147ec768c5 --ignored
--nocapture` -> 1 passed, exit 0, output pasted above (fixture row measured,
corpus SKIP message printed). Duration math sanity: 145152/2/48000 = 1.512s
matches the printed seconds.

## 3. Findings

F-001 [S3]
Where: crates/codecs/tests/mp2_bench.rs:49-52
Code:
```rust
    eprintln!(
        "{label}: {inter_channel} inter-channel samples, median {median:.0} samples/s \
         = {realtime:.0}x realtime ({seconds:.3}s of audio)"
    );
```
What is wrong: the result line — the entire output of this test — encodes
no build-profile marker. Live proof this session: the same line shape reads
"median 4143096 samples/s = 43x realtime" from the debug binary and
"118.2 M ≈ 1231× realtime" in the release run recorded at worklog
:9029 — a 28.5× difference with nothing in the line to separate them. The
worklog transcription honestly says "(release, ...)", but any future
machine harvest of bench lines (the stated purpose: "the number goes into
the card's worklog entry") can silently mix a debug number into a release
series. Same defect class as huffman_bench F-001 (S3, W06-A03).
What correct looks like: print `std::cfg!(debug_assertions)` or
`profile!(...)` in the line.
Confidence: HIGH — Verification: ran (both numbers produced: mine debug,
worklog's release).

F-002 [S4]
Where: crates/codecs/tests/mp2_bench.rs:59-71 (+ tests/mp2/mod.rs:43)
Code:
```rust
    let Some(corpus) = mp2::corpus_dir() else {
        eprintln!(
            "corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP corpus rows"
        );
        return;
    };
```
What is wrong: 2 of the 3 measured rows depend on a machine-specific
/tmp-resident corpus (default path /tmp/my-project/corpus-v4,
tests/mp2/mod.rs:43) that does not exist in this sandbox and is not in the
repo — observed SKIPping live. The doc comment's per-file frame counts
(154 / 56) are UNVERIFIED directly (corpus absent) though their arithmetic
cross-checks the recorded worklog figures. Blast radius: measurement
coverage only; the SKIP is loud, not a fake pass.
Confidence: HIGH — Verification: ran (SKIP observed).

## 4. Per-function table (file <100 LOC, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `bench` | 25-53 | REAL | S3 | correct median/duration math; profile-blind output (F-001) |
| `mp2_decode_throughput` | 55-72 | REAL | S4 | fixture row always-on (verified live); corpus rows env-gated (F-002) |
| `REPS` | 23 | REAL | - | 7 → median index 3 correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mp2_decode_throughput | Only on decode failure | Measurement test by design (documented); zero asserts — a decode error panics via `unreachable!` (:29/:40), but wrong output or throughput regressions do NOT fail it; correctness is pinned by the sibling mp2_corpus_differential / mp2_fuzz_survival suites, and the #[ignore] message documents the exact invocation |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the whole-stream free surface (`decode_stream`: ID3 skip + Layer II walk + CBR gate + engine decode)" | :6-8 | REAL | decode_s16le → decode_stream (mp2/decoder.rs:126) call chain verified |
| "when absent the corpus rows SKIP with a message (the fixture row is the always-on measurement)" | :11-13 | REAL | executed: SKIP message printed, fixture row measured |
| "an ignored measurement test ... never the normal gate run" | :1-4 | REAL | #[ignore] with exact run command in the message; baseline counts it among the 51 ignored |
| corpus facts "154 frames" / "56 frames" | :9-11 | REAL-UNPROVEN | arithmetic matches worklog :9029 (354,816 / 64,512 inter-channel); files absent — not directly probed |

## 7. Dead / duplicated / unreachable code
None. `#[ignore]` is deliberate and self-documenting.

## 8. Dependencies & flags
Test-only; uses tests/mp2/mod.rs helpers + std::time::Instant. No features.

## 9. Open questions for the next agent
- Will 5.X1's throughput table (worklog :9996 re-run: 0119 105.7M = 1198×,
  0005 99.5M = 6220×) be re-harvested mechanically? If yes, F-001 becomes
  an S2 data-integrity risk for that table.
