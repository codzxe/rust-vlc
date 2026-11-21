# AUDIT: crates/demux/src/ogg/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 99 / 4 (fuzz_demux_ogg + 3 budget consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | Harness: via fuzz target + integration tests (fuzz/fuzz_targets/fuzz_demux_ogg.rs exists and calls it — verified; crates/demux/tests/corpus_ogg.rs:290,294,299,319 and tests/ogg_fixtures.rs:620,629,634 call it). Baseline: corpus_oggs_survive_the_fuzz_harness + ogg_fixtures_and_derivatives_survive_the_fuzz_harness both `ok`, corpus present (no SKIP message in log). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
This is what the ASF fuzz harness (my file 7) should have been: one pass drains every
event to EOS under a 500k-event budget, then fires three data-derived seek probes and
drains 64 events after each — the post-seek reanchor paths are fuzz surface too, and
the harness actually reaches them because it starts from real Ogg bytes (the
integration suites feed it committed fixtures AND corpus files with 4 KiB-stride
truncation and single-bit-flip mutation families, all green in the baseline with the
corpus present). The wiring claim is real in the direction that matters: the
cargo-fuzz target file exists, is registered in fuzz/Cargo.toml:118-119 and the
README registry, and calls this harness verbatim. Only two hygiene defects: an
infallible `TimeBase::new` wrapped in `if let Ok` whose dead Err arm would silently
skip every seek probe, and a "Wiring status" paragraph that says "until it lands"
about a target file that has landed.

## 2. What actually works (verified)
- Harness executed on real bytes in baseline: `corpus_oggs_survive_the_fuzz_harness
  ... ok` (nofailfast log :2006; corpus present — no "corpus absent — SKIP" eprintln
  anywhere in the log, checked by rg) and
  `ogg_fixtures_and_derivatives_survive_the_fuzz_harness ... ok` (:2172). The corpus
  sweep feeds every corpus file plus 4 KiB-stride truncation prefixes and
  single-bit flips (corpus_ogg.rs:288-300, read).
- The cargo-fuzz wiring is real: fuzz/fuzz_targets/fuzz_demux_ogg.rs read in full —
  `fuzz_target!(|data: &[u8]| { demux::ogg::fuzz::fuzz_demux_ogg(data); });`;
  registered in fuzz/Cargo.toml:118-119 and the README registry table.
- Budget structure verified: the drain loop increments per event and breaks at
  FUZZ_EVENT_BUDGET (fuzz.rs:73-76); each seek probe drains at most
  FUZZ_POST_SEEK_DRAIN (91-96); saturating arithmetic throughout
  (:82-85); no allocation in the harness beyond the one `data.to_vec()` (:51).
- Seek probes reach real code: targets derive from the observed last pts ticks
  (:61-68, 82-85), so a stream that demuxed anything gets in-extent probes; an
  all-Missing-stamps stream still probes tick 0 (the reanchor-to-start lane).

## 3. Findings

### F-001 [S4] Infallible constructor guarded by `if let Ok` — dead Err arm silently skips all seek probes
Where: crates/demux/src/ogg/fuzz.rs:80-83
Code:
```rust
    if let Ok(base) = TimeBase::new(1, 48_000) {
        for fraction in 1..=FUZZ_SEEK_PROBES {
            let ticks = last_granule.saturating_mul(fraction as i64)
                / i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3);
```
What is wrong: `TimeBase::new(1, 48_000)` cannot fail (denominator 48 000 ≠ 0) —
the `if let Ok` arm is dead code. If a refactor ever made the Err arm live (or a
reader believes it can be), the failure mode is silent: the whole seek-probe block
is skipped and the harness quietly loses its post-seek coverage. Same pattern one
line later: `i64::try_from(FUZZ_SEEK_PROBES)` (usize 3 → i64) cannot fail;
`unwrap_or(3)` is a dead default.
Why it matters: Silent-skip branches in a fuzz harness narrow coverage without any
signal — the exact anti-pattern the ASF harness's doc warns about. Unlike page.rs's
documented-unreachable arms, these carry no comment at all.
What correct looks like: `let base = TimeBase::new(1, 48_000).unwrap_or_else(|e|
unreachable!("{e}"));` (test-adjacent code may use unreachable per the house
pattern) or a plain `must()` — loud, not silent.
Confidence: HIGH — Verification: sed -n '80,87p' (quote verified); TimeBase::new's
failure condition is a zero/overflow denominator (workspace type, 1/48000 is valid).

