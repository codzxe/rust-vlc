# AUDIT: crates/subtitles/src/vobsub/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Lines / functions | 54 / 1 |
| File verdict | REAL — small, honest, exercised harness (in-crate survival tests), libFuzzer target deliberately unwired (disclosed) |
| Reachable from shipped entry point? | NO — crate-level UNREACHABLE from shipped binaries; harness itself reachable from `tests/dvb_vobsub_fuzz_survival.rs` only |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/dvb_vobsub_fuzz_survival-712e2bfc04f56940` -> 2 passed, 0 failed (exit 0, 0.72 s); `ls fuzz/fuzz_targets/` (no fuzz_sub_vobsub.rs); doc 04 §8.5 + feature-matrix row reads |

## 1. One-paragraph verdict
The harness does exactly what its 54 lines promise: one full idx+sub parse, one cue-render
pass, five truncation re-parses — and its "absence-of-panic/hang/unbounded-alloc, nothing
else" contract is honestly scoped. It is genuinely exercised: the committed survival test
drives it through a clean pass, systematic truncation sweeps of BOTH halves, and 400 seeded
LCG mutation rounds (byte flips, sets, splices, idx-text mutations) — executed, 2/2 pass.
The problem is one level up: the feature matrix marks VobSub (and DVB) "implemented" on all
desktop platforms while this file's own doc admits the registered libFuzzer target was never
created — a direct violation of the repo's own doc 04 §8.5 registry rule ("may not reach
`implemented` until its target exists ... and has a seed corpus committed").

## 2. What actually works (verified)
— verified by `./target/debug/deps/dvb_vobsub_fuzz_survival-712e2bfc04f56940` -> `test
result: ok. 2 passed; 0 failed` — `vobsub_fuzz_survival_mutations_and_truncations` calls
`fuzz_sub_vobsub` via `vobsub_invariants` (line 305) on the clean seed, every truncation cut
of sub and idx, and all 800 mutated variants (400 sub-mutations + 400 idx-mutations), also
asserting the damage cap and per-cue rgba stride each pass.

## 3. Findings
F-001 [S2 — cross-reference]
Where: crates/subtitles/src/vobsub/fuzz.rs:16-26 vs docs/04-codec-format-roadmap.md:607-611 vs docs/feature-matrix.toml:1042
Code:
```
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_vobsub.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
//! decision (AGENTS.md §3.2, C5); the `fuzz/` tree is outside this
//! card's file scope. Until the Lead wires it, the harness logic is
//! proven on stable by the deterministic survival runs in
```
```
- A parser crate may not reach `implemented` until its target exists, runs 10
  minutes clean in CI (release mode), and has a seed corpus committed.
```
```
status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }
```
What is wrong: The matrix row `subtitle-vobsub` claims "implemented" on 3 platforms, but the
registered libFuzzer target does not exist (`ls fuzz/fuzz_targets/`: only fuzz_sub_srt,
fuzz_sub_vtt, fuzz_sub_ass — no vobsub/dvb/cc/pgs), violating doc 04 §8.5's own gate. Doc 04
closes with "This roadmap is the contract the feature matrix will be audited against. When
reality disagrees with a row, the row loses." The same applies to row `subtitle-dvb-subs`
(feature-matrix.toml:1002, "implemented" — its notes even admit "no corpus lane, record
owed"). Why it matters: the repo's own promotion rule was skipped for exactly the bitmap
subtitle formats whose decoders are also product-unreachable. What correct looks like:
status "partial" until the target + corpus land, or a Lead-recorded waiver.
Confidence: HIGH — Verification: read (all three files) + ls of fuzz/fuzz_targets.
Flag for: W19-A06 (sub-vobsub doc), matrix-chunk owner of rows ~995-1050.

F-002 [S4]
Where: crates/subtitles/src/vobsub/fuzz.rs:41-42
Code:
```
    let doc = parse_vobsub(idx_data, sub_data);
    // Exercise the render path over whatever survived.
    let _ = vobsub_cues(&doc, 0);
```
What is wrong: `vobsub_cues`' Result is swallowed and the render path is exercised ONLY on
the full-parse document — the five truncation re-parses never reach the cue render. If the
render path systematically errored (or panicked only on damaged docs), this harness would
not notice; coverage for the render-on-damaged-doc case is zero. Mitigated by one unit test
(vobsub/mod.rs:606 `vobsub_cues(&doc, 7).must()` — happy path only). What correct looks
like: `vobsub_cues` inside `vobsub_invariants` in the survival test (parse AND render per
mutation).
Confidence: HIGH — Verification: read + rg (vobsub_cues has exactly 3 call sites).

## 4. Per-function table (file ≤100 LOC — table optional, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_sub_vobsub | 39-54 | REAL | S4 | full pass + render + 5 truncation re-parses; bounds: slices `..cut` always ≤ len; damage cap 64 + SPU 128 KiB verified present (vobsub/mod.rs:57, SPU_MAX_SIZE re-export) |

## 5. Test honesty
No tests in this file. The harness itself is the tested artifact (see §2). The doc's
"proven on stable by the deterministic survival runs" claim is TRUE (executed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted idx+sub pass through **every** decode surface" | L4-10 | REAL | parse_vobsub + vobsub_cues cover idx/PES/SPU/RLE/palette/crop/cue paths (call graph verified by rg) |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | L11-14 | REAL | `let _ =` swallows outcomes; survival tests assert only invariants |
| "Until the Lead wires it, the harness logic is proven on stable by the deterministic survival runs" | L23-26 | REAL | executed: 2/2 survival tests pass |
| "output ≤ O(input) + the 128 KiB SPU accumulation cap; damage ledger capped" | L30-32 | REAL | SPU_MAX_SIZE + MAX_DAMAGE_EVENTS=64 in vobsub/mod.rs:57 |

## 7. Dead / duplicated / unreachable code
Not dead — called by tests/dvb_vobsub_fuzz_survival.rs:305. Product-unreachable (crate-level).

## 8. Dependencies & flags
crate::vobsub only. No features. The libFuzzer shim is absent by disclosed decision.

## 9. Open questions for the next agent
- W19-A06 / matrix-chunk owner: downgrade or waive `subtitle-vobsub` + `subtitle-dvb-subs`
  "implemented" per doc 04 §8.5 (F-001)?
- W09-A09 (spu.rs): spu.rs:490 has a `debug_assert_eq!` ("cmds2 offset arithmetic") guarding
  decode-path arithmetic — in release it is unchecked; verify the invariant is structural.
