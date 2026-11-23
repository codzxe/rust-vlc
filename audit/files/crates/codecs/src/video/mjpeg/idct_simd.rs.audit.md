# AUDIT: crates/codecs/src/video/mjpeg/idct_simd.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 457 / 6 (+2 tests) |
| File verdict | REAL (kernel bit-exact and EXECUTED by me) — SIMD branch default-OFF in shipped builds (documented, measurement-backed) |
| Reachable from shipped entry point? | Dispatch YES: `idct_block_dispatch` ← `decoder.rs:262` (MjpegDecoder scan path) ← registry → CLI. The SSE4.1 branch itself: only when the undocumented env knob `LUMEN_MJPEG_IDCT_SIMD` is set AND SSE4.1 present — no CI lane, script, or doc sets it (rg over .github/scripts/docs/tools: zero references outside this file). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::idct_simd` → 2 passed (exit 0, scalar-vs-scalar); `LUMEN_MJPEG_IDCT_SIMD=1 ./target/debug/deps/... mjpeg::idct_simd --nocapture` → 2 passed (exit 0, 256-block sweep THROUGH the real SSE4.1 kernel on this SSE4.1 machine); `mjpeg::idct` → 12 passed; read docs/perf/mjpeg-simd-20261024.md |

## 1. One-paragraph verdict
Hostile-focus answers: **Correct?** Yes — the two's-complement radix split (w = 256·(w>>8) + (w&255)) with `_mm_madd_epi16` partials is mathematically exact; I verified every bound in the proof (|q·Q| ≤ 2,147,450,880 < 2³¹ fits mullo_epi32; |A|·|w_hi| lane sums ≤ 1,053,454,784 < 2³¹; w_hi ∈ ±4096 fits i16), the A_COLS transpose indexing (Σ_v A[v][y]·w[v][u]), the canonical 8×16 unpacklo/hi transpose, and the DCT basis constants against cos((2y+1)vπ/16)·32768 — all correct. I then EXECUTED the 256-block randomized bit-exactness sweep through the real kernel (env knob set, SSE4.1 CPU): byte-equal to scalar, including hostile ±32768×65535 clamp edges and truncated outputs. **Actually used?** The dispatch is wired into the production decoder, but the SIMD branch is opt-in via an env knob because the measured ablation (docs/perf/mjpeg-simd-20261024.md §4.1: avi-av 1.021× SIMD-slower, heavy 0.995× SIMD-slower, N=7/N=4 medians) showed no win — an honest, evidence-backed keep-scalar decision. The real defects are documentation rot and a CI coverage gap: the module header still says "UNMEASURED [VERIFY]" while the dispatch cites the completed measurement; mod.rs still calls this a "stub … not yet wired"; and the flagship equality test never touches the kernel in default runs because the env gate is closed and nothing in CI ever opens it.

## 2. What actually works (verified)
— verified by `LUMEN_MJPEG_IDCT_SIMD=1 ./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::idct_simd --nocapture` → "2 passed" on this SSE4.1 machine: the 256-block seeded sweep (density mix 0..1, random quant tables, truncated-output variants) ran through `idct_block_sse41` and stayed byte-equal with the scalar path; plus the 4-case direct-kernel test (zero/dc/hostile-neg/hostile-pos). Unsafe-block audit: every block is inside `#[target_feature]` fns entered only after `is_x86_feature_detected!`, each carries a SAFETY comment, all loads/stores are unaligned intrinsics on caller-valid 16-byte rows — no invariant left unproven.

