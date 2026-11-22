# AUDIT: crates/input/src/contract.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 652 / 14 production items (1 fault-injecting source, 1 stream wrapper, 5 `#[track_caller]` contract asserts) + 7 `#[test]` fns |
| File verdict | REAL (as this crate's unit-test infrastructure) — the *cross-crate reuse story* in its docs is PAPER-ONLY |
| Reachable from shipped entry point? | **NO — by design and honestly so**: `crates/input/src/lib.rs:64-65` gates `pub mod contract` behind `#[cfg(any(test, feature = "contract"))]`; the `contract` Cargo feature (Cargo.toml:25-30) is enabled by **nobody** (repo-wide Cargo.toml grep: zero `features = [... "contract" ...]`), so the module compiles only for the input crate's own unit tests. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | rg for `contract` feature enablement across all Cargo.tomls; rg for `SyntheticStream`/`assert_stream_contract` consumers (hits outside input are comments only); sed quote verification |

## 1. One-paragraph verdict

A well-built executable-contract module: `ScriptSource` (an in-memory byte source with scriptable truncation and read faults plus read instrumentation) behind the real `BufferedReader`/`SourceStream` shape, and five `#[track_caller]` assertion suites that pin the access API's semantics — varied-chunk reconstruction, position tracking, sticky clean EOF, seek round-trips, typed past-end/before-start seek errors, the bounded-memory proof (one full-size bulk read + at most one refill, no speculative over-read, buffer never grows), the `read_into` borrow twin, and window-bound peek honesty. The 7 in-file tests are genuinely behavioral. Two problems: the module's raison d'être — "every access implementation — this crate, `streaming` (Phase 6), the conformance kit (doc 06 §7.2) — runs the same executable contract from its own tests" — is false today (no other crate compiles this module; the `contract` feature has zero enablers; the only matches outside `input` are comments), and `ScriptSource::read` contains a silently-skipped-copy guard that would return `Ok(n)` with an untouched buffer if its invariant ever broke.

## 2. What actually works (verified)

- The cfg gate is real (lib.rs:64-65, Cargo.toml:25-30): release binaries carry none of this module. The doc's "Test/conformance-only surface (audit F-2)" self-label matches the code.
- The contracts are assert-dense and behavioral: e.g. `assert_bounded_read` (373-417) pins an exact read-count (`served_before + if data.len() == want { 2 } else { 1 }`) — a real I/O-shape oracle, not a tautology.
- W00 baseline cargo-test.log shows the input crate suite green in the workspace run (UNVERIFIED by my own execution — ADAPTATION).

## 3. Findings

### F-001 [S3] — The advertised reuse of the executable contract by other access implementations is paper: zero external consumers, the `contract` feature is never enabled, the "conformance kit" does not run it

Where: crates/input/src/contract.rs:21-23; the enablement reality at crates/input/Cargo.toml:25-30.

Code (21-23, verbatim):
```rust
//! Every access implementation — this crate, `streaming` (Phase 6), the
//! conformance kit (doc 06 §7.2) — runs the same executable contract
//! from its own tests:
```

Code (Cargo.toml:27-30, verbatim):
```toml
# it via cfg(test); this feature is how integration tests, the future doc 06
# §7.2 conformance kit, and any downstream test harness opt in. Release
# binaries (no feature) carry zero of its panicky conformance vocabulary.
contract = []
```

What is wrong: repo-wide grep over every Cargo.toml shows nothing enables `input/contract`. The only code that ever compiles this module is the input crate's own `cfg(test)` unit suite (the 7 in-file tests). `crates/streaming` does not use it; `crates/demux` ships its own separate in-memory stream (`crates/demux/src/avi/test_support.rs` `BytesStream` — a hand-rolled near-duplicate, not this contract); `crates/playback-engine/tests/scenarios_b/host.rs:49` mentions `input::ScriptSource`/`SyntheticStream` in a **comment** while (per that file's own imports) using its own fixtures. Doc 06 §7.2's conformance kit is itself a doc section about plugins, not a runnable kit that opts in.

