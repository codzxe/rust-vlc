# AUDIT: crates/codecs/examples/color_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 1021 / ~28 (incl. frozen `legacy` kernel module) |
| File verdict | REAL (executed; 23.5/24 configs completed, all bit-exact checks true) |
| Reachable from shipped entry point? | NO — cargo example, a dev measurement tool; not compiled into lumen-cli/desktop release binaries (expected for a bench) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `timeout 280 ./target/debug/examples/color_bench audit-w06a08` -> exit 124 (timeout after 2030 output lines; analysis below); sed/rg verification |

## 1. One-paragraph verdict
This is a genuinely honest benchmark harness — better than most: arms interleaved
within each round, 21 timed reps after 2 warm-ups, every raw rep printed as TSV,
medians computed, machine/load header and footer printed by the run, and a
byte-exact equality check of every fixed-point arm against the frozen `legacy`
kernel executed on the bench data itself. I ran the prebuilt debug binary: 92
MEDIAN lines across 23.5 of 24 configs, every check
`legacy=true,current=true,scalar=true` (output saved:
audit/runs/W06-A08.color_bench.out). The defects: the doc's `simd` arm implies a
live SIMD entry that does not exist anywhere (`simd_available()` in color.rs is a
hard-coded `false`), the "DCE guard" checksum is discarded by every caller and
guards nothing, and the run's tail (4K 4:4:4 + decode-share) is unexecuted here.

## 2. What actually works (verified)
— `timeout 280 ./target/debug/examples/color_bench audit-w06a08` -> 2030 lines;
header prints cpu/cores/loadavg/seed/protocol; 92 MEDIAN lines; zero `=false`
checks (`rg "=false" audit/runs/W06-A08.color_bench.out` -> no matches).
Bit-exactness legacy==current==scalar confirmed on gray/420/422/444 at
640x480, 1280x720, and 3840x2160 (except 444/smooth/3840x2160, cut by timeout).

## 3. Findings
F-001 [S3]
Where: crates/codecs/examples/color_bench.rs:36-39; crates/codecs/src/video/mjpeg/color.rs:474-479, 359-364
Code:
//! - `simd` — the live kernel's SIMD entry when the CPU + build offer
//!   it (added with the W6-03 optimization; absent from the baseline
//!   phase binary — recorded here so the delta in binary contents
//!   between phases is explicit, not silent).
...
pub(crate) fn simd_available() -> bool {
    false
}
What is wrong: the bench doc sells a `simd` arm measuring "the live kernel's
SIMD entry when the CPU + build offer it", but `simd_available()` returns a
constant `false` and `compose_row_simd` aliases `compose_row` ("until the SIMD
kernel lands"). No build, no CPU, ever arms it. My run confirms: no `simd`
MEDIAN line exists in 92 lines of output. Why it matters: a reader of the bench
doc believes a SIMD kernel exists behind CPU detection; the "W6-03
optimization" that landed is the bench itself (worklog archive-2026-10.md:8128
calls commit 0b18a10 "W6-03 color-pipeline bench", not an optimization).
What correct looks like: doc says plainly "simd arm reserved; no SIMD kernel
exists yet; it never runs in any current binary".
Confidence: HIGH — Verification: ran (no simd lines in output) + read.

F-002 [S4]
Where: crates/codecs/examples/color_bench.rs:496-511 (checksum) and 692-726 (callers)
Code:
fn compose_frame<C: RowComposer>(composer: &C, w: usize, h: usize, out: &mut [u8]) -> u64 {
    ...
    let mut sum = 0u64;
    for (i, byte) in out.iter().enumerate().step_by(4097) {
        sum = sum.wrapping_add(u64::from(*byte).wrapping_mul(i as u64 & 0xff));
    }
    sum
}
What is wrong: the doc on this fn claims "Returns a checksum over the output
(DCE guard + cross-arm equality evidence)" but every caller (the `Arm` closures
at 692-726, `compose_frame_scalar`, `compose_frame_simd`) discards the u64. A
guard whose result is never consumed cannot prevent dead-code elimination; the
real anti-DCE evidence is the `scratch == reference` byte comparison (which is
fine). The checksum loop is dead code that a release build will delete.
Confidence: HIGH — Verification: read (all call sites).

F-003 [S4]
Where: crates/codecs/examples/color_bench.rs:930-936, 941-944
Code:
        let image = match codecs::video::mjpeg::decode_image(&jpeg) {
            Ok(image) => image,
            Err(e) => unreachable!("dense 4K {tag} frame decodes: {e}"),
        };