## 3. Findings
F-001 [S3]
Where: idct_simd.rs:327-331 (test doc), 130 (gate)
Code:
```
    /// Scalar vs dispatched path, byte-equal. On x86_64 with SSE4.1
    /// this exercises the SIMD kernel (the deployment case); on
```
(against L130) `if std::env::var_os("LUMEN_MJPEG_IDCT_SIMD").is_some()`
What is wrong: the doc comment is false in default runs. `idct_block_dispatch` takes the SIMD branch only when the env knob is set; no CI workflow, script, or doc ever sets it (rg across .github/, scripts/, docs/, tools/: zero references outside this file). So in every default `cargo test`/CI run, `dispatch_is_bit_exact_vs_scalar` compares scalar against SCALAR — its 256-block sweep, structured fixtures and hostile clamp cases never execute the kernel; only the 4-case `sse41_kernel_direct_matches_scalar_when_present` does. Worse, docs/perf/mjpeg-simd-20261024.md §3 cites this test as the bit-exactness evidence for the SIMD landing ("256-block seeded equality, dispatch vs scalar"). Why it matters: a future kernel regression in the swept case space would be invisible to CI. What correct looks like: the test sets the env var itself (e.g., `std::env::set_var` at test start, or a dedicated `#[test]` that calls the kernel directly for the full sweep). Confidence: HIGH — Verification: read + executed both gate states.
F-002 [S4]
Where: idct_simd.rs:64-70 vs 117-125
Code:
```
//! # UNMEASURED [VERIFY]
//!
//! No perf numbers are claimed here yet. The before/after run under
```
What is wrong: the module header's "UNMEASURED [VERIFY]" section says the measurement "is the successor session's first job", but the dispatch comment (L117-125) already cites the completed ablation (docs/perf/mjpeg-simd-20261024.md — which exists and I read: §4.1 medians, SIMD slower both fixtures). The header is stale and contradicts the file's own lower half. Confidence: HIGH — Verification: read (both sections + the perf doc).
F-003 [S4]
Where: mjpeg/mod.rs:115-118 (about this module)
Code:
```
// (idct_simd: card 7-b stub — SIMD plan + dispatch skeleton, not yet
// wired; see that module's docs before implementing.)
pub(crate) mod idct;
#[allow(unused)]
pub(crate) mod idct_simd;
```
What is wrong: both the comment and the attribute are false today: the module is a full working SSE4.1 kernel (not a "plan + dispatch skeleton"), it IS wired (decoder.rs:95 imports and :262 calls `idct_block_dispatch`), and it is not unused. Stale commentary that actively misdescribes shipped code. Confidence: HIGH — Verification: read + rg callers.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DCT_A_Q16 | 88-97 | REAL | — | verified against cos((2y+1)vπ/16)·32768 (23170/32138/27246/18205/6393/30274/12540 all correct); duplication from idct.rs documented (its copy is cfg(test)) |
| A_COLS (const fn) | 102-115 | REAL | — | A_COLS[y][v] = A[v][y]; max 32138 fits i16; const-evaluated transpose correct |
| idct_block_dispatch | 126-142 | REAL | — | env-knob + runtime-detect double gate; scalar default; SAFETY comment present; the env knob is repo-wide undocumented (F-001 context) |
| hsum32 | 150-164 | REAL | — | pair-swap shuffle adds, integer-exact; unsafe fn under target_feature(sse2), subsumed by sse4.1 gate |
| transpose8x16 | 170-202 | REAL | — | canonical Intel 8×i16 unpacklo/hi pattern — verified lane-by-lane |
| idct_block_sse41 | 210-306 | REAL | — | stage 1 exact (sign/zero extends + mullo_epi32 within i32, clamp ±2²⁰, split); stage 3 madd bounds proven; stage 4 scalar pass-2 identical; prefix-write contract matches scalar (tested at lens 0/1/7/24/63); all-zero-AC delegates to scalar fast path |
| dispatch_is_bit_exact_vs_scalar | 332-423 | REAL (kernel arm env-gated off in CI — F-001) | — | I executed the kernel arm manually: 256 blocks + 64×6 single-AC + dc sweeps + hostile max + gradient + truncated, all byte-equal |
| sse41_kernel_direct_matches_scalar_when_present | 427-456 | REAL | — | 4 direct-kernel cases run unconditionally on SSE4.1 — the only default-run kernel coverage |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dispatch_is_bit_exact_vs_scalar | YES for the kernel — but ONLY when the env knob is set (F-001); in default runs it can never fail for a kernel bug (scalar vs scalar) | executed both states |
| sse41_kernel_direct_matches_scalar_when_present | YES (4 cases incl. both hostile clamp edges) — but narrow: no random sweep, no truncated-output cases | executed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Bit-exactness … pinned by the randomized equality test below" | idct_simd.rs:48-50 | REAL-UNPROVEN in CI / REAL in my run | I ran the sweep with the knob set (pass); CI never sets it (F-001) |
| "No perf numbers are claimed here yet" | idct_simd.rs:64-66 | STALE/FALSE | measurement exists: docs/perf/mjpeg-simd-20261024.md §4.1 (F-002) |
| "scalar is the default … SIMD on the SLOW side of noise" | idct_simd.rs:117-125 | REAL | perf doc verified: 1.021×/0.995× SIMD-slower medians; env-gated default confirmed by read+execution |
| "the ONE sanctioned unsafe exception in the repo" | idct_simd.rs:1-3 | REAL-UNPROVEN | no other unsafe_op_allow found by me in codecs (spot checks only); full-tree census not run (UNVERIFIED, low value) |

## 7. Dead / duplicated / unreachable code
The SIMD branch is unreachable in shipped builds by deliberate policy (documented, measured). `#[allow(unused)]` on the module is stale (F-003). DCT basis duplication from idct.rs is documented and justified.

## 8. Dependencies & flags
No external deps; `#![allow(unsafe_code)]` is the workspace-deny exception, scoped to this file. cfg(target_arch = "x86_64") guards all kernel code; non-x86_64 compiles the scalar dispatch only. Release lane: scalar path always; kernel compiled in but env-gated.

## 9. Open questions for the next agent
- idct.rs owner: confirm `shr32_round`/`idct8` are the exact pass-2 primitives the scalar path uses (the equality test makes this near-certain, but the shared-import coupling is the bit-exactness linchpin).
- CI owner: consider adding one lane with LUMEN_MJPEG_IDCT_SIMD=1 so the kernel sweep runs in CI (F-001).
