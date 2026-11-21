# AUDIT: crates/media-probe/src/contract.rs (510 lines, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 510 / 22 items (2 types + 2 impls + trait + 2 assert fns + 5 tests + accessors) |
| File verdict | REAL (unit half executed green; the real-stack conformance half it exists for has ZERO committed executions — F-001) |
| Reachable from shipped entry point? | NO by design — `#[cfg(any(test, feature = "contract"))]` (lib.rs:160-161), feature `contract = []` (Cargo.toml:36), no shipped dependency enables it (apps/cli/Cargo.toml:56 plain dep); "release binaries carry none of it" claim VERIFIED |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | read whole file; grep of nofailfast log → 5/5 unit tests ok at lines 3317-3321 and `probe_io` "running 0 tests" at 3416-3420; grep of test-workspace-allfeatures.log.txt → no probe_io execution (died at first-vertical evidence.rs, line 2468, after 116 binaries); grep ci.yml/ci-full.yml for the all-features test string; verified lib.rs:159-161 cfg gate and Cargo.toml:30-36 feature |

## 1. One-paragraph verdict
The instrumented-source machinery is real, careful and fully unit-tested: both ledger
types record `(requested, served)` pairs with faulted reads excluded, the seek/read
implementations are bounds-checked with typed errors, the depth-bound assertion encodes
the fill loop's own arithmetic, and all five unit tests — including catch_unwind proofs
that the assertions actually fire — passed in the nofailfast baseline (4534/7/49 suite).
The module is correctly cfg-gated out of release binaries. The defect is evidentiary:
this module's stated purpose is to power the real-access-stack conformance suite
(`tests/probe_io.rs`, 10 tests, "the same executable proof serves … the doc 06 §7
conformance kit"), and that suite has executed ZERO tests in every committed baseline —
the default workspace run compiles it empty (feature off) and the all-features run died
fail-fast at first-vertical before reaching it. The probe-depth AC over the real stack is
therefore UNPROVEN in committed evidence, exactly like the input-crate contract suite the
orchestrator had to re-run for W03 (orchrun-test-input-contract). Residue: one
mislabeled catch_unwind that never exercises the total-served guard it names, and an
unpinned ledger-exclusion contract.

## 2. What actually works (verified)
— 5/5 unit tests ok: nofailfast log lines 3317-3321 (`chunk_limit_simulates_short_reads`,
`counting_source_serves_and_instruments`, `counting_stream_forwards_and_instruments`,
`depth_assertions_fail_loudly_on_violations`, `unseekable_and_fault_scripting_work`).
— cfg gate verified: lib.rs:160-161 `#[cfg(any(test, feature = "contract"))] pub mod
contract;` + Cargo.toml:36 `contract = []`; no shipped crate turns the feature on
(apps/cli/Cargo.toml:56 plain dependency) — the "release binaries carry none of it" claim
holds.
— Ledger semantics traced: `CountingSource::read` pushes to `requests`/`served` only on
the success path (the scripted fault returns at 166-168 before the ledger); the
`assert_probe_depth_bounded` loop (332-346) enforces per-request `request <= budget`
with budget spent by served bytes (`budget -= got.min(budget)` — underflow-safe) and a
total guard; `counting_stream_forwards_and_instruments` pins the adapter's parallel
ledgers `&[12, 100]` / `&[12, 4]` exactly.
— `CountingSource::seek` handles `SeekOrigin::Current` with `checked_add` and refuses
negative/out-of-range targets typed (189-215); `read` slices with `take <= available` —
no panic path found.

## 3. Findings

### F-001 [S3] The real-stack conformance suite this module exists for has never executed a single test in any committed run
Where: crates/media-probe/src/contract.rs:18-21 (the claim); evidence: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3416-3420; crates/media-probe/tests/probe_io.rs:1-30
Code (contract.rs:18-21):
```
//! The `assert_*` checks encode the probe-side contract
//! exactly as the engine will consume it, so the same executable proof
//! serves this crate's tests, the future playback-engine wiring tests
//! and the doc 06 §7 conformance kit:
```
Evidence (nofailfast log, verbatim):
```
     Running tests/probe_io.rs (target/release/deps/probe_io-43f3d2f4e08220b6)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
What is wrong: `probe_io.rs` is `#![cfg(feature = "contract")]` (probe_io.rs:30) — the
default workspace test run compiles it to zero tests (the log above), and the only
committed all-features run (test-workspace-allfeatures.log.txt) aborted fail-fast at
`tests/integration/first-vertical/tests/evidence.rs` (log line 2468, "error: test failed
… EXIT=101") after 116 binaries — media-probe was never reached. No `orchrun-*` file in
the baseline covers `-p media-probe --features contract`. So the depth-bound AC over the
REAL access stack (`CountingStream` wrapping `input`'s `FileAccess`/`FileStream`,
probe_io.rs:8-10, 36-38, with `assert_probe_depth_bounded` at 115/142/198) — the exact
thing this module was built to prove — has no execution evidence anywhere in the repo's
committed runs. The probe_io.rs doc's own claim "CI's `--all-features` always does"
(ci-facing claim) is not backed either: ci.yml:234 runs the string, but on the current
tree that run dies at the same first-vertical failures before media-probe, per the
committed all-features log.
Why it matters: the probe-depth bound (card 2.A2's "probe-depth bound tested" AC) is an
architecture guarantee the engine relies on (bounded I/O before container selection);
its only green evidence is synthetic in-memory sources. The identical gap in the input
crate was considered worth an orchestrator run by three W03 agents
(orchrun-test-input-contract) — this is the same class of hole one crate later.
What correct looks like: run `cargo test -p media-probe --features contract --release
--locked` in the wave gate and commit the log (see REQUEST-ORCH-RUN in my log); pin the
suite's execution in CI with `continue-on-error: false` and a non-fail-fast or
package-scoped ordering so a first-vertical failure cannot mask it again.
Confidence: HIGH — Verification: read both baseline logs by grep; counted 10 `#[test]`
in probe_io.rs.

### F-002 [S4] The total-served guard is never exercised; the test that claims to fire it fires the per-request assert instead
Where: crates/media-probe/src/contract.rs:453-463
Code:
```
        // A short read spends only what it served: 6 asked, 4 served
        // leaves budget for a later ask — the ledger semantics.
        let mut short = CountingSource::new(b"abcd");
        short.set_chunk_limit(4);
        let _ = short.read(6); // asks 6, serves 4
        let _ = short.read(2); // asks 2, serves 0 (EOS)
        assert_probe_depth_bounded(&short, 6);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_probe_depth_bounded(&short, 5)
        }));
        assert!(caught.is_err(), "total-served bound must fire at 5");
```
What is wrong: with requests `[6, 2]` and a bound of 5, the FIRST assertion in
`assert_probe_depth_bounded` fires (`read #0 asked for 6 with only 5 of the 5-byte budget
left`, contract.rs:335-339) — not the total-served bound the catch_unwind's message
names. Total served is 4 ≤ 5, so the total assert (343-346) would NOT fire at all. The
total guard exists for exactly one scenario the per-request assert cannot catch: a stream
that SERVES more bytes than requested (a real `SourceStream` contract violation); that
scenario is untested anywhere in the crate. The message mislabels what was proven.
Why it matters: a future refactor that drops or breaks the total-served assert (the only
over-serving guard) passes this suite unchanged.
What correct looks like: a ledger stub whose `served` exceeds `requests` (ReadLog is a
trait — one six-line fake) driven through `assert_probe_depth_bounded` to prove the
total assert fires; fix the message.
Confidence: HIGH — arithmetic traced by hand from the quoted code.

### F-003 [S4] The faulted-read ledger-exclusion contract has no assertion
Where: crates/media-probe/src/contract.rs:405-417
Code:
```
        source.fail_next_read();
        let err = match source.read(2) {
            Ok(_) => unreachable!("scripted failure"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "probe-test.scripted-read-failure");
```
What is wrong: the module doc (contract.rs:43-44) makes "faulted reads are excluded —
they served nothing" a contract of the ledger, `CountingSource::read` implements it (the
fault returns at 166-168 before `requests.push`), but the test never asserts
`requests()`/`served()` after the faulted read — the exclusion is documented, implemented,
and unpinned. Same for `CountingStream` (294-296: the `?` forwards the error before the
ledger pushes).
What correct looks like: `assert_eq!(source.requests(), &[])` (or the post-fault
cumulative ledger) in `unseekable_and_fault_scripting_work`.
Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-36 | REAL | S3 | the "serves the conformance kit" claim is currently evidence-free (F-001) |
| `ReadLog` (trait) | 49-52 | REAL | — | parallel ledgers; consumed by the assert |
| `CountingSource` (struct) | 57-74 | REAL | — | fields consistent with docs |
| `CountingSource::new` | 84-101 | REAL | — | `unreachable!` on a literal URI — sanctioned test vocabulary, documented |
| `with_uri` | 104-108 | REAL | — | extension-hint tests |
| `set_unseekable` / `set_chunk_limit` / `fail_next_read` | 111-129 | REAL | — | the three scripted faults the doc promises |
| accessors (4) | 132-149 | REAL | — | trivial |
| `SourceStream::read` | 165-179 | REAL | — | ledger push only on success; `take <= available` — no panic; short-read-to-EOS is Ok(data) (pinned by test) |
| `SourceStream::seek` | 181-218 | REAL | — | checked_add for Current; negative and past-end typed; Start bounds-checked |
| `SourceStream::position`/`uri`/`size`/`supports_seek` | 152-163, 220-222 | REAL | — | `size` always Some (test-source simplification, fine) |
| `ReadLog for CountingSource` | 225-229 | REAL | — | |
| `CountingStream` (struct + new + accessors) | 240-278 | REAL | — | |
| `SourceStream for CountingStream` | 281-310 | REAL | — | read logs after success (fault excluded ✓); seek forwarded unlogged (documented 238-239) |
| `ReadLog for CountingStream` | 312-316 | REAL | — | |
| `assert_probe_depth_bounded` | 324-347 | REAL | S4 | underflow-safe budget; per-request + total guards; total guard untested (F-002) |
| `assert_position_restored` | 352-359 | REAL | — | weak form only (position), honestly documented ("stronger form is tested where the data is at hand") |
| `counting_source_serves_and_instruments` | 365-397 | REAL | — | ledgers `[4,100,8]`, short-read, EOS-empty, seek round-trip — all absolute |
| `unseekable_and_fault_scripting_work` | 399-418 | REAL | S4 | fault exclusion unpinned (F-003) |
| `chunk_limit_simulates_short_reads` | 421-438 | REAL | — | loop with `read(30-got.len())`, total 30 pinned |
| `depth_assertions_fail_loudly_on_violations` | 440-464 | REAL | S4 | first catch_unwind correct; second mislabeled (F-002) |
| `counting_stream_forwards_and_instruments` | 466-509 | REAL | — | adapter ledgers exact; depth bound 112 pass / 15 fire — messages correct here |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| counting_source_serves_and_instruments | YES | byte-exact reads and ledgers |
| unseekable_and_fault_scripting_work | YES for the fault/unseekable behavior; NO for ledger exclusion | F-003 |
| chunk_limit_simulates_short_reads | YES | total_served 30 with limit 7 |
| depth_assertions_fail_loudly_on_violations | YES for the per-request bound; NO for the total bound | F-002 |
| counting_stream_forwards_and_instruments | YES | exact ledgers + both bound outcomes |
| (tests/probe_io.rs — the real-stack consumer) | UNKNOWN | zero committed executions (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "release binaries carry none of it" | contract.rs:4-10, lib.rs:31-33, Cargo.toml:30-35 | REAL | cfg gate verified at lib.rs:160-161; no shipped dep enables the feature |
| "the same executable proof serves … the doc 06 §7 conformance kit" | contract.rs:19-21 | REAL-UNPROVEN | probe_io.rs exists and imports it (26-27) but executed 0 tests in every committed run (F-001) |
| "faulted reads are excluded — they served nothing" | contract.rs:43-44 | REAL | implemented (166-168, 294-296) but unpinned by any assert (F-003) |
| "assert! / unreachable! are the sanctioned test-side vocabulary" | contract.rs:33-36 | REAL | workspace lints deny panic!/unwrap (baseline clippy -D warnings clean, INDEX #5) |

## 7. Dead / duplicated / unreachable code
Nothing dead; the module is unreachable from shipped builds **by design and correctly so**
(verified). The `crate::codecs::tests/contract.rs` / `codec-api` contract modules are the
same pattern elsewhere (not this file's problem).

## 8. Dependencies & flags
Only `media_types` + `plugin_api` — no I/O, no unsafe, no external crates. The
`contract = []` feature has no defaults leaking (verified Cargo.toml:29-36). The
integration consumer is feature-gated coherently (`#![cfg(feature = "contract")]` at
probe_io.rs:30).

## 9. Open questions for the next agent
1. If the orchestrator's wave-gate run of `-p media-probe --features contract` lands,
   update the F-001 verdict from UNPROVEN to REAL/PASS accordingly (my log carries the
   REQUEST-ORCH-RUN).
2. probe_io.rs's depth bound (`bound` at 115/142/198) — check what value it uses against
   the registry's configured `max_lookahead`; a bound looser than the configured one
   proves nothing (I did not audit probe_io.rs — not my assignment).
3. The input crate's contract suite passed on re-run (orchrun-test-input-contract 6/6);
   media-probe's is the sibling gap — whoever owns audit/docs/ gate-parity claims should
   reconcile "CI's --all-features always does" with the fail-fast reality that a
   first-vertical failure masks every later crate.
