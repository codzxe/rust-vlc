# AUDIT: crates/playback-engine/tests/reopen_alloc.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 154 / 2 (cycle helper + 1 test) |
| File verdict | REAL (exact counting-allocator proof of engine-lifecycle allocation-neutrality across 1000 reopen cycles; honestly scoped to fakes) |
| Reachable from shipped entry point? | The engine under test is the shipped `playback_engine::Engine` (apps/cli/src/host.rs:231/983). The demux/decode stages are the tests/common fakes — the proof covers the ENGINE's session/pipeline/queue/outbox lifecycle, not real decoder allocations. Test binary is CI-only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `sed -n`/`rg` verification incl. tests/common/mod.rs fakes (exit 0); baseline cargo-test-workspace.txt:4360-4363 — 1 passed in 0.05 s |

## 1. One-paragraph verdict
A clean, honest measurement test: 100 warmup cycles size the engine's reused pools, then a guard window runs 1000 full open→play→stop→close cycles and asserts EXACT allocation flatness (`delta.live_bytes == 0` — one leaked byte fails), with per-cycle liveness enforced (`frames > 0` every cycle, `played >= 1000`, `alloc_calls > 0`). The docstring scopes itself precisely: fakes, not real decoders; the functional half lives in model_walk.rs; the 4 h soak stays 3.E4's. I found no defects; the checklist below records what was hunted.

## 2. What actually works (verified)
— Baseline: cargo-test-workspace.txt:4360-4363 — `reopen_cycles_are_allocation_neutral_in_steady_state ... ok` (0.05 s for 1100 cycles).
— The measurement discipline is correct: the guard baseline is taken AFTER the warmup (line 130, following the 100-cycle loop at 124-127), so pool-sizing allocations are excluded from the claim; the delta is read after all 1000 measured cycles.
— Per-cycle hygiene: the outbox is drained at cycle end (line 110) so nothing one cycle emitted is attributed to the next; the collector is dropped inside the cycle function, so no frame buffer survives into the delta window.

## 3. Findings
None. Zero-findings justification — exactly what was checked and how:
- **Oracle strength**: the flatness assertion is exact equality (`delta.live_bytes == 0`, line 139), not a budget — any per-cycle accretion, however small, fails. The two sanity assertions kill the degenerate-pass modes (a cycle that allocates nothing means the loop didn't run; a cycle that delivered no frames means it didn't play).
- **Liveness bounds**: the 24-poll pump to `Playing` is asserted after the loop (lines 85-89) — hitting the bound without playing fails loudly; stop→close each asserted to reach `Idle` (line 102).
- **Measurement-window integrity**: verified that the warmup/measured split, the guard placement, and the per-cycle collector drop cannot leave residual allocations inside the measured window; `serialize_tests()` plus one-test-per-binary guards the process-global allocator from sibling pollution (the stated design reason, lines 9-11).
- **Fakeness audit**: FakeDemuxer/FakeDecoder/FakeProvider read in tests/common/mod.rs:86-184 — the decode stage is a 1:1 packet→frame fake, so the proof covers engine-side structures only. The docstring's own claim line ("everything the open built, the close must free") is therefore engine-scoped, and the file says so ("on the fakes", line 3) — no over-claim. The real-decoder analogue is track_switch_alloc.rs (baseline :4597-4601, passes) — not my file.
- **Error handling**: every submit is `unwrap_or_else(|e| unreachable!("... : {e}"))` with context — loud, not swallowed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cycle | 49-112 | REAL | — | one full lifecycle; bounded pumps; loud failures; outbox drained |
| test reopen_cycles_are_allocation_neutral_in_steady_state | 114-154 | REAL | — | warmup → guard → 1000 cycles → exact-zero delta + sanity |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| reopen_cycles_are_allocation_neutral_in_steady_state | YES | a single byte leaked per reopen cycle accumulates to 1000× and fails the exact-zero assertion; a cycle that stops playing fails the per-cycle frames>0 assert; a no-op loop fails the alloc_calls/played sanity asserts |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "1000 × open → play → stop → close on the fakes, memory measured FLAT ... runtime evidence, not syntactic green" | header:1-7 | REAL | the test does exactly this and passes (baseline :4363) |
| "a reopen cycle must be allocation-neutral in steady state (everything the open built, the close must free)" | header:16-17 | REAL (engine-scoped) | exact-zero delta over 1000 cycles; fakes admitted |
| "the functional half ... lives in tests/model_walk.rs; the 4 h wall-clock soak stays 3.E4's" | header:5-7 | REAL (scoping) | both targets exist and run in the baseline |

## 7. Dead / duplicated / unreachable code
None. Single test, all helpers used, no `#[ignore]`, no feature gates.

## 8. Dependencies & flags
playback-engine, plugin-api, media-types, commands, testkit (CountingGlobalAlloc as the process-global `#[global_allocator]`, line 35-36). Runs in the default lane (baseline :4360-4363).

## 9. Open questions for the next agent
- tests/track_switch_alloc.rs is the REAL-decoder counting-allocator proof this file defers to — whoever owns it should confirm the fake-free provider path (it passed in baseline :4597-4601).
- The proof is reopen-on-ONE-engine; fresh-`Engine`-per-cycle churn (new engine object each time) is the soak's open-close-churn scenario — no coverage gap, just different scopes; do not merge the claims.
