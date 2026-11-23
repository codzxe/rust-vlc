# AUDIT: crates/media-types/src/time.rs
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-23 |
| Lines / functions | 1858 / 40 production items + 24 tests (4 test helpers) |
| File verdict | REAL (core conversions) with an UNREACHABLE ~150-line public sub-API (Scaler tier) |
| Reachable from shipped entry point? | YES for TimeBase/Timestamp/convert/as_nanos/Stamp/Duration/Rate; NO for the entire Scaler tier |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `cargo test -p media-types time::` (exit 0; 23 passed/0 failed/1 ignored); `cargo test -p media-types --release -- --ignored --nocapture m18_bench` (exit 0; full table below); baseline `audit/runs/baseline/cargo-test-workspace.txt` PACKAGE media-types: 76 passed/0 failed/1 ignored |

## 1. One-paragraph verdict
The timing math is real and unusually well-defended: every conversion is an exact rational
rescale with checked arithmetic, ties-away-from-zero rounding, no panics on any input
(degenerate struct-literal bases are typed errors, not crashes — and struct-literal bases are a
genuine in-tree caller shape, 15+ sites in demux/media-clock), and the M18 fast paths are held
to bit-identity with a kept i128 oracle by proof tests covering >1M cases which I executed and
which pass. The file's fraud is architectural, not arithmetic: the entire Tier-1 `Scaler`
subsystem (6 public methods, ~150 lines plus ~200 lines of proof tests) has ZERO production
callers — the module docs claim "Hot loops (video scheduler, playback pipeline) hold one
`Scaler`" and none do — so M18's headline 1.60× win is unreachable in the shipped build, while
the production per-stamp path gets parity at best (measured 3.66 vs 3.42 ns on this box). A
cluster of smaller doc lies (inverted "ticks per second" unit doc, an `# Errors` section on an
infallible constructor) and a test-only API tail (`FromStr`, `Duration::display`,
`as_secs_exact`) round it out.

