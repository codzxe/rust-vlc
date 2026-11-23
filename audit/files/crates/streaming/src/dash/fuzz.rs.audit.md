# AUDIT: crates/streaming/src/dash/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 237 / 2 prod fns (fuzz_demux_mpd, walk) + 4 tests |
| File verdict | REAL — the harness exists, is wired to a registered cargo-fuzz target, and its 4 in-crate tests execute (I ran them); one of the four mutation operators is a no-op (identity splice) and the "global segments" ceiling can be overshot by up to one per-rep budget |
| Reachable from shipped entry point? | N/A-by-design for the binary (fuzz harness); the harness itself IS reachable from the registered fuzz target fuzz/fuzz_targets/fuzz_demux_mpd.rs (`fuzz_target!(… streaming::dash::fuzz::fuzz_demux_mpd(data))`, registered at fuzz/Cargo.toml:293-294). Actual libFuzzer campaigns need Lead nightly wiring (disclosed); until then the logic runs via the deterministic sweep. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/streaming-032d0d746c2d060d dash::fuzz` → 4 passed exit 0 (same on streaming-c6cf1f9f1d2bf8f0); ls fuzz/fuzz_targets + crates/playlists/tests/fixtures/dash (4 MPDs present); sed quote verification |

## 1. One-paragraph verdict
A real, bounded fuzz harness: one pass = full untrusted `parse_mpd_bytes` + a ceiling-capped resolution walk over every representation, with the honest contract that it pins absence-of-panic/hang/unbounded-alloc only (specific outcomes live in the resolver suites). The cargo-fuzz shim exists and is registered, so the `+FB` acceptance criterion has a genuine target — though no libFuzzer campaign has actually run (nightly wiring pending, disclosed in both this file and the shim). I executed the 4 in-crate tests: hostile minima, 4 committed fixtures, a 600-round mutation sweep, and a ceiling-enforcement proof (320 == 32 reps × 10) — all pass. Two defects: the sweep's "splice" operator re-concatenates the SAME source at the SAME cut, which is the identity function (25% of the 600 rounds mutate nothing), and the check-then-act ceiling lets the returned segment total overshoot `WALK_TOTAL_SEGMENTS` by up to one per-rep budget (harness accounting only — time/memory stay bounded by the resolver's own budgets).

## 2. What actually works (verified)
— verified by `./target/debug/deps/streaming-032d0d746c2d060d dash::fuzz` → `test result: ok. 4 passed` (exit 0, 0.08s): empty/hostile inputs survive, all 4 committed fixtures parse and plan within ceilings, 600 deterministic mutations produce zero panics with ≥1 parse survivor (sweep is non-vacuous), and the 40-rep synthetic manifest stops at exactly 320 planned segments.

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/dash/fuzz.rs:181-187
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
What is wrong: this "mutation" takes `source[..cut]` and appends `source[cut..]` — the two halves of the SAME buffer in the ORIGINAL order — which reassembles `source` byte-for-byte. It is the identity function. The obvious intent (splicing halves of the two different fixtures, base and list) is not what the code does. Consequence: rounds where `rng % 4 == 2` (≈150 of 600) feed the pristine fixture to the harness, diluting the sweep's diversity while the test's framing claims four mutation classes (truncate / byte-flip / splice / window). The sweep still exercises 3 real operators, and the invariant assertions still hold — but the campaign is weaker than documented. What correct looks like: `let other = if source.ptr_eq(&list) { &base } else { &list }; m.extend_from_slice(&other[cut..]);` or any cross-source splice.
Confidence: HIGH — Verification: read (identity is provable by inspection; not executed separately)

F-002 [S4]
Where: crates/streaming/src/dash/fuzz.rs:65-67 vs doc 32-35
Code:
```
                if reps >= WALK_REPRESENTATIONS || total_segments >= WALK_TOTAL_SEGMENTS {
                    return Some(total_segments);
                }
```
What is wrong: the ceiling is checked BEFORE planning a representation, but one representation may plan up to `MAX_PLAN_SEGMENTS` segments; entering the check with `total_segments == MAX-1` and planning a full-budget rep returns up to `2 × MAX_PLAN_SEGMENTS − 1` — the doc's "global ceilings — [`WALK_TOTAL_SEGMENTS`] segments planned across the whole manifest" is not a hard cap on the planned total. Blast radius: harness accounting only (the per-rep resolver budgets and the 32-rep cap keep time and memory bounded regardless); the in-test assertion `planned <= WALK_TOTAL_SEGMENTS` (L200) holds today only because no mutant/fixture happens to plan that much.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_demux_mpd | 51-53 | REAL | — | entry; errors swallowed by documented contract |
| walk | 58-83 | REAL | S4 | bounded parse+resolve; F-002 check-then-act overshoot |
| fixture (test helper) | 91-100 | REAL | — | panics loudly if a fixture is missing (unlike probe.rs's silent skips) — correct posture |
| SplitMix64 | 104-113 | REAL | — | standard splitmix64 |
| empty_and_hostile_minima_survive | 116-128 | REAL | — | no-panic pins (weak by design, disclosed) |
| committed_fixtures_survive_within_ceilings | 133-149 | REAL | — | 4 real MPDs |
| mutation_sweep_never_panics_and_stays_bounded | 158-208 | REAL | S3 | F-001: splice operator is identity |
| walk_ceilings_stop_the_traversal | 213-236 | REAL | — | exact 320 bound proves the rep cap bites |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_hostile_minima_survive | PARTIAL | absence-of-panic only (the stated contract) |
| committed_fixtures_survive_within_ceilings | YES | parse must succeed + plan within bound (loud panic on missing fixture) |
| mutation_sweep_never_panics_and_stays_bounded | PARTIAL | no-panic + non-vacuity (`parsed_ok > 0`); 1 of 4 operators is a no-op (F-001) |
| walk_ceilings_stop_the_traversal | YES | exact count 320 pins both ceilings |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted pass through parse_mpd_bytes … plus a bounded segment-plan resolution walk" | fuzz.rs:5-10 | REAL | ran walk via the 4 tests |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:12-14 | REAL (honest scoping) | |
| "The cargo-fuzz target lives at fuzz/fuzz_targets/fuzz_demux_mpd.rs" | fuzz.rs:20-21 | REAL | file exists, registered at fuzz/Cargo.toml:293-294 |
| "global ceilings — WALK_TOTAL_SEGMENTS segments planned across the whole manifest" | fuzz.rs:32-34 | FALSE-at-the-margin | F-002: total can overshoot by < one per-rep budget |
| "600 splitmix64-driven structured mutations" | fuzz.rs:152-154 | OVERSTATED | F-001: ~150 of the 600 are identity no-ops |

## 7. Dead / duplicated / unreachable code
The identity splice arm (F-001) is dead weight in the sweep. No other duplication.

## 8. Dependencies & flags
playlists (parse_mpd_bytes), super::resolve (plan_representation, MAX_PLAN_SEGMENTS). No features. The fuzz/ package compiles on stable per the shim's note (UNVERIFIED — I did not run cargo check in fuzz/ per load discipline).

## 9. Open questions for the next agent
- Has ANY libFuzzer campaign actually run on fuzz_demux_mpd (corpus artifacts in fuzz/corpus/)? I see fuzz/corpus/ referenced in fuzz/Cargo.toml comments but did not verify target corpus contents.
- Fix F-001 before trusting sweep-coverage numbers in any gate doc.
