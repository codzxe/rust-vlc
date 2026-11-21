# AUDIT: crates/codec-api/src/contract.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Chunk boundary audited | **lines 1-453** (part1 of 2; ends at `flush_drains`'s closing brace — the natural function boundary. Part2 (W06-A02) = 455-869: corrupt_input_recovers_or_errors_typed, sink_errors_propagate, encoder mirrors, in-file tests) |
| Lines / functions | 453 / 14 (ContractViolation type + 2 impls, 4 sinks, 2 failing sinks, marker/error/  detector, 2 agreement fns, stale check, flush_drains) |
| File verdict (this part) | UNREACHABLE (test kit compiled into the shipped lib; quality is REAL and executed — see below) |
| Reachable from shipped entry point? | NO production caller: `codec_api::contract` consumers = crates/codec-api/tests/contract_suite.rs and crates/codecs/tests/contract.rs ONLY (rg over crates/ apps/ tools/; media-probe/input hits are their own unrelated `contract` modules). Deliberate design per codec-api/Cargo.toml:25-27 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg "contract::|codec_api::contract" repo-wide (callers); baseline test log :246-250, :259-268 (suite executed, 8 passed incl. rogue-decoder catch), :911-924 (codecs contract, 9 passed); sed -n quote verification |

## 1. One-paragraph verdict
This is the executable decoder/encoder contract suite — and unusually for this
repo, it does what it claims: the checks are real finite driving loops with
typed violation reporting, and the baseline proves both directions (real
codecs pass it: codecs/tests/contract.rs 9/9 ok, baseline :911-924; a
deliberate ROGUE decoder is caught: `rogue_decoders_are_caught_by_the_suite …
ok`, baseline :267). Part1's own logic — outcome/sink agreement, stale-drop,
drain-to-sticky-Drained — is correct by close reading: every arm I traced
matches its documented rule, sinks count saturatingly, the sticky check
re-flushes with a fresh sink. The findings are three S4s: the flush-drain
bound's "each call must make progress" is not enforced on the Recovered arm;
the `Recovered`-unconstrained agreement hole lets a decoder report
`Recovered { skipped: 0 }` with any non-Corrupt reason while emitting
arbitrary frames; and this test kit ships inside the production library
(compiles into every shipped binary, called by nothing at runtime) — a
documented but real layering cost. No S0-S3 found in 453 lines.

## 2. What actually works (verified)
— Baseline execution, both directions:
  `audit/runs/baseline/test-workspace.log.txt:259-268` — contract_suite 8/8
  ok, including `rogue_decoders_are_caught_by_the_suite` (a mutation-style
  test: deliberately broken decoders MUST be flagged — this is the suite
  proving it can fail) and `mock_decoder_passes_the_contract_suite`.
  `:911-924` — codecs/tests/contract.rs 9/9 ok (the suite against the REAL
  codec set: stale drops, flush sticky, sink propagation, outcome/sink
  agreement across the corpus).
— Sticky-Drained verification (415-428): after the first Drained, a SECOND
  flush with a fresh sink must also be Drained with zero accepts — read; this
  catches decoders that "drain" but keep emitting.
— `decode_outcome_agrees_with_sink` (288-314): Emitted{0} rejected,
  Emitted{n}≠accepts rejected, NeedMoreInput/Drained with accepts rejected,
  `_ => Ok(())` covers exactly the legal remainder — read, arm-by-arm.
— `is_sink_failure` (213-225): verbatim context-code match OR full source
  chain walk with downcast — read.

## 3. Findings

