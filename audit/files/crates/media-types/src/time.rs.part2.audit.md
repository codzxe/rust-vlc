# AUDIT: crates/media-types/src/time.rs#part2 (lines 620–1238)
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-21 |
| Lines / functions | 619 (chunk 620–1238 of 1858) / 40 rows |
| File verdict | REAL (Scaler block inside chunk = UNREACHABLE dead API) |
| Reachable from shipped entry point? | YES (lumen CLI → demux/mp4 TrackTiming construction → time types; playback-engine/src/seek.rs:448 `EngineError::from(TimeError)`; baseline rc-smoke shows `TimingDurationKnown { duration: Timestamp(3000000) }`) |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:5 |
| Commands run | `rg "TimeError::Discontinuity"` → constructed at media-clock/src/clock.rs:763,968; `rg "TrackTiming"` → 18 production constructors (demux mp4/mkv/avi/ts/ogg/wav/…, playback-engine, plugin-api); `rg "gaps:"` → every site `Vec::new()`, `rg "\.gaps\b"` → 0 reads; `rg -i "scaler" --glob '*.rs'` (whole tree) → only time.rs self-references; `rg "FromStr for Timestamp"` → none; `rg "as_secs_exact|map_timestamp"` → apps/web/src/host.rs:392 + render/present.rs:204 use map_timestamp, as_secs_exact test-only; `rg "Rate::new"` → media-clock/src/drift.rs:131 (production); `rg "EngineError" seek.rs` → seek.rs:448 From<TimeError> used; sed -n on every quoted range (all verified verbatim) |

## 1. One-paragraph verdict
The chunk's dominant content — `TimeError` (+Display/Error/From-EngineError), `Stamp`, `TimeSpan`, `TrackTiming`, `Nanoseconds`, `Duration`, `Rate` — is genuinely implemented, panic-free (all arithmetic is division/modulo on u64/i128 with checked narrowing), and reachable from the shipped `lumen` CLI through demux track construction and the playback seek path. Two real defects: (1) the entire `Scaler` tier (~130 LOC of Granlund–Montgomery batch-conversion API, lines 503–633 incl. this chunk's `Scaler::nanos`) has **zero production callers** anywhere in the tree while its docs explicitly claim "Hot loops (video scheduler, playback pipeline) hold one `Scaler`" — a false architectural claim; (2) `TrackTiming.gaps` is declared spec surface ("container-declared gaps (edits, splices)") that is never populated and never read — MP4 multi-entry `elst` edits surface only as a damage note. Additionally, no baseline run executed this file's test module at all (both workspace test runs aborted before `media-types`), so the 15 in-chunk tests are REAL-UNPROVEN despite being well-constructed.

## 2. What actually works (verified)
— Timestamp/TimeBase/Duration runtime execution in the shipped binary: baseline `audit/runs/baseline/rc-smoke-engine-only.log.txt` step 5 "command-layer-play: PASS (reached Playing, ended Stopped, exit 0)" and INDEX MP4 matrix `TimingDurationKnown { duration: Timestamp(3000000) }` (mp4-smoke-open-session.txt) — both prove the types constructed and converted on the demux→pipeline path.
— From<TimeError> for EngineError used on the seek path: `rg` → playback-engine/src/seek.rs:448 `EngineError::from(error)`.
— Compilation of the chunk: baseline build-release-workspace.log.txt exit 0 ("CLEAN").

## 3. Findings

### F-001 [S2] — Scaler API is dead in production; docs claim hot loops use it
Where: crates/media-types/src/time.rs:495-497 (claim), 503-633 (API; this chunk owns 619-632)
Code:
```rust
/// sympathy). Hot loops (video scheduler, playback pipeline) hold one
/// `Scaler` next to their base pair instead and convert by
/// multiply+shift alone — zero division instructions.
```
What is wrong: `rg -i "scaler" --glob '*.rs'` over the ENTIRE tree returns only time.rs itself (definition, doc comments, and test module at 1514/1533/1577). No crate — video-engine, playback-engine, apps/, tools/ — constructs or holds a `Scaler`. The doc claim names two concrete consumers that do not exist.
Why it matters: M18's Tier-1 optimization (the measured 1.60× win, 2.10 vs 3.37 ns, quoted at line 48) is unreachable in every shipped path; the delivered per-frame conversion perf is Tier-2/reference only. ~130 LOC of pub API plus its proof tests and the `m18_bench` narrative describe an integration that was never wired. This is doc-vs-code drift on a headline optimization (verdict for the Scaler claim: FALSE; for the Scaler code: UNREACHABLE).
What correct looks like: Either video-engine/src/scheduler.rs and playback-engine hold a `Scaler` for their base pairs (the documented design), or the docs/`#[must_use]` API are cut/reworded to "provided for future hot loops, currently unused".
Confidence: HIGH — Verification: read + whole-tree rg (three searches: `Scaler`, `scaler_to`, `nanos_scaler`).

