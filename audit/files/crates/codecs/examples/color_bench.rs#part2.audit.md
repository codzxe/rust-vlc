# AUDIT: crates/codecs/examples/color_bench.rs#part2 (lines 511-1021)
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 511 of 1022 (chunk part2; boundary: `compose_frame` body :496-511 closes part1/A03 — I audited its call sites; part2 = `FloatComposer` :513 → `dense_4k_jpeg` end :1021) / 16 items |
| File verdict | REAL (harness executed; committed raw output `docs/perf/raw/w6-03-color-bench-baseline.txt` matches the protocol exactly) |
| Reachable from shipped entry point? | NO — a `cargo run --example` dev artifact; never in `lumen`/`lumen-tauri` payloads (by design). Compiled by baseline `cargo test`/clippy `--all-targets` (INDEX.md #3-#6). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg "color_bench" docs/ (committed output found + "re-run owed" debt admissions); wc/rg -c on the raw output (2016 REP / 96 MEDIAN / 2 SHARE lines — arithmetically consistent with 24 configs × 4 arms × 21 reps); rg "simd_available\|compose_row_simd" color.rs (:477-479 const false, :360-362 scalar forward); read of part1 context (:280-399) for the RowComposer/legacy seam; independent re-derivation of dense_4k_jpeg's 129-bits/block and buffer sufficiency |

## 1. One-paragraph verdict
Part2 is the benchmark driver: arms, interleaved timing protocol, machine header/footer, the f32 reference composer, and the whole-decode "share" context. The protocol is honest as designed — 2 warm-ups + 21 timed reps, arms interleaved per round, raw per-rep TSV, medians, loadavg captured at both ends, bit-exactness of every fixed-point arm against the frozen legacy kernel asserted on the bench data itself — and the committed run `docs/perf/raw/w6-03-color-bench-baseline.txt` (phase `baseline-2`) is exactly what the code prints: 2016 REP lines = 24 configs × 4 arms × 21 reps, 96 MEDIAN lines all carrying `legacy=true,current=true,scalar=true`, 2 SHARE lines. The defect cluster: the harness exists to measure a W6-03 before/after optimization whose "after" arm (`simd`) is structurally dead — the live `color.rs::simd_available()` returns constant `false` and `compose_row_simd` forwards to scalar — so the A/B experiment has only ever recorded its A leg, a debt the worklog itself admits. Around that: a panic-on-empty path in `decode_share`'s median, a checksum "DCE guard" that is actually discarded dead code, and a share metric that divides compose-of-smooth by decode-of-dense content.

## 2. What actually works (verified)
— Committed execution: `docs/perf/raw/w6-03-color-bench-baseline.txt` — header (`cpu=… cores=2`, `loadavg_start="1.27 0.90 0.45"`, `seed=0x5eed000000000603 reps=21 warmup=2 interleaved`), 2016 REP + 96 MEDIAN + `SHARE … 422 … share_pct=16.3` / `444 … 9.4`, footer `loadavg_end="2.95 1.56 0.74"`. Counts re-derived: 4 samplings × 2 fills × 3 sizes × 4 arms = 96 MEDIAN; × 21 reps = 2016 REP. Consistent.
— Bit-exactness evidence is real: every MEDIAN line's `bitexact_vs_legacy` column shows `legacy=true,current=true,scalar=true` — the live kernel reproduces the frozen pre-W6-03 kernel byte-for-byte on all 24 configurations.
— Auditor re-derivation: `dense_4k_jpeg`'s entropy budget is sufficient — per block DC(1+2 bits) + 63×AC(1+1) = 129 bits; `blocks*129/8 + 1` bytes gives ≥ blocks·129+1 bits (floor+1 slack covers the remainder); SOF height-before-width, SOS Ss=0/Se=63, DQT/DHT layouts all conform to the JPEG segment grammar the decoder accepts (proven by the run itself: `decode_image` succeeded or `unreachable!` at :932 would have killed it).

## 3. Findings

### F-001 [S3] The harness's A/B purpose is half-fulfilled: the `simd` arm is structurally dead and the "after" phase was never run
Where: crates/codecs/examples/color_bench.rs:704-719 (arm construction), 36-39 (doc); cross-file color.rs:356-362, 474-479
Code:
```
                if now::simd_available() {
                    arms.push(Arm {
                        name: "simd",
```
Cross-file (crates/codecs/src/video/mjpeg/color.rs:474-479):
```
/// Whether the SIMD compose entry is available in this build on this
/// CPU (the W6-03 bench seam; `false` until the kernel lands).
#[allow(dead_code)] // baseline commit only: unused until the W6-03 rewrite arms it
pub(crate) fn simd_available() -> bool {
    false
}
```
What is wrong: `simd_available()` is a constant `false` in the live kernel the example `#[path]`-includes, and `compose_row_simd` (:360-362) forwards to the scalar `compose_row`. The `simd` arm can never be armed; the committed output confirms it (96 MEDIANs = 4 arms, checks column lists only legacy/current/scalar). The header doc (:24-39) sells five arms including "simd — the live kernel's SIMD entry when the CPU + build offer it (added with the W6-03 optimization; absent from the baseline phase binary)". The W6-03 optimization never landed: `compose_frame_simd` (:905-914) is unreachable dead code in every run this tree can produce, and the worklog admits the follow-up run is owed ("W5-14 color_bench re-run owed at merge", docs/worklog/archive-2026-10.md:8191; "should be re-run at merge", docs/perf/v1-input-readpath-20261024.md:114).
Why it matters: The file's stated purpose is "before vs after, under one identical protocol" (:2-4). Only "before" (baseline-2) exists. Any future reader citing this harness as evidence of the W6-03 color-pipeline optimization is citing an experiment that has not happened. The disclosure is present but scattered across three files; the harness itself runs to completion looking whole.
What correct looks like: Land the SIMD kernel or delete the `simd` arm + `compose_frame_simd` + the color.rs dead seams; if kept, print an explicit `# simd_arm=absent (kernel not landed)` line so the output self-describes its missing leg.
Confidence: HIGH — Verification: read both files; committed output counted; worklog greps.
### F-002 [S4] `decode_share` can panic on an empty median, and its per-rep guard silently drops samples
Where: crates/codecs/examples/color_bench.rs:946-950, 609-612
Code:
```
            let elapsed = start.elapsed().as_nanos();
            if rep >= 2 && decoded.data.len() == image.data.len() {
                times.push(elapsed);
            }
        }
        let full = median(&mut times);
```
What is wrong: The `data.len()` equality guard drops a rep from timing WITHOUT failing or logging — and if it (or any decode error path) dropped all 17 reps, `median` (`samples[samples.len() / 2]`, :609-612) indexes an empty slice and panics. The guard itself is decoration: it compares only lengths, never contents, so it cannot detect the corruption it implies it is guarding against.
Why it matters: Bench tool, deterministic decoder — practically unreachable; but a silent sample drop skews a median with no trace in the raw output (violating the file's own "honest-numbers" mandate), and the empty case converts a data problem into a panic.
What correct looks like: `assert_eq!(decoded.data.len(), image.data.len())` (loud), or count-and-report dropped reps in the SHARE line.
Confidence: HIGH — Verification: read; no execution path exercised (example never run in baseline — see §2 provenance note).
### F-003 [S4] The checksum "DCE guard" is discarded dead code; the doc misattributes the equality evidence
Where: crates/codecs/examples/color_bench.rs:690-726 (closures drop the return), 916-922 (`checksum`), 496-499 (doc, part1 boundary)
Code:
```
                    run: Box::new(|out: &mut [u8]| {
                        let composer = build_now(&planes, w, h);
                        compose_frame_scalar(&composer, w, h, out);
                    }),
```
```
fn checksum(out: &[u8]) -> u64 {
    let mut sum = 0u64;
    for (i, byte) in out.iter().enumerate().step_by(4097) {
```
What is wrong: `compose_frame`/`compose_frame_scalar`/`compose_frame_simd` return a checksum documented as "DCE guard + cross-arm equality evidence" (:498-499), but every timed closure discards it. The real anti-DCE is the `out` writes (observable side effect); the real equality evidence is the `scratch == reference` full comparison at :737-739. The checksum loops are pure arithmetic whose results are unused — LLVM may remove them entirely (which is harmless for timing purity, but then the "guard" guards nothing and never did).
Why it matters: Doc claims a mechanism that is not in effect; a future editor trusting the comment could remove the `scratch == reference` comparison believing the checksum covers it.
What correct looks like: Delete `checksum` and the return values, or fold the checksum into the printed output; fix the :498 comment to name `scratch == reference` as the evidence.
Confidence: HIGH — Verification: read all call sites (690-726, 963); no other consumer (rg `checksum(` — :506-510 in part1, :902, :913, :916-922 only).
### F-004 [S4] SHARE metric divides compose-of-smooth-planes by decode-of-dense-JPEG
Where: crates/codecs/examples/color_bench.rs:928-957
Code:
```
        let jpeg = dense_4k_jpeg(h_sampling, v_sampling);
        ...
        let planes = build_planes(3840, 2160, sampling, Fill::Smooth);
```
What is wrong: The denominator times a full decode of a synthetic dense-AC JPEG (worst-case entropy content); the numerator times the isolated compose of SMOOTH gradient planes (cache-friendly locality). The two legs of the share have different content statistics, so `share_pct` is not the compose share of the decode shown — it is an approximation the output line does not disclose.
Why it matters: The committed SHARE numbers (16.3% / 9.4%) will be quoted; their numerator/denominator content mismatch is invisible in the raw line. Context metric only — the doc does call it "context" — hence S4.
What correct looks like: Compose the planes harvested from the actual decoded frame, or print `content=smooth-vs-dense` in the SHARE line.
Confidence: HIGH — Verification: read; both constructors traced (`dense_4k_jpeg` :980-1021 vs `build_planes(..., Fill::Smooth)` :957).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FloatComposer` (struct) | 515-522 | REAL | — | Measurement-only arm; carries (data, stride) pairs. |
| `impl RowComposer for FloatComposer::row` | 524-601 | REAL | — | Gray path, h_max==2 fused pairs + odd tail, per-pixel else branch; nearest-left chroma matches legacy `sample_up` semantics (x/2); bounds safe on bench planes (y/c rows re-derived: row_y/v_max ≤ c_rows−1); f32 rounding exempt from bit-exactness by design (:513-514, :734-736). |
| `FloatComposer::c_stride` | 603-607 | REAL | — | `map_or(0, …)` — gray → 0; cb_row unused in gray path. |
| `median` | 609-612 | REAL | S4 | Upper-middle element for even lengths (all in-tree callers pass odd counts); panics on empty (F-002). |
| `print_machine_header` | 614-640 | REAL | — | /proc-only, honest "unknown" fallback off-Linux; seed line matches Rng const :443. |
| `print_load_footer` | 642-650 | REAL | — | Symmetric loadavg capture; present in committed output. |
| `ArmRun` / `Arm` | 652-659 | REAL | — | Type alias + struct; no issues. |
| `main` | 661-784 | REAL | S3 | Protocol implemented exactly as documented (2+21, interleaved, warm-ups excluded at :750, REP before MEDIAN, per-arm checks string); `arms[0]`-is-legacy assumption at :731 fragile but correct by construction order; `simd` arm dead (F-001). |
| `build_legacy` | 786-827 | REAL | — | Geometry re-derived: luma geom (h_max,v_max,w,h), chroma (1,1,⌈w/h_max⌉,⌈h/v_max⌉) — matches decoder plane math. |
| `build_now` | 829-868 | REAL | — | Verbatim twin of build_legacy against the `#[path]`-included live kernel — the duplication is documented (:786-787). |
| `build_float` | 870-887 | REAL | — | Gray → (None, None); consistent with FloatComposer contract. |
| `compose_frame_scalar` | 889-903 | REAL | — | Pins scalar path; row loop matches `assemble_rgba_into` shape; checksum return dropped (F-003). |
| `compose_frame_simd` | 905-914 | REAL-UNPROVEN | S3 | Dead in every producible run (simd_available() ≡ false); forwards to scalar dispatch via color.rs:360-362 (F-001). |
| `checksum` | 916-922 | REAL | S4 | Result unused everywhere → dead; doc misattribution (F-003). |
| `decode_share` | 927-975 | REAL | S4 | `unreachable!` on decode failure = loud bench failure (acceptable); silent rep drop + empty-median panic (F-002); content mismatch (F-004). |
| `dense_4k_jpeg` | 980-1021 | REAL | — | Segment grammar + 129-bit/block budget re-derived and sufficient; duplicate-symbol Huffman tables (two 1-bit codes → same symbol) are wasteful-but-legal and proven decodable by the committed run. |

## 5. Test honesty
No tests in this file (an example, not a test target). The honesty artifacts are the run protocol itself:
| Artifact | Would it catch a broken kernel? | Reason |
|---|---|---|
| `bitexact_vs_legacy` per MEDIAN line | YES | Full-frame byte equality vs the frozen pre-optimization kernel, on both fills — any semantics change in color.rs flips `current=true` to `false` in the committed protocol. |
| REP raw rows (21/arm) | YES | Manual re-median possible; committed file has all 2016 rows. |
| SHARE lines | WEAK | Context only; content mismatch (F-004) and no cross-check between the two legs. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the pre-W6-03 kernel, kept verbatim … Frozen; never edited after this file lands" | color_bench.rs:26-28 | REAL-UNPROVEN | Provenance merge commit 62c48b6 cited but not verifiable without git (SESSION-2: no git); internal consistency (bitexact=true) supports it. |
| "this example always benchmarks the exact shipped code" | color_bench.rs:29-31 | REAL | `#[path = "../src/video/mjpeg/color.rs"] mod now;` (:332) — same source file; color.rs has no `crate::`-external imports (only `#[cfg(test)] use super::*` at :483), so the include is faithful. |
| "medians, N≥15 recorded per-rep raw data" | color_bench.rs:4-5 | REAL | Committed output: 21 reps/arm, 15 reps in SHARE legs, all rows present. |
| Arms list including `simd` "when the CPU + build offer it" | color_bench.rs:36-39 | SHALLOW | Arm exists in code but is unconditionally absent from every run (simd_available ≡ false) — F-001. Disclosed at :37-38 ("absent from the baseline phase binary") but the optimization it awaits never landed. |
| "floats … this arm is context, never a candidate default" | color_bench.rs:40-42 | REAL | float_ref excluded from bit-exactness checks (:734-736) and from any default path. |

## 7. Dead / duplicated / unreachable code
- `compose_frame_simd` (:905-914) + the `if now::simd_available()` arm (:711-719) — dead in every producible run (F-001); mirrors the `#[allow(dead_code)]` seams in color.rs:359/:476.
- `checksum` (:916-922) — computed-then-discarded (F-003).
- `build_legacy`/`build_now` duplication — documented and inherent to the `#[path]`-include approach (distinct types); accepted.
- Not shipped: example binary only; no shipped entry point compiles it into a payload.

## 8. Dependencies & flags
No external deps; `std` only. The `#[path]`-include trick means the benchmarked kernel inherits the example crate's compilation — same package (codecs), so same feature set (codecs features apply to its examples); no cfg divergence found in color.rs. Timing uses `std::time::Instant` (monotonic). No unsafe in this part.

## 9. Open questions for the next agent
- part1 (A03): the legacy kernel copy (:66-328) — verify it truly matches merge-commit 62c48b6's color.rs (needs git history; SESSION-2 forbids me). If it drifted, every `bitexact_vs_legacy=true` is against a moving oracle.
- Whoever audits `color.rs` proper: the `#[allow(dead_code)]` scalar/simd seams (color.rs:351/:359/:476) are the other half of F-001 — the removal decision belongs with the kernel, not the bench.
- The W6-03 "after" run: REQUEST-ORCH-RUN only if the Lead wants the debt closed: `cargo run --release -p codecs --example color_bench -- after-1` (needs the SIMD kernel landed first — currently impossible).
