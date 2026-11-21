# AUDIT: crates/media-types/src/time.rs#part3 (lines 1239–1858)
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-21 |
| Lines / functions | 620 (chunk 1239–1858 of 1858) / 9 test fns (+2 helper closures) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | Test target: compiled by `cargo test --workspace` (member `crates/*`), but NOT executed in baseline (fail-fast abort — F-001). Code under test: YES (call chain: `lumen` CLI → `apps/cli/src/demux_host.rs:178,558,708` `Timestamp::convert(&NANO_BASE)` → `try_fast_convert`/`convert_reference`, time.rs:175-180) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `rg -n "^(pub |)(fn|struct|...)"` (outline); `sed -n` (all quotes verified); `rg -n "media_types|time::tests|mulshift" audit/runs/baseline/test-workspace*.log.txt` → **zero hits** (no execution evidence); `rg -n "Running unittests" test-workspace.log.txt` → 12 binaries, none `media_types`; `rg -n "cargo test" .github/workflows/ci.yml` → `cargo test --workspace --all-features`, no `--no-fail-fast` |

Chunk boundaries: part1 = A02 (1–619), part2 = A03 (620–1238), part3 = mine
(1239–1858). Part3 opens mid-function: `mulshift_magic_is_bit_exact_for_small_divisors`
starts at 1237 (its `#[test]`/signature lines belong to part2's tail); I audited
the complete test despite the 2-line overlap. All of part3 is `#[cfg(test)]` code.

## 1. One-paragraph verdict
This chunk is the M18 "proof" tail of the test module: eight equivalence/property
tests plus one `#[ignore]`d benchmark for the division-by-invariant-multiplication
fast paths (`try_fast_convert`, `try_fast_nanos`, `Scaler`, `magic_u64`,
`div_round_mulshift`, `reduce_smooth`). The tests are genuinely hostile by
construction — independent oracles (`p / d` hardware division at 1243, the kept
`i128` reference at 1249), domain-edge straddling (`probe_ticks`), a
fast-path-hit-rate floor (1410) that defeats the "silently always fall back"
tautology, and degenerate struct-literal bases (1618). But **not one of them has
any executed evidence**: both baseline `cargo test --workspace` runs fail-fast
at `apps/desktop` (`zero_alloc_transport`), which sorts before `media-types`,
so the crate's entire unit-test binary never ran, and CI (ci.yml:234,
ci-full.yml:169) has the same no-`--no-fail-fast` blind spot with a known-failing
test in the tree. Code quality: real; proof status: asserted, not executed.

## 2. What actually works (verified)
— Verified by `rg -n "as_nanos\(\)|\.convert\(&|scaler_to\(" apps/ crates/` →
production conversions are called from the shipped `lumen` CLI
(`apps/cli/src/demux_host.rs:178,558,708`), so the code these tests prove is
on the shipped open/playback path (baseline rc-smoke exercised those
conversions indirectly: `audit/runs/baseline/rc-smoke-engine-only.log.txt` PASS 6/6).
— Verified by reading `Timestamp::convert`/`as_nanos` (time.rs:175-197): they
do route through `try_fast_*` with reference fallback, so the tests exercise
the real production entry points, not a copy.
Nothing in this chunk itself was executed — see F-001.

## 3. Findings