## 2. What actually works (verified)
— `cargo test -p media-types time::` -> 23 passed, 0 failed, 1 ignored (m18 bench), 0.73s. All
fast-path equivalence proofs pass: `convert_fast_path_is_bit_identical_to_reference`,
`as_nanos_fast_path_is_bit_identical_to_reference`, `scaler_is_bit_identical_to_reference`,
`mulshift_magic_is_bit_exact_for_small_divisors` (>1.8M cases), `reduce_smooth_preserves_the_rational_exactly`.
— `cargo test -p media-types --release -- --ignored --nocapture m18_bench` -> reproduces the
in-tree numbers on this 2-core box: 90kHz→ns reference 3.42 ns, Tier-2 3.66 ns, Scaler 2.14 ns
(1.60×); 90kHz→ms 3.97/3.93/3.71; 10y→µs 4.24/3.94/3.74. The module-doc benchmark claims
(2.10 vs 3.37, 1.60×) are honest measurements.
— Math independently re-derived (not just trusted): conversion formula
`ticks·from.num·to.den / (from.den·to.num)` is correct for tick=num/den seconds; magnitude+
sign decomposition equals ties-away-from-zero on the signed i128 path (checked sign cases);
u64 products `(2^32−1)² < 2^64` cannot overflow; i128 products `< 2^127` cannot overflow;
`magic_u64` exactness bound `p < 2^k/(d−1)` holds because `k = 63+bitlen(d)`; i64::MIN always
falls back to the reference (unsigned_abs · n ≥ 2^63 always); denominator ≤ 0 → typed
Overflow, never a divide-by-zero panic.
— Production reachability verified by rg: `Timestamp::convert` called from
crates/demux/src/{wav,asf,mkv,ps,ivf,aiff,avi,ogg}.rs, crates/video-engine/src/scheduler.rs:721,
crates/media-clock/src/mapping.rs, apps/cli/src/containers/*.rs, apps/web/src/{host,open}.rs;
`as_nanos` from demux (mp4/mod.rs, adts, wav, aiff, avi, ogg), subtitles (pgs, dvb,
compositing), playback-engine (pipeline.rs, seek.rs, session.rs); `Rate::new/as_parts` from
media-clock (drift.rs, mapping.rs); `Duration::nanos` from media-clock/src/mapping.rs:234;
`TimeError::Discontinuity` constructed in media-clock/src/clock.rs (production, not
paper-only); `TrackTiming`/`TimeSpan::UNKNOWN`/`Stamp::timestamp` used across demux.
— Degenerate-base defense is justified, not paranoia: `TimeBase { num: 1, den: rate }` struct
literals exist in demux production (adts/mod.rs:626, es/annexb.rs:421, ogg/mod.rs:203,
ts/mod.rs:961, ivf/mod.rs:547, media-clock/src/mapping.rs:50 NANO_BASE).

## 3. Findings
F-001 [S2]
Where: crates/media-types/src/time.rs:493-497 (and 30-32, 48, 170-171, 591)
Code:
```
/// sympathy). Hot loops (video scheduler, playback pipeline) hold one
/// `Scaler` next to their base pair instead and convert by
/// multiply+shift alone — zero division instructions.
```
What is wrong: The doc asserts the video scheduler and playback pipeline hold a `Scaler`. They
do not. `rg '\.scaler_to\(|nanos_scaler\(' --glob '**/src/**/*.rs'` over the whole repo returns
exactly one file: this one (the definitions). `rg 'Scaler'` adds only test files
(crates/playback-engine/tests/property/timestamp_arithmetic.rs, crates/audio-output/tests).
Zero production callers exist for `scaler_to`, `nanos_scaler`, `Scaler::convert_ticks`,
`Scaler::nanos`, `Scaler::from_base`, `Scaler::target_base`. Why it matters: M18's only
measured win (1.60× on "the dominant per-frame shape") is a batch tier no shipped code path
reaches; the per-stamp production tier delivers parity (my run: as_nanos Tier-2 3.66 ns vs
reference 3.42 ns). The doc manufactures an architectural justification that does not exist and
inflates the delivered value of the optimization. What correct looks like: either wire the
scheduler/pipeline loops to hold a Scaler (the doc's own design), or mark the tier as
reserved/test-only and delete the false "hot loops hold one" claim.
Confidence: HIGH — Verification: ran (rg sweeps + the benchmark itself).

F-002 [S3]
Where: crates/media-types/src/time.rs:57
Code:
```
/// Ticks per second = `num / den` (e.g. `1/90000` for MPEG timing).
```
What is wrong: The unit doc is inverted. In this code a tick is `num/den` SECONDS — ticks per
second is `den/num`. Proof: convert_reference (L443-447) computes
`ticks·from.num/from.den` as seconds; the in-file test `convert_rescales_mpeg_to_milliseconds`
passes 90000 ticks @ 1/90000 → 1 s, i.e. 90000 ticks/second = den/num, not 1/90000 ticks/second.
The same wrong sentence is copied verbatim from docs/01-architecture.md:1177. Why it matters:
this is the unit definition of the A/V-sync substrate; an integrator reading the doc would
construct TimeBase{90000, 1} for MPEG and be off by a factor of 8.1e9. What correct looks like:
"seconds per tick = num/den (ticks per second = den/num)". Fix both this file and doc 01.
Confidence: HIGH — Verification: read + ran the conversion tests.

F-003 [S3]
Where: crates/media-types/src/time.rs:141-149
Code:
```
    /// Construct a timestamp (the base is validated by construction).
    ///
    /// # Errors
    /// Propagates [`TimeBase::new`]'s error for zero components.
    pub fn new(ticks: i64, base: TimeBase) -> Timestamp {
        Timestamp { ticks, base }
```
What is wrong: `Timestamp::new` returns `Timestamp`, not `Result` — it cannot propagate any
error, and it accepts any `TimeBase` including struct literals with zero components (a real
caller shape, see F-001 evidence list). The `# Errors` section is false documentation on an
infallible function, and "validated by construction" is false for the literal-constructed bases
this very file defends against elsewhere (L473-476, L1613-1616). What correct looks like: delete
the `# Errors` section; say "the base is trusted — construct via `from_parts` for validation".
Confidence: HIGH — Verification: read (infallible signature; struct-literal callers listed).

F-004 [S3]
Where: crates/media-types/src/time.rs:113-133 (FromStr for TimeBase), 881-904 (FromStr for Rate), 816-831 (Duration::display), 785-788 (Nanoseconds::as_secs_exact)
Code:
```
    /// Parse `"num/den"` (e.g. `"1/90000"`). Enables the serialization
    /// round-trip required by doc 01 §2.2(e) without pulling serde into
    /// this std-only crate `[PROPOSAL]`.
```
What is wrong: All four items are production-dead. The only `.parse()` callers of TimeBase/Rate
in the entire repo are the in-file round-trip tests (`rg ':(TimeBase|Rate) = .*\.parse\(\)'`
-> time.rs only). No production serializer parses these strings, so the claimed "serialization
round-trip required by doc 01 §2.2(e)" has no consumer. `Duration::display` and
`as_secs_exact` likewise appear only in this file's tests (the wire layer uses
commands::catalog::types::Duration, a different type; the UI formats in
ui/src/components/slider/formatTime.ts). Dead public API on the "most-reviewed crate" with a
doc rationale that names a requirement nothing implements. What correct looks like: delete, or
wire the config/playlist layer that the §2.2(e) story implies.
Confidence: HIGH — Verification: ran rg sweeps across all `**/*.rs` and `apps/*/src`.