What is wrong: a measurement tool that aborts by panic on fixture failure
instead of printing a typed failure line; three `unreachable!`s in decode_share.
Acceptable in an example, but it converts "fixture regressed" into a crash with
no partial results. What correct looks like: eprintln + early return.
Confidence: HIGH — Verification: read. (Never triggered in my run — decode_share
was never reached; see F-004.)

F-004 [S4]
Where: crates/codecs/examples/color_bench.rs:927-975, 980-1021
What is wrong: UNVERIFIED by execution: the debug binary needs >280 s on this
2-core sandbox for the full matrix, so `decode_share` (the whole-decode share
measurement, incl. `dense_4k_jpeg` construction and the `unreachable!` guards)
never ran. The JPEG-construction code is read-verified structurally correct
(SOF height-then-width order correct, SOS Ss=0/Se=63, 129 bits/block matches the
2-code DHTs), but the claim "dense 4K frame decodes" has no run behind it in
this audit.
Confidence: HIGH that it is unexecuted here — Verification: UNVERIFIED
(reason: cargo-build-budget — a release rebuild of the codecs example exceeds
the sandbox budget; timeout 280 on prebuilt debug binary).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| legacy::ycc_to_rgb / shr16_round | 75-96 | REAL | — | Q16 full-range BT.601, round-half-away |
| legacy::RowSampler::{new,sample,step} | 142-179 | REAL | — | exact incremental floor(x·h/h_max), checked arithmetic |
| legacy::RgbComposer::{new,compose_row,...} | 197-327 | REAL | — | hot/cold split, cold path checked indexing |
| mod now (#[path] include) | 330-333 | REAL | — | benches the actual shipped color.rs source — good design |
| RowComposer trait + impls | 337-353 | REAL | — | name-split avoids dyn shadowing |
| Fill/Sampling enums | 356-397 | REAL | — | |
| Rng (xorshift64*) | 414-425 | REAL | — | seed fixed + printed |
| build_planes | 430-494 | REAL | — | MCU-padded geometry mirrors decoder |
| compose_frame | 500-511 | REAL | S4 | checksum dead (F-002) |
| FloatComposer::row | 530-600 | REAL | — | structurally matched, odd-width tail handled |
| median | 609-612 | REAL | — | all uses odd-length (21, 15) |
| print_machine_header/footer | 614-650 | REAL | — | /proc Linux-only, "unknown" fallback |
| main | 661-784 | REAL | — | verified by run: interleaving, warmup, checks, TSV |
| build_legacy/build_now/build_float | 788-887 | REAL | — | duplication is the #[path] price, documented |
| compose_frame_scalar/simd | 891-914 | REAL | S3 | simd variant can never differ from scalar (F-001) |
| checksum | 916-922 | REAL | S4 | dead result |
| decode_share | 927-975 | REAL-UNPROVEN | — | never executed here (F-004) |
| dense_4k_jpeg | 980-1021 | REAL-UNPROVEN | — | structurally correct; decode success unexecuted |

## 5. Test honesty
No tests in this file (it is a measurement program). Its self-validation
(bit-exact equality lines) is genuine and ran (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "medians, N≥15 recorded per-rep raw data" | L4-5 | REAL | run output: 21 REP lines per arm/config |
| "2 untimed warm-up rounds, then 21 timed reps, arms interleaved" | L46-48 | REAL | code 744-753 + run |
| "one `RgbComposer` built per frame" | L10-11 | REAL | composer built inside each rep closure (692-701) |
| "`simd` — the live kernel's SIMD entry when the CPU + build offer it" | L36-37 | FALSE | simd_available() == false const; no simd lines in run (F-001) |
| "checksum ... (DCE guard ...)" | L498-499 | FALSE | result discarded everywhere (F-002) |
| "legacy — kept verbatim ... @ merge commit 62c48b6" | L27-28 | UNVERIFIED | would need git history; git is forbidden to agents |

## 7. Dead / duplicated / unreachable code
`compose_frame_simd` + the `simd` arm construction (711-719) — dead in every
build until a real SIMD kernel exists. `checksum` — dead result. The file is a
dev tool; unreachable from shipped binaries by design.

## 8. Dependencies & flags
None beyond std + the `#[path]`-included `color.rs` + `codecs::video::mjpeg`
public API. No features.

## 9. Open questions for the next agent
- Whoever audits `crates/codecs/src/video/mjpeg/color.rs`: the W6-03 seams
  (`compose_row_scalar/simd`, `simd_available`) carry `#[allow(dead_code)]`
  "baseline commit only" comments — is the W6-03 rewrite still pending, or was
  it abandoned with only the bench landing?
- The 5.X1b specialized direct-row loops in `compose_color_row` are the only
  real current-vs-legacy delta; a release-mode run would quantify it (budget
  blocked here).
