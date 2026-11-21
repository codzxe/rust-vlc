# AUDIT: crates/media-types/src/time.rs (part1, lines 1–633)
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2026-11-21 |
| Lines / functions | 633 audited (of 1858; chunk part1 = 1–619, extended to 633 to close `Scaler::nanos`) / 32 items (26 fns/methods, 3 structs, 3 consts) |
| File verdict | REAL (part1 code live from shipped CLI; `Scaler` tier UNREACHABLE in production) |
| Reachable from shipped entry point? | YES for `TimeBase`/`Timestamp`/`convert`/`as_nanos` (chain: `lumen` main → apps/cli/src/demux_host.rs:178,558,708 `stamp.convert(&media_clock::NANO_BASE)` → time.rs:175; demux avi.rs:640 / wav.rs:905 → `Timestamp::new`+`as_nanos`). NO for `Scaler`/`scaler_to`/`nanos_scaler`/`convert_ticks`/`Scaler::nanos` — zero production callers (grep over crates/ apps/). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `rg "Scaler|scaler_to|nanos_scaler|convert_ticks" crates apps` → only media-types + playback-engine/tests/property/timestamp_arithmetic.rs (test-only). `rg "\.convert\(" crates apps` → CLI demux_host + containers live callers. `rg "as_parts|from_parts|from_base|target_base|parse::<TimeBase>"` → as_parts live; from_parts test-only; from_base/target_base zero callers. `sed -n` on every quoted range (bytes cross-checked with `od -c` where display was suspect). Baseline: `audit/runs/baseline/INDEX.md` + test-workspace.log.txt parsed (88 sections/1686 tests, **no media-types section**). |

## 1. One-paragraph verdict
The core conversion machinery (checked i128 reference + u64 fast paths) is real, carefully written, panic-free, and reachable from the shipped `lumen` binary on every demux/playback path; I hand-traced the rounding, sign, overflow, and Granlund–Montgomery magic-number proofs and found no divergence between fast and reference tiers. The rot is around the edges: the M18 `Scaler` batch tier — the headline "1.60×, zero divisions" optimization — is wired into **nothing** in production while the module and struct docs assert in present tense that "hot loops (video scheduler, playback pipeline) hold one Scaler"; the repo's own worklog admits adoption was deferred as a follow-up. The "pinned by benchmark" claim rests on an `#[ignore]`d manual benchmark that never runs; `Timestamp::new`'s rustdoc claims error propagation from an infallible constructor; `FromStr for TimeBase` and `Scaler::from_base/target_base` are dead public API.

## 2. What actually works (verified)
— `TimeBase::new`/`Timestamp::convert`/`as_nanos` live in the shipped binary: verified by `rg` call chain above + baseline `audit/runs/baseline/INDEX.md` MP4 smoke (`event: TimingDurationKnown { duration: Timestamp(3000000) }`) showing Timestamp values flowing through `lumen open`.
— Fast/reference tier equivalence: hand-traced. Ties away from zero (`-7/4→-2`, `-6/4→-2` both tiers), i64::MIN identity corner, degenerate struct-literal bases (`n==0||d==0` bail → reference → `Overflow`/`Ok(0)`), magic bound `p·m < 2^128`, `m ≤ 2^64`, `k = 127−lz(d) ≥ 64`, `((p·m)>>64)>>(k−64) == p·m>>k`, `q·d ≤ p`. All `as`-casts are the documented domain-proven narrowings (lines 362, 412, 414, 437, 599, 602, 624); no wrapping/saturating/unwrap/expect/index in lines 1–633 (grep-verified; only 4 `debug_assert!`s on caller-pre-checked invariants, which I verified hold at every call site).
— 90 kHz→µs reduction example from the bench docs re-derived by hand: `n=10⁶,d=9·10⁴ → n=100,d=9` after 2/5-strip; matches the code comment.

## 3. Findings

