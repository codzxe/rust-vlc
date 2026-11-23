# AUDIT: crates/streaming/tests/netfuzz_sweep.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 409 / 9 prod helpers + 4 tests (2 default, 2 `#[ignore]`d 60s) |
| File verdict | REAL (test harness; executed — 3000 iterations × 2 targets, pass; deterministic) — with two contract-overclaim defects (F-001/F-002) |
| Reachable from shipped entry point? | N/A (test file). The harness bodies it drives (`netfuzz/fuzz.rs`, `playlists::fuzz`) exercise shipped-code surfaces: `playlists::parse_media_playlist`/`parse_master_playlist`, `streaming::hls::master::select_variant_index`, `streaming::hls::timestamp_map`, `streaming::mockhttp::fuzz_response_head`, `streaming::fetch::ByteRange`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/netfuzz_sweep-179086f8a3bb1747 --nocapture` -> "2 passed; 2 ignored" (3000 iters/target: headers 902ms, HLS 1202ms), exit 0 |

## 1. One-paragraph verdict
A competent, honest-in-most-part stable-toolchain fuzz runner: it reuses the libFuzzer harness bodies verbatim (`#[path]`-included, never rewritten), drives them with a deterministic xorshift64* generator over REAL committed fixture seeds through six mutation flavors (bit flips, truncations, hostile tag/line soup splices, head+tail, over-limit), and prints per-run iteration counts. Its historical value is real: this sweep found the `pts_for_wallclock_ms` i64 overflow (recorded 2026-11-09 in the harness comment), which was subsequently FIXED in `timestamp_map.rs:133-136` with saturating arithmetic and pinned by a dedicated test (`far_from_anchor_wallclock_saturates_instead_of_overflowing`, timestamp_map.rs:300-333) — a complete fuzz→fix→test chain I verified end to end. The defects: the module doc claims "damage surfaces as typed errors or damage warnings inside the harness (D-3/D-9)" but the harness discards every Result/Option — a regression that swapped a typed error for a wrong success value would pass forever; and the "no hang" half of the survival contract has no detection mechanism at all (a hung parser hangs the test indefinitely; no watchdog).

