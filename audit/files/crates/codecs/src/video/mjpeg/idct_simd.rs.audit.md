# AUDIT: crates/codecs/src/video/mjpeg/idct_simd.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 457 / 15 items (module docs+proof, unsafe allow, 2 consts, dispatch, 2 unsafe helpers, SSE4.1 kernel, TestRng, 2 tests) |
| File verdict | REAL-UNPROVEN — the dispatch + scalar default are REAL and executed in the shipped lane; the SSE4.1 kernel (the file's reason to exist, "the ONE sanctioned `unsafe` exception") never executes in any default configuration and its bit-exactness is NOT pinned by any executed test (F-001) |
| Reachable from shipped entry point? | Dispatch: YES (call chain: `lumen open`/`decode-bench` → apps/cli/src/pipeline_host.rs:242-245 `MjpegDecoder::new` → mjpeg/mod.rs:362 `decode_scan` → decoder.rs:635 `H::transform` per block → decoder.rs:261-263 `idct_block_dispatch` — EXECUTED: baseline mp4-smoke-bench-avi-mjpeg-pcm.txt `v_decoded=60`). Kernel arm: NO in every default run — gated on `LUMEN_MJPEG_IDCT_SIMD` env var (line 130), which nothing in the repo, CI, or tests ever sets (`rg LUMEN_MJPEG_IDCT_SIMD` over the whole tree → only this file, lines 123+130). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 1 |
| Commands run | `Read` full file; `rg -n "LUMEN_MJPEG_IDCT_SIMD"` over the whole repo (2 hits, both in-file — no CI/script/test sets it); `grep -n "idct_simd" audit/runs/baseline/test-workspace.log.txt` (719-720: both tests ok); hand-verification of the module's i32 bound arithmetic (below); `_mm_shuffle_epi32` immediate decoding (0b10_11_00_01 = _MM_SHUFFLE(2,3,0,1) pair-swap ✓, 0b01_00_11_10 second swap ✓); mutation analysis of the kernel tests (below); `rg -n "idct_block_dispatch" decoder.rs` (per-block call site). |

## 1. One-paragraph verdict
The arithmetic design here is genuinely excellent — the two's-complement `w = 256·w_hi + w_lo` split keeps every `i32` lane provably under 2³¹ (I re-derived all four bounds myself: |q·Q| ≤ 32768·65535 < 2³¹; madd lanes ≤ 2·32138·4096; four-lane hsum ≤ 8·32138·4096 ≈ 1.05·10⁹ < 2³¹; |256·Dhi+Dlo| ≤ 2.7·10¹¹ in i64), the unsafe blocks are properly `target_feature`-gated with real safety comments, and the LEAD DECISION to keep scalar as default is honest and evidence-based. But the file's verification story collapses on two facts: (1) the headline bit-exactness test (`dispatch_is_bit_exact_vs_scalar`, with its 256-block randomized sweep and per-position single-AC fixtures — the only tests that could catch a transpose bug) runs through `idct_block_dispatch`, which falls through to the scalar path unless an env var that nothing in the repository ever sets is present — so in the baseline run it compared the scalar function against itself, and the module doc's claim that bit-exactness is "pinned by the randomized equality test below" is false as executed; the only test that touches the kernel at all (4 cases) is transpose-invariant and would pass with the transpose deleted. (2) The env-gate itself is evaluated **per decoded block** — a `std::env::var_os` (environment lock + key scan) on every block of every MJPEG frame on the default scalar path, ~49,000 lookups per 1080p frame, in a codebase whose own perf discipline (D-1/W2-06) eliminated per-frame allocations.

## 2. What actually works (verified)
— Dispatch executed in the shipped lane: baseline `mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_decoded=60`, every block through `idct_block_dispatch`'s scalar arm (env unset).
— The i32-exactness proof independently re-derived and confirmed: split identity exact for two's complement; |w_hi| ≤ 2¹² and |w_lo| ≤ 255 after the ±2²⁰ clamp; both madd partials and their hsums < 2³¹ with ~2× margin; `mid` reconstruction in i64 identical to the scalar formula. The `_mm_mullo_epi32` product bound −32768·65535 = −2,147,450,880 > i32::MIN ✓.
— `hsum32` shuffle arithmetic decoded by hand: imm 0b10_11_00_01 gives lane pair-swap, 0b01_00_11_10 completes the four-lane sum — integer-exact, no rounding.
— Unsafe hygiene: `#[target_feature(enable = "sse4.1")]` kernel entered only behind `is_x86_feature_detected!("sse4.1")` (dispatch 131-138; direct test 431/450-451); sse2 helpers' safety docs state the subsumption; all `loadu/storeu` are on provably in-bounds 16-byte rows of `&[i16;64]`/`&[u16;64]`/local arrays (checked index arithmetic: rows v·8..v·8+8, 8 rows).
— Prefix-write contract (`out.len().min(64)`, 304-305) matches the scalar path's short-tail contract (idct test `short_output_slices_are_written_prefix_only`, baseline 717).
— Both tests ran ok in baseline (test-workspace.log.txt:719-720).

## 3. Findings

### F-001 [S2] The kernel's bit-exactness is pinned by a test that, in every recorded run, compared the scalar function with itself — the env-gated dispatch makes the strongest tests tautological, and the 4-case direct test is transpose-invariant
Where: crates/codecs/src/video/mjpeg/idct_simd.rs:130 (gate), 332-341 (the tautological call), 327-331 (doc claiming otherwise), 428-455 (direct test)
Code:
```
        if std::env::var_os("LUMEN_MJPEG_IDCT_SIMD").is_some() // lumen-allow-literal: engine-internal opt-in env knob name, not product identity
            && std::arch::is_x86_feature_detected!("sse4.1")
        {
```
(the dispatch every `idct_block_dispatch` call — including every call from the test at 340 `idct_block_dispatch(block, qt, &mut simd);`)
What is wrong: `dispatch_is_bit_exact_vs_scalar` is the module's evidence that the SSE4.1 kernel is bit-exact (module doc 48-50: "the output is byte-equal by construction, pinned by the randomized equality test below"). But the dispatch's SIMD arm requires `LUMEN_MJPEG_IDCT_SIMD` to be set, and no test, script, workflow, or doc in the repository ever sets it (`rg` over the whole tree: only lines 123 and 130 of this file mention it). In the baseline run (test ok, log line 719) the "dispatch" side was therefore `super::idct::idct_block` — the same function the reference side calls at 339. The 256-block randomized sweep, the per-position single-AC fixtures (373-381), the hostile clamps, and the truncated-prefix checks all validated scalar-vs-scalar.
The only executed kernel coverage is `sse41_kernel_direct_matches_scalar_when_present` (428-455): four cases — all-zero, dc-only (single nonzero at (0,0)), and two uniformly-filled hostile blocks. All four are invariant under an 8×8 transpose error: a single entry at (0,0) maps to (0,0), and uniform blocks are permutation-invariant. Mutation analysis (reasoned, not run): delete the `transpose8x16` calls (pass `hi`/`lo` untransposed to stage 3) — every test in this module still passes with the env var unset, and even the direct 4-case test still passes; only the env-gated differential could catch it, and it never ran against the kernel.
Why it matters: The LEAD DECISION (117-125) records that the kernel "stays compiled, tested (bit-exactness suite)" — the cited suite does not test it. The "ONE sanctioned `unsafe` exception in the repo" is, in evidence terms, unvalidated: a transpose or lane-mapping bug in the kernel would ship green today and only detonate when someone sets the opt-in env var for the re-measurement the decision record itself calls for.
What correct looks like: The tests must control the gate directly: set the env var in the test (or add a `dispatch_force_simd` test-only entry, or a `OnceLock<bool>` static the test can prime). At minimum, the module doc and the test doc (327-331, which lists the non-x86_64 degradation but NOT the far more common env-unset degradation) must state that the kernel arm only runs when the environment opts in.
Confidence: HIGH — Verification: read + repo-wide rg of the env var + baseline log (both tests ok) + mutation reasoning; UNVERIFIED only in the trivial sense that I did not execute the mutated build (prohibited).

### F-002 [S2] Per-block `std::env::var_os` on the default scalar hot path — an environment lock+scan for every decoded block of every MJPEG frame
Where: crates/codecs/src/video/mjpeg/idct_simd.rs:126-142 (the lookup at 130)
Code:
```
pub(crate) fn idct_block_dispatch(block: &[i16; 64], qt: &[u16; 64], out: &mut [u8]) {
    #[cfg(target_arch = "x86_64")]
    {
        // Opt-in only (see the LEAD DECISION note above).
        if std::env::var_os("LUMEN_MJPEG_IDCT_SIMD").is_some()
```
What is wrong: `idct_block_dispatch` is called once per decoded block from the scan walk (decoder.rs:635 `H::transform` → 261-263). With the default scalar path, every one of those calls performs `std::env::var_os` — on Unix an environ read-lock plus a linear scan of the process environment for a key that is absent (the common case). At 1080p 4:2:0 that is ≈48,960 lookups per frame (120×68 MCUs × 6 blocks), ≈2.9M per second at 60 fps — paid entirely on the path the LEAD DECISION chose as default, in a decoder whose own perf history (D-1/W2-06, decoder.rs:27-63) counts per-frame allocation churn in MB/s. The compiler cannot hoist it: another thread may call `set_var`.
Why it matters: A measurable per-block constant overhead on the one shipped decode lane (MJPEG), invisible at the small fixture scale the baseline bench ran (60 tiny frames in 0.006 s) but scaling linearly with pixels; it directly contradicts the module's own performance discipline.
What correct looks like: Resolve the gate once per process: `static SIMD_ON: OnceLock<bool> = OnceLock::new();` (env read + feature detect on first block), or a `LazyLock<bool>` — the env-var contract ("opt-in at launch") is preserved exactly.
Confidence: HIGH on the call frequency (traced: decoder.rs:625-637 loop × per-block transform) and on the absence of caching (read); the absolute per-call cost is platform-dependent (UNVERIFIED numerically — no benchmark in the sandbox; the fixture bench cannot resolve it).
 
### F-003 [S4] The kernel is dead-by-default — and the direct test silently no-ops on non-SSE4.1 CPUs, so even the 4-case pin is unverifiable from the logs
Where: crates/codecs/src/video/mjpeg/idct_simd.rs:428-431 (silent skip), 117-125 (LEAD DECISION)
Code:
```
    #[test]
    fn sse41_kernel_direct_matches_scalar_when_present() {
        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("sse4.1") {
```
What is wrong: (a) By documented design the kernel only runs behind an env var no one sets — it is compiled into every shipped binary and executes never; the file's 300 lines of unsafe SIMD are a re-measurement fixture, not a shipped feature. That is an honest, recorded decision — but it means the "ONE sanctioned unsafe exception" carries pure weight. (b) The direct test's body is entirely conditional on SSE4.1; on a non-SSE4.1 CPU it passes having executed zero assertions, and nothing in the test output distinguishes that from a real pass — the baseline log's `ok` (line 720) therefore cannot prove the 4 cases even ran (the sandbox is x86_64, so they almost certainly did — UNVERIFIED from evidence alone).
Why it matters: A test that can pass by skipping is a coverage hole the logs cannot expose; combined with F-001 the kernel's executed evidence reduces to "probably 4 transpose-invariant cases, once".
What correct looks like: `assert!(executed || cfg!(not(target_arch="x86_64")))`-style sentinel, or print/assert the feature-detection result.
Confidence: HIGH — Verification: read + baseline log.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + bit-exactness proof | 1-70 | REAL | S2 | bounds re-derived and CORRECT; but "pinned by the randomized equality test below" false as executed → F-001 |
| `#![allow(unsafe_code)]` + cfg import | 72-83 | REAL | — | sanctioned exception, documented; import correctly cfg-gated |
| `DCT_A_Q16` const | 88-97 | REAL | — | Q16 basis, max 32138 (fits i16); duplicated from idct.rs's cfg(test) copy — documented reason |
| `A_COLS` const | 102-115 | REAL | — | const-evaluated transpose of the basis; values fit i16 ✓ |
| `idct_block_dispatch` | 126-142 | REAL | S2 | executed (scalar arm) in shipped lane; per-block env lookup → F-002; SIMD arm env-gated → F-001 |
| `hsum32` (unsafe, sse2) | 150-164 | REAL | — | shuffle imm decoded by hand ✓ exact |
| `transpose8x16` (unsafe, sse2) | 170-202 | REAL | — | canonical unpack pattern; correctness NOT pinned by any executed kernel test (F-001 mutation analysis) |
| `idct_block_sse41` (unsafe, sse4.1) | 210-306 | REAL-UNPROVEN | — | flat-block delegate ✓; stage-1 split exact ✓; stage-3 madd bounds ✓; stage-4 scalar reuse ✓; prefix write ✓; never executed in any default run; only transpose-invariant cases executed |
| test `TestRng` | 312-325 | REAL | — | xorshift64* standard constants |
| test `dispatch_is_bit_exact_vs_scalar` | 327-423 | SHALLOW (as executed) | S2 | reference side real; dispatch side = same scalar fn when env unset → tautology in baseline → F-001 |
| test `sse41_kernel_direct_matches_scalar_when_present` | 425-456 | REAL-UNPROVEN | S4 | 4 kernel cases, all transpose-invariant; silently skips without SSE4.1 → F-003 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| dispatch_is_bit_exact_vs_scalar (baseline 719 ok) | NO (for any kernel defect, in default env) | dispatch side is the scalar fn itself when `LUMEN_MJPEG_IDCT_SIMD` is unset; the suite would stay green with the transpose deleted, the madd indices permuted, or the split arithmetic wrong — F-001 |
| sse41_kernel_direct… (baseline 720 ok) | Only for defects that break uniform/zero/DC blocks | all 4 cases transpose-invariant; would NOT catch transpose/layout bugs; silently no-ops without SSE4.1 — F-003 |
Mutation checks (reasoned): (a) delete `transpose8x16` → both tests still pass; (b) swap `d_hi`/`d_lo` indexing `[y*8+u]`→`[u*8+y]` → both tests still pass; (c) break the scalar fallback (`+128`→`+127`) → dispatch test FAILS (proving the test is wired to the scalar side only).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the output is byte-equal by construction, pinned by the randomized equality test below (deterministic seed) plus the existing end-to-end reference digests" | idct_simd.rs:48-50 | FALSE (as executed) | the randomized test's SIMD side never ran (env unset, F-001); the end-to-end digests run the scalar default → they pin nothing about the kernel |
| "The SSE4.1 kernel stays compiled, tested (bit-exactness suite), and reachable … via the LUMEN_MJPEG_IDCT_SIMD=1 opt-in" | idct_simd.rs:121-123 (LEAD DECISION) | PARTLY FALSE | compiled ✓, reachable-with-env ✓, "tested (bit-exactness suite)" — the suite is scalar-vs-scalar by default (F-001) |
| "No perf numbers are claimed here yet" | idct_simd.rs:64-70 | REAL | honest UNMEASURED marker; ablation lives in docs/perf/mjpeg-simd-20261024.md (1.021×/0.995×, SIMD on the slow side of noise — the scalar default's basis) |
| "a safety comment on every unsafe block" | idct_simd.rs:4-5 | REAL | verified: 229-228, 258, 275-276, plus fn-level # Safety sections on all three unsafe fns |
| bit-exactness bounds (41-43) | idct_simd.rs:41-43 | REAL | re-derived: 8·32138·4096 = 1,053,097,984 (doc: 1,053,454,784 — same order, both < 2³¹; the doc's constant is the 32138·32768-flavored variant, conservative either way) |

## 7. Dead / duplicated / unreachable code
- The SSE4.1 kernel (210-306) + `hsum32` + `transpose8x16` + `A_COLS`: dead in every default configuration (env opt-in only; nothing sets it).
- `DCT_A_Q16` duplicates idct.rs's `DCT_A` — documented (cfg(test) gating of the original), acceptable.

## 8. Dependencies & flags
- No Cargo features; `cfg(target_arch = "x86_64")` only. The `#![allow(unsafe_code)]` is the repo's single sanctioned exception (contract rule 5 / D-10) — hygiene counts confirm the codecs crate's unsafe lives here and the hw/av1 files (INDEX.md §Hygiene).
- Depends on `super::idct::{idct8, shr32_round}` (idct.rs — another agent's file) for pass 2 — the bit-exactness story leans on those two being identical in both paths (same functions, so yes).
- Reachable from the shipped MJPEG lane (§ header chain); the kernel arm additionally requires an env var that is a de-facto internal knob.

## 9. Open questions for the next agent
- idct.rs owner: confirm `MAX_COEFF = 1<<20` and the clamp order in the scalar path match stage-1's `_mm_min_epi32(_mm_max_epi32(p, nlim), lim)` (NaN-free integer clamp — order only matters for the bound, which holds either way).
- Whoever re-runs the ablation: set `LUMEN_MJPEG_IDCT_SIMD=1` in the test environment once and watch `dispatch_is_bit_exact_vs_scalar` — that is the first time the kernel's randomized differential will actually execute; fix F-002's OnceLock at the same time so the re-measurement isn't polluted by per-block env lookups on the scalar arm too.
