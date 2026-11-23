# AUDIT: crates/demux/src/ps/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W05-A07 |
| Date | 2025-05-24 |
| Lines / functions | 84 / 1 fn + 3 consts (module < 100 LOC: condensed table below) |
| File verdict | REAL (deterministic-sweep harness; the libFuzzer campaign lane itself never evidenced as run) |
| Reachable from shipped entry point? | NO (release lane: nothing in apps/cli or desktop calls it). Reached by: (a) `ps::tests::fuzz_lite_*` (tests.rs:512/515/530 — I ran them, 2/2), (b) libFuzzer shim fuzz/fuzz_targets/fuzz_demux_ps.rs:19. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p demux --lib ps::tests::fuzz` -> 2 passed, exit 0; sed quote verification (4 blocks byte-verbatim); rg over fuzz/, .github/workflows, scripts/quality |

## 1. One-paragraph verdict
The harness does exactly what its contract says — construct, start, bounded drain (500k events), three SCR-fraction seek probes each followed by a 64-event drain — and it is panic-free by construction in the ways I could check: `Timestamp::from_parts` never panics (media-types/time.rs:155-158 is a plain constructor; negative wrapped ticks from the `as i64` cast are legal `Timestamp` values), every `Result` is swallowed, and the only allocation is the one `data.to_vec()`. The two in-crate sweeps pass (my run, 2/2, matching baseline cargo-test-workspace.txt:2052-2053). The real defect is evidentiary: the file is named and documented as the `+FB` fuzz surface, but the only executions that have provably happened are the deterministic 960-mutation sweep (mutations confined to each fixture's first 512 bytes) — the libFuzzer campaign lane contradicts itself in-repo (fuzz/README.md "SCAFFOLDED, NOT WIRED" vs .github/workflows/audit.yml:325-353 which installs nightly + cargo-fuzz and schedules 5 shards), and no in-repo artifact proves a single campaign run of `fuzz_demux_ps`.

## 2. What actually works (verified)
— `cargo test -p demux --lib ps::tests::fuzz` -> `test result: ok. 2 passed; 0 failed` (exit 0): `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` (2 fixtures x 120 rounds x 8 flips + truncations) and `fuzz_lite_clean_fixtures_drain_without_terminating_errors` (4 fixtures).
— Budget/loop bounds verified by read: `FUZZ_EVENT_BUDGET` 500_000 (line 56-58 break), `FUZZ_POST_SEEK_DRAIN` 64 (line 75), `FUZZ_SEEK_PROBES` 3 (line 64).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ps/fuzz.rs:1-7 + fuzz/fuzz_targets/fuzz_demux_ps.rs:8-14 + fuzz/README.md:17-35 + .github/workflows/audit.yml:325-353
Code:
```
//! construct, `start`, drain every event to end-of-stream (bounded),
//! then probe seeks at data-derived targets. **The harness contract
//! is absence-of-panic/hang/unbounded-allocation, nothing else** —
```
What is wrong: The fuzz lane's own documents disagree: fuzz/README.md says "Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding ... nothing in the local gate or CI builds this directory *in campaign mode*", while .github/workflows/audit.yml's `fuzz:` job installs `dtolnay/rust-toolchain@nightly` + `cargo-fuzz` and runs `scripts/quality/25_fuzz_campaign.sh 180` over all 35 targets on a nightly schedule — i.e., CI IS wired for campaigns. Whichever is true, no in-repo artifact (fuzz/artifacts/, docs/audit/reports/fuzz/) evidences that `fuzz_demux_ps` has ever executed under libFuzzer; the only proven executions are the deterministic sweeps. The `+FB`/fuzz card's real coverage claim therefore rests on 2 fixtures x 120 deterministic rounds with mutations biased into the first 512 bytes (tests.rs:509 `let at = ((state >> 33) as usize) % 512.min(bytes.len());`). What correct looks like: one source of truth for the lane status + a committed campaign log/artifact, or an honest "sweeps only, campaign pending" in this module doc. Confidence: HIGH — Verification: read (README + workflow + shim + script); campaign execution UNVERIFIED (sandbox has no CI run history; no local nightly toolchain — cargo-build-budget). Severity S3 because the fuzz claim is the file's raison d'être and its strongest form is unevidenced.

F-002 [S4]
Where: crates/demux/src/ps/fuzz.rs:50-52
Code:
```
            Ok(DemuxEvent::Packet(_)) | Ok(_) => {
                last_scr = demuxer.current_scr();
            }