### F-002 [S3] — TrackTiming.gaps: spec-declared field never written non-empty, never read
Where: crates/media-types/src/time.rs:773-774
Code:
```rust
    /// Container-declared gaps (edits, splices) in track time.
    pub gaps: Vec<TimeSpan>,
```
What is wrong: every production constructor (demux/src/mp4/mod.rs:907, mkv/header.rs:246, avi/header.rs:308, ts/mod.rs:984, ogg/mod.rs:1116, wav/mod.rs:689, aiff, adts, asf, ivf, ps, es/{mp3,annexb}, playback-engine/src/{pipeline.rs:2758,3218, tracks.rs:442}, plugin-api/src/demux.rs:293, transcode runner tests) writes `gaps: Vec::new()`; `rg "\.gaps\b"` finds zero readers.
Why it matters: doc 01 §3.1 (docs/01-architecture.md:1158) declares `pub timing: TrackTiming, // span, time base, known gaps` — the "known gaps" third of the contract is unimplemented surface. MP4 multi-entry `elst` (the canonical gap source) only pushes a damage note (`demux/src/mp4/mod.rs:1209 track.damage.push((0, "elst-multi-entry"))`), never a gap entry. Downstream gap-aware logic cannot exist because the data is never emitted.
What correct looks like: demuxers fill `gaps` from edit lists/splices, or the field is removed until a consumer exists.
Confidence: HIGH — Verification: read + rg over crates/ and apps/ (writers and readers both enumerated).

### F-003 [S4] — Timestamp Display claims "round-trippable"; no parser exists
Where: crates/media-types/src/time.rs:637-639
Code:
```rust
        // Ticks + base — the unambiguous, round-trippable rendering
        // (seconds renderings can be lossy and invite parse-back bugs).
        write!(f, "{}@{}", self.ticks, self.base)
```
What is wrong: `rg "FromStr for Timestamp"` over the whole tree: no `impl FromStr for Timestamp` anywhere; nothing parses `"ticks@num/den"` back. TimeBase (113-131) and Rate (881-898) each have a FromStr twin; Timestamp does not.
Why it matters: the doc claim ("round-trippable", doc 01 §2.2(e) serialization round-trip) is aspirational, not implemented — a naming/doc lie of the kind this audit exists to catch.
What correct looks like: either a `FromStr for Timestamp` parsing `"{ticks}@{num}/{den}"`, or the comment says "rendering carries all state; parser not yet provided".
Confidence: HIGH — Verification: read + rg whole tree.

### F-004 [S4] — Nanoseconds::as_secs_exact has no in-tree caller outside its own test
Where: crates/media-types/src/time.rs:784-789
Code:
```rust
    /// Exact seconds when the value divides evenly, else `None`.
    pub fn as_secs_exact(self) -> Option<u64> {
        self.0
            .is_multiple_of(1_000_000_000)
            .then_some(self.0 / 1_000_000_000)
    }
```
What is wrong: only references in the tree are the definition and its two assertions at time.rs:1096/1103.
Why it matters: dead pub API — speculative surface that ships untested-by-use.
What correct looks like: a caller, or removal.
Confidence: HIGH — Verification: rg whole tree (crates/, apps/, tools/).

### F-005 [S4] — Duration::display ("--:--" renderer) never called by shipped code
Where: crates/media-types/src/time.rs:813-818
Code:
```rust
    /// Format as `H:MM:SS.mmm` for display; `Unknown` renders as `"--:--"`
    /// (a deliberately non-numeric string — it can never be parsed back
    /// into a number, so no sentinel-value confusion is possible).
    pub fn display(&self) -> String {
```
What is wrong: only in-tree call is the unit test at time.rs:1089. No CLI/Tauri/web surface renders durations through it (UI formats in TypeScript; CLI prints raw events).
Why it matters: dead pub API carrying a doc claim about UI behavior nobody exercises; the MP §8 "never -1" story is real at the type level, but this renderer is unshipped.
What correct looks like: wire it into the CLI/UI duration output or drop it.
Confidence: HIGH — Verification: rg whole tree.