F-005 [S4]
Where: crates/media-types/src/lib.rs:54 vs lib.rs:61-68 (prelude), touching time.rs exports
Code (lib.rs:61-68 prelude list):
```
    pub use crate::time::{
        Duration, Nanoseconds, Rate, Stamp, TimeBase, TimeSpan, Timestamp, TrackTiming,
    };
```
What is wrong: The headline re-export list (lib.rs:54) includes `Scaler`; the prelude omits
it. The two lists are documented as kept in sync at Lead review time. Inconsistent export
surface for the one type the M18 docs call the batch form. Limited blast radius (Scaler is
production-dead anyway, F-001).
Confidence: HIGH — Verification: read both lists.

F-006 [S4]
Where: crates/media-types/src/time.rs:1713-1714
Code:
```
    #[test]
    #[ignore = "manual M18 benchmark — run with --release --nocapture"]
```
What is wrong: Nothing functionally — the ignore is honest, labeled, and I ran it
successfully (see §2). Recorded only because the wave-0 baseline counts 66 `#[ignore]` tests:
this one is a benchmark, not a correctness test being hidden, and its numbers reproduce. The
module doc's measured claims (L42-48) cite this benchmark and are consistent with my run
(2.14 vs 3.42 ns, 1.60×) — but note the claims describe a Scaler tier no production code uses
(F-001).
Confidence: HIGH — Verification: ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TimeBase::new | 77-91 | REAL | — | zero-reject + gcd reduce; matches doc 01 §3.2; test-pinned |
| TimeBase::as_parts | 94-96 | REAL | — | used by demux (asf/ogg) |
| gcd | 99-104 | REAL | — | Euclid, correct |
| Display for TimeBase | 106-110 | REAL | — | "num/den" round-trips with FromStr |
| FromStr for TimeBase | 113-133 | REAL / UNREACHABLE | S3 | parses+validates correctly; zero production callers (F-004) |
| Timestamp::new | 147-149 | REAL | S3 | false `# Errors` doc (F-003) |
| Timestamp::from_parts | 155-158 | REAL | — | validates base; used by subtitles/codecs |
| Timestamp::convert | 175-181 | REAL | — | hot path; bit-identity proven; production-reachable |
| Timestamp::as_nanos | 192-198 | REAL | — | negative→typed Overflow; production-reachable |
| div_round_i128 | 213-231 | REAL | — | checked ties-away-from-zero; denom≤0→None |
| gcd64 | 283-288 | REAL | — | Scaler-setup only (dead tier) |
| reduce_smooth | 307-319 | REAL | — | exact 2/5 reduction; 100k-case proof passed |
| magic_u64 | 334-339 | REAL | — | GM reciprocal; bounds re-derived, hold |
| div_round_u64 | 344-350 | REAL | — | single-division round-half-up; no overflow (r<d) |
| div_round_mulshift | 359-370 | REAL | — | multiply+shift; `p−q·d` safe given proven floor |
| try_fast_convert | 378-420 | REAL | — | tier-2; identity incl. i64::MIN; degenerate→fallback |
| try_fast_nanos | 423-440 | REAL | — | negative→reference (error policy in one place) |
| convert_reference | 443-459 | REAL | — | i128 oracle; correct; kept verbatim per doc |
| as_nanos_reference | 461-481 | REAL | — | same |
| TimeBase::scaler_to | 525-548 | REAL-UNPROVEN / UNREACHABLE | S2 | correct (proof tests pass) but no production caller |
| TimeBase::nanos_scaler | 558-560 | UNREACHABLE | S2 | wraps scaler_to(NANOS_BASE) |
| Scaler::from_base | 566-568 | UNREACHABLE | S3 | test-only |
| Scaler::target_base | 572-574 | UNREACHABLE | S3 | test-only |
| Scaler::in_fast_domain | 582-584 | REAL | — | cfg(test) by design, fine |
| Scaler::convert_ticks | 593-607 | REAL-UNPROVEN / UNREACHABLE | S2 | bit-identical per test; zero production callers |
| Scaler::nanos | 618-640 | REAL-UNPROVEN / UNREACHABLE | S2 | arbitrary-scaler semantics documented honestly |
| Display for Timestamp | 635-639 | REAL | — | "{ticks}@{base}"; lossless rendering claim holds |
| Display for TimeError | 672-700 | REAL | — | full context; no paths/secrets leaked |
| From<TimeError> for EngineError | 704-720 | REAL | — | Clock stage, Corrupt/Timing, source preserved |
| Stamp::timestamp | 733-739 | REAL | — | Missing→TimeError::Missing; used in demux |
| Stamp::map_timestamp | 742-748 | REAL | — | used in playback-engine/apps-web |
| TimeSpan::UNKNOWN | 759-763 | REAL | — | used in demux (seek targets) |
| TrackTiming (struct) | 768-779 | REAL | — | populated by every demuxer |
| Nanoseconds::as_secs_exact | 785-788 | REAL / UNREACHABLE | S3 | test-only (F-004) |
| Duration::nanos | 806-812 | REAL | — | used by media-clock/mapping.rs:234 |
| Duration::display | 816-831 | REAL / UNREACHABLE | S3 | correct H:MM:SS.mmm; no production caller (F-004) |
| Rate::new | 854-867 | REAL | — | zero-reject + reduce; used by media-clock |
| Rate::as_parts | 870-872 | REAL | — | used by media-clock/drift.rs |
| Display for Rate | 875-879 | REAL | — | round-trips |
| FromStr for Rate | 881-904 | REAL / UNREACHABLE | S3 | test-only (F-004) |
| tests: Lcg, literal_tb, probe_ticks | 888-1230 | REAL | — | deterministic LCG; probes straddle domain edges incl. ±2 around 2^63/n |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mulshift_magic_is_bit_exact_for_small_divisors | YES | compares magic against real `/` for ALL d≤4096 dense + 20k random d≤u64::MAX with tie-straddling p values; >1.8M assertions |
| convert/as_nanos/scaler _is_bit_identical_to_reference | YES | direct equality vs i128 oracle over grids incl. i64::MAX/MIN, ±2 around the 2^63 domain edge, degenerate bases; also asserts fast-path HITS (fast_hits·2 > nonneg) so a silently-always-fallback tier fails |
| fast_domain_covers_realistic_stream_positions | YES | asserts 4-hour positions in 9 common bases take the fast tier and extremes fall back — kills the "fast path never taken" degeneration |
| degenerate_literal_bases_still_match_the_reference | YES | zero-component struct literals must match the reference (Overflow/Ok(0)), no panic |
| reduce_smooth_preserves_the_rational_exactly | YES | cross-multiplication equality + factor exhaustion + divisor checks, 100k cases |
| convert_rounds_ties_away_from_zero | YES | pins the rounding direction absolutely (±0.5 → ±1), not just self-consistency |
| convert_overflow_is_an_explicit_error / nanos_overflow_errors_rather_than_wrapping | YES | i64::MAX corners must produce TimeError::Overflow with context fields |
| timebase_rejects_zero_components / rate_is_exact_and_reduced | YES | zero-reject and 10/8→5/4 reduction pinned |
| duration_unknown_is_never_zero_or_neg_one | YES | "--:--" + Missing mapping + 1:02:03.004 formatting pinned |
| as_nanos_matches_reference_for_common_bases | PARTIAL | tolerance ±1 ns vs truncated "exact" would hide a ≤1ns rounding bug; superseded by the bit-identity tests |
| m18_bench_10m_conversions (#[ignore]) | N/A (benchmark) | honest manual bench; I ran it, numbers reproduce (F-006) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "All arithmetic is checked; overflow returns TimeError::Overflow... never wrap-around" | time.rs:5-6 | REAL | re-derived all paths; i128/u128 products bounded; checked_mul/add/sub in div_round_i128; tests pass |
| "convert and as_nanos run per packet and per frame on every pipeline path" | time.rs:10-12 | REAL | rg: demux (10 formats), video-engine/scheduler.rs, media-clock/mapping.rs, playback-engine, apps/cli, apps/web |
| "Hot loops (video scheduler, playback pipeline) hold one Scaler" | time.rs:495-497 | FALSE | rg: zero production callers (F-001) |
| "90 kHz → ns runs 1.60× faster through a Scaler (2.10 vs 3.37 ns)" | time.rs:42-48 | REAL (measurement) / misleading (reachability) | my run: 2.14 vs 3.42 = 1.60×; but no production code uses Scaler |
| "Ticks per second = num / den" | time.rs:57 | FALSE | code: seconds-per-tick = num/den (F-002) |
| "Timestamp::new ... # Errors: Propagates TimeBase::new's error" | time.rs:143-146 | FALSE | infallible signature (F-003) |
| "FromStr enables the serialization round-trip required by doc 01 §2.2(e)" | time.rs:126-128 | UNREACHABLE | no production parser (F-004) |
| TimeError = exactly the three spec variants | time.rs:645-646 vs doc 01 §3.2 | REAL | enum matches docs/01-architecture.md:1191-1195; Discontinuity constructed in media-clock/src/clock.rs |
| "a timestamp value and its absence are different types" | time.rs:3-4 vs code | REAL | Stamp/Duration enums; no -1/0 sentinels anywhere in file |

## 7. Dead / duplicated / unreachable code
- Scaler tier: scaler_to, nanos_scaler, Scaler::{from_base,target_base,convert_ticks,nanos} —
  production-dead (F-001). gcd64 exists solely for it.
- FromStr (TimeBase, Rate), Duration::display, Nanoseconds::as_secs_exact — test-only (F-004).
- NANOS_BASE const is private and used by as_nanos fallback + tests; media-clock keeps its own
  public NANO_BASE literal — deliberate duplication, documented.

## 8. Dependencies & flags
Std-only crate (Cargo.toml: "No dependencies. Deliberately boring"); dev-dep testkit only.
No feature gates in this file; `#[cfg(test)]` only on in_fast_domain. No unsafe. No #[allow].
Clippy baseline CLEAN (audit/runs/baseline/cargo-clippy.txt).

## 9. Open questions for the next agent
- Was M18's Scaler tier ever wired into video-engine/scheduler.rs and later unwired, or was the
  "hot loops hold one Scaler" doc aspirational from day one? (Check git history is
  orchestrator-side; the doc's D-1/D-2 rationale reads as post-hoc.)
- Does the commands/catalog wire layer (file 5 of my run) ever serialize Rate/TimeBase as
  "num/den" strings — if yes, F-004's FromStr verdict changes to reachable.
- media-clock/src/mapping.rs has its own timestamp_to_nanos via convert; confirm in W02's
  media-clock audit that it doesn't reintroduce an i128 division per stamp (M18's stated cost).
