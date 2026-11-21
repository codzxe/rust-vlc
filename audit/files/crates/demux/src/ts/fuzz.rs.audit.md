# AUDIT: crates/demux/src/ts/fuzz.rs (lines 1-86, whole file)
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2026-11-21 |
| Lines / functions | 86 / 5 items (module docs, `FUZZ_EVENT_BUDGET`, `fuzz_demux_ts`, 2 tests) |
| File verdict | SHALLOW (real harness + real libFuzzer delegation; its stable "proof" lane never executes the harness core, and three coverage claims are false) |
| Reachable from shipped entry point? | NOT on the shipped binary path — the caller is the fuzz crate's shim `fuzz/fuzz_targets/fuzz_demux_ts.rs` (verified to exist and delegate to `demux::ts::fuzz::fuzz_demux_ts`; registry row `fuzz/README.md:62`; baseline gate #18 fuzz_registry_check PASS, 35 targets). The demuxer surface it guards IS shipped (`apps/cli/src/containers/ts.rs:110-153`). The in-crate sweeps executed: `ts::fuzz::tests::empty_and_sync_inputs_survive` / `hostile_wide_inputs_survive` ok (test-workspace:1902-1903) — but see F-003: they never reach the drain |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | full Read; `rg "fuzz_demux_ts"` repo-wide → shim + README registry row + no other callers; read of `fuzz/fuzz_targets/fuzz_demux_ts.rs` (delegation verified); input-by-input trace of every committed sweep input against `TsDemuxer::new`/`start` semantics (empty/1-byte/188/4096-byte lanes); read of `tests/ts_fuzz_survival.rs` (the adjacent suite that actually drains — via its own helper, not this harness); sed -n verification of every quote; baseline log greps |

