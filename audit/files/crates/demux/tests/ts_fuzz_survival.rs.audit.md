# AUDIT: crates/demux/tests/ts_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 239 / 6 (Lcg×2 methods, base_stream, survive + 3 tests) |
| File verdict | REAL (2 of 3 lanes) — the corpus lane is VACUOUS in every recorded run |
| Reachable from shipped entry point? | N/A (integration test); exercises `demux::ts::TsDemuxer`, CLI-wired via containers/mod.rs `("mpegts") => ts::open(stream, opts)` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p demux --test ts_fuzz_survival` -> exit 0, 3 passed in 0.04s; corpus existence check: `/tmp/my-project/corpus-v4/media/0091-bframe-storm.ts` MISSING, `LUMEN_CORPUS` NOT set (checked without printing values) — corpus lane skipped vacuously in BOTH my run and baseline cargo-test-workspace.txt; sed quote verification |

## 1. One-paragraph verdict
Two of the three lanes are genuine fuzz-lite: 512 seeded rounds × 1-8 mutations (byte flips, truncations, garbage splices, and wild 5-byte packet-header randomization) over a real synthetic multi-program TS (PAT/PMT/SDT/CAT + PES payloads, built with the shared byte-level builders), each round double-run for determinism, with a pristine-stream sanity check (0 PSI CRC failures, PES > 0) and an `outcomes_ok > 256` guard that proves the mutations actually exercise walkable paths rather than a demuxer that refuses everything. The truncation sweep cuts at every 61st byte. The third lane — the file's headline "real ffmpeg-authored TS" differential — has NEVER RUN: the corpus path is absent and the env var unset, so `corpus_head_mutations_survive` early-returns through a loud-but-invisible eprintln and counts as PASSED in the baseline and in my run. That is environment-gated evidence accounting as a pass. A mutation-operator comment also claims a header/PSI bias the code does not implement.

## 2. What actually works (verified)
— verified by `cargo test -p demux --test ts_fuzz_survival` -> `test result: ok. 3 passed; 0 failed` (my run, exit 0, 0.04s).
— verified by read: base_stream builds a real 2-program TS with correct PSI section shapes (PAT→2 PMTs, SDT service names, CAT, PCR-bearing PES packets); the determinism assertions compare full `TsStats` equality and typed error codes.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/ts_fuzz_survival.rs:198-209
Code:
```
    let dir = std::path::PathBuf::from(
        std::env::var("LUMEN_CORPUS")
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string()),
    );
    let path = dir.join("0091-bframe-storm.ts");
    if !path.exists() {
        eprintln!(
            "SKIP (loud): corpus head absent at {} — fuzz lane not run",
            path.display()
        );
        return;
    }
```
What is wrong: The only real-corpus fuzz lane silently converts to a vacuous PASS when the corpus is absent — which it is, in this sandbox and in the recorded baseline. Why it matters: the header (lines 7-9) sells "the first 64 KiB of a real ffmpeg-authored TS, mutated the same way" as part of the suite's coverage; no recorded run has ever executed it. A regression that only manifests on real-world TS bytes (TEI/PUSI/PID mutations of ffmpeg output, line 224) is invisible to every green run. What correct looks like: `#[ignore]` with a documented filter, or a harness that counts and fails on zero corpus lanes, or commit a 64 KiB corpus head as a fixture. Confidence: HIGH — Verification: ran + file-existence check (corpus MISSING, env unset). The lane's body is UNVERIFIED — never executed anywhere I can find.

F-002 [S4]
Where: crates/demux/tests/ts_fuzz_survival.rs:111-114
Code:
```
                0 => {
                    // Flip a byte (biased into packet headers/PSI).
                    let at = rng.below(bytes.len());
                    bytes[at] ^= (rng.next() as u8) | 0x01;
                }
```
What is wrong: The comment claims the flip is "biased into packet headers/PSI"; the code picks `at` uniformly over the whole stream — no bias exists. (Header coverage does exist via mutation arm 3, so the coverage claim is salvaged by a different arm, but this comment is false as written.) What correct looks like: fix the comment or implement the bias. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/ts_fuzz_survival.rs:177-191
Code:
```
    for cut in (0..base.len()).step_by(61) {
        let outcome = survive(&base[..cut]);
        if cut < 188 {
```
What is wrong: For cuts ≥ 188 the outcome's error code is never inspected — only the implicit no-panic/no-hang property is tested there; the typed-code sanity check runs only below one packet. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/tests/ts_fuzz_survival.rs:87-97
Code:
```
    loop {
        if matches!(demuxer.next_event()?, DemuxEvent::EndOfStream) {
            break;
        }
    }
```
What is wrong: No event budget in the drain loop (contrast ivf/fuzz.rs's `FUZZ_EVENT_BUDGET`) — a demuxer regression that loops forever on mutated input would hang the suite (CI timeout) instead of failing fast. Inherent fuzz-lite limitation; the 0.04s runtime shows no current pathology. Confidence: HIGH — Verification: read + ran (no hang observed).

## 4. Per-function table (file >100 LOC — all functions)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg::next / below | 21-36 | REAL | S4 | biased modulo; deterministic |
| base_stream | 41-84 | REAL | — | real 2-program TS: PAT/2×PMT/SDT/CAT, 24 PES packets, one PCR |
| survive | 87-99 | REAL | S4 | typed outcome; no event budget (F-004) |
| mutations_never_panic_and_stay_deterministic | 99-170 | REAL | — | 512 rounds × 4 mutation classes; determinism double-run; pristine sanity; outcomes_ok > 256 coverage guard |
| truncation_sweep_over_packet_boundaries | 172-192 | REAL | S4 | step 61 sweep; code check only below 188 (F-003) |
| corpus_head_mutations_survive | 196-239 | REAL-UNPROVEN | S3 | never executed in any recorded run (F-001); body logic is sound by read |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutations_never_panic_and_stay_deterministic | YES | any panic propagates; nondeterminism fails the TsStats equality; a refuse-everything regression fails outcomes_ok > 256; pristine-stream sanity fails on PSI CRC or zero PES |
| truncation_sweep_over_packet_boundaries | PARTIALLY | no-panic/no-hang for all cuts; typed-code check only below 188 (F-003) |
| corpus_head_mutations_survive | NO (in this environment) | early-returns vacuously — F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every round must produce a typed outcome, never a panic, never a hang" | header lines 3-5 | REAL (synthetic lane) | 512 rounds pass, 0.04s |
| "The corpus lanes ... get a second lane when present: the first 64 KiB of a real ffmpeg-authored TS" | header lines 7-9 | REAL-UNPROVEN | lane never ran: corpus path missing + env unset (my check + baseline) |

## 7. Dead / duplicated / unreachable code
The corpus lane's body (lines 210-238) is dead in every recorded execution environment.

## 8. Dependencies & flags
`demux::ts::test_support` builders (pat_body/pmt_body/psi_packets/ts_packet/stream_of) — real byte-level PSI/PES construction, not mocks. Reads `LUMEN_CORPUS` env var (not printed here); no feature gates.

## 9. Open questions for the next agent
- Who owns the corpus-v4 asset set? If 0091-bframe-storm.ts is available anywhere in the repo/CI, wire it as a committed 64 KiB fixture head so this lane actually runs.
- ts/mod.rs owner: does the production walk have its own event/loop budget (making F-004 moot)?