## 2. What actually works (verified)
— `./target/debug/deps/netfuzz_sweep-179086f8a3bb1747 --nocapture` -> `netfuzz-sweep: fuzz_http_headers 3000 iterations in 902.543564ms` + `netfuzz-sweep: fuzz_hls_playlist 3000 iterations in 1.202523749s`, "test result: ok. 2 passed; 2 ignored", exit 0.
— Fuzz→fix→test chain verified by read: netfuzz/fuzz.rs:68-75 records the overflow finding; timestamp_map.rs:119-137 implements `saturating_sub`/`saturating_mul`; timestamp_map.rs:300-333 pins exact saturated outputs at i64::MIN/MAX.
— Determinism: fixed seeds `HLS_SEED = 0x5eed_0000_0000_0e14`, `HEAD_SEED = 0x5eed_0000_0000_0e1f` (netfuzz_sweep.rs:343-344) — reproducible.

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/netfuzz_sweep.rs:13-15
Code: `//! bytes. The contract is the harness's own: survival (no panic, no\n//! hang, no unbounded allocation); damage surfaces as typed errors or\n//! damage warnings inside the harness (D-3/D-9).`
What is wrong: the second sentence is not enforced by anything. The harness bodies discard every `Result`/`Option` (netfuzz/fuzz.rs:91-121 `if let Ok(...)`, :132 `let _ = streaming::mockhttp::fuzz_response_head(data)`), and the sweep's loop is `(target.passes)(&input)` — the only observable failure modes are panic, OOM, or hang. If `parse_media_playlist` regressed to returning a typed error for every input (or worse, a silently-wrong Ok), the sweep stays green. The claim "damage surfaces as typed errors … inside the harness" describes what the code under test is *supposed* to do, not what the harness *checks*. Why it matters: evidence strings built on this sweep (docs/formats/streaming-hls.md network-scenarios) inherit an overstated contract. What correct looks like: either delete the sentence or add spot-asserts (e.g. at least one input per run must produce a typed error) — the cue_suite.rs fuzz already shows the stronger shape (invariant asserts on every successful parse). Confidence: HIGH — Verification: read + ran.
F-002 [S3]
Where: crates/streaming/tests/netfuzz_sweep.rs:311-326
Code: `fn sweep(target: &Target, limit: Limit, seed: u64) -> (u64, Duration) {\n    let started = Instant::now();\n    let mut rng = Rng::new(seed);\n    let mut iteration = 0u64;\n    loop {\n        match limit {\n            Limit::Iters(max) if iteration >= max => break,\n            Limit::Time(budget) if started.elapsed() >= budget => break,\n            _ => {}\n        }`
What is wrong: "no hang" is claimed as part of the survival contract (:13) but there is no per-input or per-run watchdog — an input that sends the parser into an infinite loop simply never returns, the test hangs indefinitely (cargo has no per-test timeout), and CI either times out at the job level with an unhelpful signal or hangs. The 60s `Limit::Time` budget bounds only *willing* returns; it cannot preempt a stuck call. What correct looks like: run passes on a helper thread and fail if an iteration exceeds a per-input deadline (the demux/codecs sweeps this file claims as twins have the same gap — cross-file note, not verified here). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/streaming/tests/netfuzz_sweep.rs:391-394
Code: `fn netfuzz_sweep_hls_playlist_60s() {\n    let target = hls_target();\n    let (iterations, elapsed) = sweep(&target, Limit::Time(SWEEP_BUDGET), HLS_SEED);\n    assert!(elapsed >= SWEEP_BUDGET, "the budget must be honored");`
What is wrong: tautological assert — for `Limit::Time`, `elapsed >= budget` is the loop's ONLY exit condition, so the assertion cannot fail (same in the headers twin at :402-405). It documents intent but proves nothing. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Rng::new/next_u64/below/byte | 82-113 | REAL | — | xorshift64* + splitmix64 decorrelator, zero-state immune, div-by-zero guarded |
| hls_seeds | 121-137 | REAL | — | reads the 6 committed real fixtures cross-crate (paths verified to exist — see fixtures audits) |
| header_seeds | 142-159 | REAL | — | 5 authored heads + 1 cross-format real playlist |
| HLS_SOUP / HEAD_SOUP | 163-197 | REAL | — | genuinely hostile shapes (overflow numbers, control bytes, invalid UTF-8 `\xC3\x28`) |
| Target / generate | 204-289 | REAL | — | 6 flavors incl. deterministic over-limit (+257 B past 64 KiB) every 64th iteration |
| rng_pick | 293-299 | REAL | — | empty-seed case answers empty input |
| Limit / sweep | 301-326 | REAL | S3 | F-002 no hang watchdog |
| hls_target / head_target | 346-371 | REAL | — | harness bodies verbatim (`playlists::fuzz::fuzz_playlist_m3u` + `fuzz_hls_playlist`; `fuzz_http_headers`) |
| netfuzz_sweep_hls_playlist_survives | 373-379 | REAL | — | executed, 3000 iters |
| netfuzz_sweep_http_headers_survives | 381-387 | REAL | — | executed, 3000 iters |
| *_60s (ignored ×2) | 389-409 | REAL | S4 | documented re-run command; F-003 tautological budget assert |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| netfuzz_sweep_hls_playlist_survives | YES (panic only) | 3000 deterministic mutated inputs through the real parsers; a panic fails the test. Would NOT fail on wrong-value or hang regressions (F-001/F-002) |
| netfuzz_sweep_http_headers_survives | YES (panic only) | same |
| *_60s (ignored) | N/A in default lane | documented opt-in; the assert is tautological (F-003) |
Historical honesty: this runner's harness DID catch a real overflow once (the 2026-11-09 recording), so the panic-detection half is proven-in-anger, not theoretical.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "It runs the SAME harness bodies the libFuzzer shims run — the #[path]-included netfuzz/fuzz.rs module … reused verbatim, never rewritten" | netfuzz_sweep.rs:8-11 | REAL | `#[path = "netfuzz/fuzz.rs"] mod netfuzz;` (:67-68); fuzz/fuzz_targets shims verified to exist (fuzz.rs:24-26 comment; shims not in my assignment) |
| "survival (no panic, no hang, no unbounded allocation); damage surfaces as typed errors or damage warnings" | netfuzz_sweep.rs:13-15 | PARTIAL/FALSE-in-part | panic: enforced; unbounded alloc: only via the 64 KiB caps + machine OOM; hang: no mechanism (F-002); typed errors: never checked (F-001) |
| "Fixed seeds … any survival regression is exactly reproducible" | netfuzz_sweep.rs:59-60 | REAL | seeds are consts (:343-344) |
| "the #[ignore]d long variants run a fixed 60-second budget each and print their ACTUAL iteration count" | netfuzz_sweep.rs:53-55 | REAL | verified by read (println with iterations + elapsed) |

## 7. Dead / duplicated / unreachable code
None. The generator is documented as a near-verbatim twin of the demux/codecs sweep generators (acceptable duplication for test isolation; noted, cross-file).

## 8. Dependencies & flags
std + testkit + playlists + streaming (dev-deps). No new RNG dependency (C5 honored). No feature gates. `#![allow(dead_code)]` lives in the included fuzz.rs (justified: each libFuzzer shim binary uses only its twin — verified comment fuzz.rs:52-56).

## 9. Open questions for the next agent
- fuzz/fuzz_targets/*.rs (libFuzzer shims) — not my assignment: confirm they call the same twins and keep an in-crate gate call first, as the comment promises.
- The demux/codecs `fuzz_loop_stable.rs` twins: same no-watchdog gap as F-002? (cross-file, unverified here)
- docs/formats/streaming-hls.md network-scenarios: do the recorded ACTUAL iteration counts match a fresh 60s run on this machine? UNVERIFIED — I did not run the ignored 60s variants (budget discipline; ~2 min total, run them if the orchestrator wants the recorded-runs cross-check).
