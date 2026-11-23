# AUDIT: crates/streaming/tests/netfuzz/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 163 / 2 harnesses + 2 helpers + 2 consts |
| File verdict | REAL (harness bodies; executed via the stable sweep — 6000 total iterations pass; historically found a real overflow that was fixed and unit-pinned) |
| Reachable from shipped entry point? | N/A (test harness). Its two consumers: the stable sweep (`netfuzz_sweep.rs:67-68`) and the libFuzzer shims `fuzz/fuzz_targets/fuzz_hls_playlist.rs:35-36` + `fuzz_http_headers.rs:44-45` — both verified to `#[path]`-include this exact file, so the "included twice, verbatim, never rewritten" claim (:22-23) is TRUE. The surfaces it drives are shipped library code (playlists parse, hls::master/timestamp_map, mockhttp grammar, fetch::ByteRange). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | via `./target/debug/deps/netfuzz_sweep-179086f8a3bb1747 --nocapture` -> 2 passed (3000 iters/target); shims verified by read (grep `#[path]` in both fuzz_targets) |

## 1. One-paragraph verdict
The two harness bodies are real and correctly scoped: HLS interpretation passes over lossy-decoded hostile text (media model touches, master model + ABR pick at four bandwidth extremes including u64::MAX, the X-TIMESTAMP-MAP wall-clock surface with u64 PTS probes) and the HTTP-header pass (the socket path's real response-head grammar + Range value formatting under hostile u64 pairs). The file's history is its best evidence: it found the `pts_for_wallclock_ms` i64 multiply overflow (recorded :68-75, 2026-11-09), the fix landed in `timestamp_map.rs:133-136` (saturating_sub/saturating_mul) and is pinned by exact-value unit tests at `timestamp_map.rs:300-333` — a complete, verifiable fuzz→fix→test chain. The defects: the documented contract "Malformed input must surface as a typed error or a damage warning" is unenforceable by this harness because every Result/Option is discarded (:34-36 vs :91-121); and the WALL_PROBE_ROAM comment is now stale — it says the overflow is "for the follow-up (src is outside this wave's ownership)" but the follow-up HAS landed, and the clamp now also prevents the harness from ever exercising the fix's saturation branches (roam is capped at exactly half the saturation threshold).

## 2. What actually works (verified)
— Executed through the sweep: 3000 mutated HLS inputs + 3000 header inputs, no panics (`./target/debug/deps/netfuzz_sweep-179086f8a3bb1747`, "2 passed").
— Fuzz→fix→test chain: fuzz.rs:68-75 records the finding; `timestamp_map.rs:133-136` `let delta_ms = wallclock_ms.saturating_sub(self.local_ms); let delta_ticks = delta_ms.saturating_mul(PTS_HZ as i64) / 1000;`; unit test `far_from_anchor_wallclock_saturates_instead_of_overflowing` (:300-333) pins exact saturated outputs (7_078_106_103 at both i64::MAX/1000 and i64::MAX).
— Verbatim double-inclusion verified: `#[path = "../../crates/streaming/tests/netfuzz/fuzz.rs"] mod netfuzz;` in BOTH fuzz_targets shims (grep, lines 35/44).

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/netfuzz/fuzz.rs:34-36
Code (verbatim, 4 lines, :34-37):

        //! No panic, no hang, no unbounded allocation. Malformed input must
        //! surface as a typed error or a damage warning — every `Result` and
        //! `Option` below is discarded exactly because the *verdict* is the
        //! invariant, not the value. Integer overflow while walking attacker
What is wrong: the paragraph asserts a contract ("malformed input must surface as typed error or damage warning") that the harness cannot falsify — every verdict is discarded (:91 `if let Ok(...)`, :104 `let _ = select_variant_index(...)`, :113/:120 `let _ = map.wallclock_ms_for_pts(pts)` / `let _ = map.pts_for_wallclock_ms(wall)`, :132 `let _ = fuzz_response_head(data)`, :144-145 `let _ = ...to_header_value()`). A regression replacing any typed error with a silent wrong-value `Ok` passes this harness forever. The honest sentence is the first one (survival); the second is aspiration written as contract. Why it matters: this file's doc is the quoted contract for both the stable sweep and the libFuzzer campaign; evidence consumers inherit the overstatement. What correct looks like: state "survival only — verdicts are not checked here" and pin the typed-error requirement in a separate assertion-bearing test (the cue_suite fuzz shows the shape). Confidence: HIGH — Verification: read + ran.
F-002 [S4]
Where: crates/streaming/tests/netfuzz/fuzz.rs:70-75
Code (verbatim, 6 lines):

        /// **Recorded finding (2026-11-09, this sweep)**: probing with a wall
        /// value further than ~10^14 ms from the anchor overflows the
        /// `(wallclock_ms - local_ms) * 90_000` intermediate — a panic under
        /// `overflow-checks`, a silent wrap without them. The mapping's own
        /// doc contracts it for values near anchors; the fuzz harness stays
        /// inside the arithmetic's safe domain and pins that boundary here
        /// for the follow-up (src is outside this wave's ownership).
What is wrong: stale on two counts. (a) The follow-up HAS landed — `timestamp_map.rs:133-136` saturates and :300-333 pins it; "pins that boundary here for the follow-up" describes a state that no longer exists. (b) The clamp `WALL_PROBE_ROAM = i64::MAX / (2 * 90_000)` (:75) caps probes at exactly HALF the saturation threshold (`i64::MAX / 90_000`), so the harness can never exercise the fix's new `saturating_mul` saturation branch — the code the finding motivated is untested by the fuzzer that found it (it IS unit-tested; this is about the fuzz lane's coverage honesty). Confidence: HIGH — Verification: read + arithmetic (5.1e13 < 1.02e14).
F-003 [S4]
Where: crates/streaming/tests/netfuzz/fuzz.rs:112-120
Code: `let pts = read_u64(data, 0);\n        let _ = map.wallclock_ms_for_pts(pts);\n        // Wall-clock probes: roamed around the tag's own anchor (see\n        // WALL_PROBE_ROAM — the inverse map's i64 arithmetic is only\n        // safe within that neighborhood of the anchor, which is also\n        // far past its ±13.25 h contract).\n        let roam = read_i64(data, 8).clamp(-WALL_PROBE_ROAM, WALL_PROBE_ROAM);`
What is wrong: probe values come only from bytes 0..16 of the input — for the timestamp-map surface, the vast majority of a typical input influences only the tag *text*, never the probe numbers. Combined with F-002's clamp the inverse-map probing is near-inert. Hygiene; the forward map (`wallclock_ms_for_pts`, any u64) is properly unbounded. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_hls_playlist | 82-122 | REAL | S3/S4 | three passes: media model, master+ABR (4 bandwidths incl. u64::MAX), timestamp-map both directions; verdicts discarded (F-001), probe near-inert (F-002/F-003) |
| fuzz_http_headers | 127-146 | REAL | S3 | real response-head grammar + Range formatting under hostile u64; verdicts discarded |
| read_u64 / read_i64 | 150-163 | REAL | — | zero-padded LE probes, no panic surface |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (as fuzz harnesses: panic detection) | YES (panic only) | the 2026-11-09 overflow proves panic detection works in anger |
| (typed-error contract) | NO | all verdicts discarded (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "It is #[path]-included twice, verbatim, never rewritten" | fuzz.rs:22-23 | REAL | both shims verified (fuzz_targets :35-36, :44-45) |
| "Malformed input must surface as a typed error or a damage warning" | fuzz.rs:34-35 | UNENFORCED | every Result/Option discarded (:91-145) — F-001 |
| "Inputs over HLS_INPUT_CAP are exercised as their prefix" | fuzz.rs:45-46 | REAL | `&data[..data.len().min(HLS_INPUT_CAP)]` (:83) |
| "The response-head parse is capped at the transport's own 64 KiB head budget inside fuzz_response_head" | fuzz.rs:48-50 | REAL-UNPROVEN (mockhttp::fuzz_response_head not my file; the sweep's over-limit flavor exercises the branch and survives) | |
| "the fuzz harness stays inside the arithmetic's safe domain and pins that boundary here for the follow-up" | fuzz.rs:73-75 | STALE | follow-up landed (timestamp_map.rs:133-136) — F-002 |

## 7. Dead / duplicated / unreachable code
`#![allow(dead_code)]` (:56) is justified and documented: each shim binary uses only its twin, the sweep uses both (:52-55 comment — verified accurate). No other dead code.

## 8. Dependencies & flags
No deps beyond the crates under test (playlists, streaming). Release fuzz keeps `overflow-checks` per :38-40 (fuzz/ profile — not independently verified, UNVERIFIED: fuzz/Cargo.toml profile not read).

## 9. Open questions for the next agent
- mockhttp::fuzz_response_head — is the 64 KiB cap and the typed over-limit branch exactly as described? (mockhttp.rs audit owner)
- fuzz/Cargo.toml: confirm `[profile.release] overflow-checks = true` for the campaign — the harness doc's integer-overflow safety claim depends on it. UNVERIFIED.