## 1. One-paragraph verdict
The harness is real code with a real cargo-fuzz delegation, but every claim that gives it authority is false or unproven: the "damage-log cap" it leans on for its no-unbounded-allocation guarantee does not exist (the ledger grows 16 bytes per damaged packet with no ceiling and no drain — my ts/mod.rs#part1 F-005); the "PES timing model rides that drain" cannot happen because the harness never registers a `PesConsumer` and the demuxer discards PES payload when none is registered; and the "proven on stable by the deterministic sweeps below" claim collapses under tracing — every single committed input dies at `start()` (empty-source refusal, not-ts, or no-pat), so the budget loop, all four match arms, and the EOS/error exits of `fuzz_demux_ts` have never executed in a stable test. The genuine survival coverage lives next door in `tests/ts_fuzz_survival.rs` (512 LCG mutation rounds over a valid multi-program base stream, start+drain) — through its own `survive()` helper, not through this function.

## 2. What actually works (verified)
- The delegation chain is intact: `fuzz/fuzz_targets/fuzz_demux_ts.rs` calls `demux::ts::fuzz::fuzz_demux_ts(data)` verbatim; `fuzz/README.md:62` carries the registry row; the baseline's fuzz-registry gate passed (INDEX.md #18, 35 targets, diff empty). — verified by read + baseline.
- The budget loop's shape is correct: check-before-increment bounds iterations to exactly `FUZZ_EVENT_BUDGET`; `Err(_) => break` implements the documented "typed errors swallowed here" policy. — verified by read.
- `data.to_vec()` + `mem_stream` (the shared AVI seam) — one bounded copy, no unbounded allocation in the harness itself. — verified by read.
- The committed sweeps DO prove the construct/refusal prefix panic-free on their inputs: `[]` (empty-source typed refusal), `[0x47]` / `[0x47; 188]` / `[0x47; 4096]` (sync-scan + no-pat typed refusal — traced: PID 0x747, afc=0, no payload, PSI never fed), `[0xff; 4096]` / `[0x00; 4096]` (not-ts refusal), 96 small deterministic buffers. All executed ok (test-workspace:1902-1903). — verified by trace + baseline.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ts/fuzz.rs:20-23 and 34-36
Code:
```rust
//! Event budget: hard cap below; positions are monotone. Memory: the
//! demuxer's own ceilings (section assembler caps, damage-log cap)
//! apply — this harness adds none.
```
```rust
/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budget above.
```
What is wrong: there is no damage-log cap. `DamageLog::note` (avi/header.rs:397-400) appends one `(u64, &'static str)` per recovery decision with no bound; `TsDemuxer` resets it once at `start` (ts/mod.rs:1092) and never drains during the walk; every TEI packet, bad header, CC jump, PSI CRC failure and skipped SI section mints an entry for the whole file. An all-TEI input grows the ledger at ~16 bytes per 188-byte packet (~8.5% of input size in RAM, no ceiling). The harness's headline contract — "never allocates without a ceiling — by construction of the demuxer" — is false, and the "damage-log cap" it cites does not exist (full analysis in my ts/mod.rs#part1 F-005; W04-A05 filed the AVI-context twin as avi/header.rs F-001).
Why it matters: this is the fuzz layer whose entire job is the absence-of-unbounded-allocation property; a libFuzzer campaign (or a shipped `lumen open` on a damaged capture — same demuxer, containers/ts.rs wiring never reads `damage_events`) would silently allocate input-proportionally. The shipped-binary variant of a "cannot happen" panic in this very family already happened once: `orchrun-ts188-panic-repro.log.txt` (188-byte TS file → shipped `lumen open` panics in the media-probe sniff, EXIT=101) — the panic-free-by-construction brand needs claims that survive tracing.
What correct looks like: give `DamageLog` a `MAX_DAMAGE_EVENTS` cap with a fold-into-one truncation event (the avi/index.rs coalescing precedent), then the two doc sentences become true as written.
Confidence: HIGH — Verification: read of both doc sites + the ledger (avi/header.rs:384-401) + no-drain grep (ts/mod.rs `damage`: 685, 1092, 660-668 only; none in apps/cli/src/containers/ts.rs).

### F-002 [S3]
Where: crates/demux/src/ts/fuzz.rs:5-8
Code:
```rust
//! drain every event to end-of-stream (budgeted) — the 188-byte
//! packet walk, PSI assembly (PAT/PMT/SDT), resync-on-garbage and the
//! PES timing model all ride that drain. **The harness contract is
//! absence-of-panic/hang/unbounded-allocation, nothing else** —
```
What is wrong: "the PES timing model all ride that drain" is structurally impossible. The harness constructs a bare `TsDemuxer` and never calls `set_pes_consumer`; with `pes: None` the demuxer counts PES packets/bytes and discards the payload (ts/mod.rs:930-938, my part1 F-008 cross-ref) — `pes.rs`'s assembler, PTS/DTS timing model, and discontinuity ledger are unreachable from this harness. The one shipped consumer (`BridgeConsumer`, containers/ts.rs:153) is registered by the CLI, not here.
Why it matters: a reader sizing fuzz coverage from the doc (or an auditor trusting "PES timing model" is fuzzed) overcounts by an entire 600-line subsystem; the fuzz target that claims MPEG-TS coverage silently skips the PES half.
What correct looks like: either register a collecting consumer in the harness (a 10-line `PesConsumer` impl pushing into a counter — the streaming tests' `PesCounter` shape) or strike "the PES timing model" from the claim.
Confidence: HIGH — Verification: read of the harness (no registration) + part1's verified delivery semantics.

### F-003 [S3]
Where: crates/demux/src/ts/fuzz.rs:15-18 (claim), 64-85 (the sweeps)
Code:
```rust
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's TS corpora.
```
```rust
    #[test]
    fn empty_and_sync_inputs_survive() {
        fuzz_demux_ts(&[]);
        fuzz_demux_ts(&[0x47]);
        fuzz_demux_ts(&[0x47; 188]);
    }
```
What is wrong: no committed input ever reaches the harness core. Traced input by input: `[]` → `SourceReader` refuses the empty source (`demux.source.empty`, avi/riff.rs:233-235) → early return; `[0x47]` → open-sync accepts at 0 but the PSI window holds less than one packet → `demux.ts.no-pat` → early return; `[0x47; 188]` and `[0x47; 4096]` → every packet is all-0x47 (PID 0x747, afc=0, no payload — the PAT PID is never fed) → no-pat → early return; `[0xff; 4096]` / `[0x00; 4096]` → no sync in the open-scan window → `demux.ts.not-ts` → early return; the 96 deterministic 1..96-byte buffers → too small to carry a PAT → early return. Every path exits at `if demuxer.start().is_err() { return; }` (45-47): the budget check, all four `match` arms, and the EOS/error exits have zero stable executions. The claim "the harness logic is meanwhile proven on stable by the deterministic sweeps below" is true only for the construct/refusal prefix. The genuine drain coverage lives in `tests/ts_fuzz_survival.rs` (512 LCG mutation rounds over a valid multi-program base stream, `survive()` = start + drain-to-EOS, executed ok in baseline test-workspace:2246-2248) — but that suite drives its own `TsDemuxer` helper, not `fuzz_demux_ts`, so the delegated body the libFuzzer campaign will run remains untested on stable.
Why it matters: the harness function is the exact code a nightly campaign executes; a broken budget loop or match arm (e.g. an arm that `break`s on `Recovered`) would ship undetected — the committed tests cannot fail for any defect in the loop, only for panics in `new`/`start`.
What correct looks like: one sweep input that passes `start` — e.g. `fuzz_demux_ts(&test_support::single_program_ts())` plus byte-flip mutations of it (the ts_fuzz_survival recipe in-crate) — making the drain, budget, and match arms reachable; then the "proven by the sweeps" sentence becomes true.
Confidence: HIGH — Verification: input-by-input trace against the audited `start` semantics (part1) + read of ts_fuzz_survival.rs's helper (87-97: it is a different function).

### F-004 [S4]
Where: crates/demux/src/ts/fuzz.rs:54-55
Code:
```rust
            Ok(DemuxEvent::Packet(_))
            | Ok(DemuxEvent::Recovered { .. })
```
What is wrong: the `Packet` arm is dead in this harness — `TsDemuxer` never mints `DemuxEvent::Packet` (part1-verified: the demuxer queues only `Discontinuity`/`Recovered`), and with no consumer registered nothing else can. The arm makes the match read as if packet delivery were exercised (pairs with F-002's false claim).
Why it matters: dead arms in a fuzz harness misrepresent coverage to every reader; harmless at runtime.
What correct looks like: keep it only with a comment ("future: the consumer lane"), or register the consumer that would make it live (F-002's fix).
Confidence: HIGH — Verification: read + part1 grep (no `DemuxEvent::Packet` construction in ts/mod.rs).

### F-005 [S4]
Where: crates/demux/src/ts/fuzz.rs:28-30
Code:
```rust
/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane capture's packet count]`.
pub const FUZZ_EVENT_BUDGET: usize = 200_000;
```
What is wrong: the rationale's arithmetic is inverted. A "largest sane capture" of 10 minutes at ~5 Mbit/s is ≈375 MB ≈ 2M packets — 200 000 is an order of magnitude BELOW that, not two above. The budget is fine for fuzz-sized inputs (the harness copies `data` wholesale, so inputs are small), but the PROPOSAL note justifies the number with a comparison that points the wrong way.
Why it matters: PROPOSAL annotations are the audit trail for magic numbers; a wrong one will be quoted back as authority.
What correct looks like: re-anchor the rationale to what the budget actually bounds (events per fuzz pass on small inputs), not to capture sizes it does not dominate.
Confidence: HIGH — Verification: read + arithmetic.

### F-006 [S4]
Where: crates/demux/src/ts/fuzz.rs:10-11 (claim), disproof in the baseline
Code:
```rust
//! `Recovered` events, both swallowed here; `ts_corpus` /
//! `ts_pes_corpus` pin the specific outcomes.
```
What is wrong: "ts_corpus / ts_pes_corpus pin the specific outcomes" — in the recorded baseline those suites skip-passed (corpus-v4 absent: ts_corpus 0.03 s, ts_pes_corpus 0.02 s — a real ffprobe differential cannot run that fast; both tests `continue`/`eprintln!("SKIP (loud)…")` and pass on zero lanes, ts_corpus.rs:216-222). The pinning claim is REAL-UNPROVEN in the recorded evidence.
Why it matters: the harness doc cites outcome-pinning suites whose "ok" lines in the baseline prove nothing; same evidence-class issue I corrected in my ts/mod.rs#part1 and mp4/test_support.rs audits.
What correct looks like: cite the suites with their corpus-gated status stated, or make them fail loudly when the corpus is absent.
Confidence: HIGH — Verification: read of the skip paths + baseline timings (test-workspace:2240-2257 area).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-25 | SHALLOW | S2/S3/S4 | F-001 (no damage-log cap), F-002 (PES unreachable), F-003 (sweeps prove only the prefix), F-006 (corpus pinning skip-passed); wiring-status paragraph itself is honest and verified (target + README row exist) |
| `FUZZ_EVENT_BUDGET` | 28-30 | REAL | S4 | value sane for fuzz inputs; rationale inverted (F-005) |
| `fuzz_demux_ts` | 35-58 | REAL (core untested) | S2/S3/S4 | budget loop correct by read; early returns make 3 exit paths; F-002 (no consumer), F-004 (dead Packet arm); allocation = input copy + demuxer's unbounded damage ledger (F-001) |
| test `empty_and_sync_inputs_survive` | 64-68 | SHALLOW | — | never passes `start` (F-003); pins only construct/refusal prefix |
| test `hostile_wide_inputs_survive` | 71-85 | SHALLOW | — | same — all inputs refused at `start`/not-ts; the 96 deterministic buffers likewise |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `empty_and_sync_inputs_survive` (64-68) | NO for anything in the drain loop (never entered); YES for a panic in `mem_stream`/`TsDemuxer::new`/`start` on empty/sync-shaped inputs | all three inputs exit at the `start().is_err()` early return (F-003 trace) |
| `hostile_wide_inputs_survive` (71-85) | NO for the drain loop; YES for a panic in the open-sync scan / not-ts / no-pat refusals on garbage | every input refused before the loop (F-003 trace) |
| (adjacent, not this file) `tests/ts_fuzz_survival.rs::mutations_never_panic_and_stay_deterministic` | YES | 512 mutation rounds over a VALID base stream with start+drain and determinism asserts — the coverage this file's doc claims for itself |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the … PES timing model all ride that drain" | fuzz.rs:5-8 | **FALSE** | no `set_pes_consumer` in the harness; `pes: None` discards payload (ts/mod.rs:930-938) — F-002 |
| "The harness contract is absence-of-panic/hang/unbounded-allocation" | fuzz.rs:8-9 | SHALLOW | the unbounded-allocation half is false: no damage-log cap (F-001) |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below" | fuzz.rs:16-18 | **FALSE** (drain half) | every committed input fails `start` — the budget loop/match arms never execute (F-003); ts_fuzz_survival covers the property via its own helper, not this function |
| "the cargo-fuzz target is fuzz/fuzz_targets/fuzz_demux_ts.rs" | fuzz.rs:15-16 | REAL | file exists, delegates verbatim; README registry row :62; baseline fuzz-registry gate PASS |
| "Memory: the demuxer's own ceilings (section assembler caps, damage-log cap) apply" | fuzz.rs:21-23 | **FALSE** (damage-log half) | no cap exists — DamageLog::note pushes unconditionally, never drained in TS wiring (F-001; part1 F-005) |
| "Never panics, never hangs, never allocates without a ceiling — by construction" | fuzz.rs:34-36 | SHALLOW | no hang path found (budget + monotone walk — verified); the allocation half false (F-001); "by construction" is an assertion, not a proof — cf. the shipped 188-byte-TS panic in the sniff layer (orchrun-ts188-panic-repro.log.txt) |
| "ts_corpus / ts_pes_corpus pin the specific outcomes" | fuzz.rs:10-11 | REAL-UNPROVEN | both suites skip-passed in the baseline (corpus-v4 absent) — F-006 |

## 7. Dead / duplicated / unreachable code
- The `Ok(DemuxEvent::Packet(_))` arm (54) — unreachable in this harness (F-004).
- Nothing else dead; the harness itself is unreachable from the shipped binary by design (fuzz-crate-only caller) — that is correct architecture, not a defect.

## 8. Dependencies & flags
- `plugin_api::{DemuxEvent, Demuxer}` + `super::test_support::mem_stream` (the shared AVI seam) only; no unsafe; no feature gates; compiled unconditionally (pub module) — the libFuzzer shim in the `fuzz` crate is the only external consumer.

## 9. Open questions for the next agent
- W05-A02 (ts/tests.rs): the one-line fix that makes F-003 false — `fuzz_demux_ts(&single_program_ts())` plus a mutation sweep over it; also worth pinning that the budget actually terminates a pathological event stream.
- Whoever owns DamageLog (avi/header.rs — W04-A05 filed it): the cap fix (F-001) belongs there; this file and my part1 audit both cite it.
- A10 (ts/mod.rs part2): if you file the PES-error swallow (my part1 F-002 cross-ref), note that this harness's `Err(_) => break` masks it by design — the two behaviors compose into "a PES consumer error ends the fuzz pass silently", which is correct for fuzzing but worth stating.