Why it matters: the module's whole value proposition — one executable contract shared by every access implementation — currently covers exactly one implementation (the one it lives in). A second access implementation could violate the contract silently because nothing compiles the checks against it. The phrasing "runs the same executable contract from its own tests" is present-tense false.

What correct looks like: `crates/streaming/Cargo.toml` dev-depends `input` with `features = ["contract"]` and its HTTP/ranged tests call `assert_stream_contract`; or the doc says "future" instead of present tense.

Confidence: HIGH — Verification: read + exhaustive grep (feature enablement, symbol consumers).

### F-002 [S4] — `ScriptSource::read`'s bounds guard silently skips the copy and still returns `Ok(n)`

Where: crates/input/src/contract.rs:149-157.

Code (149-155, verbatim):
```rust
        let available = self.visible_len.saturating_sub(self.pos);
        let n = (out.len() as u64).min(available) as usize;
        let start = self.pos as usize;
        let end = start + n;
        if end <= self.data.len() {
            out[..n].copy_from_slice(&self.data[start..end]);
        }
```

What is wrong: if `end > data.len()` the copy is skipped but the function still advances `pos` and returns `Ok(n)` — the caller receives n bytes of whatever was previously in `out` (or zeros). I verified the invariant that makes this unreachable today (`pos ≤ visible_len ≤ data.len()` always, since `shrink_to` only lowers `visible_len` and `seek` refuses targets beyond it), so this is a latent trap, not a live bug — but the guard converts a would-be logic error into silent garbage, the exact "swallowed error" pattern §3.2 hunts, in a module whose job is to be the honesty oracle.

Why it matters: an oracle that can lie silently under a future invariant break undermines every test built on it.

What correct looks like: `unreachable!` or a typed error in the else branch (this is test-only code; panicking loudly is the sanctioned vocabulary per the module docs, lines 52-55).

Confidence: HIGH — Verification: read + invariant derivation.

### F-003 [S4] — Near-duplicate in-memory stream types across crates

Where: crates/input/src/contract.rs:69-224 (`ScriptSource`/`SyntheticStream`) vs crates/demux/src/avi/test_support.rs (`BytesStream`, "the `input` crate's `SyntheticStream` precedent").

What is wrong: two hand-rolled in-memory `SourceStream` implementations with the same positional semantics (clean EOF, typed out-of-range seeks, position unchanged on failure) exist in two crates, and the second one exists *because* the first is cfg-gated away from other crates' tests (test_support.rs:4-7 says so). The duplication is the direct cost of F-001's unshared contract.

Confidence: HIGH — Verification: read of both files.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-55 | REAL / claims PAPER | S3 | F-001 reuse story |
| `ScriptSource` (struct) | 69-81 | REAL-UNPROVEN | — | test fixture with instrumentation |
| `ScriptSource::new` | 85-94 | REAL-UNPROVEN | — | copies data |
| `shrink_to` | 100-102 | REAL-UNPROVEN | — | monotone min |
| `fail_next_read` | 105-110 | REAL-UNPROVEN | — | one-shot fault |
| `max_read_seen`/`reads_served` | 113-121 | REAL-UNPROVEN | — | instrumentation accessors |
| `Seekable for ScriptSource` | 124-140 | REAL-UNPROVEN | — | typed out-of-range; position unchanged on failure |
| `RawRead for ScriptSource::read` | 143-158 | REAL-UNPROVEN | S4 | F-002 silent skip-copy guard |
| `SyntheticStream` + `new`/`source`/`shrink_to` | 163-190 | REAL-UNPROVEN | — | BufferedReader wrap; synthetic URI literal labeled |
| `SourceStream for SyntheticStream` | 192-224 | REAL-UNPROVEN | — | delegates; `supports_seek() = true` |
| `peek_into`/`read_exact_into` | 231-239 | REAL-UNPROVEN | — | window math delegates |
| `assert_truncated` | 244-251 | REAL-UNPROVEN | — | typed matcher, `unreachable!` on wrong shape |
| `assert_stream_contract` | 257-360 | REAL-UNPROVEN | — | 25 assertions incl. sticky EOF, seek error codes, cursor-unchanged-on-failure |
| `assert_bounded_read` | 373-417 | REAL-UNPROVEN | — | exact read-count oracle; capacity never grows; no over-read |
| `assert_read_into_contract` | 426-490 | REAL-UNPROVEN | — | borrow twin + API-agreement pass |
| `assert_peek_into_contract` | 496-535 | REAL-UNPROVEN | — | no-advance, no-I/O, window-bound |
| tests (7) | 546-651 | REAL-UNPROVEN | — | §5 |

