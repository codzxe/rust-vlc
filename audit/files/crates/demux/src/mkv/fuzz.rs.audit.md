# AUDIT: crates/demux/src/mkv/fuzz.rs (full file, 84 lines)
| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-21 |
| Lines / functions | 84 (85 incl. trailing) / 5 items (module doc, 1 const, `fuzz_demux_mkv`, 2 tests) + cross-ref to the libFuzzer shim |
| File verdict | REAL (harness + deterministic sweeps baseline-executed; contract documented honestly) — with an executed-evidence hole: the seek path never sees hostile input in any committed lane, and the libFuzzer campaign has never been built |
| Reachable from shipped entry point? | N/A by design (fuzz-only entry): the in-crate `fuzz_demux_mkv` is called by the libFuzzer shim `fuzz/fuzz_targets/fuzz_demux_mkv.rs` (fuzz package, not shipped) and by the in-crate tests. The SUBJECT (`MkvDemuxer`) is reachable: `lumen open x.mkv` → `containers::open` → `"matroska" => mkv::open` (containers/mod.rs:57) → `MkvDemuxer::new/start/next_event` — the exact methods the harness drives |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | full Read of fuzz.rs 1-85; Read fuzz/fuzz_targets/fuzz_demux_mkv.rs (shim); `rg -n "mkv::fuzz::tests" orchrun-test-workspace-nofailfast.log.txt` → 2/2 ok (:1752-1753); `rg -n "malformed_derivatives…"` → ok (:2145, the 18 committed malformed fixtures executed); `rg -n "corpus_matroska_seek_roundtrip" -A 8 crates/demux/tests/mkv_corpus.rs` → loud-skip gate; `ls fuzz/fuzz_targets/` → 8+ targets incl. fuzz_demux_mkv.rs; `rg -n "mod test_support" crates/demux/src/{avi,mkv,mp4,wav}/mod.rs` → avi/mp4/wav have one, mkv none; `rg -ln "avi::test_support"` → 8 cross-format consumers; baseline full-gate #18 (fuzz_registry_check PASS, 35 targets, diff empty); `sed -n` on every quoted range |

## 1. One-paragraph verdict
This is a clean, honestly-documented fuzz harness whose loop is airtight (budget checked before increment, every `DemuxEvent` variant matched, `Err` breaks so sticky errors cannot spin) and whose panic-freedom claim is backed by executed evidence: the in-crate hostile sweeps (empty, EBML magic, all-0xFF, all-0x00, 48 deterministic patterns) and 18 committed malformed-derivative fixtures all passed in the baseline. Two evidence holes remain. First, the module doc's claim that "Seek probing rides the mkv integration suites (cues-indexed)" is false as executed evidence: the only cues-indexed seek suite (`corpus_matroska_seek_roundtrip`) loud-skips in every committed run because the corpus is absent — and unlike the MP4 harness, which probes three data-derived seeks per hostile pass, this harness contains no seek probing at all, so `MkvDemuxer::seek` has never executed against hostile bytes anywhere in the committed record. Second, the libFuzzer campaign this file fronts has never been built ("Lead-owned nightly wiring" pending; the shim only `cargo check`s on stable), and the fuzz-registry gate that passes in CI verifies registry↔file consistency, not any execution.