### F-001 [S4] flush_drains's "each call must make progress" is not enforced on the Recovered arm
Where: contract.rs:64-69 (doc) and 443-447 (code)
Code:
```rust
/// How many `flush` calls the drain check will make before declaring
/// the drain unbounded. Generous: each call must make progress (accept
/// at least one frame), so this bounds buffered backlog, not per-call
/// work.
const MAX_FLUSH_CALLS: u32 = 1024;
```
```rust
            Ok(outcome @ DecodeOutcome::Recovered { .. }) => {
                // Recovery mid-drain is legal (e.g. HwFallback); the
                // loop's bound still applies.
                decode_outcome_agrees_with_sink(outcome, sink.accepted() - before)?;
            }
```
What is wrong: the Emitted arm requires progress (437-441: "flush made no
progress"); the Recovered arm does not. A decoder that answers every flush
with `Recovered { skipped: 0 }` and zero accepts passes all 1024 iterations
before the "never reached Drained" violation fires — 1024 full flush calls of
possibly unbounded per-call work, contradicting the doc's claim that the
bound "bounds buffered backlog, not per-call work".
Why it matters: the termination guarantee holds (1024 iterations), so this is
enforcement/doc drift, not a hang; a hostile decoder burns the budget slowly.
What correct looks like: require progress on Recovered too (accepted delta or
skipped > 0), or fix the doc.
Confidence: HIGH — Verification: read.

### F-002 [S4] `Recovered` is unconstrained in outcome/sink agreement — a legal escape hatch for emitting decoders
Where: contract.rs:281-284
Code:
```rust
/// Outcome/sink agreement for decode calls: `Emitted { frames: n }`
/// means exactly `n` accepts happened in that call (and `n >= 1`);
/// `NeedMoreInput`/`Drained` mean zero; `Recovered` is unconstrained
/// (concealed output is legal).
```
What is wrong: any decoder can accept 5 frames into the sink and report
`Recovered { skipped: 0, reason: <not CorruptInput> }` — the agreement check
returns Ok. The corrupt-input check (part2, 475-479) only demands skipped ≥ 1
when the reason is CorruptInput. The hole is DOCUMENTED ("concealed output is
legal") and motivated (concealment), but nothing distinguishes "recovered
with concealed output" from "recovered while emitting an unaccounted frame
count".
Why it matters: the suite's strongest invariant (frames are counted exactly)
is opt-out per call via one variant; a sloppy decoder that conflates Emitted
into Recovered passes.
What correct looks like: extend DecodeOutcome::Recovered with a frames count
(a codec-API change), or document the accounting rule consumers must use
instead.
Confidence: HIGH — Verification: read; the vp9 decoder's take_damage relies
on exactly this hole (my vp9 audit — noted there).
Severity: S4 (documented design choice; no shipped behavior depends on the
count).

### F-003 [S4] Test kit ships inside the production library
Where: crates/codec-api/src/lib.rs:109 (`pub mod contract;`) and codec-api/Cargo.toml:25-27
Code (Cargo.toml:25-27):
```
# No dev-dependencies: the contract suite lives in the library itself
# (`src/contract.rs`) so Phase-2 codec crates can call it without a new
# edge in the dependency graph.
```
What is wrong: the module (sinks, checks, violation type) is compiled into
`libcodec_api` and linked into every shipped `lumen` binary via the codecs
dependency, but has zero runtime callers. It is test infrastructure riding a
production crate — the documented motivation (avoiding a dev-dependency edge)
is a convenience argument, not an architectural one.
Why it matters: dead code in shipped artifacts (stripped only if LTO/
dead-code-elimination cooperates), plus a standing invitation for production
code to grow dependencies on test-kit types.
What correct looks like: a `codec-api-testkit` crate (dev-dependency) or
`#[doc(hidden)]`-gated feature; at minimum a lib.rs doc note that the module
is dev-surface.
Confidence: HIGH — Verification: rg callers (tests only) + read.
Severity: S4 (documented, bounded).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ContractViolation (+detail/Display/Error) | 76-101 | REAL | — | string-detail only, no machine code — documented at 71-75; fine for a test kit |
| CountingFrameSink | 107-142 | REAL | — | saturating counter + retained frames + reset |
| CountingPacketSink | 144-180 | REAL | — | mirror |
| SinkFailureMarker / sink_failure | 186-209 | REAL | — | Present/Device/Disconnected shape "no decoder mints itself" (197-198) — a documented heuristic; a hw decoder surfacing display loss as the same shape would collide (see F-003 note in §9) |
| is_sink_failure | 213-225 | REAL | — | verbatim code OR full source-chain downcast walk |
| FailingFrameSink | 230-252 | REAL | — | counts attempts (distinguishes swallow from never-called) |
| FailingPacketSink | 254-277 | REAL | — | mirror |
| decode_outcome_agrees_with_sink | 288-314 | REAL | S4 | exact Emitted/NeedMoreInput/Drained rules; Recovered hole F-002; executed both directions in baseline |
| encode_outcome_agrees_with_sink | 321-347 | REAL | — | exact mirror |
| stale_packet_is_dropped_on_sight | 360-397 | REAL | — | fixture-bug guard, zero-accept assert, post-drain usability; executed (codecs contract suite) |
| flush_drains | 405-453 | REAL | S4 | bounded 1024; Drained sticky re-check with fresh sink; Emitted progress check; Recovered arm drift F-001; executed |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| contract_suite: rogue_decoders_are_caught_by_the_suite (baseline :267) | Yes — in the CAUGHT direction | deliberately rogue decoders must produce violations; this is the mutation test most suites lack |
| contract_suite: mock_decoder/encoder_passes_the_contract_suite (:264-266) | Yes — in the PASS direction | a well-behaved mock must clear every check |
| in-file contract::tests (baseline :246-250) | Yes | agreement rejects lies and accepts truth; sinks retain/reset; failing sinks count; marker detected verbatim+chained |
| codecs/tests/contract.rs 9 lanes (:911-924) | Yes, against the REAL decoders | suite run per codec incl. corpus outcome agreement |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every check is a finite driving loop, so a decoder that never returns hangs the test" | 54-56 | REAL | read: all loops bounded (1024 max); F-001 notes one arm's bound is looser than documented |
| "Panic-freedom cannot be enforced without poisoning the worker" | 57-58 | REAL | honest limitation, documented |
| "Recovered is unconstrained (concealed output is legal)" | 283-284 | REAL (documented hole) | F-002 |
| "Generous: each call must make progress" | 65-66 | SHALLOW | not enforced on the Recovered arm (F-001) |

## 7. Dead / duplicated / unreachable code
- The module itself is unreachable from shipped entry points (tests-only
  consumers; see header chain). It is the canonical example of
  real-code-no-caller in this repo — but by explicit design (Cargo.toml
  comment), which is why this is S4 and not an S3 "UI wired to nothing"
  class finding.
- decode/encode agreement functions are near-duplicates by design (mirrors);
  acceptable.

## 8. Dependencies & flags
- No features, no unsafe, no external deps (crate policy: media-types only —
  honored: imports at 60-63 are crate-internal + media-types).
- Compiles into every shipped build (see F-003).

## 9. Open questions for the next agent
- part2 (W06-A02): does `corrupt_input_recovers_or_errors_typed`'s
  ErrorKind::Decoder/Corrupt-only acceptance reject hw decoders that answer
  corruption with ErrorKind::Device (the vaapi lane's DeviceLost)? If yes,
  the suite itself would flag the hw lane's error mapping — check
  h264/decode.rs's H264HwError→EngineError mapping.
- codec-API evolution: if DecodeOutcome::Recovered grows a frames count
  (F-002), vp9's take_damage and the pipeline's damage accounting need a
  coordinated update.
