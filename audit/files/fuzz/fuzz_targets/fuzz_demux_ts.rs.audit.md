# AUDIT: fuzz/fuzz_targets/fuzz_demux_ts.rs
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 20 / 1 fuzz_target! macro invocation |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — fuzz/ own workspace root (fuzz/Cargo.toml:87); release lane never builds it; harness fn has no production caller |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | read shim + harness (crates/demux/src/ts/fuzz.rs); ls fuzz/corpus/fuzz_demux_ts/ (2 seeds); baseline greps (:2074-2075, :2417-2431, full_gate.txt:56-58); rg fuzz_loop_stable.rs |

## 1. One-paragraph verdict
Identical shape to its 34 siblings: a 20-line libFuzzer shim, registered
([[bin]] fuzz/Cargo.toml:145-150), whose campaign has never been built or run
(nightly LEAD-DECISION outstanding; README:165-168 admits "No such run has
happened yet"; fuzz/artifacts|coverage|target all absent — reproduced by ls for
the adpcm sibling, same tree state). The harness it delegates to is real and
better-covered on stable than most: the 188-byte walk + PSI assembly + resync
drain (ts/fuzz.rs:37-60) is executed by 2 in-crate sweeps (baseline :2074-2075),
3 ts_fuzz_survival tests (:2419-2423) and ts_pes_corpus's fuzz-lite loop — all
green in the orchestrator baseline. Registry wiring gate-verified (full_gate.txt:57-58).

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt:2074-2075 `ts::fuzz::tests::empty_and_sync_inputs_survive
... ok` / `hostile_wide_inputs_survive ... ok`; :2417-2423 ts_fuzz_survival.rs
3 passed (corpus_head_mutations_survive / truncation_sweep_over_packet_boundaries /
mutations_never_panic_and_stay_deterministic); :2425-2431 ts_pes_corpus.rs 4 passed
incl. fuzz_lite_mutations_never_panic_or_hang.
— registry gate: full_gate.txt:57-58 `fuzz-registry: CLEAN — 35 targets, diff
empty, corpora present / fuzz-registry: PASS` (rg-proves shim → demux::ts::fuzz::fuzz_demux_ts).
— seed corpus: fuzz/corpus/fuzz_demux_ts/ = seed_hls_pl0.ts + seed_mpeg2_in_ts.ts
(49,152 B, sha256 2921740245e9… per W16-A02.fixture-manifest.tsv:14 — MPEG-TS data).

## 3. Findings
F-001 [S3]
Where: fuzz/fuzz_targets/fuzz_demux_ts.rs:7-12; fuzz/README.md:17, :36-41, :165-168
Code: `// Wiring status (honest): building the campaign needs the Lead's
// nightly wiring (see fuzz/README.md); until then this package
// compiles on stable (`cargo check` in fuzz/) and the harness logic
// is proven by the in-crate deterministic hostile-input sweeps.`
What is wrong: the coverage-guided campaign for the network-facing MPEG-TS
container — arguably the highest-value fuzz target in the registry (broadcast
bytes are attacker-shaped by default) — has never run. No artifacts/coverage/
target directories exist; no CI green run (ci.yml:244-246 runs the registry
check only; audit.yml:325-339 fuzz tier is nightly/dispatch with no committed
record). Wiring status honestly documented — the finding is the absence, not a lie.
Confidence: HIGH — Verification: read + ls (sibling dirs absent).

F-002 [S4]
Where: crates/demux/tests/fuzz_loop_stable.rs:60 vs README:207
Code: `use demux::avi::fuzz::fuzz_demux_avi;`
What is wrong: the stable seeded-randomized loop layer (xorshift N-pass, the
strongest stable proxy for the missing campaign) imports only the AVI harness —
ts is skipped by the random layer too.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/src/ts/fuzz.rs:30-32
Code: `/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane capture's packet count]`.`
What is wrong: the panic/hang cap that makes the harness's "never hangs" claim
mechanical is tagged [PROPOSAL] — a bound nobody ratified; 200k events at ~0
budget each is fine, but the marker says the number is folklore, not measurement.
What correct looks like: measure the largest committed TS fixture's event count
and set the cap to a stated multiple, drop the PROPOSAL tag.
Confidence: MEDIUM — Verification: read (tag semantics inferred from marker text).

## 4. Per-function table
File <100 LOC — single fuzz_target! invocation (:18-20) delegating to
demux::ts::fuzz::fuzz_demux_ts (fuzz.rs:37-60: mem_stream → TsDemuxer::new →
start → budgeted next_event drain; Packet/Recovered/Discontinuity continue,
EndOfStream/Err break, hard return at FUZZ_EVENT_BUDGET). Harness REAL and
executed on stable; shim REAL-UNPROVEN (compile claim = dated manual runs only;
cargo absent in this sandbox).

## 5. Test honesty
N/A (no tests in this file). Surrogates honest: the sweeps exercise the exact
contract (no panic/hang); ts_fuzz_survival's determinism test additionally pins
reproducibility.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness logic ... proven on stable by the deterministic sweeps below and the crate's TS corpora" | ts/fuzz.rs:17-18 | REAL | baseline :2074-2075, :2417-2431 |
| "compiles on stable (cargo check in fuzz/)" | shim :9 | REAL-UNPROVEN | dated manual runs (README:189-193); no gate/baseline record compiles fuzz/ |
| registry row `fuzz_demux_ts` → demux::ts::fuzz::fuzz_demux_ts (5.I1) | README:62 | REAL | registry gate PASS full_gate.txt:57 |

## 7. Dead / duplicated / unreachable code
Registered, not dead; unreachable from shipped build by design (test/fuzz lane only).

## 8. Dependencies & flags
libfuzzer-sys 0.4.13; demux path dep; same [profile.release] overflow-checks=true.

## 9. Open questions for the next agent
- Same as the adpcm target: has any audit.yml nightly fuzz-tier run actually
  executed (Actions history invisible from sandbox — UNVERIFIED)?
- seed_hls_pl0.ts is a cross-format seed (HLS playlist bytes into the TS
  demuxer?) — its provenance is worth a data audit when that corpus file comes up.
