# AUDIT: crates/demux/src/ivf/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 98 / 1 function (fuzz_demux_ivf) + 3 budget consts |
| File verdict | REAL (harness executed by the green ivf unit + integration suites) |
| Reachable from shipped entry point? | The harness is test/fuzz surface; the demuxer under test (`IvfDemuxer`) IS shipped (containers/mod.rs "ivf" arm). Cargo-fuzz target exists and calls it (fuzz/fuzz_targets/fuzz_demux_ivf.rs:19). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 'ivf'` -> 23 passed (includes the sweeps that drive this harness: ivf/tests.rs:501, 526-531); rg callers: ivf/tests.rs + tests/ivf_fixtures.rs:297,301 + fuzz target |

## 1. One-paragraph verdict
A clean, small fuzz harness in the house style (construct → start → bounded drain → 3 seek probes with bounded post-drain), and unlike the OGG twin its bounded-cost claims actually hold: the IVF walk is strictly positional (12-byte frame header + ceiling-checked body: `FRAME_CEILING_BYTES` 16 MiB typed refusal at ivf/mod.rs:457-460 before any allocation, pool capped 2×4 MiB), so there is no recovery-scan or candidate-amplification surface — the bytes-bounded budget genuinely bounds CPU. The harness is genuinely exercised: the ivf unit suite (23 green, my run) drives it with LCG flip sweeps plus an every-prefix truncation sweep and wild edges, the integration suite drives it over committed fixtures (ivf_fixtures.rs:297-301), and the cargo-fuzz target calls it verbatim. Only trivial dead-pattern nits.

## 2. What actually works (verified)
- 23/23 ivf tests pass (`./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 'ivf'`), including `fuzz_lite_mutations...` (LCG flips + determinism cross-check every 64 rounds, ivf/tests.rs:494-522) and `fuzz_lite_truncations_never_panic_or_hang` (every prefix of the demo seed + `[]`/`"DKIF"`/zero-header/`[0xFF;128]` edges, tests.rs:524-534).
- Bounds verified in the demuxer: `MAX_FRAME_BYTES = 4 MiB` pool (mod.rs:118, 338), `FRAME_CEILING_BYTES = 16 MiB` refusal before allocation (mod.rs:457-460) — no unbounded allocation from header values in the fuzz surface.
- Doc's wiring story verified: fuzz/fuzz_targets/fuzz_demux_ivf.rs exists and calls `demux::ivf::fuzz::fuzz_demux_ivf(data)` (line 19).

## 3. Findings

F-001 [S4]
Where: crates/demux/src/ivf/fuzz.rs:79-82
Code:
```rust
    if let Ok(base) = TimeBase::new(1, 30) {
        for fraction in 1..=FUZZ_SEEK_PROBES {
            let ticks = last_ticks.saturating_mul(fraction as i64)
                / i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3);
```
What is wrong: two dead-pattern guards — `TimeBase::new(1, 30)` cannot fail (nonzero coprime inputs), and `i64::try_from(FUZZ_SEEK_PROBES)` with the constant 3 cannot fail, so `unwrap_or(3)` masks a compile-time-known value. Pure hygiene; no behavioral risk (same pattern as the ogg twin). Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

Zero S0–S3 findings. Checked and how: (a) panic surface — none in the harness (no unwrap/expect/index; all Results matched); (b) hang surface — every loop has a hard cap (FUZZ_EVENT_BUDGET 500k, FUZZ_POST_SEEK_DRAIN 64, FUZZ_SEEK_PROBES 3) and the underlying walk is positional/monotone (verified in mod.rs, unlike OGG's recovery paths); (c) allocation surface — pool + ceiling verified at mod.rs:118/122/338/457-460; (d) reachability of the harness itself — called from unit tests (ran green), integration tests (baseline: ivf_fixtures 5 tests), and the cargo-fuzz target; (e) contract honesty — the module doc's "absence-of-panic/hang/unbounded-allocation, nothing else" matches the body exactly (typed errors swallowed at 52-57, 70, 87-89).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_demux_ivf | 49-98 | REAL | S4 | construct/start/drain/seek-probe contract; executed green via suites; F-001 dead guards |

## 5. Test honesty
The tests that drive this harness (another agent's ivf/tests.rs, ivf_fixtures.rs) do assert more than no-panic where it matters (the determinism cross-check at tests.rs:507-522 pins identical terminal codes across two runs). The harness itself asserts nothing by design — documented.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the harness logic is proven by the deterministic in-crate sweeps ... run by the unit + integration suites" | fuzz.rs:16-19 | REAL | rg: ivf/tests.rs:501,526-531; tests/ivf_fixtures.rs:297,301; both ran green (mine + baseline) |
| "Packet memory by MAX_FRAME_BYTES (pool) and FRAME_CEILING_BYTES (typed refusal before any allocation)" | fuzz.rs:26-29 | REAL | mod.rs:118/122/338/457-460 |
| "the walk is strictly positional and monotone" | fuzz.rs:23-25 | REAL | IVF frame walk verified positional (no recovery/resync loop in mod.rs's read path) |

## 7. Dead / duplicated / unreachable code
Structural twin of ogg/fuzz.rs and adts fuzz harnesses (documented duplication, house style). The `Ok(base)`/`unwrap_or(3)` guards are dead (F-001). No unreachable code.

## 8. Dependencies & flags
`super::IvfDemuxer`, `super::test_support::mem_stream`, `media_types::time`, `plugin_api`. No features. Part of the demux lib (compiled release lane).

## 9. Open questions for the next agent
- ivf/mod.rs itself is another agent's file: the 16 MiB FRAME_CEILING refusal is sticky-fatal — same policy question as OGG's 256 KiB packet cap (legal-but-large frames hard-fail rather than damage-note); a >4 MiB VP9 keyframe is real (though rare).
- The `TimeBase::new(1, 30)` probe base assumes 30 fps targets — the demuxer re-maps to its own base, so this only shapes probe distribution; no action needed.