## 5. Test honesty (7 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `synthetic_stream_passes_its_own_contract` (547-551) | YES | runs the 25-assertion suite |
| `synthetic_stream_passes_the_read_into_contract` (557-561) | YES | borrow-twin suite |
| `synthetic_stream_passes_the_peek_into_contract` (565-569) | YES | peek suite incl. I/O-count oracle |
| `bounded_read_proof_on_synthetic_source` (572-581) | YES | exact served-read counts, tail EOF |
| `shrink_mid_read_is_a_typed_truncation_error` (587-598) | YES | (expected, found) = (1000, 400), error code pinned |
| `scripted_read_failure_propagates_typed` (602-608) | YES | error code pinned |
| `seek_within_buffer_serves_no_reads` (613-650) | YES | read-count oracle for buffer-hit vs refill seeks |

Caveat: these tests exercise the synthetic source + BufferedReader — the real `FileAccess` path's bounded behavior is covered by `crates/input/tests/bounded_reads.rs` (my file #17, separate audit). The contracts themselves have never been run against any other access implementation (F-001).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every access implementation — this crate, `streaming` (Phase 6), the conformance kit (doc 06 §7.2) — runs the same executable contract" | contract.rs:21-23 | FALSE (present tense) | only this crate compiles the module; feature never enabled (F-001) |
| "release binaries carry none of it" | contract.rs:4-8 | REAL | cfg gate verified at lib.rs:64-65 |
| "integration tests … enable the `contract` feature" | contract.rs:5-6 / Cargo.toml:27 | FALSE | input's own integration tests (tests/bounded_reads.rs) use the public API, not the contract; no Cargo.toml enables it |
| Card 2.A1 "no unbounded buffering" AC | module docs 41-48 | REAL-UNPROVEN | assert_bounded_read is a genuine proof over the synthetic source; runs only in unit tests |

## 7. Dead / duplicated / unreachable code

- The `contract = []` Cargo feature: declared, documented, never enabled — a dead feature flag whose only documented users do not exist (F-001).
- `assert_*` functions are unreachable outside `cfg(test)` by design — not a defect, but their only current callers are the 7 in-file tests.

## 8. Dependencies & flags

Depends on `media-types` (SourceUri, EngineError, CorruptDetail), `plugin-api` (SourceStream/SeekOrigin/SourcePosition). The `contract` feature is additive-empty (`contract = []`) — it exists purely as a compile gate. No workspace member or workflow enables it (checked all Cargo.tomls and the release workflow's feature set is wave-1/17 territory).

## 9. Open questions for the next agent

- Does `crates/streaming`'s HTTP ranged reader have its own contract tests, and do they duplicate these assertions? (W10.)
- Doc 06 §7.2 conformance kit — does any code exist for it at all (plugin-host wave 12)? If not, the reference is PAPER-ONLY twice over.
- `BufferedReader` (crates/input/src/buffered.rs) is the actual implementation under test — another agent's file; my bounded_reads.rs audit covers its file-path behavior.