## 2. What actually works (verified)
- `fuzz_demux_mkv` loop (48-60): budget gate before increment (no off-by-one overrun), exhaustive `match` over `DemuxEvent` (Packet/Recovered/Discontinuity continue; EndOfStream and `Err(_)` break — a sticky demuxer error terminates the pass immediately, so no error-loop hang); early returns for unconstructible stream/demuxer and failed `start` are the documented tolerance. The harness itself has no panic path (no unwrap/index/arithmetic).
- Executed hostile-input evidence: `mkv::fuzz::tests::empty_and_marker_inputs_survive` and `hostile_wide_inputs_survive` ok (nofailfast:1752-1753) — empty input, the bare EBML magic `1A 45 DF A3`, 4096×0xFF (maximal-varint false positives), 4096×0x00 (zero-length id/size), and 48 length-1..48 deterministic patterns. Plus `malformed_derivatives_are_typed_or_recover_never_panics` (mkv_fixtures.rs:345, baseline ok :2145) driving the demuxer over 18 committed truncation/flip/garbage derivatives of real ffmpeg-authored mkv/webm fixtures — a genuinely strong deterministic complement.
- The shim (`fuzz/fuzz_targets/fuzz_demux_mkv.rs`) exists and is a faithful thin wrapper: `fuzz_target!(|data: &[u8]| { demux::mkv::fuzz::fuzz_demux_mkv(data); })` — no logic drift between the campaign entry and the in-crate harness.
- Registry gate: `bash scripts/fuzz_registry_check.sh` PASS in full-gate (#18: 35 targets, diff empty) — the doc §8.5 registry row maps to a real file.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/mkv/fuzz.rs:16-19 (delegation claim); evidence: crates/demux/tests/mkv_corpus.rs:228-232 (skip gate); contrast: crates/demux/src/mp4/fuzz.rs:4-6 (seek probing present)
Code:
```rust
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's integration corpora. Seek probing rides the
//! mkv integration suites (cues-indexed); this pass covers the full
//! header/cluster/block walk, which is where untrusted bytes land.
```
```rust
fn corpus_matroska_seek_roundtrip() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent");
        return;
    };
```
```rust
//! One call = one full untrusted-container pass: construct, `start`,
//! drain every event to end-of-stream (bounded), then probe seeks at
//! data-derived targets and drain a little after each. **The harness
```
What is wrong: seek-path fuzzing for MKV is delegated by this doc to "the mkv integration suites (cues-indexed)" — but the only cues-indexed seek suite is `corpus_matroska_seek_roundtrip`, which takes the loud-skip branch in every committed run (`/tmp/my-project/corpus-v4` absent; skip = eprintln + return, still reports `ok` — both corpus tests ok with `finished in 0.00s`, nofailfast:2100-2103). The deterministic seek tests that DO run use well-formed hand-built fixtures only (`chapter_targets_seek_lands_at_or_before`, mkv_extras_wiring.rs:609; the container `start_time_skips_packets…` lane). The MP4 harness probes three data-derived seeks per hostile pass (`FUZZ_SEEK_PROBES: usize = 3`, mp4/fuzz.rs:33); this one probes none.
Why it matters: `MkvDemuxer::seek` (cue-index arithmetic over file-supplied positions) has never executed against hostile or malformed bytes in any committed lane — the exact class of input the harness exists to feed it. A corrupted-cues file seeked by a user is a shipped-reachable path (lumen open + seek on a damaged mkv) with zero hostile-input execution evidence.
What correct looks like: port the MP4 harness's seek-probe pattern (2-3 data-derived spans, small post-seek drains) into `fuzz_demux_mkv` after the walk, and reword the doc to name the executed evidence instead of a suite that skips.
Confidence: HIGH — Verification: read; skip gate quoted; baseline timings cited; MP4 harness contrast read.

### F-002 [S4]
Where: crates/demux/src/mkv/fuzz.rs:14-15; fuzz/fuzz_targets/fuzz_demux_mkv.rs:6-9; baseline full-gate #18
Code:
```rust
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_mkv.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
```
```rust
// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
```
What is wrong: the libFuzzer campaign has never been built or run in any committed evidence — the shim compiles, the registry gate (35 targets, diff empty) verifies name↔file consistency, and nothing anywhere executes a single fuzzing minute. The status is documented honestly in both files, which is why this is S4 and not higher: the gap is the absence of coverage, not a false claim.
Why it matters: the module's headline contract ("never panics, never hangs, never allocates without a ceiling") is proven only against the deterministic sweeps and 18 malformed fixtures — real campaign discovery (deep varint states, laced-block edge shapes, cue-table perversions) has never happened for Matroska.
What correct looks like: a CI lane (weekly is enough) that runs `cargo +nightly fuzz run fuzz_demux_mkv -max_total_time=…` once the nightly wiring lands, with the crash corpus committed.
Confidence: HIGH — Verification: shim read; full-gate #18 cited (registry check only); no fuzz-run artifacts anywhere in audit/runs/baseline/.

### F-003 [S4]
Where: crates/demux/src/mkv/fuzz.rs:31-33
Code:
```rust
/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane Matroska's packet count]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;
```
What is wrong: the annotation's arithmetic is wrong as stated — a 2-hour 25 fps Matroska carries ~180k video + ~8k audio packets ≈ 190k events, the SAME order of magnitude as the 200k budget, not two below it; a 3-hour movie exceeds it. The budget is fine for its actual job (fuzz inputs are KB-sized, and exhaustion just returns), but the "largest sane" justification is off by two orders.
Why it matters: PROPOSAL annotations are the repo's audit-trail vocabulary for ceiling rationale; a ceiling whose stated rationale doesn't compute invites either blind trust or blind inflation later.
What correct looks like: "≈ one event per packet; fuzz-sized inputs stay orders below; real-file scale is the integration suites' job" — or raise the budget if real-file passes are intended.
Confidence: HIGH — Verification: read; arithmetic shown.

### F-004 [S4]
Where: crates/demux/src/mkv/fuzz.rs:39 (with crates/demux/src/avi/mod.rs:121)
Code:
```rust
pub fn fuzz_demux_mkv(data: &[u8]) {
    let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) else {
        return;
    };
```
What is wrong: the Matroska harness depends on the AVI module's test utilities for its in-memory stream. MKV has no `test_support` of its own (`rg "mod test_support"` over avi/mkv/mp4/wav: avi, mp4, wav have one; mkv does not), and `avi::test_support` is consumed by 8+ files across formats (mkv_extras_wiring, mkv_fixtures, adversarial_gaps, examples, apps/cli) — it is the crate's de-facto shared fixture plumbing, mislocated under `avi`. It is also a plain `pub mod` (avi/mod.rs:121), not cfg(test)-gated, so it compiles into every build of the demux crate.
Why it matters: naming that lies about scope ("avi" test support serving the whole crate), plus a cross-format layering dependency that makes the mkv fuzz harness transitively depend on the avi module. Hygiene, not behavior.
What correct looks like: a crate-level `demux::testkit` (or `demux::test_support`) module owning `mem_stream`, with avi/mp4/wav re-exporting or dropping their local twins; cfg-gate it if nothing in a shipped build needs it (apps/cli does use it today — in tests and possibly hosts, so at minimum the doc should say so).
Confidence: HIGH — Verification: read + greps (consumers listed; gating checked).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-25 | REAL (honest) | S3/S4 | contract + bounds documented; seek delegation claim false-as-executed (F-001); campaign status admitted (F-002) |
| `FUZZ_EVENT_BUDGET` | 31-33 | REAL | S4 | enforced at 50-52; annotation math wrong (F-003) |
| `fuzz_demux_mkv` | 38-61 | REAL | — | airtight loop (budget-before-increment, exhaustive event match, Err breaks); no panic paths in the harness itself; early-return tolerance documented |
| `empty_and_marker_inputs_survive` | 67-71 | REAL | — | panic-contract only; baseline ok (:1752) |
| `hostile_wide_inputs_survive` | 73-83 | REAL | — | all-FF/all-00/48 patterns; panic-contract only; baseline ok (:1753) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_and_marker_inputs_survive` / `hostile_wide_inputs_survive` | Only for panic/hang/oom on those exact shapes | no assertions by design (the documented contract); a seek-path panic, a cluster-walk panic on other shapes, or any wrong-value bug passes green |
| (context) `malformed_derivatives_are_typed_or_recover_never_panics` (mkv_fixtures.rs:345) | Only for panic/hang on the 18 committed derivatives | asserts typed-or-recovered per file — stronger than the sweeps but still fixture-bound |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:4-8 | REAL | loop has no assertion surface; sweeps + malformed fixtures ok (nofailfast:1752-1753, :2145) |
| "Seek probing rides the mkv integration suites (cues-indexed)" | fuzz.rs:18-19 | FALSE (as executed evidence) | the cues-indexed suite skips in every committed run (mkv_corpus.rs:228-232, corpus absent); no deterministic hostile seek test exists (F-001) |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below and the crate's integration corpora" | fuzz.rs:16-17 | PARTIAL | sweeps + malformed fixtures executed ✓; the "integration corpora" half is the skip-ok corpus lanes (mp4/ogg/mkv pattern, F-002 context) |
| "Never panics, never hangs, never allocates without a ceiling — by construction of the demuxer and the budget" | fuzz.rs:35-37 | REAL-UNPROVEN at campaign scale | proven for the executed shapes only; the campaign that could falsify it has never run (F-002) |
| registry row `fuzz_demux_mkv` (doc 04 §8.5) | fuzz.rs:1-2 | REAL | target file exists; registry check PASS (full-gate #18, 35 targets) |

## 7. Dead / duplicated / unreachable code
- Nothing dead in-file. Cross-file placement issue: `avi::test_support` as the crate-wide mem-stream utility (F-004) — three per-format `test_support` modules exist (avi/mp4/wav) while mkv leans on avi's.
- The fuzz package (fuzz/*) is workspace-external by design (Lead-owned); no dead code there beyond the never-executed campaign (F-002).

## 8. Dependencies & flags
- No feature gates; `pub mod fuzz` inside `pub mod mkv` — compiled into every demux build (its only callers are the shim + tests, so the linker drops it from the shipped binary; the lumen CLI does not expose a fuzz entry). Imports: `plugin_api::{DemuxEvent, Demuxer}`, `super::MkvDemuxer`, `crate::avi::test_support` (F-004). No unsafe.
- Baseline execution: 2/2 in-crate tests ok (nofailfast:1752-1753); malformed-derivative complement ok (:2145); fuzz-registry gate PASS (full-gate #18).

## 9. Open questions for the next agent
- For the mod.rs owner: does `MkvDemuxer::seek` re-validate cue positions against the stream extent before seeking (hostile-cue robustness)? No executed lane probes it (F-001) — a targeted hostile-cues unit test would close the gap without nightly.
- For Wave 17 (scripts/CI): a nightly fuzz lane for at least the demux targets would convert F-002 from admitted-gap to executed evidence.
- REQUEST-ORCH-RUN (optional): `LUMEN_CORPUS=<dir> cargo test --release -p demux --test mkv_corpus` — executes the seek roundtrip once with the corpus present (closes the F-001 delegation half).