### F-006 [S4] — Rate::as_parts and Rate FromStr are test-only in-tree
Where: crates/media-types/src/time.rs:869-898
Code:
```rust
    /// The rate as an exact rational `(num, den)`, already reduced.
    pub fn as_parts(self) -> (u32, u32) {
        (self.num, self.den)
    }
```
What is wrong: `Rate::as_parts` callers: media-clock/src/drift.rs:272 only (inside `mod tests` starting drift.rs:181). `impl FromStr for Rate` is exercised only by its own roundtrip test (time.rs:1120-1129); no config/IPC path parses `"5/4"`. (TimeBase::as_parts, by contrast, IS used by demux tests and production.)
Why it matters: dead pub API; the "doc 01 §2.2(e) serialization round-trip" claim is real only in the test module.
What correct looks like: a settings/control path that parses rates from strings, or removal.
Confidence: HIGH — Verification: rg whole tree + sed context check of drift.rs:181/272.

### F-007 [S4] — Two test names promise more than the body delivers
Where: crates/media-types/src/time.rs:1009-1011 vs 1024-1028; 1046-1047
Code:
```rust
        // Deterministic sweep instead of floats: for a grid of bases and
        // ticks, the i128 reference and as_nanos agree within rounding.
```
and
```rust
            let exact =
                i128::from(ticks) * i128::from(base.num) * 1_000_000_000i128 / i128::from(base.den);
```
and
```rust
            if let Ok(mid) = ts.convert(&b2)
                && let Ok(back) = mid.convert(&b1)
```
What is wrong: (a) `as_nanos_matches_reference_for_common_bases` never calls `as_nanos_reference` — it recomputes its own truncating i128 division with ±1 tolerance, so it is an independent-oracle test, not a reference-equivalence test (the real bit-exactness proofs live in part3's `fast_paths_*` tests); (b) `convert_roundtrip_is_identity_when_exact` has no `else`/assert on the error path — a mutation making `convert` always return `Err` leaves this test green (only sibling `convert_is_exact_when_bases_agree` catches it), and the name's "identity when exact" is never actually checked for exactness.
Why it matters: names are evidence claims; a future reader trusts "matches reference" and skips the real proof.
What correct looks like: rename to `as_nanos_within_one_of_exact_i128` / `convert_roundtrip_stays_within_tolerance`; assert a non-vacuous pass count.
Confidence: HIGH — Verification: read (full bodies 1008-1063).

### F-008 [S3] — Baseline never executed this file's tests (both workspace runs aborted before media-types)
Where: audit/runs/baseline/test-workspace.log.txt (ends `error: test failed, to rerun pass \`-p desktop --test zero_alloc_transport\`` / `TEST_EXIT=101`), audit/runs/baseline/test-workspace-allfeatures.log.txt (ends `error: test failed, to rerun pass \`-p first-vertical --test evidence\``)
Code:
```text
error: test failed, to rerun pass `-p desktop --test zero_alloc_transport`
TEST_EXIT=101
```
What is wrong: cargo's fail-fast stopped both baseline test runs at `apps/desktop` / `first-vertical` (alphabetically before `media-types`); neither log contains a `media_types-<hash>` unittest binary. All unit-test crates after "desktop" (media-types, media-clock lib tests, playback-engine, streaming, subtitles, transcode, video-engine, …) have ZERO executed evidence in baseline. INDEX.md #3/#4's "1681 passed / 1 failed / 24 ignored" totals do not disclose the abort.
Why it matters: every "tests pass" citation for this file (and ~15 other crates) is unproven; a broken time.rs test suite would not have failed the baseline.
What correct looks like: a `--no-fail-fast` workspace run, or per-crate runs, recorded in baseline.
Confidence: HIGH — Verification: `rg "Running unittests" | rg media` on both logs → 0 matches; verified tail of both logs (quoted above).
REQUEST-ORCH-RUN: `cargo test -p media-types --lib --release --locked`

## 4. Per-function table (chunk 620–1238)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Scaler::nanos` (body 619-632; signature at 618 owned by part1) | 619-632 | REAL (code) / UNREACHABLE (production) | S2 (F-001) | Correct: fast path gated on `to == NANOS_BASE && d != 0 && p128 < 2^63`, else `as_nanos_reference`. No callers outside time.rs tests. |
| `Display for Timestamp::fmt` | 635-641 | REAL | S4 (F-003) | "round-trippable" claim false — no FromStr. |
| `enum TimeError` (3 variants) | 646-666 | REAL | — | Overflow constructed in convert paths; Discontinuity at media-clock/src/clock.rs:763,968; Missing via Stamp/Duration methods. |
| `Display for TimeError::fmt` | 668-696 | REAL | — | All three arms format context; matches test 1133-1152. |
| `impl Error for TimeError` | 698 | REAL | — | Empty impl, standard. |
| `From<TimeError> for EngineError::from` | 700-719 | REAL | — | Used at playback-engine/src/seek.rs:448. Maps all variants to Corrupt{Timing}/Clock — design choice, loses variant info except `context.code`. |
| `enum Stamp` | 723-726 | REAL | — | Present/Missing; used everywhere. |
| `Stamp::timestamp` | 733-738 | REAL | — | Missing→TimeError::Missing; used by apps/cli containers (aiff.rs:110 etc.). |
| `Stamp::map_timestamp` | 742-747 | REAL | — | Used apps/web/src/host.rs:392, render/present.rs:204. |
| `struct TimeSpan` | 752-755 | REAL | — | Used in demux/pipeline. |
| `TimeSpan::UNKNOWN` (impl) | 757-763 | REAL | — | Used at 20+ sites incl. production (demux/ts/mod.rs:982, asf, playback). |
| `struct TrackTiming` | 768-775 | REAL | S3 (F-002) | `gaps` field dead: never non-empty, never read. |
| `struct Nanoseconds` | 781 | REAL | — | Interchange unit, used widely. |
| `Nanoseconds::as_secs_exact` | 785-789 | REAL (test-only use) | S4 (F-004) | No in-tree production caller. |
| `enum Duration` | 795-798 | REAL | — | packet.duration across demux; mp4mux.rs:289. |
| `Duration::nanos` | 806-811 | REAL | — | Used media-clock/src/mapping.rs:234. |
| `Duration::display` | 816-830 | REAL (test-only use) | S4 (F-005) | Correct arithmetic (u64 div/mod, no overflow); unshipped. |
| `struct Rate` | 839-842 | REAL | — | Used by media-clock (Rate::new drift.rs:131 production). |
| `Rate::ONE` | 846 | REAL | — | Clock construction default. |
| `Rate::new` | 854-867 | REAL | — | Zero-component rejected (Unsupported/Control); gcd-reduced; production caller drift.rs:131. |
| `Rate::as_parts` | 870-872 | REAL (test-only use) | S4 (F-006) | Only drift.rs:272 (tests mod). |
| `Display for Rate::fmt` | 876-879 | REAL | — | Pairs with FromStr in test. |
| `FromStr for Rate::from_str` | 886-898 | REAL (test-only use) | S4 (F-006) | Only its own roundtrip test. |
| `mod tests` open + `Lcg::{next,below}` | 901-920 | REAL-UNPROVEN | S3 (F-008) | Deterministic LCG, `% n` bias irrelevant; `below(0)` would divide-by-zero but all callers pass ≥1. Not executed in baseline (F-008). |
| `timebase_rejects_zero_components` | 924-928 | REAL-UNPROVEN | — | Would fail if `new` stopped rejecting zeros; doesn't pin error kind/stage. |
| `timebase_reduces_to_lowest_terms` | 930-938 | REAL-UNPROVEN | — | Concrete value assertion (2/180000→1/90000). |
| `timebase_display_parse_roundtrip` | 940-948 | REAL-UNPROVEN | — | 4 bases; would fail on parse or format drift. |
| `convert_is_exact_when_bases_agree` | 952-959 | REAL-UNPROVEN | — | Mutation-killer for always-Err convert. |
| `convert_rescales_mpeg_to_milliseconds` | 961-973 | REAL-UNPROVEN | — | 90000→1000 ticks; catches unit confusion. |
| `convert_rounds_ties_away_from_zero` | 975-989 | REAL-UNPROVEN | — | ±0.5 tie both signs. |
| `convert_overflow_is_an_explicit_error` | 991-1006 | REAL-UNPROVEN | — | Asserts full error payload (from/to/ticks). |
| `as_nanos_matches_reference_for_common_bases` | 1008-1034 | REAL-UNPROVEN | S4 (F-007a) | Name lies: no `as_nanos_reference` call; independent i128 oracle ±1. Bound math `u64::MAX/1e9/num*den` verified correct. |
| `convert_roundtrip_is_identity_when_exact` | 1036-1063 | REAL-UNPROVEN | S4 (F-007b) | Vacuous on Err; tolerance bound reasoning sound. |
| `nanos_overflow_errors_rather_than_wrapping` | 1065-1069 | REAL-UNPROVEN | — | i64::MAX@1/1 → Err. |
| `stamp_missing_maps_to_time_error_missing` | 1073-1085 | REAL-UNPROVEN | — | Covers both methods, both variants. |
| `duration_unknown_is_never_zero_or_neg_one` | 1087-1106 | REAL-UNPROVEN | — | "1:02:03.004" hand-verified (3723004 ms → 1:02:03.004). |
| `rate_is_exact_and_reduced` | 1110-1117 | REAL-UNPROVEN | — | 10/8→5/4; zero rejection. |
| `rate_display_parse_roundtrip` | 1119-1129 | REAL-UNPROVEN | — | 4 rates. |
| `time_error_displays_with_context` | 1133-1152 | REAL-UNPROVEN | — | Pins Display text AND EngineError stage/kind/code — strongest test in chunk. |
| `literal_tb` (helper) | 1165-1167 | REAL | — | Struct-literal bases to bypass constructor — good adversarial choice. |
| `probe_ticks` (helper) | 1177-1228 | REAL | — | Domain-edge probes (2^63 boundary ±2, warm-zone sub-multiples, i64::MIN mirror). Sound; `.max(0)` on line 1221 defensive (edge is always ≥0). |
| `mulshift_magic_is_bit_exact_for_small_divisors` (signature only; body 1239+ belongs to part3/A01) | 1237-1238 | — | — | Boundary noted. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 15 named tests above | Yes (each), EXCEPT `convert_roundtrip_is_identity_when_exact` alone vs always-Err mutation | Concrete value/payload assertions; the roundtrip test's `if let Ok` skips error cases vacuously (F-007b) — cross-caught by `convert_is_exact_when_bases_agree`. |
| None executed in baseline | — | F-008: both workspace runs aborted before media-types. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Hot loops (video scheduler, playback pipeline) hold one `Scaler`" | time.rs:495-497, module doc 30-37 | FALSE | Whole-tree rg: zero non-test references (F-001). |
| "pub timing: TrackTiming, // span, time base, known gaps" | docs/01-architecture.md:1158 | SHALLOW (gaps third) | gaps never non-empty/never read (F-002). |
| "conversion failure is data, not a panic (MP §8)" | time.rs:3-6 module doc | REAL | No unwrap/panic in chunk; all conversions checked; smoke runs show typed errors (h264.sw.unsupported lane is decoder, not time). |
| "Missing … presentation scheduler treats it as infer-from-previous + duration … observable event" | time.rs:661-665 | REAL | video-engine/src/scheduler.rs:99-108, 376, 665 `inferred_pts` counter implements it. |
| "Semantics (tested by property tests)" | docs/01-architecture.md §3.2 preamble | REAL-UNPROVEN | LCG sweeps exist but never executed in baseline (F-008). |
| "round-trippable rendering" (Timestamp) | time.rs:637 | FALSE | No FromStr for Timestamp (F-003). |

## 7. Dead / duplicated / unreachable code
- `Scaler` + `scaler_to`/`nanos_scaler`/`convert_ticks`/`nanos`/`from_base`/`target_base` (503-633): UNREACHABLE in production (F-001).
- `TrackTiming.gaps` (774): dead field (F-002).
- `Nanoseconds::as_secs_exact` (785), `Duration::display` (816), `Rate::as_parts` (870), `FromStr for Rate` (881): test-only pub API (F-004/5/6).
- `gcd` (99-105, part1) duplicated by `gcd64` (283-288, part1) and `Rate::new` reuses part1's `gcd` — fine.

## 8. Dependencies & flags
No new deps (std-only crate per doc 01 §2.2(c) — verified: no `use` of external crates in chunk beyond `crate::error`/`crate::ids`). `is_multiple_of` (787) requires Rust ≥1.87 — repo pins 1.99.1 (rust-toolchain.toml, baseline INDEX). No feature gates in chunk. `#[cfg(test)]` correctly fences `in_fast_domain` (581, part1) and the test module (901).

## 9. Open questions for the next agent
- Part3 (A01): do `fast_paths_*` / `scaler_*` proof tests at 1239+ actually execute both tiers (or only the fallback)? If the fast tier were dead code inside the tests too, F-001's blast radius grows.
- Whoever audits video-engine/src/scheduler.rs: confirm whether it re-derives conversions per frame (paying the reference/Tier-2 cost the Scaler was built to avoid) — direct evidence for F-001's perf impact.
- REQUEST-ORCH-RUN: `cargo test -p media-types --lib --release --locked` (F-008).