### F-001 [S2] Entire M18 proof suite has zero executed evidence; baseline + CI blind spot hides it
Where: crates/media-types/src/time.rs:1237-1857 (all of part3) + audit/runs/baseline/test-workspace.log.txt:2411-2412
Code (baseline log tail, verbatim):
```
error: test failed, to rerun pass `-p desktop --test zero_alloc_transport`
TEST_EXIT=101
```
What is wrong: `cargo test --workspace --locked` (baseline run #3) and the
`--all-features` variant (run #4) both abort at the first failing test binary
(cargo's default fail-fast). The binaries run in package order
(audio-engine … control-server, demux, **desktop**), and `media-types` sorts
after `desktop`, so the `media_types-<hash>` unittest binary — containing every
test in this chunk — was never started. `rg "media_types|mulshift|time::tests"`
over both baseline logs: zero hits. INDEX.md's "1681 passed / 1 failed / 24
ignored" is an accurate tally of a *partial* run (~12 of ~40 lib unittest
binaries executed). Same defect in CI: `.github/workflows/ci.yml:234` and
`ci-full.yml:169` run `cargo test --workspace --all-features` with no
`--no-fail-fast`, and the tree currently contains a known-failing test
(desktop `zero_alloc_transport`, baseline #3), so CI would abort before
media-types on every push.
Why it matters: the fast paths guarded by these proofs run per packet/frame on
the shipped MP4/AVI open path. Their bit-exactness is currently *asserted by
inspection only*. Any regression in `magic_u64`/`div_round_mulshift`/
`reduce_smooth` introduced after the last full local run would be invisible to
every recorded gate.
What correct looks like: `cargo test --workspace --no-fail-fast` (or fix the
desktop failure) in baseline and CI, so every member's tests execute and are
counted; INDEX.md should state the run aborted early.
Confidence: HIGH — Verification: read (log tails + binary list enumerated) /
ran(baseline cite: test-workspace.log.txt:2411, test-workspace-allfeatures.log.txt tail, full-gate.log.txt disk-exhaustion abort).

### F-002 [S4] Comment claims "seed printed on failure" — no failure message prints the seed
Where: crates/media-types/src/time.rs:1300-1302 (and 1464-1466)
Code:
```
        // ceiling. Deterministic LCG — seed printed on failure.
        let mut rng = Lcg(0x5EED_D1CE);
```
What is wrong: every assert in these tests prints the *case* values (`p`, `d`,
`base`, `ticks`), never the seed. Reproducibility comes from the seed being a
hard-coded literal in the source, not from failure output.
Why it matters: comment asserts a debugging affordance that does not exist;
an engineer chasing a failure will look for a seed in output that never appears.
What correct looks like: "seed is the literal in the source (0x5EED_D1CE /
0x1118_C0DE); failure messages print the case values".
Confidence: HIGH — Verification: read (assert messages at 1311, 1484 checked).

### F-003 [S4] `#[ignore]`d benchmark called "the regression tripwire" — a tripwire CI never trips
Where: crates/media-types/src/time.rs:1710-1714
Code:
```
    /// measured 3.7× SLOWER than the reference — restructured; this
    /// benchmark stays in-tree as the regression tripwire.
    #[test]
    #[ignore = "manual M18 benchmark — run with --release --nocapture"]
```
What is wrong: an `#[ignore]`d test is excluded from every default `cargo test`
run (baseline ignored-count includes it), so as a "regression tripwire" it is
inert unless a human remembers the manual invocation; the doc's measured
numbers ("W3-06, Xeon, release, min-of-5 × 10M, ns/conv", lines 1701-1705)
have no corresponding run artifact anywhere in the repo or baseline.
Why it matters: the M18 perf claims (D-6 "measured, not claimed") rest on an
unreproducible, never-run-in-gate benchmark; a future regression in the fast
tiers' speed is undetectable by any automated gate.
What correct looks like: either wire a reduced-size variant into CI (e.g. N=100k
with a generous ceiling) or state plainly "manual-only, no automated perf gate".
Confidence: HIGH — Verification: read (#[ignore] semantics; baseline INDEX #3/#4
ignored counts 24/25 include it; no perf-run artifacts in audit/runs/baseline/).

### F-004 [S4] Duplicated 18-line verification closure inside one test
Where: crates/media-types/src/time.rs:1242-1259 vs 1307-1325
Code (first, 1242-1248):
```
            let mut check = |p: u64| {
                let want_floor = p / d;
                let got_floor = ((u128::from(p) * m) >> k) as u64;
                assert_eq!(got_floor, want_floor, "floor diverged: p {p}, d {d}");
                let fast = div_round_mulshift(p, m, k, d);
                let div = div_round_u64(p, d);
                assert_eq!(fast, div, "mulshift vs div diverged: p {p}, d {d}");
```
What is wrong: the dense-sweep `check` closure and the randomized-sweep `check`
closure are near-identical (same three-way oracle comparison, differing only in
`cases` capture shape). A future fix to one (e.g. tightening an oracle) can
silently miss the other.
Why it matters: duplicated verification logic in the file whose entire purpose
is verification rigor.
What correct looks like: one `fn check_case(p: u64, d: u64, m: u128, k: u32,
cases: &mut u64)` used by both loops.
Confidence: HIGH — Verification: read (both closures compared line-by-line).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mulshift_magic_is_bit_exact_for_small_divisors` (test) | 1237-1367 | REAL-UNPROVEN | — | Exhaustive d≤4096 sweep + 20k random d to u64::MAX; three independent oracles (`p/d`, `div_round_u64`, `div_round_i128`); asserts case-count floors (1293, 1360) so the sweep cannot silently shrink. Not executed in baseline (F-001). Mutation: corrupt `magic_u64` → floor assert at 1245 fires; corrupt `div_round_mulshift` rounding → 1248 fires. |
| `as_nanos_fast_path_is_bit_identical_to_reference` (test) | 1370-1418 | REAL-UNPROVEN | — | Production `Timestamp::new(t,base).as_nanos()` vs `as_nanos_reference`; den≤4096 grid; hit-rate floor `fast_hits*2 > nonneg` (1410) defeats always-fallback tautology. Mutation: make `try_fast_nanos` always None → 1410 fails. |
| `convert_fast_path_is_bit_identical_to_reference` (test) | 1421-1495 | REAL-UNPROVEN | — | from×to grid incl. real bases (90k, 44.1k, 48k, 24k/1001, ns); random pairs; probes i64::MAX/MIN and domain edge ±1 (1475-1483). Mutation: flip tie-rounding in `try_fast_convert` → edge probes diverge. |
| `scaler_is_bit_identical_to_reference` (test) | 1498-1546 | REAL-UNPROVEN | — | `Scaler::convert_ticks`/`nanos` vs references over gcd-reduced domain edge (1519) — straddles a *different* edge than the smooth-reduced one, covering both tiers' boundaries. Mutation: wrong `Scaler.d` sentinel handling → degenerate test 1613 catches. |
| `fast_domain_covers_realistic_stream_positions` (test) | 1549-1611 | REAL-UNPROVEN | — | Positive-space guard: 4h positions in 9 common bases must take fast tiers (1571-1589); extremes must fall back (1592-1594); 30-day warm-stage reduce-retry (1599-1610). Directly refutes "fast path never used" tautology. |
| `degenerate_literal_bases_still_match_the_reference` (test) | 1614-1651 | REAL-UNPROVEN | — | Zero-component struct literals (0/1, 1/0, 0/0, 2/0) × {0,±1,i64::MAX,MIN} — no panic, reference-identical. Guards the `div_round_i128` `denom <= 0 → None` contract (M18 regression named at 211-212). |
| `reduce_smooth_preserves_the_rational_exactly` (test) | 1654-1674 | REAL-UNPROVEN | — | u128 cross-multiplication invariance + 2/5-exhaustion + divisor-only removal, 100k random pairs. Mutation: divide only `n` → 1662 fails. |
| `five_pow_table_is_exact` (test) | 1677-1688 | REAL-UNPROVEN | — | Hard-coded table vs runtime `u64::pow` — a real check, not a tautology. |
| `m18_bench_10m_conversions` (test, `#[ignore]`) | 1715-1857 | REAL-UNPROVEN | — | Manual benchmark; 9 interleaved legs, min-of-5; black_box'd bases defeat LLVM constant-divisor folding (1721-1731) — methodologically sound; but never runs in any gate (F-003). Doc numbers (1701-1705) unverifiable. |

## 5. Test honesty
These ARE the tests. Meta-check applied:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 8 active tests | YES (by construction, unexecuted) | Each compares production paths against an independent oracle (`p/d` raw division; kept `i128` reference bodies at 443-483) plus coverage floors that fail if the fast path stops engaging. Reasoned through mutations listed per-row in §4; NOT verified by execution (F-001). |
| `m18_bench_10m_conversions` | N/A | `#[ignore]`d — never runs in `cargo test`; cannot catch anything automatically. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Measured (W3-06, Xeon, release, min-of-5 × 10M, ns/conv): … 1.60×…" | time.rs:1701-1705 comment | REAL-UNPROVEN | Methodology exists in-tree (1715-1857); no run artifact anywhere in repo/baseline; bench is #[ignore]d. |
| "this benchmark stays in-tree as the regression tripwire" | time.rs:1712 | SHALLOW | #[ignore] excludes it from every automated gate; tripwire only trips if manually invoked. |
| "seed printed on failure" | time.rs:1301, 1465 | FALSE | Failure messages (1311, 1484) print case values only, never the seed. |
| "The contract is enforced by the `fast_paths_*` / `scaler_*` proof tests at the bottom of this file" | time.rs:266-267 (part2) | REAL-UNPROVEN | Tests exist and are rigorous by inspection (this chunk), but were never executed in any recorded run (F-001). |

## 7. Dead / duplicated / unreachable code
- F-004 duplication (two `check` closures in `mulshift…`).
- `m18_bench_10m_conversions` is effectively dead in automation (#[ignore], no
  gate invokes `--ignored`).
- No unreachable production code in this chunk (it is all `#[cfg(test)]`).

## 8. Dependencies & flags
- `#[cfg(test)] mod tests` — std-only (Lcg hand-rolled at 908-921, comment
  cites "std-only crate, doc 01 §2.2(c)"); no proptest dependency — consistent
  with crate's zero-dep stance (verified: no dev-dependencies beyond testkit
  in crates/media-types/Cargo.toml:29).
- No feature flags gate this chunk; `cargo test -p media-types` compiles it
  unconditionally.

## 9. Open questions for the next agent
- W02-A02/A03 (parts 1/2): the production fast paths themselves
  (`try_fast_convert` 378-415, `try_fast_nanos` 423-438, `div_round_mulshift`
  359-366) — my chunk proves them *by test*, but since no run evidence exists
  (F-001), the proofs' guarantee is currently unbacked. A `REQUEST-ORCH-RUN`
  for `cargo test -p media-types --release --locked` is the cheapest way to
  convert this whole chunk to REAL.
- Who owns the fail-fast blind spot? It affects every crate sorting after
  `desktop` (media-clock, media-types, packet-queue, playback-engine,
  platform-*, …) — cross-cutting; recommend an orchestrator-level finding.
