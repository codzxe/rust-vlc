# AUDIT: crates/input/src/seekable.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 126 / 2 traits (3 methods) + 1 fn + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `resolve_origin` is called from crates/input/src/file.rs:231 (`SourceStream::seek` on the production file access) and contract.rs:216 (test-only); file access is on the `lumen` open path (demux_host). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `cat` (exit 0); `rg resolve_origin` over crates/input/src. No cargo (adaptation). |

## 1. One-paragraph verdict

A small, clean trait module: `Seekable` (position/Option-length/typed seek), `RawRead` (chunked reads, `Ok(0)` = end-for-now), and one shared helper `resolve_origin` whose i128 checked math is correct at both boundaries (negative target → `input.seek.before-start`; > u64::MAX → `input.seek.overflow`; u64::MAX itself resolves). The single test pins all four cases including the exact-boundary resolution. The only observation worth recording is that the module's seek semantics (beyond-length = typed error) are a documentation contract that implementations — not the trait — must enforce.

## 2. What actually works (verified)

- `resolve_origin` (98-112 region): `i128::from(from) + i128::from(delta)` cannot overflow; the `< 0` and `> u64::MAX` arms are exact; `Start(to)` passes through (any u64 is a legal absolute target; range policing is the trait implementor's, matching the documented semantics). Hand-verified against the test's four cases.
- Trait docs state the value/absence split (`Option<u64>` length) and the `Ok(0)`-is-not-error convention — consistent with `RawRead`'s contract text.

## 3. Findings

### F-001 [S4] Beyond-length seek policing is documented here but enforced nowhere in this file
Where: crates/input/src/seekable.rs:16-20 (doc) vs the trait at 44-67 (no enforcement possible)
Code (16-20, verbatim):
```rust
//! - Seeking **beyond** the current length is a typed error
//!   (`input.seek.out-of-range`): a demuxer that asks for a byte that
//!   does not exist is a caller bug, reported truthfully (MP §8), not a
//!   POSIX-style silent past-end cursor.
```
What is wrong: nothing in this file — a trait cannot enforce it. The claim's truth lives in file.rs/buffered.rs (other agents). Recorded so those audits check `input.seek.out-of-range` actually fires on past-end seeks (the doc promises it; a silent POSIX cursor would contradict the module).
Confidence: HIGH that the enforcement is not in this file — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Seekable::position`/`length`/`seek` (trait) | 39-67 | REAL | — | pure declarations; semantics in docs |
| `RawRead::read` (trait) | 70-93 | REAL | — | contract: short reads are data; Err leaves position unchanged (implementor's duty) |
| `resolve_origin` | 98-112 | REAL | — | i128 checked math, exact boundaries; production caller file.rs:231 |
| test `resolve_origin_math_is_checked` | 117-126 region | REAL | — | pins both error codes + the u64::MAX boundary resolution |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| resolve_origin_math_is_checked | YES (reasoned) | exact values for Start/Current, both typed error codes asserted by `context.code`, and the u64::MAX-1 +1 boundary — a wrapping or off-by-one implementation fails |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Seek-to-exact-length legal, beyond-length typed error | seekable.rs:13-20 | UNVERIFIED (implementations) | flagged F-001 for file.rs/buffered.rs agents |
| `resolve_origin` rejects before-start/overflow typed | seekable.rs:89-96 | REAL | code + boundary test read-verified |

## 7. Dead / duplicated / unreachable code

None. `resolve_origin` is `pub(crate)` with exactly two callers (file.rs production, contract.rs test).

## 8. Dependencies & flags

media-types (EngineError) + plugin-api (SeekOrigin). No features, no unsafe. Std-only.

## 9. Open questions for the next agent

- Does file.rs's `seek` actually return `input.seek.out-of-range` for targets > length (the doc contract)? And does `Err` leave the position unchanged as `RawRead` promises?