### F-001 [S2]
Where: crates/media-types/src/time.rs:489-497 (and module doc 30-37)
Code:
```
/// would tax every packet and frame header (D-1/D-2 mechanical
/// sympathy). Hot loops (video scheduler, playback pipeline) hold one
/// `Scaler` next to their base pair instead and convert by
/// multiply+shift alone — zero division instructions.
```
What is wrong: The docs assert as present-tense fact that the video scheduler and playback pipeline hold a `Scaler`. They do not. `rg "Scaler|scaler_to|nanos_scaler|convert_ticks"` over crates/ and apps/ finds zero production callers — only media-types itself and `crates/playback-engine/tests/property/timestamp_arithmetic.rs` (a test). The repo's own worklog confesses: `docs/worklog/archive-2026-10.md:7506` — "Follow-ups for the Lead: (1) sibling adoption cards — hold a `Scaler` in the hot loops that currently call `ts.convert(&NANO_BASE)`/`ts.as_nanos()` per frame … M18's scope was media-types only."
Why it matters: The M18 headline claim ("the dominant per-frame shape (90 kHz → ns) runs 1.60× faster through a `Scaler`", module doc lines 46-48) is unrealizable in the shipped binary — the 1.60× tier is dead code; only the per-call tier-2 (documented as parity on in-`u64` dividends) is live. Rustdoc ships a false statement about production wiring; auditors and maintainers reading the module docs will believe the pipeline uses the reciprocal cache.
What correct looks like: Either wire `Scaler` into the five call sites the worklog lists (video-engine scheduler, playback-engine pipeline, cli demux_host, media-clock mapping, plugin-api demux) or change the docs to conditional tense ("hot loops *should* hold…; adoption is a pending follow-up per worklog 7506").
Confidence: HIGH — Verification: read + `rg` caller sweep + worklog cross-quote.

### F-002 [S3]
Where: crates/media-types/src/time.rs:143-148
Code:
```
    /// Construct a timestamp (the base is validated by construction).
    ///
    /// # Errors
    /// Propagates [`TimeBase::new`]'s error for zero components.
    pub fn new(ticks: i64, base: TimeBase) -> Timestamp {
        Timestamp { ticks, base }
    }
```
What is wrong: The function is infallible (`-> Timestamp`) yet carries an `# Errors` section claiming it "Propagates `TimeBase::new`'s error" — impossible; no `Result`, no propagation. "The base is validated by construction" is also not enforced: `TimeBase`'s fields are `pub` and sibling crates construct by struct literal (`crates/media-clock/src/mapping.rs:50-53` `pub const NANO_BASE: TimeBase = TimeBase { num: 1, den: 1_000_000_000 }`), which this very file admits at lines 208-212 ("struct literals bypass the constructor").
Why it matters: This is the most-used constructor in the timing vocabulary; its rustdoc lies twice (validation guarantee, error propagation). Callers reading it will assume zero-component bases are impossible, when the code below (fast paths, reference) is written precisely because they are possible.
What correct looks like: Delete the `# Errors` section; reword to "The base is *assumed* valid — `TimeBase::new` validates, but pub struct literals bypass it; conversion surfaces handle degenerate bases defensively."
Confidence: HIGH — Verification: read + `rg "NANO_BASE" crates/media-clock/src/mapping.rs` (struct-literal const confirmed).

### F-003 [S3]
Where: crates/media-types/src/time.rs:116-119
Code:
```
    /// Parse `"num/den"` (e.g. `"1/90000"`). Enables the serialization
    /// round-trip required by doc 01 §2.2(e) without pulling serde into
    /// this std-only crate `[PROPOSAL]`.
```
What is wrong: The claimed consumer-facing requirement ("serialization round-trip required by doc 01 §2.2(e)") is realized by nothing: repo-wide sweep of every file mentioning `TimeBase` found zero `.parse()`/`from_str` calls targeting it (the three `.parse()` hits in TimeBase-using files parse `i64`/`u64`/`usize` — track_switch_fixture.rs:111, scenarios_b/host.rs:1730/2303). The only exercise is the crate's own `timebase_display_parse_roundtrip` unit test.
Why it matters: Dead public API justified by a requirement no shipped code implements — doc-01 §2.2(e) compliance is claimed but unproven by any caller. Classic vocabulary-crate drift: API added for a spec bullet, never adopted.
What correct looks like: Either a caller that round-trips TimeBase strings (config, control protocol, fixtures) or removal/`#[cfg(test)]`-gating of the impl with the doc requirement marked unrealized.
Confidence: HIGH — Verification: read + repo-wide caller sweep (documented above).

