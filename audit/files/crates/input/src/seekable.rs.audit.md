# AUDIT: crates/input/src/seekable.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 126 / 5 production units (module doc, 2 traits, 1 fn) + 1 test |
| File verdict | REAL (the shipped open path's seek vocabulary; executed green in committed runs) |
| Reachable from shipped entry point? | YES, EXECUTED. Call chain: `lumen open <file>` → apps/cli/src/host.rs:904 `open_source` → apps/cli/src/demux_host.rs:385-392 `input::FileAccess::new().open(&uri)` → `FileStream` (input/src/file.rs:202) whose `SourceStream::seek` (file.rs:230-233) calls this file's `resolve_origin` (seekable.rs:86) and whose engine `BufferedReader<FileSource>` runs on this file's traits (`impl Seekable` file.rs:71, `impl RawRead` file.rs:101). Executed evidence: rc-smoke 6/6 (baseline INDEX.md #20); MP4 smoke "Demux works … seekable" (INDEX.md §MP4 SMOKE); input lib 33/33 green incl. `seekable::tests::resolve_origin_math_is_checked` (orchrun 2603-2608, test at line 2633); bounded_reads 5/5 incl. `seek_positions_and_bounds_over_a_real_file` (2642-2650); file_stream 9/9 (2653-2663). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read via Read tool; `rg -n "seekable" crates/input/src/lib.rs` (not feature-gated — always compiled); `rg -n "Seekable\|RawRead\|resolve_origin" crates apps --glob '*.rs'` (user map: file.rs + contract.rs impls; false-positive `ErrorCode::NotSeekable` items excluded by reading each hit); `sed -n '378,440p' apps/cli/src/demux_host.rs` (open wiring), `sed -n '60,140p;200,260p' crates/input/src/file.rs` (impls honor the doc's seek semantics); `sed -n '248,268p' crates/streaming/src/ranged.rs` + `rg -n "before-start|overflow" crates/streaming/src/ranged.rs crates/input/src/error.rs` (duplicate copy + duplicated code literals); `rg -n "enum SeekOrigin" -A5 crates/plugin-api/src/access.rs` (delta is i64 → i128 math provably overflow-free); `sed -n '120,175p' crates/input/tests/bounded_reads.rs` (executed static-length seek-bounds test); `sed -n '2653,2674p'` orchrun (file_stream 9/9, truncation 0 tests); `rg -n "fn length" -A3` over the three impls (no `None` implementation exists) |

## 1. One-paragraph verdict
This is the seek vocabulary the shipped binary actually runs: 126 lines,
two traits, one checked helper, zero unsafe, zero panics, and semantics
that match its documentation and are pinned by tests that executed
green in the no-fail-fast baseline — including the exact headline
contract (seek-to-length is a clean EOF, one-past-length is the typed
`input.seek.out-of-range` with the cursor unmoved, asserted over a real
file in bounded_reads.rs:137-170). The `resolve_origin` math is
provably sound (`i128::from(u64) + i128::from(i64)` cannot overflow;
before-start and past-`u64::MAX` both typed; boundary case pinned in
the test). Three real defects, none fatal: the helper is copy-pasted
verbatim into streaming/ranged.rs with the two error-code strings
duplicated as raw literals (and a third variant in hls/media.rs with
its own codes) — three copies of the same checked math with no
cross-crate pin, though both crates already depend on plugin-api where
the math could live; the live-length dimension of the seek bound
(seeking against a *shrunk* file) has zero executed evidence (its only
test is feature-gated and compiled to 0 tests in every committed
baseline); and the `None`-length contract sentence describes a case no
implementation in the workspace can produce.

## 2. What actually works (verified)
— `resolve_origin` checked math — verified by trace + executed test:
Start passes through (u64 cannot be negative), Current computes in
i128 (max magnitude ≈ 2.8e19, no overflow possible), `target < 0` →
`input.seek.before-start`, `target > u64::MAX` → `input.seek.overflow`,
`u64::MAX` itself resolves. Test `resolve_origin_math_is_checked`
(seekable.rs:111-125) pins all five cases; ran green (orchrun 2633).
— Seek semantics doc vs code — verified: FileSource::seek (file.rs:89-98)
allows `to == length` (EOF position) and returns `seek_out_of_range()`
(code `input.seek.out-of-range`, error.rs:84-89) only for `to > len`;
executed over a real file: seek-to-500-of-500 → clean EOF read, seek
501 → typed error with `position() == 500` after (bounded_reads.rs:
162-169, green, orchrun 2647).
— RawRead contract vs code — verified: `Ok(0)` only at end (std file
semantics; executed by `clean_eof_at_declared_length` green,
buffered.rs:654 + `empty_file_is_a_clean_immediate_eof` green,
file_stream.rs:145); `Err` leaves position unchanged (file.rs:101-107
updates `self.pos` only after a successful read; executed by the
failed-seek-never-moves assert).
— The trait shape is honestly minimal — no `Send` supertrait exactly as
the doc says (concrete sources carry it; `FileSource` is Send by
`std::fs::File`), no OS types in signatures (ARCH-6).
— Doc claim "used by the file access stack and the synthetic
conformance source alike" — verified: file.rs:231 and contract.rs:216
both call it.

## 3. Findings

### F-001 [S3]
Where: crates/input/src/seekable.rs:86-100 (duplicated at
crates/streaming/src/ranged.rs:253-268; third variant at
crates/streaming/src/hls/media.rs:366-396)
Code:
```rust
pub(crate) fn resolve_origin(from: u64, origin: SeekOrigin) -> Result<u64, EngineError> {
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
            let target = i128::from(from) + i128::from(delta);
            if target < 0 {
                Err(crate::error::seek_before_start())
```
(the copy:)
```rust
fn resolve_origin(from: u64, origin: SeekOrigin) -> Result<u64, EngineError> {
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
            let target = i128::from(from) + i128::from(delta);
            if target < 0 {
                Err(seek_before_start())
```
What is wrong: the checked seek math exists three times. The
streaming/ranged.rs copy is line-for-line identical (only the error
constructor paths differ), and it re-declares the stable codes as raw
string literals (`"input.seek.before-start"` ranged.rs:156,
`"input.seek.overflow"` ranged.rs:166) instead of sharing input's
constructors — so the "stable code" contract is pinned only by two
independent literal tables with no cross-crate test. The hls/media.rs
variant reimplements the same math with deliberately different codes
(`streaming.hls.seek-before-start` / `seek-overflow`, media.rs:385-392).
The duplication is *acknowledged* in the copy's own doc comment
("(the input contract's shared math; the codes are its stable ones)",
ranged.rs:252-253), and the boundaries rule forbids
streaming→input in production (boundaries.toml:65-67, "production
streaming never depends on input") — but both crates depend on
`plugin-api` (input/Cargo.toml:34, streaming/Cargo.toml:35), where
`SeekOrigin` itself lives (access.rs:89-94). The math could sit next to
the type it resolves; nothing architectural forces three copies.
Why it matters: this is the seek-boundary arithmetic of the shipped
player — a future fix to one copy (e.g. a clamp change, a new origin
variant) that misses the others produces per-lane seek drift that no
test catches, because the codes are stable strings duplicated as
literals rather than shared.
What correct looks like: move `resolve_origin` (and the two error
constructors, or at least their code constants) into `plugin-api`
beside `SeekOrigin`, take the error-minting as a parameter or return a
typed enum the caller maps; both crates call it.
Confidence: HIGH — Verification: read + diff of both bodies + error
literal grep in both crates.

### F-002 [S4]
Where: crates/input/src/seekable.rs:16-19 (contract) vs
crates/input/src/file.rs:89-98 (implementation) vs
crates/input/tests/truncation.rs:194 (the only live-length test)
Code:
```rust
//! - Seeking **beyond** the current length is a typed error
//!   (`input.seek.out-of-range`): a demuxer that asks for a byte that
//!   does not exist is a caller bug, reported truthfully (MP §8), not a
```
What is wrong: the contract is executed-verified only for a *static*
length (bounded_reads.rs:137-170, green). The live dimension —
FileSource::length() re-stats on every call (file.rs:79-87) and seek
checks against that fresh stat — has exactly one behavioral test,
`seek_beyond_the_shrunk_length_is_typed_out_of_range`
(truncation.rs:194-200), and it is feature-gated: truncation.rs
compiled to **0 tests** in the no-fail-fast baseline (orchrun
2668-2672: "running 0 tests"). So the "current length" half of the
headline claim has never executed anywhere. (Corroborates and is
covered by the previous session's REQUEST-ORCH-RUN:
`cargo test -p input --features contract --release --locked
--no-fail-fast`.)
Why it matters: the distinction between open-time length and live
length is the whole point of this contract sentence; an unexecuted
branch of the shipped seek path is exactly what the fail-fast baseline
hides (the W02 repair run proved 6 of 7 failures were hidden that way).
What correct looks like: run the contract-feature suite in CI (the
pending orchestrator run), or move an un-gated live-shrink seek test
into bounded_reads.rs.
Confidence: HIGH — Verification: read + baseline run line numbers;
the behavior itself is trace-verified (file.rs seek checks
`self.length()` which re-stats).
Note (file.rs's owner, not this file): the stat-then-lseek sequence is
also a TOCTOU window — a concurrent truncate between `length()` and
`file.seek` lets the cursor land past EOF with no typed error (std
allows sparse seeks). Unreachable in the CLI's single-owner usage, but
worth a line in the file.rs/buffered.rs audit.

### F-003 [S4]
Where: crates/input/src/seekable.rs:20-21 (and 41-44)
Code:
```rust
//! - Sources whose length is unknown (`None`) accept any target; the
//!   underlying store is the judge.
```
What is wrong: no implementation of `Seekable` in the workspace ever
returns `None` from `length()` — FileSource: live stat with
`fallback_len: Some` (file.rs:76-87), ScriptSource: `Some(visible_len)`
(contract.rs:129-131), the test double: `Some` (buffered.rs:605-607).
The streaming crate does not implement this trait at all. The
`None`-length semantics (accept any target, store judges) are
contract-only vocabulary with zero implementations and zero tests —
the `Option` typing is honest MP §8 design, but the documented None
behavior cannot be exercised by anything that exists.
Why it matters: a future implementor (e.g. a pipe or live stream
adapter) will read this sentence as a tested contract; it is not.
What correct looks like: either a test in this file with a `None`
double asserting `seek(any)` is delegated (one mock, ten lines), or a
doc marker that no None-source exists yet.
Confidence: HIGH — Verification: read + grep of every `fn length`
implementation in the crate.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-29 | REAL | S4 | Seek semantics verified (§2); None-length sentence is dead vocabulary (F-003); threading claims match code (no Send supertrait). |
| `Seekable` trait | 37-54 | REAL, EXECUTED | — | Position/length/seek vocabulary; on the shipped open path via FileSource (file.rs:71). |
| `RawRead` trait | 65-72 | REAL, EXECUTED | — | Ok(0)-only-at-end + Err-keeps-position contracts honored by FileSource (file.rs:101-110) and pinned by executed tests. |
| `resolve_origin` | 86-100 | REAL, EXECUTED | S3 | i128 math provably overflow-free; before-start/overflow typed; F-001 triplication. Runs on every file seek (file.rs:231). |
| `tests::resolve_origin_math_is_checked` | 111-125 | REAL, EXECUTED | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `resolve_origin_math_is_checked` (111) | YES | Pins: Start(3)→3; Current(+5)→15; Current(−10)→0 (exact zero, not error); Current(−1) at 0 → `input.seek.before-start`; Current(+2) at u64::MAX → `input.seek.overflow`; Current(+1) at u64::MAX−1 → u64::MAX (boundary legal). Mutation check: wrap the math in u64 → the overflow case returns a wrong Ok or panics → both asserts fail; drop the `< 0` check → before-start assert fails. EXECUTED green (orchrun 2633). |
| (adjacent, not this file) `seek_positions_and_bounds_over_a_real_file` (bounded_reads.rs:137) | YES | Pins the trait's doc semantics over a real file: window-hit/backward/forward seeks, seek-to-length clean EOF, past-length typed error with cursor unmoved, legal seek afterwards. EXECUTED green (orchrun 2647). |
| Not covered anywhere (executed): seek vs a live-shrunk length (F-002); a `None`-length source (F-003). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Seeking to exactly length() is legal — the EOF position; the next read returns a clean end-of-stream" | module doc 14-15 | REAL, EXECUTED | FileSource::seek allows `to == len` (file.rs:89-98); bounded_reads.rs:162-164 green. |
| "Seeking beyond the current length is a typed error (input.seek.out-of-range)" | module doc 16-19 | REAL, EXECUTED for static lengths; UNEXECUTED for live lengths | bounded_reads.rs:166-169 green; the live-shrink test never ran (F-002, truncation.rs 0 tests in baseline). |
| "Sources whose length is unknown (None) accept any target" | module doc 20-21 | REAL as contract, UNIMPLEMENTED | No `None` impl exists (F-003). |
| "Ok(0) only at the underlying end of stream" (RawRead) | module doc 9-10, 63 | REAL, EXECUTED | std file semantics + clean-EOF tests green (buffered.rs:654, file_stream.rs:145). |
| "Err: a typed error; the position is unchanged" | RawRead doc 64 | REAL, EXECUTED | file.rs:101-107 (pos updated only on success); failed-seek-never-moves assert green. |
| "resolve_origin … used by the file access stack and the synthetic conformance source alike" | fn doc 77-80 | REAL | file.rs:231, contract.rs:216. |
| "audit F-2 moved it here from the test-side contract module" | fn doc 78-80 | REAL | contract.rs:11 repeats the story; the move's purpose (feature-gate contract without touching production) matches lib.rs:64-65 gating. |

## 7. Dead / duplicated / unreachable code
- F-001: `resolve_origin` exists 3× (input, streaming/ranged verbatim,
  hls/media method variant). The ranged.rs copy's error-code literals
  are a second, un-pinned copy of input's stable codes.
- The `None` arm of `length()` is vocabulary without an implementation
  (F-003) — not dead code (the trait must allow it), but unexercised.
- Nothing unreachable: every item is on the shipped open path or its
  test suite.

## 8. Dependencies & flags
- Deps: `media_types::EngineError`, `plugin_api::SeekOrigin` — both
  declared (input/Cargo.toml:33-34), both used. Module NOT
  feature-gated (`pub mod seekable;` lib.rs:68) — always in release
  builds, unlike contract.rs. No unsafe, no allocation, no I/O —
  pure trait definitions + arithmetic.

## 9. Open questions for the next agent
- F-001's shared-math move touches plugin-api (owner: whoever holds
  plugin-api/src/access.rs) and both streaming call sites — the
  hls/media.rs lane uses intentionally different codes, so a shared
  helper needs a code-parameterization story, not a naive lift.
- The TOCTOU note in F-002 (stat→lseek race) belongs to the file.rs
  audit (not assigned to me); flagging so it is not lost.
- The pending REQUEST-ORCH-RUN from my contract.rs audit (cargo test
  -p input --features contract) would close F-002's execution gap —
  same run, no new request needed.