### F-002 [S4] "Wiring status (honest)" says "until it lands" about a target that has landed
Where: crates/demux/src/ogg/fuzz.rs:12-19
Code:
```rust
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_ogg.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI/WAV cards recorded; until it lands the harness logic is
//! proven by the deterministic in-crate sweeps (bit-flip/truncation
//! families over the committed fixtures, run by the integration
//! suite) and by the corpus dirty/garbage mutation dirs.
```
What is wrong: The target file EXISTS (verified: fuzz/fuzz_targets/fuzz_demux_ogg.rs,
registered in fuzz/Cargo.toml:118-119 and the README registry). What has not landed
is the nightly libFuzzer CAMPAIGN (the target file's own header says "building the
campaign needs the Lead's nightly wiring"). "Until it lands" attached to the file
path states the opposite of the tree's reality.
Why it matters: Same doc slot the ASF harness used for a fully false claim; here it
is only stale/imprecise, but a reader auditing wiring status gets the wrong state.
(Contrast file 7, F-001, where the same sentence pattern was a fiction.)
What correct looks like: "The target file is landed and registered; the nightly
campaign build is pending Lead wiring; meanwhile the harness is proven by …".
Confidence: HIGH — Verification: ls + cat of fuzz/fuzz_targets/fuzz_demux_ogg.rs;
fuzz/Cargo.toml:118-119 read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` / `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 37-45 | REAL | — | Documented PROPOSAL budgets; all three consumed in the harness. |
| `fuzz_demux_ogg` | 50-99 | REAL | S4 | Full-drain + seek-probe + post-seek-drain structure — the strongest harness shape in this wave's file set. Panics impossible by construction (all results matched, never unwrapped). F-001's silent-skip guards live at :80-83. `data.to_vec()` copies input once per pass (fine). |

## 5. Test honesty
No in-module tests. The harness's honesty lives in the integration suites:
| Test (integration) | Would it fail if the code were broken? | Reason |
|---|---|---|
| corpus_oggs_survive_the_fuzz_harness (corpus_ogg.rs:281-300) | YES (panics/hangs in any lane the corpus reaches) | Real corpus bytes + truncation/flip families; a panic anywhere in the full drain or the seek probes fails the test. Ran in baseline with the corpus present (no SKIP). |
| ogg_fixtures_and_derivatives_survive_the_fuzz_harness (ogg_fixtures.rs, :620-634 call sites) | YES | Committed fixture derivatives (truncated/patched chains) through the same harness; baseline `ok` (:2172). |
| corpus_oggs_match_the_ffmpeg_truth (corpus_ogg.rs) | YES | Behavioral differential — not fuzz, but the same corpus lane. Baseline `ok`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-container pass: construct, start, drain every event to end-of-stream (bounded), then probe seeks" | fuzz.rs:4-6 | REAL | Code matches (50-99); reached by the integration sweeps in baseline. |
| "the harness logic is proven by the deterministic in-crate sweeps (bit-flip/truncation families over the committed fixtures, run by the integration suite)" | fuzz.rs:16-19 | REAL | corpus_ogg.rs:288-300 + ogg_fixtures.rs:620-634; both `ok` in baseline. |
| "until it lands" (the cargo-fuzz target file) | fuzz.rs:16 | FALSE-as-stated (stale) | The file has landed (F-002). |
| "Recovery is bounded by MAX_RECOVERY_SCAN_BYTES; packet memory by MAX_PACKET_BYTES" | fuzz.rs:26-30 | UNVERIFIED | Those constants and their enforcement live in ogg/mod.rs (another agent's file); the references resolve (super:: paths), enforcement not re-audited here. |

## 7. Dead / duplicated / unreachable code
- F-001's two dead defensive arms (the only dead code found).
- No duplication.

## 8. Dependencies & flags
- `pub mod fuzz` in ogg/mod.rs — unconditional; reachable only via the fuzz target
  and tests, not the shipped `lumen` binary path (the Ogg DEMUXER is reachable via
  the "ogg" container arm; the fuzz harness itself is test-only surface). Std +
  workspace crates; no unsafe; no feature gates.

## 9. Open questions for the next agent
- ogg/mod.rs auditor: confirm MAX_RECOVERY_SCAN_BYTES and MAX_PACKET_BYTES are
  actually enforced on every path the harness can drive (fuzz.rs:26-30 claims it).
- The seek probes ride a hard-coded 1/48 000 base — if OggDemuxer::reanchor
  cross-base conversion clamps or errors on out-of-extent targets, probes on
  non-48 kHz streams may all land at the same edge; worth one look in mod.rs.