### F-004 [S3]
Where: crates/media-types/src/time.rs:566-574
Code:
```
    pub fn from_base(&self) -> TimeBase {
        self.from
    }

    /// The base this scaler converts to.
    #[must_use]
    pub fn target_base(&self) -> TimeBase {
        self.to
    }
```
What is wrong: Zero callers anywhere in the repo — not in production, not in tests, not even in media-types' own test module (grep finds only doc-link references at 586/609). Pure dead public API.
Why it matters: Untested, unused accessors on an already-unreachable type (`Scaler`, F-001) compound the dead surface; they also can't be exercised, so their `#[must_use]` contract is decorative.
What correct looks like: Delete both, or add callers in the Scaler adoption work (F-001 fix).
Confidence: HIGH — Verification: read + `rg "from_base\(\)|target_base\(\)"` over crates/ apps/ → no hits.

### F-005 [S4]
Where: crates/media-types/src/time.rs:44-46 and 1713-1715
Code:
```
//! `scaler_*` proof tests; speed is pinned by the measured
//! `m18_bench_10m_conversions` benchmark — Xeon, 10M conversions:
```
```
    #[test]
    #[ignore = "manual M18 benchmark — run with --release --nocapture"]
    fn m18_bench_10m_conversions() {
```
What is wrong: "Speed is pinned by" an `#[ignore]`d benchmark that no automated run executes (baseline run #3/#4 lists 24/25 ignored; this is one of them). Nothing pins the 1.60×/2.10 ns numbers; they are a one-machine manual record. Also the doc's test-family name "`fast_paths_*`" matches no actual test (real names: `convert_fast_path_is_bit_identical_to_reference` etc. — singular "fast_path").
Why it matters: Overstated enforcement language in module docs; a reader concludes CI would catch a perf regression when nothing would.
What correct looks like: "speed was *measured* by the manual in-tree benchmark (run with `--release --nocapture`)" — drop "pinned".
Confidence: HIGH — Verification: read; `#[ignore]` attribute quoted verbatim; baseline INDEX run #3 (24 ignored).

### F-006 [S4]
Where: audit evidence gap (not a code defect): audit/runs/baseline/test-workspace.log.txt
Code:
```
     Compiling media-types v0.1.0 (/home/z/my-project/audit-workspace/crates/media-types)
```
What is wrong: media-types compiles in the baseline log, but no `media_types-<hash>` unittest section exists in either baseline test log (parsed: 88 sections / 1686 tests visible; zero containing "media"; all-features log never mentions media-types at all). The ~58 lib tests — including the fast-path bit-identity proofs that are this file's only execution evidence — cannot be cited from baseline. The INDEX's 1681-passed total presumably includes them, but the per-test section was not captured.
Why it matters: The M18 correctness contract ("bit-identical, enforced by proof tests") has no citable executed run in the audit corpus; only the worklog's own claim (`docs/worklog/archive-2026-10.md:7498` "58 passed / 0 failed / 1 ignored").
What correct looks like: A captured `cargo test -p media-types --release` transcript in a future baseline refresh.
Confidence: HIGH — Verification: parsed both baseline logs programmatically (python3 section-by-binary mapping).
REQUEST-ORCH-RUN (logged in W02-A02.log.md): `cargo test -p media-types --release 2>&1 | tail -20`

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `struct TimeBase` | 62-66 | REAL | — | 8-byte Copy; pub fields bypass `new` validation (F-002 context); embedded in every Stamp/Timestamp |
| `TimeBase::new` | 77-90 | REAL | — | rejects zero components with typed EngineError (Demux/Unsupported); gcd-reduces so `1/90000`==`2/180000`; no overflow path |
| `TimeBase::as_parts` | 94-96 | REAL | — | LIVE: apps/cli/src/containers/{ps.rs:236,ts.rs:193,es.rs:139,adts.rs:128} |
| `gcd` (u32) | 99-105 | REAL | — | Euclid, terminates, no overflow |
| `Display for TimeBase` | 107-111 | REAL | — | `{num}/{den}`; consumer of record is the crate's own round-trip test only |
| `FromStr for TimeBase` | 113-132 | REAL (unreachable in prod) | S3 | F-003; parsing itself correct: rejects negatives/`a/b/c`/missing den via u32 parse + `split_once` |
| `struct Timestamp` | 136-140 | REAL | — | live everywhere (baseline smoke shows `Timestamp(3000000)` in events) |
| `Timestamp::new` | 147-149 | REAL | S3 (docs) | F-002; infallible; hottest constructor (demux avi/wav/aiff) |
| `Timestamp::from_parts` | 155-157 | REAL (test-only) | S4 | callers: subtitles tests only (compositing_ac.rs:16, compositing_alloc.rs:25, timing_clock.rs:31) |
| `Timestamp::convert` | 175-180 | REAL | — | LIVE (cli demux_host.rs:178,558,708; containers/*.rs); fast-then-reference dispatch |
| `Timestamp::as_nanos` | 192-197 | REAL | — | LIVE (playback-engine pipeline.rs:2540,2592, seek.rs:320, session.rs:2818; demux wav:905, avi:640, aiff:542, mp4:1872,2065, adts:969, ogg:1196) |
| `const NANOS_BASE` | 200-204 | REAL | — | private, reduced `1/1e9` |
| `div_round_i128` | 213-227 | REAL | — | checked mul/add; denom ≤ 0 → None (degenerate literal bases); ties away from zero hand-verified on -7/4, -6/4 |
| `const FAST_DOMAIN_LIMIT` | 279 | REAL | — | 2^63; the three documented properties re-derived and hold |
| `gcd64` | 283-288 | REAL | — | construction-time only |
| `const FIVE_POW` | 294 | REAL | — | 16+8+4+2+1=31 ≥ max feasible common 5-exponent (5^28 > u64::MAX, so ≤ 27); greedy binary decomposition correct |
| `reduce_smooth` | 307-318 | REAL | — | debug_assert-only guard; both callers pre-check `n==0||d==0` (verified at 396-398, 424) so the `>>= 64` degenerate case is unreachable |
| `magic_u64` | 334-339 | REAL | — | k=127−lz(d)∈[64,127]; m ≤ 2^64; proof on 320-333 re-derived — sound |
| `div_round_u64` | 344-349 | REAL | — | `d−r ≥ 1` prevents underflow; half-away on magnitude |
| `div_round_mulshift` | 359-366 | REAL | — | `((p·m)>>64) as u64 >> (k−64)` ≡ `p·m>>k`; `q·d ≤ p` keeps u64 math in range; exactness proof re-derived |
| `try_fast_convert` | 378-415 | REAL | — | identity shortcut exact incl. i64::MIN (comment's claim re-verified: reference gives same value); degenerate → None → reference; sign/tie parity hand-checked |
| `try_fast_nanos` | 423-438 | REAL | — | negative → reference → Overflow (documented); `num·1e9 ≤ (2^32−1)·1e9 < 2^64` no overflow |
| `convert_reference` | 443-457 | REAL | — | i128 products bounded: \|ticks\|·(2^32−1)² < 2^127 → no wrap; denom ≤ 0 → Overflow |
| `as_nanos_reference` | 461-483 | REAL | — | negative → `u64::try_from` fails → typed Overflow (matches doc "value is negative") |
| `struct Scaler` | 502-515 | REAL / UNREACHABLE in prod | S2 | F-001; d=0 degenerate sentinel |
| `TimeBase::scaler_to` | 524-553 | REAL (test-only callers) | — | degenerate sentinel d=0 → every apply defers to reference; full gcd64 reduce; products < 2^64 |
| `TimeBase::nanos_scaler` | 557-560 | REAL (test-only) | — | delegates to scaler_to(NANOS_BASE) |
| `Scaler::from_base` | 566-568 | DEAD | S3 | F-004 |
| `Scaler::target_base` | 572-574 | DEAD | S3 | F-004 |
| `Scaler::in_fast_domain` | 581-584 | REAL | — | `#[cfg(test)]`; used by in-crate proofs |
| `Scaler::convert_ticks` | 593-606 | REAL (test-only) | — | single widening multiply doubles as domain check + dividend — correct (traced); reduced-pair parity with reference holds because reduction preserves the rational exactly |
| `Scaler::nanos` | 618-632 | REAL (test-only) | — | fast only when `self.to == NANOS_BASE`; non-nanos scalers defer to `as_nanos_reference(ticks, from)` — documented, deliberate |

## 5. Test honesty (tests for this part live at lines 902+ — parts 2/3 territory; execution UNVERIFIED per F-006)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `convert_fast_path_is_bit_identical_to_reference` (1421) | YES (reasoned) | Domain-edge straddling probes per worklog 7504; mutating `r >= d - r` → `r > d - r` (line 348) flips every tie and fails it |
| `as_nanos_fast_path_is_bit_identical_to_reference` (1369) | YES (reasoned) | same mutation surface via `div_round_u64` |
| `scaler_is_bit_identical_to_reference` (1498) | YES (reasoned) | mutating the `>> (k - 64)` shift at 362 breaks floor-exactness on every d not a power of two |
| `degenerate_literal_bases_still_match_the_reference` (1614) | YES (reasoned) | removing the `n == 0 \|\| d == 0` bail at 396 trips `debug_assert!` in debug / yields wrong Ok in release |
| `mulshift_magic_is_bit_exact_for_small_divisors` (1238) | YES (reasoned) | exhaustive d ≤ 4096 sweep would catch any k/m off-by-one |
| `m18_bench_10m_conversions` (1715) | NO — never runs | `#[ignore]`d manual benchmark (F-005); asserts nothing, prints timings |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Hot loops (video scheduler, playback pipeline) hold one `Scaler` next to their base pair" | time.rs module doc 495-497 | FALSE | rg: zero production callers; worklog archive-2026-10.md:7506 admits non-adoption |
| "speed is pinned by the measured `m18_bench_10m_conversions` benchmark — … 1.60×" | time.rs 44-48 | SHALLOW | benchmark is `#[ignore]`d (1714); one manual Xeon record; nothing pins it |
| "Equivalence is enforced by the `fast_paths_*` / `scaler_*` proof tests" | time.rs 44-45 | REAL-UNPROVEN | tests exist (1369/1421/1498) but their execution is absent from both baseline logs (F-006); worklog 7498 claims green |
| "a timestamp value and its absence are different types, and conversion failure is data, not a panic… never wrap-around" | time.rs 3-6 | REAL (part1) | traced: no unwrap/expect/index/wrapping in 1-633; all overflow paths return `TimeError::Overflow` |
| "the base is validated by construction" / "# Errors: Propagates TimeBase::new's error" | time.rs 143-146 | FALSE | infallible signature; struct literals bypass validation (208-212 admits it) |
| "Enables the serialization round-trip required by doc 01 §2.2(e)" | time.rs 116-118 | PAPER-ONLY | no caller anywhere parses TimeBase strings (F-003) |
| "M18 CLOSED … hot loops hold one [Scaler]" | docs/worklog/archive-2026-10.md:7501-7502 | FALSE (same as row 1) | contradicted by worklog's own follow-up 7506 + grep |

## 7. Dead / duplicated / unreachable code
- `Scaler` + `scaler_to` + `nanos_scaler` + `convert_ticks` + `Scaler::nanos`: production-unreachable; exercised only by `crates/playback-engine/tests/property/timestamp_arithmetic.rs` and in-crate tests (F-001).
- `Scaler::from_base`/`target_base` (566-574): fully dead, zero callers incl. tests (F-004).
- `FromStr for TimeBase` (113-132): dead outside its own unit test (F-003).
- `Timestamp::from_parts` (155-157): test-only.
- No duplication found between fast tiers and reference (the reference is deliberately kept as oracle — documented and justified at 440-442).

## 8. Dependencies & flags
Crate is std-only by design (Cargo.toml: "No dependencies. Deliberately boring"); dev-dep `testkit` is dev-only. `[lints] workspace = true` (matches INDEX's 39/40 claim). No feature flags gate any part1 code — nothing here is cfg'd out of shipped builds except `in_fast_domain` (`#[cfg(test)]`, test introspection only). No unsafe. No license surface.

## 9. Open questions for the next agent
- A03 (part2, lines 620-1238): note my chunk extended to 633 to close `Scaler::nanos` (618-632) — do not re-audit that method; part2 opens at `Display for Timestamp` (635).
- A03/A01: do the part2/3 tests contain any test that would fail if `Scaler` were deleted from the crate? (In-crate + playback-engine property tests would — meaning the dead tier is at least pinned by tests, unlike truly untested code.)
- Whoever audits `crates/media-clock/src/mapping.rs`: `NANO_BASE` (mapping.rs:50-53) is the struct-literal proof for F-002; mapping.rs:284 is one of the five un-migrated per-frame `convert(&NANO_BASE)` sites from worklog 7506.
- Wave 16 (coverage): `cargo llvm-cov` on media-types would quantify the dead-tier gap precisely.