```
What is wrong: The `Ok(DemuxEvent::Packet(_))` alternative is dead pattern weight — `| Ok(_)` subsumes it, so `last_scr` updates on every Ok event, not just packets. Functionally harmless (SCR is pack state, refreshed per event), but the arm misleads a reader into thinking packet-selective bookkeeping happens. What correct looks like: `Ok(_) =>` alone, or match the packet case if selectivity was intended. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/src/ps/fuzz.rs:23-25
Code:
```
/// Maximum events drained per pass `[PROPOSAL: 500_000 — two orders
/// of magnitude above the largest committed fixture's event count]`.
pub const FUZZ_EVENT_BUDGET: usize = 500_000;
```
What is wrong: The "two orders of magnitude above the largest committed fixture's event count" claim has no evidence in-repo (no event-count table, no test asserting it). Plausible (fixtures < 200 KB), but unverified — if a future fixture exceeds ~5k events the budget would silently start truncating fuzz passes. Confidence: LOW on the claim's falsehood, HIGH that it is unevidenced — Verification: UNVERIFIED (would require instrumenting a drain count; cheap but out of budget for this file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_demux_ps` | 35-84 | REAL | — | bounded drain + 3 seek probes; error-swallowing by contract; panic-free paths verified (from_parts is a plain constructor); ran via both sweep tests |
| consts `FUZZ_EVENT_BUDGET`/`FUZZ_POST_SEEK_DRAIN`/`FUZZ_SEEK_PROBES` | 25-31 | REAL | S4 | F-003 unevidenced magnitude claim |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ps::tests::fuzz_lite_mutations_of_the_committed_fixtures_never_panic | YES for panics in the harness or demuxer under header-biased mutations and truncations; NO for payload-region corruption (first 512 bytes only, by design) and NO for hangs unless they exceed the test timeout | deterministic LCG, fixed seed; I ran it (ok) |
| ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors | YES for terminal errors on clean input (asserts `terminal == None`) | ran ok; asserts the harness's clean-input lane, not the hostile one |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:4-7 | REAL | loop bounded 500k (56-58), post-seek 64 (75), single to_vec (36); sweeps ran |
| "Recovery is bounded by MAX_RECOVERY_SCAN_BYTES; packet memory by MAX_PACKET_BYTES" | fuzz.rs:12-16 | REAL-UNPROVEN | constants exist (ps/mod.rs:102/107, 64 KiB each) and pool is 2-arena (mod.rs:359); enforcement is ps/mod.rs's code — another agent's file, not executed by me |
| "Never panics, never hangs, never allocates without a ceiling" | fuzz.rs:33-34 | REAL-UNPROVEN | no panic site in the harness itself; demuxer-side panic-freedom is exactly what the sweeps test (ran ok), but 960 header-biased mutations is weak proof of "never" |
| libFuzzer campaign live for PS | fuzz/README.md vs audit.yml:325 | CONTRADICTORY / UNVERIFIED | F-001 |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped binaries (no CLI/desktop consumer) — by design for a fuzz module; kept out of the release lane only by convention (no cfg gate; `pub mod fuzz` at ps/mod.rs:93 compiles it into every demux build, including release — dead weight of a few hundred bytes, hygiene note shared with `pub mod test_support` at ps/mod.rs:94).
- F-002's dead match alternative is the only in-file dead code.

## 8. Dependencies & flags
Depends on: `media_types::time` (Stamp/TimeBase/TimeSpan/Timestamp), `plugin_api` (DemuxEvent/Demuxer trait), `super::PsDemuxer`, `super::test_support::mem_stream` (which forwards to `crate::avi::test_support` — cross-module test seam compiled unconditionally). No unsafe, no features. The libFuzzer shim lives outside the workspace (fuzz/ is its own workspace root per README).

## 9. Open questions for the next agent
- Did the scheduled audit.yml `fuzz` tier ever run and produce `docs/audit/reports/fuzz/` artifacts? That resolves F-001's contradiction (I cannot see CI history from the sandbox).
- ps/mod.rs (the demuxer under test) is another agent's file: the harness's "never hangs" claim ultimately rests on `MAX_RECOVERY_SCAN_BYTES`/`MAX_PACKET_BYTES` being enforced there and on `seek`/`reanchor`/`next_event` being panic-free — my sweeps are the only execution evidence.
- The `#[must_use]` rendering artifact: terminal pipelines that strip ANSI may display `#[must_use]` as `#ust_use]` (observed on ps/mod.rs:373 during quoting — byte-verified intact via od; no defect, recording so future auditors don't file it).
