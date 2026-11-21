# AUDIT: crates/streaming/src/dash/fuzz.rs (whole file)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Lines / functions | 237 / 8 (2 consts + 2 prod fns + 2 test helpers + 4 test fns; one helper is the SplitMix64 impl) |
| File verdict | UNREACHABLE (fuzz-harness library code; its consumers are the libFuzzer shim — not a shipped entry point — and the in-crate tests, which ran green; no shipped binary path reaches DASH at all) |
| Reachable from shipped entry point? | NO. `rg -n "dash::" apps/` → zero hits; `rg -n "dash::" crates/streaming/src --glob '!dash/**'` → zero hits — the whole DASH module (resolve/fuzz/abr/sched) is library-only, unwired into the CLI/tauri. The harness's real consumers: `fuzz/fuzz_targets/fuzz_demux_mpd.rs` (verified on disk; calls `streaming::dash::fuzz::fuzz_demux_mpd`) — a libFuzzer crate outside the workspace members, whose campaign needs the Lead's nightly wiring (fuzz/README posture; the baseline `fuzz-registry` gate is a registry-vs-implementation diff, "35 targets, diff empty", NOT a campaign run) — and the in-crate tests (green in baseline `orchrun-test-workspace-nofailfast.log.txt:4994-4997`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "dash::" apps/` + `rg -n "dash::" crates/streaming/src --glob '!dash/**'` (unwired) · `cat fuzz/fuzz_targets/fuzz_demux_mpd.rs` (shim exists, calls the harness) · `rg -n "MAX_PLAN_SEGMENTS" crates/streaming/src/dash/resolve.rs` (65,536 per-rep cap; with_capacity min-capped at 595/640) · baseline `orchrun-test-workspace-nofailfast.log.txt:4994-4997` (all 4 dash::fuzz tests ok) · `full-gate.log.txt:46-48` (fuzz-registry = diff check only) · `sed -n` quote verification |

## 1. One-paragraph verdict
The harness does what its contract says — one bounded parse-plus-resolve pass, panic-free, with global walk ceilings (32 representations, total-segments cap) that a dedicated test pins exactly (320 = 32×10 planned, reps 33-40 unvisited) — and all four tests ran green in the baseline. Two defects: the mutation sweep's "splice" operator is a no-op — it re-joins `source[..cut]` with `source[cut..]`, reconstructing the unmutated input, so a quarter of the advertised "600 structured mutations" are the original fixtures wearing a mutation costume; and the sweep's boundedness assert demands `planned <= WALK_TOTAL_SEGMENTS` while the walk only enforces that bound at representation boundaries, so a single rep can legally overshoot the total by up to `MAX_PLAN_SEGMENTS − 1` and the asserted "invariant" is stronger than the code's actual contract. The whole DASH surface, this harness included, is dead weight in the shipped binary's dependency graph — real, tested, linked to nothing.

## 2. What actually works (verified)
— All 4 `dash::fuzz::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:4994-4997` (release profile).
— Hand-traced the ceilings test (213-236): 40 representations × SegmentTemplate timescale=1 duration=10 over PT100S ⇒ 10 segments each; the loop-top check (65-67) processes reps 1-32 and returns at rep 33 → exactly 320 planned — pins both the rep ceiling and the exact stop position.
— Verified the walk's memory shape: `plan_representation`'s plan is dropped after `plan.segments.len()` is counted (69-78), so peak memory is one rep's plan, itself capped at `MAX_PLAN_SEGMENTS` = 65,536 segments with `Vec::with_capacity(len.min(MAX))` (resolve.rs:595, 640) and 16 MiB URL bytes (resolve.rs:49).
— Verified the splitmix64 implementation (105-113): the canonical constants (golden gamma, 0xBF58476D1CE4E5B9 / 0x94D049BB133111EB finals, 30/27/31 shifts) — correct deterministic PRNG.
— Verified the flip operator (177): xor mask is `rng % 255 + 1` ∈ [1, 255] — never a zero (no-op) mask.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/dash/fuzz.rs:181-187 (with the doc claim at 151-156)
Code:
```
                // Splice two halves of the round's source.
                2 => {
                    let cut = (rng.next() as usize) % (source.len() + 1);
                    let mut m = source[..cut].to_vec();
                    m.extend_from_slice(&source[cut..]);
                    m
                }
```
What is wrong: the "splice" mutation re-joins the round's source with itself at the cut point — `source[..cut] ++ source[cut..]` is byte-identical to `source`. It mutates nothing. Every round where `rng % 4 == 2` (expected 150 of 600) runs an unmutated fixture through the walk while the test's doc claims "600 splitmix64-driven structured mutations … truncate / byte-flip / splice / window". The operator was presumably meant to splice across the two fixtures (`base[..cut] ++ list[cut..]`), which is exactly why two fixtures are loaded (159-160) and alternated as `source`.
Why it matters: the sweep's mutation coverage is overstated by 25% — an entire operator class (structural recombination) silently never happens; `parsed_ok > 0` (206) is trivially satisfied by the no-ops, so the sweep can look meaningful while exercising less than advertised.
What correct looks like: `let other = if round % 2 == 0 { &list } else { &base }; let mut m = source[..cut].to_vec(); m.extend_from_slice(&other[cut.min(other.len())..]);`
Confidence: HIGH — Verification: read (the two slices are adjacent halves of the same `source`; identity is provable by concatenation).

### F-002 [S4]
Where: crates/streaming/src/dash/fuzz.rs:200 (assert) vs 65-67 (the enforced check)
Code:
```
            if let Some(planned) = walk(&mutant) {
                parsed_ok += 1;
                planned_overall += planned;
                assert!(planned <= WALK_TOTAL_SEGMENTS);
            }
```
What is wrong: the walk enforces its total-segments ceiling only at representation boundaries (65-67: the check precedes `reps += 1`), so the LAST representation processed can push the returned total up to `WALK_TOTAL_SEGMENTS − 1 + MAX_PLAN_SEGMENTS` (= 131,071, since `WALK_TOTAL_SEGMENTS == MAX_PLAN_SEGMENTS == 65,536`). The sweep asserts the tighter `planned <= WALK_TOTAL_SEGMENTS`, which is NOT an invariant the walk guarantees — a mutant that survives parsing and plans enough segments to land in the overshoot window would fail this assert spuriously, misreporting a harness-contract violation as a parser bug.
Why it matters: the assert encodes a bound the code does not enforce; it happens to hold for the committed fixtures (they plan few segments) but the mutation sweep is exactly the place an overshoot-adjacent mutant could appear.
What correct looks like: assert `planned < WALK_REPRESENTATIONS-cap-accumulated bound` (i.e. `planned <= (WALK_REPRESENTATIONS - 1) * MAX_PLAN_SEGMENTS + MAX_PLAN_SEGMENTS` — or simply `planned < 2 * WALK_TOTAL_SEGMENTS`), or fix the walk to re-check after each plan if the tighter bound is the intended contract.
Confidence: HIGH — Verification: read (both the loop-top check and the per-rep plan cap traced; overshoot arithmetic exact).
Note: same assert shape at 144-147 for the committed fixtures is safe (real fixtures plan ≪ the ceiling).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WALK_REPRESENTATIONS` (const) | 43-44 | REAL | — | 32 reps/pass. |
| `WALK_TOTAL_SEGMENTS` (const) | 46-47 | REAL | — | = MAX_PLAN_SEGMENTS (65,536); enforced at rep boundaries only (F-002). |
| `fuzz_demux_mpd` | 51-53 | REAL | — | the contract is absence-of-panic; result discarded by design (doc 12-16). |
| `walk` | 58-83 | REAL | — | parse → 3-level walk → per-rep plan → count; plan dropped per iteration (bounded peak memory); worst-case work ≈ 32 × 65,536 segment plans ≈ 2.1M — bounded but heavy per fuzz iteration (acceptable per the doc's "one clean repro" rationale). |
| `tests::fixture` | 91-100 | REAL | — | reads playlists-crate fixtures via CARGO_MANIFEST_DIR relative path; missing fixture PANICS (unreachable!) — loud, not silent. |
| `tests::SplitMix64` | 104-113 | REAL | — | canonical implementation, verified. |
| `empty_and_hostile_minima_survive` | 115-128 | REAL | — | absence-of-panic over 6 minima (incl. truncation, BOM garbage); per the documented contract. |
| `committed_fixtures_survive_within_ceilings` | 132-149 | REAL | — | 4 real fixtures parse and stay under the ceiling; `unreachable!` if a real fixture refuses. |
| `mutation_sweep_never_panics_and_stays_bounded` | 157-208 | REAL (with S3 F-001) | S3/S4 | 600 rounds; operator 2 is a no-op (F-001); assert over-tight (F-002); `parsed_ok > 0` meaningfulness guard present. |
| `walk_ceilings_stop_the_traversal` | 212-236 | REAL | — | exact 320 pin — the strongest test in the file. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_hostile_minima_survive | YES (for panics) | any panic in parse/resolve on the 6 minima fails it; it cannot catch wrong-but-calm behavior (by contract). |
| committed_fixtures_survive_within_ceilings | YES | `unreachable!` on a fixture that stops parsing (parser regression) + ceiling assert (resolver regression). |
| mutation_sweep_never_panics_and_stays_bounded | YES (weakened by F-001) | panics fail it; `parsed_ok > 0` requires at least one survivor; 25% of rounds are no-ops, so effective mutation pressure is 450, not 600. |
| walk_ceilings_stop_the_traversal | YES | exact `assert_eq!(planned, 320)` — an off-by-one in either ceiling or the segment math moves the number. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted pass … plus a bounded segment-plan resolution walk" | fuzz.rs:5-10 | REAL | walk:58-83 traced; tests green. |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:12-16 | REAL | errors swallowed at 59/69 by design; tests match. |
| "Reaching a ceiling stops the walk" | fuzz.rs:36-37 | REAL | loop-top check 65-67; pinned by the 320 test. |
| "600 splitmix64-driven structured mutations" | fuzz.rs:151-156 | SHALLOW | operator 2 is an identity — 25% of rounds unmutated (F-001). |
| "a bounded plan when the parse survives" (the sweep invariant) | fuzz.rs:155-156 + 200 | SHALLOW | the asserted bound is tighter than the enforced one (F-002). |
| cargo-fuzz target exists at fuzz/fuzz_targets/fuzz_demux_mpd.rs | fuzz.rs:20-21 | REAL | file verified on disk, calls this harness. |
| fuzz campaign status ("nightly wiring pending; proven on stable by the sweep") | fuzz.rs:18-24 | REAL (honest) | baseline has registry-diff only (full-gate:46-48), no campaign run; the stable sweep is the actual evidence and it ran green. |

## 7. Dead / duplicated / unreachable code
No dead code within the file. The enclosing `dash` module is entirely unwired into shipped binaries (header proof) — consistent with the module doc's "Phase 6" framing and the matrix's DASH rows (audited by the matrix-rows agents).

## 8. Dependencies & flags
Depends on `playlists::parse_mpd_bytes` (the cross-crate 6.C1a parser) and `super::resolve::{MAX_PLAN_SEGMENTS, plan_representation}`. No feature gates; no unsafe; no panics in the harness path (the `unreachable!`s are test-only). All 4 tests green in baseline.

## 9. Open questions for the next agent
- A07/A06 (resolve.rs parts): confirm `plan_representation`'s per-rep URL-byte budget (16 MiB) is enforced during template expansion, not just declared — the walk's memory bound leans on it.
- Whoever owns fuzz wiring: when the nightly campaign lands, the no-op splice (F-001) should be fixed first — the campaign would otherwise inherit the same blind spot the stable sweep has.
