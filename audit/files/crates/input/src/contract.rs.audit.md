# AUDIT: crates/input/src/contract.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 651 / 21 (14 production units + 7 tests) |
| File verdict | REAL (as what it claims to be: test/conformance-only infrastructure; its in-file unit tests executed green in the no-fail-fast baseline) |
| Reachable from shipped entry point? | NO — **by design and verified**: `crates/input/src/lib.rs:64-65` gates the module `#[cfg(any(test, feature = "contract"))]`; `crates/input/Cargo.toml:30` declares `contract = []` with no default enable; release binaries carry none of it (doc claim, consistent with the cfg). This is a deliberate audit-F-2 decision, not a defect. The code it *tests* (`BufferedReader`, `FileAccess`) is reachable via the shipped open lane (INDEX.md: AVI open + `sample-420.mp4` play). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | full read via Read tool (651 lines); `rg -n "contract" crates/input/src/lib.rs crates/input/Cargo.toml` (gating: lib.rs:64-65,73; Cargo.toml:24-30, feature non-default); `rg -c "#\[test\]" crates/input/src/*.rs` (contract.rs 7 of input's 33 lib tests) + orchrun log line 2603-2605 `input-… running 33 tests` → `test result: ok. 33 passed` (in-file tests EXECUTED green); `rg -n "deps/input|Running" audit/runs/baseline/test-workspace-allfeatures.log.txt` (all-features run aborted after `device_discovery`/`first_vertical` binaries — **input never ran**; last Running lines 2424-2468); `sed -n '2603,2640p'` + awk over orchrun no-fail-fast log (truncation.rs binary ran with **0 tests** — feature off: "test result: ok. 0 passed; 0 failed; 0 ignored; 0 filtered out" for tests/truncation.rs); `rg -c "#\[test\]" crates/input/tests/truncation.rs` (6 tests, cfg-gated out of both committed runs); `rg -n "assert_stream_contract|assert_read_into_contract|assert_bounded_read|assert_peek_into_contract|assert_truncated" crates/ apps/ tests/ --glob '!crates/input/**'` (external reuse: NONE — demux has its own local `assert_truncated` (tests/fixtures_class_b.rs:41), media-probe has its own `media_probe::contract`); `rg -n "use input" crates/streaming/tests/*.rs` + `sed -n '55,70p' crates/streaming/Cargo.toml` (streaming uses `input::FileAccess` dev-only, never the executable contract); `rg -n "cargo test" .github/workflows/ci.yml .github/workflows/ci-full.yml` (ci.yml:234 `cargo test --workspace --all-features`, ci-full.yml:169 same) |

## 1. One-paragraph verdict
The module is honest about what it is — a scripted synthetic source plus five
`#[track_caller]` executable contracts (read/seek/EOF, borrow-twin, peek,
bounded-read, truncation) — and its in-file unit tests are real, mutation-
sensitive, and executed green (7 of input's 33 passing lib tests in the
no-fail-fast baseline). Two claims around it do not survive cross-examination:
the doc's "every access implementation … runs the same executable contract
from its own tests" is one-third true — streaming (Phase 6) never imports
`input::contract` (it uses only `input::FileAccess` as a dev-dependency for a
byte-parity proof) and the "conformance kit (doc 06 §7.2)" is future tense in
the Cargo.toml's own words; and the feature-gated integration suites that
*this* module exists to serve (`tests/truncation.rs`, 6 tests; the
contract-gated test in `tests/file_stream.rs`) have **zero executed evidence
in any committed baseline**: the no-feature runs compile truncation.rs to
zero tests, and the one --all-features run aborted at first-vertical (after
device_discovery, before input) — the same fail-fast evidence hole the
orchestrator's no-fail-fast repair run exposed at workspace scale. The
instrumented-source code itself is sound, with one silent-skip guard in
`ScriptSource::read` that should be a loud assertion.

## 2. What actually works (verified)
— In-file unit tests executed green — verified by
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` lines
2603-2605 (`Running unittests src/lib.rs (…input-8bdc49c2a5464ff2)`, 33
tests) + result line "ok. 33 passed"; contract.rs contributes exactly 7
(counted with `rg -c "#\[test\]"`).
— `ScriptSource` fault scripting — verified by trace: `shrink_to` is
monotonic (`self.visible_len.min(new_len)` — a file cannot un-truncate),
`fail_next_read` arms a one-shot typed error consumed before any
instrumentation (a failed read does not count as served), `max_read_seen`/
`reads_served` are honest instruments for the bounded-read and no-I/O-seek
proofs.
— `assert_stream_contract` (257-360) — verified by trace: varied chunk
sizes [1,2,3,7,64,1000] with position tracking after every read, sticky EOF
(twice), single big read after seek-to-0, seek-to-length = clean EOF, relative
seek round-trips (+3/−7), past-end seek → typed `input.seek.out-of-range`
with position unchanged, before-start → typed `input.seek.before-start`.
Mutation-hostile: break any StreamReader behavior and some assertion fails.
— `assert_bounded_read` (373-417) — verified by trace: pins buffer ≤
capacity, capacity never grows, source never sees a read larger than
`max(want, capacity)`, and the exact reads-served arithmetic (bulk call + one
refill when the read completes) — the executable form of card 2.A1 AC.
— `shrink_mid_read_is_a_typed_truncation_error` (587-598) — executed green
in baseline; asserts `(expected, found) == (1000, 400)` and code
`input.file.truncated` — this is the *unit-level* proof the truncation
contract lives on.
— `seek_within_buffer_serves_no_reads` (613-650) — executed green; pins
buffer-hit seeks to zero source reads via `reads_served` arithmetic.

## 3. Findings

### F-001 [S3]
Where: crates/input/src/contract.rs:21-23 (doc claim); refuted by crates/streaming/Cargo.toml:63-67 + rg over streaming tests
Code:
```rust
//! Every access implementation — this crate, `streaming` (Phase 6), the
//! conformance kit (doc 06 §7.2) — runs the same executable contract
//! from its own tests:
```
What is wrong: of the three claimed consumers of "the same executable
contract", only `input` itself runs it. `rg` over the whole tree for
`assert_stream_contract|assert_read_into_contract|assert_bounded_read|
assert_peek_into_contract|assert_truncated` (excluding crates/input) finds
zero call sites in streaming — streaming's only input usage is
`input::FileAccess` as a dev-dependency for a byte-parity e2e
(crates/streaming/Cargo.toml:63-67, tests/ranged_wav_e2e.rs). The "conformance
kit (doc 06 §7.2)" does not exist: Cargo.toml:27-28 calls it "the future doc
06 §7.2 conformance kit".
Why it matters: the doc sells cross-implementation equivalence testing ("every
access implementation runs the same contract") that has not landed for the one
other implementation that exists (streaming). A reader would believe the
network access path is held to the same read/seek/EOF/truncation bar as the
file path; it is not.
What correct looks like: either streaming's tests call
`input::contract::assert_stream_contract`/`assert_read_into_contract` on a
`RangedResource`-backed stream (dev-dep edge already exists), or the doc says
"this crate today; streaming planned".
Confidence: HIGH — Verification: read + rg (full-tree search, results above).

### F-002 [S3]
Where: crates/input/src/contract.rs:4-7 (doc) and crates/input/tests/truncation.rs:22-24 ("run with `--features contract` (CI's `--all-features` always does)"); evidence gap in audit/runs/baseline/*
Code:
```rust
//! is `#[cfg(any(test, feature = "contract"))]` — unit tests compile
//! it always, integration tests and the doc 06 §7.2 conformance kit
//! enable the `contract` feature, and **release binaries carry none
```
What is wrong: the feature-gated integration suites this module serves have
no executed evidence anywhere in the committed baselines. Proof chain:
(a) no-fail-fast workspace run (orchrun log): `tests/truncation.rs` ran with
`0 passed; 0 failed; 0 filtered out` — the whole 211-line file is
`#![cfg(feature = "contract")]` and the feature was off; (b) the only
--all-features run (test-workspace-allfeatures.log.txt) aborted at the
first-vertical failures — its last executed binaries are `device_discovery`
(2413-2424) then `first_vertical` (2433-2468); no `deps/input` binary appears
anywhere in that log. So truncation.rs's 6 tests and file_stream.rs's
contract-gated test have never been shown green in committed evidence, and
cargo's fail-fast-per-binary semantics means any earlier binary failure
(currently: the first-vertical drift) silently skips them again — the exact
mechanism the orchestrator's repair run exposed (baseline 1681/1/24 vs true
4534/7/49).
Why it matters: "CI's --all-features always does" is a promise about a runner
that no committed run substantiates for this crate, and the promise is fragile
to exactly the failure mode now present in the tree.
What correct looks like: a committed `--all-features --no-fail-fast` run
(REQUEST-ORCH-RUN below), or per-crate evidence runs (`cargo test -p input
--features contract`), cited next to the claim.
Confidence: HIGH — Verification: read + baseline logs (cited above, lines
verified with awk/rg over both log files).

### F-003 [S4]
Where: crates/input/src/contract.rs:151-157
Code:
```rust
        let start = self.pos as usize;
        let end = start + n;
        if end <= self.data.len() {
            out[..n].copy_from_slice(&self.data[start..end]);
        }
        self.pos += n as u64;
        Ok(n)
```
What is wrong: the guard's false arm is a **silent skip** — `pos` still
advances and `Ok(n)` is returned while `out[..n]` keeps the caller's previous
bytes. Today the branch is unreachable (`visible_len` starts at `data.len()`
and `shrink_to` only min-reduces, so `start + n ≤ visible_len ≤ data.len()`
always), which makes the guard a defense that lies: if the invariant ever
broke, dependent tests would compare against stale buffer contents instead of
failing loudly at the source.
Why it matters: this is test *infrastructure* — its whole job is to fail
loudly; a silent-garbage path in the fault-injection source is exactly the
tautology-shaped hole the checklist hunts.
What correct looks like: `assert!(end <= self.data.len(), "visible_len invariant broken")` before the copy (the module's own sanctioned vocabulary).
Confidence: HIGH — Verification: read (invariant chain verified: new() 88,
shrink_to() 101, read() 149-150).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ScriptSource::new` | 85-94 | REAL | — | `to_vec` copy fine for test source; `visible_len = data.len() as u64` (usize→u64 lossless). |
| `ScriptSource::shrink_to` | 100-102 | REAL | — | Monotonic min — models truncation honestly. |
| `ScriptSource::fail_next_read` | 105-110 | REAL | — | One-shot typed I/O error (Unsupported kind). |
| `ScriptSource::max_read_seen` / `reads_served` | 113-121 | REAL | — | Honest instruments; consumed by bounded-read/peek/no-I/O-seek proofs. |
| `Seekable for ScriptSource` (position/length/seek) | 124-140 | REAL | — | seek refuses `> visible_len` typed, allows `==` (EOF position); no panic paths. |
| `RawRead for ScriptSource::read` | 143-158 | REAL | S4 | F-003 silent-skip guard; failure consumed before instrumentation (correct); `saturating_sub` + `min` arithmetic safe. |
| `SyntheticStream::new` | 175-179 | REAL | — | Capacity validated by BufferedReader; synthetic URI literal carries an explicit allow-literal comment. |
| `SyntheticStream::source`/`shrink_to` | 182-189 | REAL | — | Accessors/delegation. |
| `SourceStream for SyntheticStream` (6 methods) | 192-224 | REAL | — | Thin delegation; `supports_seek() = true` (this is the seekable contract's shape); seek via `resolve_origin` + `reader.seek`. |
| `SyntheticStream::peek_into` | 231-233 | REAL | — | Delegation to reader's pure window math. |
| `SyntheticStream::read_exact_into` | 237-239 | REAL | — | Delegation. |
| `assert_truncated` | 243-251 | REAL | — | `unreachable!` on wrong kind — sanctioned test vocabulary (doc 52-55). |
| `assert_stream_contract` | 256-360 | REAL | — | See §2; the strongest check in the file; no tautologies found. |
| `assert_bounded_read` | 372-417 | REAL | — | Pins reads-served arithmetic (bulk + ≤1 refill) — implementation-specific by design (executable contract). |
| `assert_read_into_contract` | 425-490 | REAL | — | Borrow twin incl. zero-length success and read/read_into byte agreement. |
| `assert_peek_into_contract` | 496-535 | REAL | — | No-advance/no-I/O/window-bound; `expected[at..at+got]` could panic only on caller misuse (test-only). |
| 7 unit tests | 546-650 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `synthetic_stream_passes_its_own_contract` (547) | YES | Full contract over 5000 bytes, capacity 64 — any reader regression (position drift, EOF, seek, big-read) trips an assert. EXECUTED green (input lib 33/33, orchrun log). |
| `synthetic_stream_passes_the_read_into_contract` (557) | YES | Same for the borrow path incl. API-agreement pass. EXECUTED green. |
| `synthetic_stream_passes_the_peek_into_contract` (565) | YES | Peek window honesty. EXECUTED green. |
| `bounded_read_proof_on_synthetic_source` (572) | YES | Two bounded reads + EOF tail; breaks on refill-loop or capacity growth regressions. EXECUTED green. |
| `shrink_mid_read_is_a_typed_truncation_error` (587) | YES | Asserts typed kind, both lengths, and code. EXECUTED green. |
| `scripted_read_failure_propagates_typed` (602) | YES | Asserts `input.file.read-io` code. EXECUTED green. |
| `seek_within_buffer_serves_no_reads` (613) | YES | reads_served arithmetic pins buffer-hit vs refill. EXECUTED green. |
| tests/truncation.rs (6 tests, NOT in this file) | UNKNOWN | Never executed in any committed baseline (F-002) — their pass/fail state is UNVERIFIED. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "release binaries carry none of it" | contract.rs:4-11 + Cargo.toml:25-29 | REAL | lib.rs:64-65 `#[cfg(any(test, feature = "contract"))]`, feature non-default; `cargo build --release --workspace` (baseline #1) cannot include it. |
| "Every access implementation — this crate, streaming (Phase 6), the conformance kit — runs the same executable contract from its own tests" | contract.rs:21-23 | **SHALLOW/FALSE for 2 of 3** | F-001: streaming has zero input::contract call sites; conformance kit is "future" per Cargo.toml:27-28. |
| "integration tests … enable the `contract` feature" (and truncation.rs: "CI's --all-features always does") | contract.rs:5-7; truncation.rs:22-24 | REAL-UNPROVEN | The feature wiring exists (truncation.rs `#![cfg(feature = "contract")]`), but no committed run shows those tests executing (F-002); ci.yml:234 does run `--all-features` — the claim is plausible, unproven. |
| `assert_bounded_read` semantics ("one full-size source call … plus at most one capacity-bounded read-ahead refill … buffer never grows") | contract.rs:41-48, 362-371 | REAL | Code 386-416 asserts exactly this; unit test executed green. |
| "the production helper that used to live here, `resolve_origin`, moved to `seekable.rs`" | contract.rs:10-11 | REAL | `use crate::seekable::{RawRead, Seekable, resolve_origin};` (line 64); definition in seekable.rs (audited separately this wave). |

## 7. Dead / duplicated / unreachable code
- Nothing dead in-file. But the `assert_*` surface is **duplicated in spirit**
  elsewhere rather than reused: demux's tests/fixtures_class_b.rs:41 defines
  its own local `assert_truncated`; media-probe has its own
  `media_probe::contract` module with `CountingStream` — the "same executable
  contract" the doc advertises is actually three hand-parallel implementations
  (F-001's flip side).
- The whole module is unreachable from shipped binaries by design (see Field
  row) — not a defect.

## 8. Dependencies & flags
- Deps: media-types, plugin-api (workspace), testkit dev-only — matches
  Cargo.toml; no unused imports in range.
- Feature: `contract = []` (Cargo.toml:30) — no default enable, no other
  crate in the workspace enables it as a normal dependency (rg over
  `features = [... "contract"` found none; it appears only in dev contexts).

## 9. Open questions for the next agent
- Did the original CI (GitHub) ever run green on
  `cargo test --workspace --all-features` with input included? No run artifact
  in `audit/runs/baseline/` answers it — see REQUEST-ORCH-RUN in my log.
- media-probe's `contract` module (probe_io.rs) is the same pattern — check
  whether its feature-gated suite also has zero committed evidence (probe_io
  ran 0 tests in the no-fail-fast baseline too — "test result: ok. 0 passed"
  under `deps/probe_io`).
