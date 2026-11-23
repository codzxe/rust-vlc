# AUDIT: crates/codecs/tests/pcm_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 217 / 9 (DroppingSink, configured, packet_for, bench, stats, report, fixture_material, 2 tests) |
| File verdict | REAL bench harness (executed: both probes ran, floors held) — `#[ignore]`-by-design, so the release floors are never enforced automatically |
| Reachable from shipped entry point? | N/A (integration test, explicitly `#[ignore]`d with run instructions `cargo test -p codecs --release --ignored`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/pcm_bench-f7a83684f4b66a2b --ignored --nocapture` -> 2 passed / 0 failed, 0.08s (medians: s16le passthrough 68 879.3 MB/s; s16be converted 85.8 MB/s, debug build) |

## 1. One-paragraph verdict
A clean, honest measurement harness: both probes run through the real contract path (PcmDecoder::configure + per-packet decode + FrameSink) over the committed AVI fixture's own PCM chunks (60 × 5880 B, count and size asserted before timing), packets pre-built outside the timing loops, one warm-up pass, 15 timed reps, median + interpolated p95, raw per-pass numbers on stderr, machine-parseable BENCH line on stdout, and explicit floors that differ debug/release via `cfg!(debug_assertions)`. I ran both ignored tests on the prebuilt debug binary — they execute and pass (68.9 GB/s passthrough — plausible for a true zero-copy Arc handoff; 85.8 MB/s byte-swap in debug). Two hygiene issues: the debug floor for the passthrough (50 MB/s) is three orders of magnitude below observed reality, so that arm can never fire; and because both tests are `#[ignore]`d, the RELEASE floors (500/250 MB/s) are enforced by nothing in CI — a regression would only surface if a human re-runs the bench and reads the worklog.

## 2. What actually works (verified)
— `./target/debug/deps/pcm_bench-f7a83684f4b66a2b --ignored --nocapture` -> **2/2 pass**, output: `[pcm-s16le-passthrough] raw per-pass MB/s (N=15): 63282.5 … 68906.2`, `BENCH pcm_decode_s16le_passthrough: median=68879.3 MB/s p95=69022.2`; `[pcm-s16be-converted] … median=85.8 MB/s p95=86.2`. Frame-count invariants held (sink.frames == 60×16 asserted inside bench()).
— Honest protocol details verified by read: warm-up excluded from timing (L96-102), packets cloned outside timing only per-pass iteration (Arc bump), s16be twin built by real byte-swapping of the same fixture chunks (L197-206), stats median/p95 with linear interpolation (L134-144).

## 3. Findings
F-001 [S4]
Where: pcm_bench.rs:184 and 212
Code:
    let floor = if cfg!(debug_assertions) { 50.0 } else { 500.0 };
    assert!(
        median >= floor,
        "{median:.1} MB/s < {floor:.1} MB/s floor (N={REPS})"
What is wrong: the debug floor for the passthrough probe (50 MB/s) is ~1375× below the observed debug median (68 879 MB/s) — that assertion arm is unfirable and gives false confidence ("a floor is checked"). The s16be debug floor (25 vs 85.8 observed) is at least within an order of magnitude. What correct looks like: drop the debug floor or set it where a real regression (e.g. accidental copy in the passthrough) would actually cross it — an accidental per-byte copy in debug would still exceed 50 MB/s. Confidence: HIGH — Verification: ran (observed medians).
F-002 [S4]
Where: pcm_bench.rs:174-176 (and 191-193)
Code:
#[test]
#[ignore = "W5-19 throughput measurement: cargo test -p codecs --release --ignored (numbers in the W5-19 worklog entry)"]
fn pcm_decode_throughput_s16le_passthrough() {
What is wrong: the release floors (500 / 250 MB/s) are never evaluated by any automated lane — ignored tests do not run in `cargo test` (they are among the baseline's 51 ignored), and no CI step shown in the baseline runs `--release --ignored`. The floors exist only as documentation of the W5-19 worklog numbers. Inherent to the ignore-bench pattern, but the release claim is enforcement-free. Confidence: HIGH — Verification: read + baseline cargo-test summary (51 ignored, these tests not run).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DroppingSink | 44-53 | REAL | — | counts frames, asserted |
| configured / packet_for | 55-88 | REAL | — | contract-path configure with real strf |
| bench | 92-130 | REAL | — | warm-up + REPS timed passes; frame-count invariant |
| stats | 134-144 | REAL | — | median/p95 interpolation, NaN-safe cmp |
| report | 146-154 | REAL | — | machine-parseable BENCH line |
| fixture_material | 156-172 | REAL | — | committed AVI, chunk count/size asserted |
| pcm_decode_throughput_s16le_passthrough | 174-189 | REAL (ran, ignored-by-design) | S4/F-001, F-002 | floor arms nearly vacuous in debug; release floor unenforced |
| pcm_decode_throughput_s16be_converted | 191-217 | REAL (ran, ignored-by-design) | S4/F-002 | honest byte-swapped twin |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pcm_decode_throughput_s16le_passthrough | PARTIAL | frame-count + ≥50 MB/s debug floor would catch catastrophic breakage (wrong frames); a 10× throughput regression passes everywhere (F-001/F-002) |
| pcm_decode_throughput_s16be_converted | PARTIAL | same; debug floor 25 vs observed 85.8 is closer but still regression-tolerant |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "run explicitly with `cargo test -p codecs --release --ignored`; the committed numbers live in the W5-19 worklog entry" | pcm_bench.rs:2-4 | REAL | ignore reason is accurate; I ran the debug variant and got real numbers |
| "Debug builds assert only a sanity floor — the measurements are release-profile claims" | pcm_bench.rs:23-24 | REAL | honest scoping statement matching the code |
| "the passthrough must beat a plain memcpy … in release, ≥ 500 MB/s" | pcm_bench.rs:182-184 | REAL-UNPROVEN | release floor not executed here (no release bench binary; cargo-build-budget); debug observed 68 879 MB/s makes the release claim plausible |

## 7. Dead / duplicated / unreachable code
None. Both tests reachable via `--ignored` (verified by execution).

## 8. Dependencies & flags
codec-api, codecs pcm, media-types; `common` helpers (extract_avi_audio_strf/extract_avi_pcm_chunks shared with the pcm fixtures suite). No features. No unsafe.

## 9. Open questions for the next agent
- Whether any CI lane (scripts/full_gate.sh or a workflow) ever runs `--release --ignored` benches — if not, the 500/250 MB/s floors are dead assertions (F-002); the full_gate baseline FAIL suggests benches are not the gap, but confirm.
